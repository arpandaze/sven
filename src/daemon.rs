use crate::db::Database;
use crate::error::{Result, SvenError};
use daemonize::Daemonize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

// Commands that can be sent to the daemon
#[derive(Serialize, Deserialize, Debug)]
pub enum DaemonCommand {
    GetSecrets { shell: String },
    AddSecret { key: String, value: String },
    RemoveSecret { key: String },
    ListSecrets,
    Shutdown,
}

// Responses from the daemon
#[derive(Serialize, Deserialize, Debug)]
pub enum DaemonResponse {
    Secrets(Vec<(String, String)>),
    KeyList(Vec<String>),
    Success(String),
    Error(String),
}

pub struct Daemon;

impl Daemon {
    fn get_socket_path() -> Result<PathBuf> {
        dirs::runtime_dir()
            .or_else(|| Some(std::env::temp_dir()))
            .map(|mut p| {
                p.push("sven.sock");
                p
            })
            .ok_or_else(|| SvenError::ConfigError("Could not determine socket path".into()))
    }

    pub fn get_pid_file_path() -> Result<PathBuf> {
        dirs::runtime_dir()
            .or_else(|| Some(std::env::temp_dir()))
            .map(|mut p| {
                p.push("sven.pid");
                p
            })
            .ok_or_else(|| SvenError::ConfigError("Could not determine pid file path".into()))
    }

    // Start the daemon process
    pub fn start_daemon() -> Result<()> {
        // Check if daemon is already running
        if Self::is_daemon_running()? {
            return Err(SvenError::ConfigError("Daemon is already running".into()));
        }

        // Remove socket file if it exists
        let socket_path = Self::get_socket_path()?;
        if socket_path.exists() {
            std::fs::remove_file(&socket_path)?;
        }

        let pid_file_path = Self::get_pid_file_path()?;
        let stdout = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/sven-daemon.log")?;
        let stderr = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/sven-daemon.err")?;

        let daemonize = Daemonize::new()
            .pid_file(pid_file_path)
            .working_directory("/tmp")
            .stdout(stdout)
            .stderr(stderr);

        match daemonize.start() {
            Ok(_) => {
                // We're in the daemon process now
                if let Err(e) = Self::run_daemon() {
                    eprintln!("Daemon error: {}", e);
                    std::process::exit(1);
                }
                std::process::exit(0);
            }
            Err(e) => Err(SvenError::ConfigError(format!("Failed to start daemon: {}", e))),
        }
    }

    // Check if daemon is running
    pub fn is_daemon_running() -> Result<bool> {
        let pid_file_path = Self::get_pid_file_path()?;
        if !pid_file_path.exists() {
            return Ok(false);
        }

        let file = File::open(pid_file_path)?;
        let mut reader = BufReader::new(file);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        
        let pid = line.trim().parse::<u32>().map_err(|_| {
            SvenError::ConfigError("Invalid PID in PID file".into())
        })?;

        // Check if process with this PID exists
        let proc_path = PathBuf::from(format!("/proc/{}", pid));
        Ok(proc_path.exists())
    }

    // Run the daemon main loop
    fn run_daemon() -> Result<()> {
        // Initialize the daemon to get the initial secrets
        let mut db = Database::new()?;
        let secrets_vec = db.get_all_secrets()?;
        
        // Convert to a HashMap and wrap in thread-safe container
        let mut secrets_map = HashMap::new();
        for (key, value) in secrets_vec {
            secrets_map.insert(key, value);
        }
        let secrets = Arc::new(Mutex::new(secrets_map));
        
        // Create the Unix socket
        let socket_path = Self::get_socket_path()?;
        let listener = UnixListener::bind(&socket_path)?;
        
        // Set up a channel for shutdown signaling
        let (tx, mut rx) = mpsc::channel::<()>(1);
        let tx_clone = tx.clone();
        
        // Create a channel for database operations
        let (db_tx, db_rx) = std::sync::mpsc::channel();
        
        // Database thread - handles all operations that need the GPG context
        // We create a new Database instance in this thread to avoid Send issues
        std::thread::spawn(move || {
            // Create a new Database instance in this thread
            match Database::new() {
                Ok(mut db) => {
                    for cmd in db_rx {
                        match cmd {
                            DbCommand::AddSecret { key, value, resp } => {
                                let result = db.add_secret(&key, &value)
                                    .map(|_| format!("Added secret: {}", key));
                                let _ = resp.send(result);
                            },
                            DbCommand::RemoveSecret { key, resp } => {
                                let result = db.remove_secret(&key)
                                    .map(|_| format!("Removed secret: {}", key));
                                let _ = resp.send(result);
                            },
                            DbCommand::Shutdown => break,
                        }
                    }
                },
                Err(e) => {
                    eprintln!("Failed to create database in worker thread: {}", e);
                }
            }
        });
        
        // Handle client connections
        let secrets_clone = secrets.clone();
        let db_tx_clone = db_tx.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let secrets = secrets_clone.clone();
                        let tx = tx_clone.clone();
                        let db_tx = db_tx_clone.clone();
                        std::thread::spawn(move || {
                            if let Err(e) = Self::handle_client(stream, secrets, db_tx, tx) {
                                eprintln!("Error handling client: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("Error accepting connection: {}", e);
                    }
                }
            }
        });
        
        // Wait for shutdown signal
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            rx.recv().await;
        });
        
        // Send shutdown signal to database thread
        let _ = db_tx.send(DbCommand::Shutdown);
        
        // Clean up
        if socket_path.exists() {
            let _ = std::fs::remove_file(socket_path);
        }
        
        Ok(())
    }

    }

// Commands for the database thread
enum DbCommand {
    AddSecret {
        key: String,
        value: String,
        resp: std::sync::mpsc::Sender<crate::error::Result<String>>,
    },
    RemoveSecret {
        key: String,
        resp: std::sync::mpsc::Sender<crate::error::Result<String>>,
    },
    Shutdown,
}

impl Daemon {

    // Handle a client connection
    fn handle_client(
        stream: UnixStream, 
        secrets: Arc<Mutex<HashMap<String, String>>>,
        db_tx: std::sync::mpsc::Sender<DbCommand>,
        shutdown_tx: mpsc::Sender<()>
    ) -> Result<()> {
        let mut reader = BufReader::new(&stream);
        let mut request = String::new();
        reader.read_line(&mut request)?;
        
        let command: DaemonCommand = serde_json::from_str(&request)
            .map_err(|e| SvenError::ConfigError(format!("Invalid command: {}", e)))?;
        
        let response = match command {
            DaemonCommand::GetSecrets { shell: _ } => {
                let secrets_guard = secrets.lock().unwrap();
                let secrets_vec: Vec<(String, String)> = secrets_guard.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                DaemonResponse::Secrets(secrets_vec)
            },
            DaemonCommand::ListSecrets => {
                let secrets_guard = secrets.lock().unwrap();
                let keys: Vec<String> = secrets_guard.keys()
                    .cloned()
                    .collect();
                DaemonResponse::KeyList(keys)
            },
            DaemonCommand::AddSecret { key, value } => {
                // Create a channel for the response
                let (resp_tx, resp_rx) = std::sync::mpsc::channel();
                
                // Send the command to the database thread
                db_tx.send(DbCommand::AddSecret {
                    key: key.clone(),
                    value: value.clone(),
                    resp: resp_tx,
                })?;
                
                // Wait for the response
                match resp_rx.recv() {
                    Ok(Ok(msg)) => {
                        // Update the in-memory cache
                        let mut secrets_guard = secrets.lock().unwrap();
                        secrets_guard.insert(key, value);
                        DaemonResponse::Success(msg)
                    },
                    Ok(Err(e)) => DaemonResponse::Error(format!("Failed to add secret: {}", e)),
                    Err(e) => DaemonResponse::Error(format!("Failed to communicate with database thread: {}", e)),
                }
            },
            DaemonCommand::RemoveSecret { key } => {
                // Create a channel for the response
                let (resp_tx, resp_rx) = std::sync::mpsc::channel();
                
                // Send the command to the database thread
                db_tx.send(DbCommand::RemoveSecret {
                    key: key.clone(),
                    resp: resp_tx,
                })?;
                
                // Wait for the response
                match resp_rx.recv() {
                    Ok(Ok(msg)) => {
                        // Update the in-memory cache
                        let mut secrets_guard = secrets.lock().unwrap();
                        secrets_guard.remove(&key);
                        DaemonResponse::Success(msg)
                    },
                    Ok(Err(e)) => DaemonResponse::Error(format!("Failed to remove secret: {}", e)),
                    Err(e) => DaemonResponse::Error(format!("Failed to communicate with database thread: {}", e)),
                }
            },
            DaemonCommand::Shutdown => {
                let _ = shutdown_tx.blocking_send(());
                DaemonResponse::Success("Daemon shutting down".into())
            }
        };
        
        let response_json = serde_json::to_string(&response)?;
        let mut writer = &stream;
        writeln!(writer, "{}", response_json)?;
        
        Ok(())
    }
}

// Client for communicating with the daemon
pub struct DaemonClient {
    socket_path: PathBuf,
}

impl DaemonClient {
    pub fn new() -> Result<Self> {
        let socket_path = Daemon::get_socket_path()?;
        Ok(Self { socket_path })
    }
    
    pub fn is_daemon_running() -> Result<bool> {
        Daemon::is_daemon_running()
    }
    
    // Send a command to the daemon and get the response
    pub fn send_command(&self, command: DaemonCommand) -> Result<DaemonResponse> {
        if !self.socket_path.exists() {
            return Err(SvenError::ConfigError("Daemon is not running".into()));
        }
        
        let mut stream = UnixStream::connect(&self.socket_path)?;
        let command_json = serde_json::to_string(&command)?;
        writeln!(stream, "{}", command_json)?;
        
        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader.read_line(&mut response)?;
        
        let response: DaemonResponse = serde_json::from_str(&response)
            .map_err(|e| SvenError::ConfigError(format!("Invalid response: {}", e)))?;
            
        Ok(response)
    }
    
    // Get all secrets from the daemon
    pub fn get_secrets(&self, shell: &str) -> Result<Vec<(String, String)>> {
        match self.send_command(DaemonCommand::GetSecrets { shell: shell.to_string() })? {
            DaemonResponse::Secrets(secrets) => Ok(secrets),
            DaemonResponse::Error(e) => Err(SvenError::ConfigError(e)),
            _ => Err(SvenError::ConfigError("Unexpected response from daemon".into())),
        }
    }
    
    // List all secret keys from the daemon
    pub fn list_secrets(&self) -> Result<Vec<String>> {
        match self.send_command(DaemonCommand::ListSecrets)? {
            DaemonResponse::KeyList(keys) => Ok(keys),
            DaemonResponse::Error(e) => Err(SvenError::ConfigError(e)),
            _ => Err(SvenError::ConfigError("Unexpected response from daemon".into())),
        }
    }
    
    // Add a secret through the daemon
    pub fn add_secret(&self, key: &str, value: &str) -> Result<String> {
        match self.send_command(DaemonCommand::AddSecret { 
            key: key.to_string(), 
            value: value.to_string() 
        })? {
            DaemonResponse::Success(msg) => Ok(msg),
            DaemonResponse::Error(e) => Err(SvenError::ConfigError(e)),
            _ => Err(SvenError::ConfigError("Unexpected response from daemon".into())),
        }
    }
    
    // Remove a secret through the daemon
    pub fn remove_secret(&self, key: &str) -> Result<String> {
        match self.send_command(DaemonCommand::RemoveSecret { key: key.to_string() })? {
            DaemonResponse::Success(msg) => Ok(msg),
            DaemonResponse::Error(e) => Err(SvenError::ConfigError(e)),
            _ => Err(SvenError::ConfigError("Unexpected response from daemon".into())),
        }
    }
    
    // Shutdown the daemon
    pub fn shutdown_daemon(&self) -> Result<String> {
        match self.send_command(DaemonCommand::Shutdown)? {
            DaemonResponse::Success(msg) => Ok(msg),
            DaemonResponse::Error(e) => Err(SvenError::ConfigError(e)),
            _ => Err(SvenError::ConfigError("Unexpected response from daemon".into())),
        }
    }
}