use std::time::{Duration, Instant};
use tokio::time::sleep;

use crate::api::traits::BytebaseApi;
use crate::api::types::{Rollout, TaskStatus};
use crate::error::AppError;

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(2);
const NOT_STARTED_TIMEOUT: Duration = Duration::from_secs(60); // 1 minute for stuck detection
const MAX_RETRIES: u32 = 5;
const RETRY_DELAY: Duration = Duration::from_secs(1);

/// Wait for a rollout to complete by polling the API.
///
/// Returns Ok(Rollout) if all tasks succeed, or Err if any task fails or timeout occurs.
pub async fn wait_for_rollout<T: BytebaseApi>(
    api_client: &T,
    project: &str,
    rollout_id: u32,
) -> Result<Rollout, AppError> {
    let start = Instant::now();
    let mut poll_count = 0;

    println!("  Waiting for rollout {} to complete...", rollout_id);

    loop {
        poll_count += 1;

        // Get rollout with retry logic
        let rollout = get_rollout_with_retry(api_client, project, rollout_id).await?;

        // Get current status summary
        let status_summary = get_status_summary(&rollout);
        print_progress(poll_count, start.elapsed(), &status_summary);

        if rollout.is_complete() {
            if rollout.is_success() {
                println!("\n  Rollout {} completed successfully.", rollout_id);
                return Ok(rollout);
            } else {
                // Build detailed error message
                let error_msg = build_failure_message(&rollout);
                println!("\n  Rollout {} failed: {}", rollout_id, error_msg);
                return Err(AppError::ApiError(error_msg));
            }
        }

        // Check if stuck in NOT_STARTED state
        if is_all_not_started(&rollout) && start.elapsed() > NOT_STARTED_TIMEOUT {
            let msg = format!(
                "Rollout {} stuck in NOT_STARTED state for {:?}. \
                Check Bytebase UI for approval requirements or configuration issues.",
                rollout_id, NOT_STARTED_TIMEOUT
            );
            println!("\n  {}", msg);
            return Err(AppError::ApiError(msg));
        }

        // Wait before next poll
        sleep(DEFAULT_POLL_INTERVAL).await;
    }
}

/// Get rollout with retry logic for transient network errors
async fn get_rollout_with_retry<T: BytebaseApi>(
    api_client: &T,
    project: &str,
    rollout_id: u32,
) -> Result<Rollout, AppError> {
    let mut last_error = None;

    for attempt in 1..=MAX_RETRIES {
        match api_client.get_rollout(project, rollout_id).await {
            Ok(rollout) => return Ok(rollout),
            Err(e) => {
                last_error = Some(e);
                if attempt < MAX_RETRIES {
                    eprintln!(
                        "  Warning: Failed to get rollout (attempt {}/{}), retrying...",
                        attempt, MAX_RETRIES
                    );
                    sleep(RETRY_DELAY).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| AppError::ApiError("Unknown error".to_string())))
}

/// Check if all tasks are in NOT_STARTED state (stuck)
fn is_all_not_started(rollout: &Rollout) -> bool {
    let tasks: Vec<_> = rollout
        .stages
        .iter()
        .flat_map(|stage| stage.tasks.iter())
        .collect();

    !tasks.is_empty() && tasks.iter().all(|task| task.status == TaskStatus::NotStarted)
}

/// Get a summary of all task statuses in the rollout
fn get_status_summary(rollout: &Rollout) -> String {
    let mut not_started = 0;
    let mut pending = 0;
    let mut running = 0;
    let mut done = 0;
    let mut failed = 0;
    let mut other = 0;

    for stage in &rollout.stages {
        for task in &stage.tasks {
            match task.status {
                TaskStatus::NotStarted => not_started += 1,
                TaskStatus::Pending => pending += 1,
                TaskStatus::Running => running += 1,
                TaskStatus::Done => done += 1,
                TaskStatus::Failed => failed += 1,
                _ => other += 1,
            }
        }
    }

    let total = not_started + pending + running + done + failed + other;

    if total == 0 {
        return "No tasks".to_string();
    }

    let mut parts = Vec::new();
    if done > 0 {
        parts.push(format!("{} done", done));
    }
    if running > 0 {
        parts.push(format!("{} running", running));
    }
    if pending > 0 {
        parts.push(format!("{} pending", pending));
    }
    if not_started > 0 {
        parts.push(format!("{} not started", not_started));
    }
    if failed > 0 {
        parts.push(format!("{} failed", failed));
    }
    if other > 0 {
        parts.push(format!("{} other", other));
    }

    format!("[{}/{}] {}", done + failed + other, total, parts.join(", "))
}

/// Print progress update (overwrites previous line)
fn print_progress(poll_count: u32, elapsed: Duration, status: &str) {
    // Use \r to overwrite the line, but print newline every 10 polls to show progress
    if poll_count.is_multiple_of(10) {
        println!("  [{:>3}s] Status: {}", elapsed.as_secs(), status);
    } else {
        print!("\r  [{:>3}s] Status: {}    ", elapsed.as_secs(), status);
        // Flush to ensure immediate display
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
}

/// Build a detailed error message for a failed rollout
fn build_failure_message(rollout: &Rollout) -> String {
    let failed_tasks: Vec<_> = rollout
        .stages
        .iter()
        .flat_map(|stage| stage.tasks.iter())
        .filter(|task| task.status == TaskStatus::Failed)
        .collect();

    if failed_tasks.is_empty() {
        return "Rollout failed with unknown error".to_string();
    }

    let task_details: Vec<String> = failed_tasks
        .iter()
        .map(|task| format!("Task '{}' (target: {})", task.name, task.target))
        .collect();

    format!(
        "Rollout failed. {} task(s) failed: {}",
        failed_tasks.len(),
        task_details.join("; ")
    )
}
