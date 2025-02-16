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

fn format_export(key: &str, value: &str, shell: &str) -> String {
    match shell {
        "fish" => format!("set -gx {} \"{}\"", key, value),
        "bash" | "sh" | "zsh" => format!("export {}=\"{}\"", key, value),
        "csh" | "tcsh" => format!("setenv {} \"{}\"", key, value),
        _ => format!("export {}=\"{}\"", key, value), // default to bash-style
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut db = Database::new()?;

    match cli.command {
        Commands::Add { key, value } => {
            db.add_secret(&key, &value)?;
            println!("Added secret: {}", key);
        }
        Commands::Remove { key } => {
            db.remove_secret(&key)?;
            println!("Removed secret: {}", key);
        }
        Commands::List => {
            let secrets = db.list_secrets()?;
            if secrets.is_empty() {
                println!("No secrets found");
            } else {
                println!("Secrets:");
                for key in secrets {
                    println!("  {}", key);
                }
            }
        }
        Commands::Export { shell } => {
            let secrets = db.get_all_secrets()?;
            for (key, value) in secrets {
                println!("{}", format_export(&key, &value, &shell));
            }
        }
    }

    Ok(())
}
