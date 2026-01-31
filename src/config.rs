use serde::Deserialize;
use std::path::Path;
use anyhow::Result;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub solana: SolanaConfig,
    pub telegram: TelegramConfig,
    pub settings: Settings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SolanaConfig {
    pub rpc_url: String,
    pub keypair_path: String,
    pub treasury_address: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub authorized_user_ids: Vec<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub scan_interval_hours: u64,
    pub dry_run: bool,
    pub whitelist: Vec<String>,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
