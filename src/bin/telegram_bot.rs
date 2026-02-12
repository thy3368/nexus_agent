// Example: Running the Telegram bot
//
// Usage:
//   1. Set the TELEGRAM_BOT_TOKEN environment variable:
//      export TELEGRAM_BOT_TOKEN="your_bot_token_here"
//
//   2. Run the bot:
//      cargo run --bin telegram_bot
//
//   3. In Telegram, search for your bot and send /start

use pretty_env_logger::env_logger;
use nexus_agent::inbound_adapter::telegram::run_telegram_bot;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    log::info!("Starting Telegram bot...");

    // Run the bot
    run_telegram_bot().await?;

    Ok(())
}
