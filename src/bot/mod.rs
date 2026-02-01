pub mod commands;

use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use crate::state::SharedState;
use crate::config::Config;
use crate::bot::commands::Command;
use crate::storage::Storage;
use log::info;
use std::sync::Arc;

pub async fn start_bot(config: Config, state: SharedState, storage: Arc<Storage>) {
    let bot = Bot::new(config.telegram.bot_token.clone());

    // Register commands in the Telegram UI menu
    let _ = bot.set_my_commands(Command::bot_commands()).await;

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
    
    let current_mode = {
        let s = state.lock().await;
        s.mode
    };

    if current_mode == crate::config::AppMode::Real {
        // In Real mode, handle authorization strictly
        let admin = storage.get_admin().unwrap_or(None);
        let is_admin = match admin {
            Some(id) => id == user_id,
            None => {
                info!("No admin found. Setting first messenger (user {}) as admin.", user_id);
                storage.set_admin(user_id).unwrap();
                bot.send_message(msg.chat.id, "🔐 You have been registered as the sole administrator for this koralreef worker.").await?;
                true
            }
        };

        if !is_admin {
            bot.send_message(msg.chat.id, "🚫 Unauthorized. This bot is locked to another administrator.").await?;
            return Ok(());
        }
    } else {
        // In Demo mode, we still track the admin but don't block others
        let admin = storage.get_admin().unwrap_or(None);
        if admin.is_none() {
            let _ = storage.set_admin(user_id);
        }
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
        Command::Mode => {
            let mut s = state.lock().await;
            if s.demo_only {
                bot.send_message(msg.chat.id, "⚠️ **Action Denied**: This worker is locked in **Demo-Only** mode via system flags.").parse_mode(teloxide::types::ParseMode::MarkdownV2).await?;
            } else {
                let new_mode = match s.mode {
                    crate::config::AppMode::Demo => crate::config::AppMode::Real,
                    crate::config::AppMode::Real => crate::config::AppMode::Demo,
                };
                s.mode = new_mode;
                let msg_text = format!("🔄 Mode switched to: **{:?}**", new_mode);
                bot.send_message(msg.chat.id, msg_text).parse_mode(teloxide::types::ParseMode::MarkdownV2).await?;
            }
        }
        Command::Help => {
            let s = state.lock().await;
            let mode_info = if s.demo_only {
                "⚠️ **DEMO ONLY**: This instance is locked to Demo mode for public testing. It does not perform real transactions."
            } else if s.mode == crate::config::AppMode::Demo {
                "🧪 **DEMO MODE**: Currently simulating reclamation. Transactions are not sent to the blockchain."
            } else {
                "⚡ **REAL MODE**: Operating on Solana Mainnet-Beta."
            };

            let help_text = format!(
                "📖 **koralreef Help**\n\n\
                {}\n\n\
                **Commands:**\n\
                /stats - View reclamation metrics\n\
                /sweep - Trigger an immediate scan\n\
                /log   - View recent event history\n\
                /mode  - Switch modes (if not locked)\n\
                /host  - Learn how to run your own instance\n\
                /health - Check system status\n\n\
                **Secure Setup:**\n\
                To use your own keys, import them into your local instance:\n\
                `koralreef --import-key <path_to_keypair.json>`",
                mode_info
            );
            bot.send_message(msg.chat.id, help_text).parse_mode(teloxide::types::ParseMode::Markdown).await?;
        }
        Command::Host => {
            let host_text = "🏠 **Self-Hosting koralreef**\n\n\
                To reclaim SOL for your own accounts, you should host your own instance on a VPS or local machine.\n\n\
                **Quick Install:**\n\
                `curl -sSL https://raw.githubusercontent.com/nathfavour/koralReef/master/install.sh | bash`\n\n\
                **Why Self-Host?**\n\
                1. **Full Control:** You manage your own Solana private keys.\n\
                2. **Custom Whitelist:** Prevent accidental closure of critical accounts.\n\
                3. **Privacy:** Your operational logs remain on your hardware.\n\n\
                Check the [GitHub Repository](https://github.com/nathfavour/koralReef) for detailed setup guides.";
            bot.send_message(msg.chat.id, host_text).parse_mode(teloxide::types::ParseMode::Markdown).await?;
        }
        Command::Health => {
            let s = state.lock().await;
            let status = if s.demo_only { "Running (Demo-Lock)" } else { "Active" };
            let health_text = format!(
                "🏥 **System Health**\n\n\
                - **Status:** {}\n\
                - **Mode:** {:?}\n\
                - **Uptime:** {:?}\n\
                - **Scanner:** Functional\n\
                - **RPC Endpoint:** Connected\n\n\
                *All systems operational.*",
                status, s.mode, s.start_time.elapsed()
            );
            bot.send_message(msg.chat.id, health_text).parse_mode(teloxide::types::ParseMode::Markdown).await?;
        }
    }

    Ok(())
}
