use crate::crypto::CryptoManager;
use crate::error::{Result, SvenError};
use rusqlite::{params, Connection};
use std::path::PathBuf;

pub struct Database {
    conn: Connection,
    crypto: CryptoManager,
}

impl Database {
    pub fn new() -> Result<Self> {
        let db_path = Self::get_db_path()?;

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;
        let crypto = CryptoManager::new()?;
        let mut db = Self { conn, crypto };
        db.init()?;
        db.crypto.ensure_key_selected(&db.conn)?;
        Ok(db)
    }

    fn get_db_path() -> Result<PathBuf> {
        dirs::config_dir()
            .map(|mut p| {
                p.push("sven");
                p.push("envs.sqlite");
                p
            })
            .ok_or_else(|| SvenError::ConfigError("Could not find config directory".into()))
    }

    fn init(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS variables (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;
        Ok(())
    }

    pub fn add_secret(&mut self, key: &str, value: &str) -> Result<()> {
        let encrypted = self.crypto.encrypt(value.as_bytes())?;
        self.conn.execute(
            "INSERT OR REPLACE INTO variables (key, value) VALUES (?1, ?2)",
            params![key, encrypted],
        )?;
        Ok(())
    }

    pub fn remove_secret(&self, key: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM variables WHERE key = ?1", params![key])?;
        Ok(())
    }

    pub fn list_secrets(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT key FROM variables ORDER BY key")?;
        let keys = stmt
            .query_map([], |row| row.get(0))?
            .collect::<std::result::Result<Vec<String>, _>>()?;
        Ok(keys)
    }

    pub fn get_all_secrets(&mut self) -> Result<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT key, value FROM variables ORDER BY key")?;
        let rows = stmt.query_map([], |row| {
            let key: String = row.get(0)?;
            let encrypted_value: String = row.get(1)?;
            Ok((key, encrypted_value))
        })?;

        let mut secrets = Vec::new();
        for row in rows {
            let (key, encrypted_value) = row?;
            let decrypted = self.crypto.decrypt(&encrypted_value)?;
            let value =
                String::from_utf8(decrypted).map_err(|e| SvenError::ConfigError(e.to_string()))?;
            secrets.push((key, value));
        }

        Ok(secrets)
    }
}
