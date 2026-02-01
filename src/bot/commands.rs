use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Kora Reclaim Bot Commands")]
pub enum Command {
    #[command(description = "Start the bot")]
    Start,
    #[command(description = "Show statistics")]
    Stats,
    #[command(description = "Manually trigger a scan and reclaim")]
    Sweep,
    #[command(description = "Show recent event history")]
    Log,
    #[command(description = "Switch between Demo and Real modes")]
    Mode,
    #[command(description = "Show help information")]
    Help,
    #[command(description = "Information about self-hosting")]
    Host,
    #[command(description = "Check system health and connectivity")]
    Health,
}
