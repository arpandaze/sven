mod crypto;
mod daemon;
mod db;
mod error;

use anyhow::Result;
use clap::{Parser, Subcommand};
use daemon::{Daemon, DaemonClient};
use db::Database;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Add {
        key: String,
        value: String,
    },
    Remove {
        key: String,
    },
    List,
    Export {
        #[arg(short, long, default_value = "fish")]
        shell: String,
    },
    Unlock,
    Status,
    Stop,
}

fn print_line(line: &str) -> std::io::Result<()> {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "{}", line)
}

fn escape_value_for_shell(value: &str, shell: &str) -> String {
    match shell {
        "fish" => {
            // Fish escapes $ with \$
            value.replace('$', "\\$")
        }
        "bash" | "sh" | "zsh" => {
            // Bash/sh/zsh escapes $ with \$, ` with \`, " with \", and \ with \\
            value
                .replace('\\', "\\\\")
                .replace('$', "\\$")
                .replace('`', "\\`")
                .replace('"', "\\\"")
        }
        "csh" | "tcsh" => {
            // csh/tcsh escapes $ with \$, ! with \!, and " with \"
            value
                .replace('$', "\\$")
                .replace('!', "\\!")
                .replace('"', "\\\"")
        }
        _ => {
            // Default to bash-style escaping
            value
                .replace('\\', "\\\\")
                .replace('$', "\\$")
                .replace('`', "\\`")
                .replace('"', "\\\"")
        }
    }
}

fn format_export(key: &str, value: &str, shell: &str) -> String {
    let escaped_value = escape_value_for_shell(value, shell);
    match shell {
        "fish" => format!("set -gx {} \"{}\"", key, escaped_value),
        "bash" | "sh" | "zsh" => format!("export {}=\"{}\"", key, escaped_value),
        "csh" | "tcsh" => format!("setenv {} \"{}\"", key, escaped_value),
        _ => format!("export {}=\"{}\"", key, escaped_value), // default to bash-style
    }
}

fn main() -> Result<()> {
    // Set up a panic hook that exits silently on broken pipe errors
    std::panic::set_hook(Box::new(|panic_info| {
        if let Some(message) = panic_info.payload().downcast_ref::<String>() {
            if !message.contains("Broken pipe") {
                eprintln!("{}", panic_info);
            }
        } else if let Some(message) = panic_info.payload().downcast_ref::<&str>() {
            if !message.contains("Broken pipe") {
                eprintln!("{}", panic_info);
            }
        } else {
            eprintln!("{}", panic_info);
        }
    }));

    let cli = Cli::parse();

    match cli.command {
        Commands::Unlock => {
            // Start the daemon
            if let Err(e) = Daemon::start_daemon() {
                eprintln!("Failed to start daemon: {}", e);
                std::process::exit(1);
            }

            // Check if the daemon is running
            let mut attempts = 0;
            let max_attempts = 5;

            while attempts < max_attempts {
                match DaemonClient::is_daemon_running() {
                    Ok(true) => {
                        if print_line("Daemon started successfully. Secrets are now unlocked and cached in memory.").is_err() {
                            std::process::exit(0);
                        }
                        return Ok(());
                    }
                    _ => {
                        attempts += 1;
                        if attempts < max_attempts {
                            std::thread::sleep(std::time::Duration::from_millis(1000));
                        }
                    }
                }
            }

            if print_line("Daemon started but may need a moment to initialize fully. Run 'sven status' to check.").is_err() {
                std::process::exit(0);
            }
        }
        Commands::Status => {
            // Check if daemon is running
            match DaemonClient::is_daemon_running() {
                Ok(true) => {
                    if print_line("Daemon is running. Secrets are unlocked and cached in memory.")
                        .is_err()
                    {
                        std::process::exit(0);
                    }
                }
                Ok(false) => {
                    if print_line("Daemon is not running. Secrets will be decrypted on demand.")
                        .is_err()
                    {
                        std::process::exit(0);
                    }
                }
                Err(e) => {
                    eprintln!("Error checking daemon status: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Stop => {
            // Stop the daemon
            match DaemonClient::is_daemon_running() {
                Ok(true) => {
                    let client = DaemonClient::new()?;
                    match client.shutdown_daemon() {
                        Ok(msg) => {
                            if print_line(&msg).is_err() {
                                std::process::exit(0);
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to stop daemon: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Ok(false) => {
                    if print_line("Daemon is not running.").is_err() {
                        std::process::exit(0);
                    }
                }
                Err(e) => {
                    eprintln!("Error checking daemon status: {}", e);
                    std::process::exit(1);
                }
            }
        }
        // For other commands, try to use the daemon if it's running
        _ => {
            let use_daemon = match DaemonClient::is_daemon_running() {
                Ok(running) => running,
                Err(_) => false,
            };

            if use_daemon {
                let client = DaemonClient::new()?;
                match cli.command {
                    Commands::Add { key, value } => match client.add_secret(&key, &value) {
                        Ok(msg) => {
                            if print_line(&msg).is_err() {
                                std::process::exit(0);
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to add secret: {}", e);
                            std::process::exit(1);
                        }
                    },
                    Commands::Remove { key } => match client.remove_secret(&key) {
                        Ok(msg) => {
                            if print_line(&msg).is_err() {
                                std::process::exit(0);
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to remove secret: {}", e);
                            std::process::exit(1);
                        }
                    },
                    Commands::List => match client.list_secrets() {
                        Ok(secrets) => {
                            if secrets.is_empty() {
                                if print_line("No secrets found").is_err() {
                                    std::process::exit(0);
                                }
                            } else {
                                if print_line("Secrets:").is_err() {
                                    std::process::exit(0);
                                }
                                for key in secrets {
                                    if print_line(&format!("  {}", key)).is_err() {
                                        std::process::exit(0);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to list secrets: {}", e);
                            std::process::exit(1);
                        }
                    },
                    Commands::Export { shell } => match client.get_secrets(&shell) {
                        Ok(secrets) => {
                            for (key, value) in secrets {
                                if print_line(&format_export(&key, &value, &shell)).is_err() {
                                    std::process::exit(0);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to export secrets: {}", e);
                            std::process::exit(1);
                        }
                    },
                    _ => unreachable!(),
                }
            } else {
                // Daemon is not running, use direct database access
                let mut db = Database::new()?;

                match cli.command {
                    Commands::Add { key, value } => {
                        db.add_secret(&key, &value)?;
                        if print_line(&format!("Added secret: {}", key)).is_err() {
                            std::process::exit(0);
                        }
                    }
                    Commands::Remove { key } => {
                        db.remove_secret(&key)?;
                        if print_line(&format!("Removed secret: {}", key)).is_err() {
                            std::process::exit(0);
                        }
                    }
                    Commands::List => {
                        let secrets = db.list_secrets()?;
                        if secrets.is_empty() {
                            if print_line("No secrets found").is_err() {
                                std::process::exit(0);
                            }
                        } else {
                            if print_line("Secrets:").is_err() {
                                std::process::exit(0);
                            }
                            for key in secrets {
                                if print_line(&format!("  {}", key)).is_err() {
                                    std::process::exit(0);
                                }
                            }
                        }
                    }
                    Commands::Export { shell } => {
                        let secrets = db.get_all_secrets()?;
                        for (key, value) in secrets {
                            if print_line(&format_export(&key, &value, &shell)).is_err() {
                                std::process::exit(0);
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    Ok(())
}
