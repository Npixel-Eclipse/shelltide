mod api;
mod cli;
mod commands;
mod config;
mod error;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

#[cfg(not(test))]
use crate::api::clients::LiveApiClient;

#[cfg(test)]
use crate::api::clients::tests::FakeApiClient;

#[cfg(not(test))]
async fn get_client() -> Result<LiveApiClient> {
    let app_config = config::load_config().await?;
    let client = LiveApiClient::new(&app_config.credentials.unwrap())?;

    Ok(client)
}

#[cfg(test)]
async fn get_client() -> Result<FakeApiClient> {
    let client = FakeApiClient::default();
    Ok(client)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Login(args) => {
            commands::login::login(args).await?;
        }
        Commands::Config(args) => {
            commands::config::config(args.command).await?;
        }
        Commands::Env(args) => {
            let mut client = get_client().await?;
            let app_config = config::load_config().await?;
            let credentials = app_config.get_credentials()?;
            client.login(credentials)?;
            commands::env::handle_env_command(args.command, &client).await?;
        }
        Commands::Migrate(args) => {
            let mut client = get_client().await?;
            let app_config = config::load_config().await?;
            let credentials = app_config.get_credentials()?;
            client.login(credentials)?;
            commands::migrate::handle_migrate_command(args, &client).await?;
        }
        Commands::Status => {
            let mut client = get_client().await?;
            let app_config = config::load_config().await?;
            let credentials = app_config.get_credentials()?;
            client.login(credentials)?;
            commands::status::handle_status_command(&client).await?;
        }
        Commands::Completion(args) => {
            commands::completion::handle_completion_command(args.shell)?;
        }
    }

    Ok(())
}
