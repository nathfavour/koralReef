use std::path::PathBuf;
use rusqlite::Connection;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::{RngCore, thread_rng};
use std::fs;
use anyhow::{Result, Context};
use zeroize::Zeroize;

pub struct Storage {
    pub base_dir: PathBuf,
    pub db_path: PathBuf,
    key: [u8; 32],
}

impl Storage {
    pub fn init() -> Result<Self> {
        let home = std::env::var("HOME").context("HOME env var not set")?;
        let base_dir = PathBuf::from(home).join(".koralReef");
        
        if !base_dir.exists() {
            fs::create_dir_all(&base_dir)?;
        }

        let key_path = base_dir.join(".key");
        let key = if key_path.exists() {
            let mut k = fs::read(&key_path)?;
            if k.len() != 32 {
                anyhow::bail!("Invalid key length");
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&k);
            k.zeroize();
            arr
        } else {
            let mut k = [0u8; 32];
            thread_rng().fill_bytes(&mut k);
            fs::write(&key_path, &k)?;
            k
        };

        let db_path = base_dir.join("koral.db");
        let storage = Self {
            base_dir,
            db_path,
            key,
        };

        storage.setup_db()?;
        Ok(storage)
    }

    fn setup_db(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                id INTEGER PRIMARY KEY,
                key TEXT UNIQUE,
                value TEXT,
                is_encrypted INTEGER DEFAULT 0
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY,
                telegram_id INTEGER UNIQUE,
                is_admin INTEGER DEFAULT 0
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                event TEXT
            )",
            [],
        )?;
        Ok(())
    }

    pub fn encrypt(&self, data: &[u8]) -> Result<String> {
        let cipher = Aes256Gcm::new(&self.key.into());
        let mut nonce_bytes = [0u8; 12];
        thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = cipher.encrypt(nonce, data)
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;
        
        let mut combined = nonce_bytes.to_vec();
        combined.extend(ciphertext);
        Ok(base64::Engine::encode(&base64::prelude::BASE64_STANDARD, combined))
    }

    pub fn decrypt(&self, encoded: &str) -> Result<Vec<u8>> {
        let combined = base64::Engine::decode(&base64::prelude::BASE64_STANDARD, encoded)?;
        if combined.len() < 12 {
            anyhow::bail!("Invalid encrypted data");
        }

        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let cipher = Aes256Gcm::new(&self.key.into());
        let nonce = Nonce::from_slice(nonce_bytes);
        
        cipher.decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))
    }

    pub fn set_setting(&self, key: &str, value: &str, encrypt: bool) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        let final_value = if encrypt {
            self.encrypt(value.as_bytes())?
        } else {
            value.to_string()
        };

        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value, is_encrypted) VALUES (?1, ?2, ?3)",
            (key, final_value, if encrypt { 1 } else { 0 }),
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare("SELECT value, is_encrypted FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query([key])?;

        if let Some(row) = rows.next()? {
            let value: String = row.get(0)?;
            let is_encrypted: i32 = row.get(1)?;
            if is_encrypted == 1 {
                let decrypted = self.decrypt(&value)?;
                Ok(Some(String::from_utf8(decrypted)?))
            } else {
                Ok(Some(value))
            }
        } else {
            Ok(None)
        }
    }

    pub fn get_admin(&self) -> Result<Option<u64>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare("SELECT telegram_id FROM users WHERE is_admin = 1 LIMIT 1")?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            Ok(Some(id as u64))
        } else {
            Ok(None)
        }
    }

    pub fn set_admin(&self, telegram_id: u64) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT OR IGNORE INTO users (telegram_id, is_admin) VALUES (?1, 1)",
            [telegram_id as i64],
        )?;
        Ok(())
    }

    pub fn log_event(&self, event: &str) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT INTO history (event) VALUES (?1)",
            [event],
        )?;
        Ok(())
    }

    pub fn get_recent_history(&self, limit: i32) -> Result<Vec<String>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare("SELECT timestamp, event FROM history ORDER BY id DESC LIMIT ?1")?;
        let rows = stmt.query_map([limit], |row| {
            let ts: String = row.get(0)?;
            let event: String = row.get(1)?;
            Ok(format!("[{}] {}", ts, event))
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn save_keypair(&self, keypair_json: &str) -> Result<()> {
        self.set_setting("solana_keypair", keypair_json, true)
    }

    pub fn get_keypair(&self) -> Result<Option<String>> {
        self.get_setting("solana_keypair")
    }
}
