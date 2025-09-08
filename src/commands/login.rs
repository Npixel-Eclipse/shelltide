use crate::api::clients::get_access_token;
use crate::cli::LoginArgs;
use crate::config::{ConfigOperations, Credentials, ProductionConfig};
use anyhow::Result;

/// Handles the `login` command.
pub async fn login(args: LoginArgs) -> Result<()> {
    let config_ops = ProductionConfig;
    login_with_config(args, &config_ops).await
}

pub async fn login_with_config<C: ConfigOperations>(args: LoginArgs, config_ops: &C) -> Result<()> {
    println!("Attempting to log in to {}...", &args.url);
    let login_response = get_access_token(
        &args.url,
        &args.service_account.clone(),
        &args.service_key.clone(),
    )
    .await?;

    println!("Successfully authenticated. Saving credentials...");
    let mut config = config_ops.load_config().await.unwrap_or_default();

    config.credentials = Some(Credentials {
        url: args.url,
        service_account: args.service_account.clone(),
        service_key: Some(args.service_key.clone()),
        access_token: login_response.token,
    });
    config_ops.save_config(&config).await?;

    println!("Credentials saved successfully.");

    Ok(())
}
