use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;
use koralreef::config::{Config, AppMode};
use koralreef::state::{AppState, SharedState};
use koralreef::core::scanner::Scanner;
use koralreef::core::reclaimer::Reclaimer;
use koralreef::bot;
use koralreef::storage::Storage;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::pubkey::Pubkey;
use teloxide::requests::Requester;
use anyhow::Context;
use std::str::FromStr;
use log::{info, error, warn};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the configuration file
    #[arg(short, long)]
    config: Option<String>,

    /// Run in dry-run mode
    #[arg(short, long)]
    dry_run: bool,

    /// Set mode (demo or real)
    #[arg(short, long)]
    mode: Option<String>,

    /// Set Telegram Bot Token
    #[arg(long)]
    token: Option<String>,

    /// Import Solana keypair from file to encrypted database
    #[arg(long)]
    import_key: Option<String>,

    /// Lock the bot in demo mode (cannot be switched via Telegram)
    #[arg(long)]
    demo_only: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();
    info!("Starting kora-reclaim-rs...");

    let storage = Arc::new(Storage::init()?);
    let cancel_token = CancellationToken::new();

    if let Some(path) = args.import_key {
        let key_data = std::fs::read_to_string(path)?;
        let _: Vec<u8> = serde_json::from_str(&key_data).context("Invalid keypair JSON format")?;
        storage.save_keypair(&key_data)?;
        info!("Solana keypair imported and encrypted successfully.");
    }

    let mut config = if let Some(path) = args.config {
        Config::load(path).unwrap_or_else(|_| Config::demo())
    } else if let Some(mode_str) = &args.mode {
        if mode_str.to_lowercase() == "real" {
             load_config_from_storage(&storage).unwrap_or_else(|_| Config::demo())
        } else {
            Config::demo()
        }
    } else {
        // Default to Demo if no config/mode specified
        Config::demo()
    };

    if let Some(token) = args.token {
        storage.set_setting("bot_token", &token, true)?;
        config.telegram.bot_token = token;
    } else if config.telegram.bot_token.is_empty() {
        if let Some(token) = storage.get_setting("bot_token")? {
            config.telegram.bot_token = token;
        }
    }

    if args.dry_run {
        config.settings.dry_run = true;
    }

    if config.telegram.bot_token.is_empty() {
        warn!("Telegram Bot Token is missing. Bot will not start.");
    }

    let mut initial_state = AppState::new(config.mode);
    if args.demo_only {
        initial_state.mode = AppMode::Demo;
        initial_state.demo_only = true;
    }
    
    let state: SharedState = Arc::new(Mutex::new(initial_state));

    let bot_state = state.clone();
    let bot_config = config.clone();
    let bot_storage = storage.clone();
    let bot_cancel = cancel_token.clone();
    
    if !bot_config.telegram.bot_token.is_empty() {
        tokio::spawn(async move {
            tokio::select! {
                _ = bot::start_bot(bot_config, bot_state, bot_storage) => {},
                _ = bot_cancel.cancelled() => {
                    info!("Shutting down bot listener...");
                }
            }
        });
    }

    let sentinel_cancel = cancel_token.clone();
    let sentinel_state = state.clone();
    let sentinel_config = config.clone();
    let sentinel_storage = storage.clone();

    tokio::spawn(async move {
        if let Err(e) = sentinel_loop(sentinel_config, sentinel_state, sentinel_storage, sentinel_cancel).await {
            error!("Sentinel loop error: {}", e);
        }
    });

    tokio::signal::ctrl_c().await?;
    info!("Shutdown signal received. Cleaning up...");
    cancel_token.cancel();
    
    sleep(Duration::from_secs(2)).await;
    info!("Goodbye!");

    Ok(())
}

fn load_config_from_storage(storage: &Storage) -> anyhow::Result<Config> {
    let token = storage.get_setting("bot_token")?.unwrap_or_default();
    let mut config = Config::demo();
    config.mode = AppMode::Real;
    config.telegram.bot_token = token;
    Ok(config)
}

async fn sentinel_loop(
    config: Config, 
    state: SharedState, 
    storage: Arc<Storage>, 
    cancel_token: CancellationToken
) -> anyhow::Result<()> {
    let bot = if !config.telegram.bot_token.is_empty() {
        Some(teloxide::prelude::Bot::new(config.telegram.bot_token.clone()))
    } else {
        None
    };

    loop {
        let current_mode = {
            let s = state.lock().await;
            s.mode
        };

        match current_mode {
            AppMode::Demo => {
                tokio::select! {
                    _ = cancel_token.cancelled() => return Ok(()),
                    _ = sleep(Duration::from_secs(60)) => {
                        let mut force = false;
                        {
                            let mut s = state.lock().await;
                            if s.mode != AppMode::Demo { continue; }
                            if s.force_run { force = true; s.force_run = false; }
                        }
                        if force || should_scan(&state, config.settings.scan_interval_hours).await {
                            let msg = "♻️ [DEMO] Simulated reclaim of 2 accounts (0.004 SOL).";
                            let _ = storage.log_event(msg);
                            if let (Some(b), Some(admin_id)) = (&bot, storage.get_admin().unwrap_or(None)) {
                                let _ = b.send_message(teloxide::types::ChatId(admin_id as i64), msg).await;
                            }
                            let mut s = state.lock().await;
                            s.last_scan_time = Some(std::time::Instant::now());
                            s.last_reclaim_summary = Some(msg.to_string());
                        }
                    }
                }
            }
            AppMode::Real => {
                let scanner = Scanner::new(&config.solana.rpc_url);
                let keypair_res = if let Some(key_json) = storage.get_keypair()? {
                    let keypair_vec: Vec<u8> = serde_json::from_str(&key_json)?;
                    Ok(Keypair::from_bytes(&keypair_vec)?)
                } else if !config.solana.keypair_path.is_empty() {
                    match std::fs::read_to_string(&config.solana.keypair_path) {
                        Ok(keypair_bytes) => {
                            let keypair_vec: Vec<u8> = serde_json::from_str(&keypair_bytes)?;
                            Ok(Keypair::from_bytes(&keypair_vec)?)
                        }
                        Err(e) => Err(anyhow::anyhow!("Failed to read keypair file at {}: {}", config.solana.keypair_path, e)),
                    }
                } else {
                    Err(anyhow::anyhow!("No keypair found in database or config file"))
                };

                let keypair = match keypair_res {
                    Ok(k) => k,
                    Err(e) => {
                        error!("Real mode initialization failed: {}", e);
                        warn!("Please import a key using --import-key or set a valid keypair_path in config.toml");
                        tokio::select! {
                            _ = cancel_token.cancelled() => return Ok(()),
                            _ = sleep(Duration::from_secs(300)) => continue,
                        }
                    }
                };

                let keypair_pubkey = keypair.pubkey();
                let treasury = Pubkey::from_str(&config.solana.treasury_address)?;
                let reclaimer = Reclaimer::new(&config.solana.rpc_url, keypair, treasury);

                tokio::select! {
                    _ = cancel_token.cancelled() => return Ok(()),
                    _ = sleep(Duration::from_secs(60)) => {
                        let mut force = false;
                        {
                            let mut s = state.lock().await;
                            if s.mode != AppMode::Real { continue; }
                            if s.force_run { force = true; s.force_run = false; }
                        }
                        if force || should_scan(&state, config.settings.scan_interval_hours).await {
                            match scanner.find_reclaimable_accounts(&keypair_pubkey, &config.settings.whitelist) {
                                Ok(accounts) => {
                                    let pubkeys: Vec<Pubkey> = accounts.iter().map(|(p, _)| *p).collect();
                                    match reclaimer.reclaim_accounts(&pubkeys, config.settings.dry_run) {
                                        Ok((lamports, count)) => {
                                            let mut s = state.lock().await;
                                            s.total_reclaimed_lamports += lamports;
                                            s.total_accounts_closed += count;
                                            s.last_scan_time = Some(std::time::Instant::now());
                                            let summary = format!("♻️ Reclaimed {} accounts ({:.4} SOL).", count, lamports as f64 / 1e9);
                                            s.last_reclaim_summary = Some(summary.clone());
                                            let _ = storage.log_event(&summary);
                                            if let (Some(b), Some(admin_id)) = (&bot, storage.get_admin().unwrap_or(None)) {
                                                let _ = b.send_message(teloxide::types::ChatId(admin_id as i64), summary).await;
                                            }
                                        }
                                        Err(e) => { let _ = storage.log_event(&format!("❌ Reclaim error: {}", e)); }
                                    }
                                }
                                Err(e) => { let _ = storage.log_event(&format!("❌ Scanner error: {}", e)); }
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn should_scan(state: &SharedState, interval_hours: u64) -> bool {
    let s = state.lock().await;
    match s.last_scan_time {
        None => true,
        Some(last) => last.elapsed() >= Duration::from_secs(interval_hours * 3600),
    }
}