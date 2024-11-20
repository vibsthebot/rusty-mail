use std::env;
use imap::Session;
use native_tls::{TlsConnector, TlsStream};
use std::net::TcpStream;
use base64::{engine::general_purpose, Engine as _};
use quoted_printable::{decode as qp_decode, ParseMode};
use mailparse::{parse_mail, MailHeaderMap};
use html2text::from_read;

/// Email client for Gmail IMAP access
pub struct Email {
    session: Session<TlsStream<TcpStream>>,
    uid_list: Vec<u32>,
}

impl Email {
    /// Create new Email client and connect to Gmail
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let username = env::var("GMAIL_USERNAME")?;
        let password = env::var("GMAIL_APP_PASSWORD")?;
        let domain = "imap.gmail.com";

        let tls = TlsConnector::builder().build()?;
        let client = imap::connect((domain, 993), domain, &tls)?;
        let mut session = client.login(&username, &password).map_err(|e| e.0)?;

        session.select("INBOX")?;

        // Search for all email UIDs
        let uids = session.uid_search("ALL")?;
        let mut uid_list: Vec<u32> = uids.iter().copied().collect();
        uid_list.sort_unstable_by(|a, b| b.cmp(a)); // Sort in reverse order

        Ok(Self {
            session,
            uid_list,
        })
    }

    /// Fetch email subjects for given page number
    pub fn fetch_subjects(&mut self, page: usize) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let start = page * 8;
        let end = (start + 8).min(self.uid_list.len());

        if start >= self.uid_list.len() {
            return Ok(Vec::new());
        }

        let mut subjects = Vec::new();
        for &uid in &self.uid_list[start..end] {
            if let Some(message) = self.session.uid_fetch(uid.to_string(), "RFC822.HEADER")?.iter().next() {
                if let Ok(headers) = std::str::from_utf8(message.header().ok_or_else(|| {
                    imap::error::Error::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Message did not have headers",
                    ))
                })?) {
                    // Extract the subject from the headers
                    for line in headers.lines() {
                        if line.to_lowercase().starts_with("subject:") {
                            let subject = line[8..].trim().to_string();
                            subjects.push(subject);
                            break;
                        }
                    }
                }
            }
        }
        Ok(subjects)
    }

    pub fn fetch_email(&mut self, index: usize) -> Result<String, Box<dyn std::error::Error>> {
        let uid = self.uid_list[index];
        let messages = self.session.uid_fetch(uid.to_string(), "RFC822")?;
        let message = messages.iter().next().ok_or("Message not found")?;
        let body = message.body().ok_or("Message did not have a body")?;

        // Parse the raw email
        let parsed_mail = parse_mail(body)?;

        // Extract and decode the email body
        let email_body = Self::get_decoded_body(&parsed_mail)?;

        Ok(email_body)
    }

    // Helper function to extract and decode the email body
    fn get_decoded_body(parsed: &mailparse::ParsedMail) -> Result<String, Box<dyn std::error::Error>> {
        if parsed.subparts.is_empty() {
            let cte = parsed.headers.get_first_value("Content-Transfer-Encoding").unwrap_or_default();
            let ct = parsed.headers.get_first_value("Content-Type").unwrap_or_default();
            let raw_body = parsed.get_body_raw()?;

            // Decode based on the encoding type
            let decoded_bytes = match cte.to_lowercase().as_str() {
                "base64" => general_purpose::STANDARD.decode(&raw_body)?,
                "quoted-printable" => qp_decode(&raw_body, ParseMode::Robust)?,
                _ => raw_body,
            };

            // Convert HTML to plain text if necessary
            let decoded_str = if ct.to_lowercase().contains("text/html") {
                let html_content = String::from_utf8_lossy(&decoded_bytes);
                // Convert HTML to plain text
                let plain_text = from_read(html_content.as_bytes(), 80)?;
                plain_text
            } else {
                // Treat as plain text
                String::from_utf8_lossy(&decoded_bytes).to_string()
            };

            Ok(decoded_str)
        } else {
            // Concatenate all subpart bodies
            let mut full_body = String::new();
            for subpart in &parsed.subparts {
                let part_body = Self::get_decoded_body(subpart)?;
                full_body.push_str(&part_body);
            }
            Ok(full_body)
        }
    }
}