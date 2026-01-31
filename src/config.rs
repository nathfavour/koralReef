use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::Result;

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq)]
pub enum AppMode {
    Demo,
    Real,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub mode: AppMode,
    pub solana: SolanaConfig,
    pub telegram: TelegramConfig,
    pub settings: Settings,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SolanaConfig {
    pub rpc_url: String,
    pub keypair_path: String,
    pub treasury_address: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub authorized_user_ids: Vec<u64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Settings {
    pub scan_interval_hours: u64,
    pub dry_run: bool,
    pub demo_only: Option<bool>,
    pub whitelist: Vec<String>,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        if !path.as_ref().exists() {
            return Ok(Self::demo());
        }
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content).unwrap_or_else(|_| Self::demo());
        Ok(config)
    }

    pub fn demo() -> Self {
        Self {
            mode: AppMode::Demo,
            solana: SolanaConfig {
                rpc_url: "https://api.devnet.solana.com".to_string(),
                keypair_path: "demo-keypair.json".to_string(),
                treasury_address: "DemoTreasury111111111111111111111111111111".to_string(),
            },
            telegram: TelegramConfig {
                bot_token: "".to_string(),
                authorized_user_ids: vec![],
            },
            settings: Settings {
                scan_interval_hours: 1,
                dry_run: true,
                demo_only: Some(true),
                whitelist: vec![],
            },
        }
    }
}