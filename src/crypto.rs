use crate::error::{Result, SvenError};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use dialoguer::Select;
use gpgme::{Context, Protocol, Validity};
use rusqlite::params;

const GPG_KEY_CONFIG: &str = "gpg_key";

pub struct CryptoManager {
    ctx: Context,
    key_id: Option<String>,
}

impl CryptoManager {
    pub fn new() -> Result<Self> {
        let ctx = Context::from_protocol(Protocol::OpenPgp)
            .map_err(|e| SvenError::GpgNotAvailable(e.to_string()))?;

        Ok(Self { 
            ctx,
            key_id: None,
        })
    }

    fn select_key(ctx: &mut Context) -> Result<String> {
        let keys: Vec<_> = ctx
            .secret_keys()?
            .filter_map(|key| key.ok())
            .filter(|key| {
                !key.is_expired() 
                && !key.is_revoked() 
                && !key.is_disabled() 
                && !key.is_invalid()
                && key.owner_trust() == Validity::Ultimate
            })
            .collect();

        if keys.is_empty() {
            return Err(SvenError::NoGpgKeys);
        }

        let key_strings: Vec<String> = keys
            .iter()
            .map(|key| {
                format!(
                    "{} ({}) <{}>",
                    key.id().unwrap_or("Unknown"),
                    key.user_ids()
                        .next()
                        .and_then(|uid| uid.name().ok().and_then(|s| Some(s)))
                        .unwrap_or("Unknown"),
                    key.user_ids()
                        .next()
                        .and_then(|uid| uid.email().ok().and_then(|s| Some(s)))
                        .unwrap_or("Unknown")
                )
            })
            .collect();

        let selection = Select::new()
            .with_prompt("Select GPG key for encryption")
            .items(&key_strings)
            .default(0)
            .interact()
            .map_err(|e| SvenError::ConfigError(e.to_string()))?;

        Ok(keys[selection].id().unwrap_or_default().to_string())
    }

    pub fn ensure_key_selected(&mut self, db: &rusqlite::Connection) -> Result<()> {
        let mut stmt = db.prepare("SELECT value FROM config WHERE key = ?1")?;
        let mut rows = stmt.query(params![GPG_KEY_CONFIG])?;
        
        let key_id = if let Some(row) = rows.next()? {
            row.get(0)?
        } else {
            let key_id = Self::select_key(&mut self.ctx)?;
            db.execute(
                "INSERT INTO config (key, value) VALUES (?1, ?2)",
                params![GPG_KEY_CONFIG, &key_id],
            )?;
            key_id
        };

        self.key_id = Some(key_id);
        Ok(())
    }

    pub fn encrypt(&mut self, data: &[u8]) -> Result<String> {
        let key_id = self.key_id.as_ref().ok_or(SvenError::NoKeySelected)?;
        let key = self.ctx.get_secret_key(key_id)?;
        if key.is_invalid() {
            return Err(SvenError::NoKeySelected);
        }
        
        let mut encrypted = Vec::new();
        self.ctx.encrypt(Some(&key), data, &mut encrypted)?;
        
        Ok(BASE64.encode(encrypted))
    }

    pub fn decrypt(&mut self, data: &str) -> Result<Vec<u8>> {
        let encrypted = BASE64.decode(data).map_err(|e| SvenError::ConfigError(e.to_string()))?;
        let mut decrypted = Vec::new();
        self.ctx.decrypt(encrypted.as_slice(), &mut decrypted)?;
        Ok(decrypted)
    }
}
