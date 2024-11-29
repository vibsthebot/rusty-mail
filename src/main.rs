use std::{env, io::{self, Write, Read}};
mod email;
use email::Email;
mod config;
use config::Config;
use lettre::{Message, SmtpTransport, Transport};
use lettre::transport::smtp::authentication::Credentials;

fn send(message: String, subject: String, recipient_email: String) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let username = env::var("GMAIL_USERNAME").map_err(|e| {
        eprintln!("Failed to get SMTP_USERNAME: {:?}", e);
        e
    })?;
    let password = env::var("GMAIL_APP_PASSWORD").map_err(|e| {
        eprintln!("Failed to get SMTP_PASSWORD: {:?}", e);
        e
    })?;
    
    let email = Message::builder()
        .from(format!("Vibhu Siddha <{}>", username).parse()?)
        .reply_to(username.parse()?)
        .to(recipient_email.parse()?)
        .subject(subject)
        .body(message)?;

    let creds = Credentials::new(username, password);

    let mailer = SmtpTransport::starttls_relay("smtp.gmail.com").map_err(|e| {
        eprintln!("Failed to create SMTP transport: {:?}", e);
        e
    })?
    .credentials(creds)
    .build();

    match mailer.send(&email) {
        Ok(_) => println!("Email sent successfully!"),
        Err(e) => eprintln!("Could not send email: {:?}", e),
    }

    Ok(())
}

// Update login function to save credentials
fn login() -> Result<(), Box<dyn std::error::Error>> {
    print!("Gmail username: ");
    io::stdout().flush()?;

    let mut username = String::new();
    io::stdin().read_line(&mut username)?;
    let username = username.trim().to_string();

    print!("Gmail app password: ");
    io::stdout().flush()?;

    let mut password = String::new();
    io::stdin().read_line(&mut password)?;
    let password = password.trim().to_string();

    // Validate credentials by creating an Email client
    match Email::new() {
        Ok(_) => {
            // Save credentials after successful login
            let config = Config { username: username.clone(), password: password.clone() };
            config.save()?;

            // Set environment variables
            env::set_var("GMAIL_USERNAME", &username);
            env::set_var("GMAIL_APP_PASSWORD", &password);

            println!("Login successful!");
            Ok(())
        }
        Err(e) => {
            eprintln!("Login failed: {}", e);
            Err(e)
        }
    }
}

fn read_emails() -> Result<(), Box<dyn std::error::Error>> {
    let mut email_client = Email::new()?;

    let mut input = String::new();
    let mut current_page = 0;

    loop {
        std::process::Command::new("clear").status().unwrap();

        let subjects = email_client.fetch_subjects(current_page)?;
        if subjects.is_empty() {
            println!("No more emails found.");
            if current_page > 0 {
                current_page -= 1;
            }
        } else {
            println!("Page {}", current_page + 1);
            //println!("------------------");
            for (i, subject) in subjects.iter().enumerate() {
                println!("{} - {}", i + 1 + current_page * 8, subject);
            }
        }

        // Existing code to get input
        print!("Enter command: ");
        io::stdout().flush()?;
        input.clear();
        io::stdin().read_line(&mut input)?;
        let input_trimmed = input.trim().to_lowercase();

        // Split the input into command and arguments
        let mut parts = input_trimmed.split_whitespace();
        let command = parts.next().unwrap_or("");
        let args: Vec<&str> = parts.collect();

        match command {
            "back" | "b" => {
                if current_page > 0 {
                    current_page -= 1;
                }
            }
            "next" | "n" => {
                current_page += 1;
            }
            "help" | "h" | "" => {
                println!("\nAvailable commands:");
                println!("  next (n)         - Show next page of emails");
                println!("  back (b)         - Show previous page of emails");
                println!("  exit (e)         - Return to main menu");
                println!("  help (h)         - Show this help");
                println!("  fetch <number>   - Fetch and display the email with the given number");
                println!("  <number>         - Go to specific page\n");
                continue;
            }
            "fetch" | "f" => {
                if args.len() < 1 {
                    println!("Usage: fetch <email number>");
                    continue;
                }
                match args[0].parse::<usize>() {
                    Ok(num) => {
                        // Adjust for zero-based index if necessary
                        let index = num - 1;
                        match email_client.fetch_email(index) {
                            Ok(email_content) => {
                                println!("{}", email_content);
                            }
                            Err(e) => {
                                eprintln!("Error fetching email: {}", e);
                            }
                        }
                    }
                    Err(_) => {
                        println!("Please provide a valid email number.");
                        continue;
                    }
                }
                println!("Press Enter to continue...");
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
            }
            // Handle page number input
            number => {
                match number.parse::<usize>() {
                    Ok(page) if page > 0 => {
                        current_page = page - 1;
                    }
                    _ => {
                        println!("Unknown command. Type 'help' or 'h' for a list of commands.");
                    }
                }
            }
        }
    }

    Ok(())
}

// Update main to load config on startup
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load saved credentials
    if let Ok(config) = Config::load() {
        env::set_var("GMAIL_USERNAME", &config.username);
        env::set_var("GMAIL_APP_PASSWORD", &config.password);
    } else {
        println!("No saved credentials found. Please use the 'login' command.");
    }

    let mut input = String::new();

    // Clear terminal on startup
    std::process::Command::new("clear").status().unwrap();

    loop {
        // Get current directory and format prompt
        let current_dir = match env::current_dir() {
            Ok(path) => path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown")
                .to_string(), // Convert to owned String
            Err(_) => "unknown".to_string(),
        };

        print!(".../{current_dir}> ");
        io::stdout().flush()?;

        input.clear();
        io::stdin().read_line(&mut input)?;
        let input_trimmed = input.trim().to_lowercase();

        match input_trimmed.as_str() {
            "quit" | "exit" | "q" => break,
            "help" => {
                println!("Available commands:");
                println!("  read  - Fetch and display email subjects");
                println!("  help  - Show this help message");
                println!("  quit  - Exit the program");
                println!("  login - Set Gmail credentials");
            }
            "read" | "r" => {
                read_emails()?;
            }
            "send" | "s" => {
                // Prompt for recipient email
                print!("Enter recipient email: ");
                io::stdout().flush()?;
                let mut recipient_email = String::new();
                io::stdin().read_line(&mut recipient_email)?;
                let recipient_email = recipient_email.trim().to_string();

                // Prompt for subject
                print!("Enter subject: ");
                io::stdout().flush()?;
                let mut subject = String::new();
                io::stdin().read_line(&mut subject)?;
                let subject = subject.trim().to_string();

                // Prompt for message
                println!("Enter your message (press CTRL+D to finish):");
                let mut message = String::new();
                io::stdin().read_to_string(&mut message)?;

                // Now you can use recipient_email, subject, and message
                println!("\n\nRecipient: {}", recipient_email);
                println!("Subject: {}", subject);
                println!("Message: {}", message);

                send(message, subject, recipient_email)?;
            }
            "login" => {
                if let Err(e) = login() {
                    eprintln!("Error: {}", e);
                }
            }
            "" => continue,
            _ => println!("Unknown command. Type 'help' for available commands."),
        }
    }

    println!("Goodbye!");
    Ok(())
}
