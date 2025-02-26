mod crypto;
mod db;
mod error;

use anyhow::Result;
use clap::{Parser, Subcommand};
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
    }

    Ok(())
}
