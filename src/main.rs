use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use kora_reclaim_rs::config::{Config, AppMode};
use kora_reclaim_rs::state::{AppState, SharedState};
use kora_reclaim_rs::core::scanner::Scanner;
use kora_reclaim_rs::core::reclaimer::Reclaimer;
use kora_reclaim_rs::bot;
use kora_reclaim_rs::storage::Storage;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::pubkey::Pubkey;
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();
    info!("Starting kora-reclaim-rs...");

    let storage = Arc::new(Storage::init()?);

    // Initial config loading logic
    let mut config = if let Some(path) = args.config {
        Config::load(path)?
    } else if let Some(mode_str) = &args.mode {
        if mode_str.to_lowercase() == "demo" {
            Config::demo()
        } else {
            // Try to load from DB or use default real skeleton
            load_config_from_storage(&storage).unwrap_or_else(|_| Config::demo())
        }
    } else {
        load_config_from_storage(&storage).unwrap_or_else(|_| Config::demo())
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

    let state: SharedState = Arc::new(Mutex::new(AppState::new()));

    let bot_state = state.clone();
    let bot_config = config.clone();
    let bot_storage = storage.clone();
    
    if !bot_config.telegram.bot_token.is_empty() {
        tokio::spawn(async move {
            bot::start_bot(bot_config, bot_state, bot_storage).await;
        });
    }

    sentinel_loop(config, state, storage).await?;

    Ok(())
}

fn load_config_from_storage(storage: &Storage) -> anyhow::Result<Config> {
    // This is a simplified loader. In a full app, we'd store all fields in SQLite.
    let token = storage.get_setting("bot_token")?.unwrap_or_default();
    let mut config = Config::demo();
    config.mode = AppMode::Real;
    config.telegram.bot_token = token;
    Ok(config)
}

async fn sentinel_loop(config: Config, state: SharedState, _storage: Arc<Storage>) -> anyhow::Result<()> {
    if config.mode == AppMode::Demo {
        info!("Sentinel running in DEMO mode.");
        loop {
            sleep(Duration::from_secs(30)).await;
            info!("Demo scan: no accounts found.");
        }
    }

    let scanner = Scanner::new(&config.solana.rpc_url);
    
    // In real mode, we need a valid keypair
    let keypair_res = if config.solana.keypair_path.is_empty() {
        Err(anyhow::anyhow!("No keypair path provided"))
    } else {
        let keypair_bytes = std::fs::read_to_string(&config.solana.keypair_path)?;
        let keypair_vec: Vec<u8> = serde_json::from_str(&keypair_bytes)?;
        Ok(Keypair::from_bytes(&keypair_vec)?)
    };

    let keypair = match keypair_res {
        Ok(k) => k,
        Err(e) => {
            error!("Failed to load keypair: {}. Sentinel will wait.", e);
            loop { sleep(Duration::from_secs(60)).await; }
        }
    };

    let keypair_pubkey = keypair.pubkey();
    let treasury = Pubkey::from_str(&config.solana.treasury_address)?;
    let reclaimer = Reclaimer::new(&config.solana.rpc_url, keypair, treasury);

    loop {
        let mut force = false;
        {
            let mut s = state.lock().await;
            if s.force_run {
                force = true;
                s.force_run = false;
            }
        }

        if force || should_scan(&state, config.settings.scan_interval_hours).await {
            info!("Starting scan...");
            
            match scanner.find_reclaimable_accounts(&keypair_pubkey, &config.settings.whitelist) {
                Ok(accounts) => {
                    info!("Found {} reclaimable accounts", accounts.len());
                    let pubkeys: Vec<Pubkey> = accounts.iter().map(|(p, _)| *p).collect();
                    
                    match reclaimer.reclaim_accounts(&pubkeys, config.settings.dry_run) {
                        Ok((lamports, count)) => {
                            let mut s = state.lock().await;
                            s.total_reclaimed_lamports += lamports;
                            s.total_accounts_closed += count;
                            s.last_scan_time = Some(std::time::Instant::now());
                            info!("Reclaim cycle complete. Reclaimed {} accounts.", count);
                        }
                        Err(e) => error!("Reclaim error: {}", e),
                    }
                }
                Err(e) => error!("Scanner error: {}", e),
            }
        }

        sleep(Duration::from_secs(60)).await;
    }
}

async fn should_scan(state: &SharedState, interval_hours: u64) -> bool {
    let s = state.lock().await;
    match s.last_scan_time {
        None => true,
        Some(last) => last.elapsed() >= Duration::from_secs(interval_hours * 3600),
    }
}