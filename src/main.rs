mod cli;

use cli::{Cli, Commands};
use nexus_agent::config::Config;
use nexus_agent::inbound_adapter::handlers;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    let cli = Cli::parse_args();

    if cli.verbose {
        tracing::info!("Verbose mode enabled");
    }

    let mut config = if let Some(config_path) = &cli.config {
        Config::load_from_file(config_path)?
    } else {
        Config::load()?
    };

    if cli.auto_approve {
        config.safety.require_approval = false;
        tracing::warn!("Auto-approve enabled - all actions will execute without confirmation!");
    }

    match cli.command {
        Some(Commands::Init) => {
            handlers::handle_init().await?;
        }
        Some(Commands::Doctor) => {
            handlers::handle_doctor(&config)?;
        }
        Some(Commands::Agent { task }) => {
            handlers::handle_agent(&task, config).await?;
        }
        Some(Commands::Chat) => {
            handlers::handle_chat(config).await?;
        }
        Some(Commands::Edit { file, instruction }) => {
            handlers::handle_edit(&file, &instruction, config).await?;
        }
        None => {
            if let Some(task) = cli.task {
                handlers::handle_agent(&task, config).await?;
            } else {
                handlers::handle_chat(config).await?;
            }
        }
    }

    Ok(())
}
