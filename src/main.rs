mod config;
mod state;
mod bot;
mod core;

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use crate::config::Config;
use crate::state::{AppState, SharedState};
use crate::core::scanner::Scanner;
use crate::core::reclaimer::Reclaimer;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use log::{info, error};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    info!("Starting kora-reclaim-rs...");

    let config = Config::load("config.toml").expect("Failed to load config.toml");
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
    // Assuming keypair is a JSON array of bytes
    let keypair_vec: Vec<u8> = serde_json::from_str(&keypair_bytes)
        .expect("Failed to parse keypair JSON");
    let keypair = Keypair::from_bytes(&keypair_vec).expect("Invalid keypair bytes");
    
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
            let keypair_pubkey = Keypair::from_bytes(&keypair_vec).unwrap().pubkey();
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

        sleep(Duration::from_secs(60)).await; // Poll every minute for force_run or interval
    }
}

async fn should_scan(state: &SharedState, interval_hours: u64) -> bool {
    let s = state.lock().await;
    match s.last_scan_time {
        None => true,
        Some(last) => last.elapsed() >= Duration::from_secs(interval_hours * 3600),
    }
}