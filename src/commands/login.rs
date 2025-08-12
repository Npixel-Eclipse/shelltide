use crate::api::clients::get_access_token;
use crate::cli::LoginArgs;
use crate::config::{self, Credentials};
use anyhow::Result;

/// Handles the `login` command.
pub async fn login(args: LoginArgs) -> Result<()> {
    println!("Attempting to log in to {}...", &args.url);
    let login_response = get_access_token(
        &args.url,
        &args.service_account.clone(),
        &args.service_key.clone(),
    )
    .await?;

    println!("Successfully authenticated. Saving credentials...");
    let mut config = config::load_config().await.unwrap_or_default();

    config.credentials = Some(Credentials {
        url: args.url,
        service_account: args.service_account.clone(),
        service_key: Some(args.service_key.clone()), // Store for potential token refresh
        access_token: login_response.token,
    });
    config::save_config(&config).await?;

    println!("Credentials saved successfully.");

    Ok(())
}
