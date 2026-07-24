use crate::error::{AppError, AppResult};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Utc;
use rand::RngCore;
use rusqlite::params;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieSiteInfo {
    pub site_id: String,
    pub updated_at: String,
}

pub struct CookieVault {
    conn: Arc<Mutex<rusqlite::Connection>>,
    cipher: Aes256Gcm,
    cookie_dir: PathBuf,
}

impl CookieVault {
    pub fn new(data_dir: PathBuf, conn: Arc<Mutex<rusqlite::Connection>>) -> AppResult<Self> {
        let cookie_dir = data_dir.join("cookies");
        std::fs::create_dir_all(&cookie_dir)?;
        let key_path = data_dir.join("vault.key");
        let key = load_or_create_key(&key_path)?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| AppError::Other(format!("cipher init: {e}")))?;
        Ok(Self {
            conn,
            cipher,
            cookie_dir,
        })
    }

    pub fn list_sites(&self) -> AppResult<Vec<CookieSiteInfo>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let mut stmt =
            conn.prepare("SELECT site_id, updated_at FROM site_cookies ORDER BY site_id")?;
        let rows = stmt.query_map([], |row| {
            Ok(CookieSiteInfo {
                site_id: row.get(0)?,
                updated_at: row.get(1)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    pub fn save_cookies(&self, site_id: &str, netscape_cookies: &str) -> AppResult<()> {
        let encrypted = self.encrypt(netscape_cookies.as_bytes())?;
        let path = self.cookie_file_path(site_id);
        std::fs::write(&path, netscape_cookies)?;
        let now = Utc::now().to_rfc3339();
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        conn.execute(
            "INSERT INTO site_cookies (site_id, encrypted_data, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(site_id) DO UPDATE SET encrypted_data = excluded.encrypted_data, updated_at = excluded.updated_at",
            params![site_id, encrypted, now],
        )?;
        Ok(())
    }

    pub fn delete_cookies(&self, site_id: &str) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        conn.execute(
            "DELETE FROM site_cookies WHERE site_id = ?1",
            params![site_id],
        )?;
        let path = self.cookie_file_path(site_id);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn cookie_file_path(&self, site_id: &str) -> PathBuf {
        self.cookie_dir.join(format!("{site_id}.txt"))
    }

    pub fn cookie_file_for_site(&self, site_id: &str) -> Option<PathBuf> {
        let path = self.cookie_file_path(site_id);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    pub fn cookie_header(&self, site_id: &str) -> AppResult<Option<String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Other(e.to_string()))?;
        let blob: Option<Vec<u8>> = conn
            .query_row(
                "SELECT encrypted_data FROM site_cookies WHERE site_id = ?1",
                params![site_id],
                |row| row.get(0),
            )
            .optional()?;
        let Some(blob) = blob else {
            return Ok(None);
        };
        let plain = self.decrypt(&blob)?;
        let text = String::from_utf8(plain).map_err(|e| AppError::Other(e.to_string()))?;
        Ok(Some(netscape_to_header(&text)))
    }

    fn encrypt(&self, plain: &[u8]) -> AppResult<Vec<u8>> {
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = self
            .cipher
            .encrypt(nonce, plain)
            .map_err(|e| AppError::Other(format!("encrypt: {e}")))?;
        let mut out = nonce_bytes.to_vec();
        out.extend(ciphertext);
        Ok(out)
    }

    fn decrypt(&self, blob: &[u8]) -> AppResult<Vec<u8>> {
        if blob.len() < 12 {
            return Err(AppError::Other("invalid encrypted blob".into()));
        }
        let (nonce_bytes, ciphertext) = blob.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| AppError::Other(format!("decrypt: {e}")))
    }
}

fn load_or_create_key(path: &PathBuf) -> AppResult<[u8; 32]> {
    if path.exists() {
        let encoded = std::fs::read_to_string(path)?;
        let bytes = STANDARD
            .decode(encoded.trim())
            .map_err(|e| AppError::Other(format!("key decode: {e}")))?;
        if bytes.len() != 32 {
            return Err(AppError::Other("invalid vault key length".into()));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        return Ok(key);
    }
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    std::fs::write(path, STANDARD.encode(key))?;
    Ok(key)
}

fn netscape_to_header(netscape: &str) -> String {
    netscape
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 7 {
                Some(format!("{}={}", parts[5], parts[6]))
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("; ")
}
