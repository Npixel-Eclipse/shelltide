use crate::api::traits::BytebaseApi;
use crate::api::types::{
    Changelog, IssueName, PostSheetsResponse, Revision, SQLDialect, SheetName, SheetRequest,
};
use crate::cli::MigrateArgs;
use crate::config::{self, Environment};
use crate::error::AppError;
use anyhow::Result;

pub async fn handle_migrate_command<T: BytebaseApi>(
    args: MigrateArgs,
    api_client: &T,
) -> Result<()> {
    let config = config::load_config().await?;

    let source_env = config
        .environments
        .get(&args.source.env)
        .ok_or_else(|| AppError::EnvNotFound(args.source.env.clone()))?;
    let target_env = config
        .environments
        .get(&args.target.env)
        .ok_or_else(|| AppError::EnvNotFound(args.target.env.clone()))?;

    println!(
        "Attempting to apply migrations from '{}' to '{}'...",
        args.source.env, &args.target.env
    );

    let source_latest_no = get_latest_done_issue_no(api_client, &source_env.project).await?;
    let target_revision = api_client
        .get_latests_revisions(&target_env.instance, &args.target.db)
        .await?;
    let target_latest_no = target_revision
        .version
        .as_ref()
        .ok_or_else(|| AppError::ApiError("Target revision missing version".to_string()))?
        .number;

    println!(
        "Source '{}' is at issue #{}, Target '{}' is at issue #{}.",
        args.source.env, source_latest_no, &args.target.env, target_latest_no
    );

    let target_version = if args.to.eq_ignore_ascii_case("LATEST") {
        source_latest_no
    } else {
        args.to.parse::<u32>().map_err(|_| {
            AppError::InvalidArgs(format!(
                "Invalid version '{}'. Must be an integer or 'LATEST'.",
                args.to
            ))
        })?
    };

    if target_latest_no >= target_version {
        println!(
            "Target environment '{}' is already up-to-date. Nothing to apply.",
            &args.target.env
        );
        return Ok(());
    }

    // Execute migrations
    println!("--- Applying Migrations ---");
    let Some((last_issue, last_sheet, all_successful)) = migrate(
        api_client,
        source_env,
        &args.source.db,
        target_env,
        &args.target.db,
        &target_revision,
        &SQLDialect::MySQL,
        target_version,
    )
    .await
    else {
        println!("No issues to apply.");
        return Ok(());
    };

    // create revision - use target version if all successful, otherwise use last applied issue
    let revision_issue_number = if all_successful {
        target_version
    } else {
        last_issue.number
    };
    
    let revision_name = format!("{}#{}", last_issue.project, revision_issue_number);
    let revision_version = format!("{}#{}", last_issue.project, revision_issue_number);
    let revision_sheet = last_sheet.to_string();
    api_client
        .create_revision(
            &target_env.instance,
            &args.target.db,
            &revision_name,
            &revision_version,
            &revision_sheet,
        )
        .await?;

    println!("--- Migration Complete ---\n");

    Ok(())
}

/// A helper function to get the highest "DONE" issue number for a project.
async fn get_latest_done_issue_no<T: BytebaseApi>(
    api_client: &T,
    project: &str,
) -> Result<u32, AppError> {
    let issues = api_client.get_done_issues(project).await?;
    Ok(issues.iter().map(|i| i.name.number).max().unwrap_or(0))
}

async fn apply_changelog<T: BytebaseApi>(
    api_client: &T,
    target_env: &Environment,
    target_database: &str,
    source_changelog: &Changelog,
    engine: &SQLDialect,
) -> Result<PostSheetsResponse, AppError> {
    // SQL check in target project
    api_client
        .check_sql(
            &target_env.instance,
            target_database,
            &source_changelog.statement.to_string(),
        )
        .await?;

    let sheet_req = SheetRequest {
        sql_statement: source_changelog.statement.clone().into(),
        engine: engine.clone(),
    };

    let sheet_response = api_client
        .create_sheet(&target_env.project, sheet_req)
        .await?;
    let plan_response = api_client
        .create_plan(
            &target_env.project,
            &target_env.instance,
            target_database,
            sheet_response.clone().name,
        )
        .await?;
    let issue_response = api_client
        .create_issue(&target_env.project, &plan_response.name)
        .await?;
    api_client
        .create_rollout(&target_env.project, plan_response.name, issue_response.name)
        .await?;

    Ok(sheet_response)
}

async fn migrate<T: BytebaseApi>(
    api_client: &T,
    source_env: &Environment,
    source_database: &str,
    target_env: &Environment,
    target_database: &str,
    target_revision: &Revision,
    engine: &SQLDialect,
    target_version: u32,
) -> Option<(IssueName, SheetName, bool)> {
    let mut last_applied = None;

    let mut changelogs = api_client
        .get_changelogs(&source_env.instance, source_database, &source_env.project)
        .await
        .ok()?
        .into_iter()
        .filter(|c| {
            c.issue.number > target_revision.version.as_ref().map_or(0, |v| v.number)
                && c.issue.number <= target_version
                && c.changed_resources
                    .databases
                    .iter()
                    .any(|d| d.name == target_database)
        })
        .collect::<Vec<_>>();

    changelogs.sort_by_key(|c| c.create_time);
    let total_changelogs = changelogs.len();
    let mut applied_count = 0;

    for cl in changelogs.into_iter() {
        match apply_changelog(api_client, target_env, target_database, &cl, engine).await {
            Ok(sheet) => {
                println!("Applied changelog: {:?}", cl.name);
                last_applied = Some((cl.issue.clone(), sheet.name));
                applied_count += 1;
            }
            Err(e) => {
                eprintln!("Error applying changelog: {e}");
                let all_successful = applied_count == total_changelogs;
                return last_applied.map(|(issue, sheet)| (issue, sheet, all_successful));
            }
        }
    }

    let all_successful = applied_count == total_changelogs;
    last_applied.map(|(issue, sheet)| (issue, sheet, all_successful))
}
