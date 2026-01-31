pub mod commands;

use teloxide::prelude::*;
use crate::state::SharedState;
use crate::config::Config;
use crate::bot::commands::Command;
use log::info;

pub async fn start_bot(config: Config, state: SharedState) {
    let bot = Bot::new(config.telegram.bot_token.clone());

    let handler = Update::filter_message()
        .filter_command::<Command>()
        .endpoint(handle_command);

    info!("Starting Telegram bot...");
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![state, config])
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
) -> ResponseResult<()> {
    if !config.telegram.authorized_user_ids.contains(&msg.from().unwrap().id.0) {
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
            let response = format!(
                "📊 Stats:\n- Total Reclaimed: {} SOL\n- Accounts Closed: {}\n- Uptime: {:?}\n- Dry Run: {}",
                s.total_reclaimed_lamports as f64 / 1_000_000_000.0,
                s.total_accounts_closed,
                uptime,
                config.settings.dry_run
            );
            bot.send_message(msg.chat.id, response).await?;
        }
        Command::Sweep => {
            let mut s = state.lock().await;
            s.force_run = true;
            bot.send_message(msg.chat.id, "Triggering manual sweep...").await?;
        }
    }

    Ok(())
}
