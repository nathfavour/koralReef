use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use kora_reclaim_rs::config::Config;
use kora_reclaim_rs::state::{AppState, SharedState};
use kora_reclaim_rs::core::scanner::Scanner;
use kora_reclaim_rs::core::reclaimer::Reclaimer;
use kora_reclaim_rs::bot;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use log::{info, error};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    /// Run in dry-run mode (overrides config)
    #[arg(short, long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();
    info!("Starting kora-reclaim-rs...");

    let mut config = Config::load(&args.config).expect("Failed to load configuration file");
    if args.dry_run {
        config.settings.dry_run = true;
    }
    let state: SharedState = Arc::new(Mutex::new(AppState::new()));

    let bot_state = state.clone();
    let bot_config = config.clone();
    tokio::spawn(async move {
        bot::start_bot(bot_config, bot_state).await;
    });

    let sentinel_state = state.clone();
    let sentinel_config = config.clone();
    
    sentinel_loop(sentinel_config, sentinel_state).await?;

    Ok(())
}

async fn sentinel_loop(config: Config, state: SharedState) -> anyhow::Result<()> {
    let scanner = Scanner::new(&config.solana.rpc_url);
    
    let keypair_bytes = std::fs::read_to_string(&config.solana.keypair_path)
        .expect("Failed to read keypair file");
    let keypair_vec: Vec<u8> = serde_json::from_str(&keypair_bytes)
        .expect("Failed to parse keypair JSON");
    let keypair = Keypair::from_bytes(&keypair_vec).expect("Invalid keypair bytes");
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
