pub mod commands;

use teloxide::prelude::*;
use crate::state::SharedState;
use crate::config::Config;
use crate::bot::commands::Command;
use crate::storage::Storage;
use log::info;
use std::sync::Arc;

pub async fn start_bot(config: Config, state: SharedState, storage: Arc<Storage>) {
    let bot = Bot::new(config.telegram.bot_token.clone());

    let handler = Update::filter_message()
        .filter_command::<Command>()
        .endpoint(handle_command);

    info!("Starting Telegram bot...");
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![state, config, storage])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: Command,
    state: SharedState,
    config: Config,
    storage: Arc<Storage>,
) -> ResponseResult<()> {
    let user_id = msg.from().unwrap().id.0;
    
    // Check if we have an admin. If not, the first person to message becomes admin.
    let admin = storage.get_admin().unwrap_or(None);
    let is_admin = match admin {
        Some(id) => id == user_id,
        None => {
            info!("No admin found. Setting user {} as admin.", user_id);
            storage.set_admin(user_id).unwrap();
            true
        }
    };

    if !is_admin && !config.telegram.authorized_user_ids.contains(&user_id) {
        bot.send_message(msg.chat.id, "Unauthorized.").await?;
        return Ok(());
    }

    match cmd {
        Command::Start => {
            bot.send_message(msg.chat.id, "Kora Reclaim Bot is active. Use /stats or /sweep.").await?;
        }
        Command::Stats => {
            let s = state.lock().await;
            let uptime = s.start_time.elapsed();
            let last_reclaim = s.last_reclaim_summary.as_deref().unwrap_or("None");
            let response = format!(
                "📊 Stats:\n- Total Reclaimed: {} SOL\n- Accounts Closed: {}\n- Uptime: {:?}\n- Last Event: {}\n- Mode: {:?}\n- Dry Run: {}",
                s.total_reclaimed_lamports as f64 / 1_000_000_000.0,
                s.total_accounts_closed,
                uptime,
                last_reclaim,
                config.mode,
                config.settings.dry_run
            );
            bot.send_message(msg.chat.id, response).await?;
        }
        Command::Sweep => {
            let mut s = state.lock().await;
            s.force_run = true;
            bot.send_message(msg.chat.id, "Triggering manual sweep...").await?;
        }
        Command::Log => {
            let logs = storage.get_recent_history(10).unwrap_or_else(|_| vec!["Failed to load logs".to_string()]);
            let response = if logs.is_empty() {
                "No events recorded yet.".to_string()
            } else {
                format!("📜 Recent History:\n{}", logs.join("\n"))
            };
            bot.send_message(msg.chat.id, response).await?;
        }
    }

    Ok(())
}
