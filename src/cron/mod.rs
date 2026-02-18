//! Cron scheduler for task scheduling
//!
//! This module provides a cron-like scheduler for executing tasks at specific
//! times or at regular intervals. It supports:
//! - One-time tasks (FireAt) that execute at a specific time
//! - Recurring tasks (Interval) that execute every N minutes (minimum 2)
//!
//! The scheduler runs as a background task and checks for due jobs every minute.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use chrono::Utc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

pub mod types;

use types::{CancelResult, Job, JobStatus, ListResult, ScheduleResult};

/// Global counter for unique job IDs
static JOB_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// The cron scheduler manages scheduled jobs
///
/// Uses Arc<RwLock<HashMap>> for thread-safe concurrent access to jobs.
/// The scheduler can be cloned to share the same job storage across tasks.
#[derive(Debug, Clone)]
pub struct CronScheduler {
    jobs: Arc<RwLock<HashMap<String, Job>>>,
}

impl CronScheduler {
    /// Creates a new CronScheduler with empty job storage
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Generates a unique job ID
    fn generate_job_id() -> String {
        let timestamp = Utc::now().timestamp_millis();
        let counter = JOB_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        format!("job_{}_{}", timestamp, counter)
    }

    /// Schedules a new FireAt job
    ///
    /// # Arguments
    /// * `command` - The command to execute
    /// * `execute_at` - ISO 8601 datetime string when to execute
    /// * `args` - Optional command arguments
    ///
    /// # Returns
    /// * `Ok(ScheduleResult)` - Job scheduled successfully
    /// * `Err(String)` - If the datetime format is invalid or time is in the past
    pub async fn schedule_fire_at(
        &self,
        command: String,
        execute_at: String,
        args: Option<Vec<String>>,
    ) -> Result<ScheduleResult, String> {
        // Parse the ISO 8601 datetime
        let execute_at = chrono::DateTime::parse_from_rfc3339(&execute_at)
            .map_err(|e| format!("Invalid datetime format '{}': {}", execute_at, e))?;
        let execute_at = execute_at.with_timezone(&Utc);

        // Validate time is in the future
        let now = Utc::now();
        if execute_at <= now {
            return Err(format!(
                "Scheduled time '{}' is in the past (current time: {})",
                execute_at.to_rfc3339(),
                now.to_rfc3339()
            ));
        }

        let job_id = Self::generate_job_id();
        let job = Job::new_fire_at(job_id.clone(), command, execute_at, args);

        let next_execution = job.next_execution().unwrap();

        {
            let mut jobs = self.jobs.write().await;
            jobs.insert(job_id.clone(), job);
        }

        info!(
            job_id = %job_id,
            execute_at = %execute_at,
            "Scheduled FireAt job"
        );

        Ok(ScheduleResult {
            success: true,
            job_id,
            message: "FireAt job scheduled successfully".to_string(),
            next_execution,
        })
    }

    /// Schedules a new Interval job
    ///
    /// # Arguments
    /// * `command` - The command to execute
    /// * `minutes` - Interval in minutes (must be >= 2)
    /// * `args` - Optional command arguments
    ///
    /// # Returns
    /// * `Ok(ScheduleResult)` - Job scheduled successfully
    /// * `Err(String)` - If the interval is invalid
    pub async fn schedule_interval(
        &self,
        command: String,
        minutes: u32,
        args: Option<Vec<String>>,
    ) -> Result<ScheduleResult, String> {
        // Validate minimum interval
        if minutes < types::MIN_INTERVAL_MINUTES {
            return Err(format!(
                "Interval must be at least {} minutes, got {} minutes",
                types::MIN_INTERVAL_MINUTES,
                minutes
            ));
        }

        let job_id = Self::generate_job_id();
        let job = Job::new_interval(job_id.clone(), command, minutes, args);

        let next_execution = job.next_execution().unwrap();

        {
            let mut jobs = self.jobs.write().await;
            jobs.insert(job_id.clone(), job);
        }

        info!(
            job_id = %job_id,
            minutes = %minutes,
            next_execution = %next_execution,
            "Scheduled Interval job"
        );

        Ok(ScheduleResult {
            success: true,
            job_id,
            message: format!(
                "Interval job scheduled successfully (every {} minutes)",
                minutes
            ),
            next_execution,
        })
    }

    /// Lists all active (scheduled, running, or failed) jobs
    ///
    /// Returns jobs sorted by next execution time.
    /// Includes Failed jobs so users can see error status.
    pub async fn list_jobs(&self) -> ListResult {
        let jobs = self.jobs.read().await;

        let mut job_infos: Vec<_> = jobs
            .values()
            .filter(|job| {
                matches!(
                    job.status,
                    JobStatus::Scheduled | JobStatus::Running | JobStatus::Failed
                )
            })
            .map(|job| job.into())
            .collect();

        // Sort by next execution time
        job_infos.sort_by_key(|info: &types::JobInfo| info.next_execution);

        ListResult { jobs: job_infos }
    }

    /// Cancels a scheduled job
    ///
    /// # Arguments
    /// * `job_id` - The ID of the job to cancel
    ///
    /// # Returns
    /// * `Ok(CancelResult)` - Job cancelled and removed successfully
    /// * `Err(String)` - If the job doesn't exist
    pub async fn cancel_job(&self, job_id: &str) -> Result<CancelResult, String> {
        let mut jobs = self.jobs.write().await;

        if jobs.remove(job_id).is_some() {
            info!(
                job_id = %job_id,
                "Cancelled and removed job"
            );

            Ok(CancelResult {
                success: true,
                message: format!("Job '{}' cancelled successfully", job_id),
            })
        } else {
            Err(format!("Job '{}' not found", job_id))
        }
    }

    /// Gets a job by ID
    pub async fn get_job(&self, job_id: &str) -> Option<Job> {
        let jobs = self.jobs.read().await;
        jobs.get(job_id).cloned()
    }

    /// Gets all jobs due for execution
    ///
    /// Returns a vector of (job_id, job) tuples for jobs that are scheduled
    /// and have a next_execution time <= now
    pub async fn get_due_jobs(&self) -> Vec<(String, Job)> {
        let jobs = self.jobs.read().await;
        let now = Utc::now();

        jobs.iter()
            .filter(|(_, job)| job.status == JobStatus::Scheduled && job.is_due(now))
            .map(|(id, job)| (id.clone(), job.clone()))
            .collect()
    }

    /// Updates a job in the scheduler
    pub async fn update_job(&self, job: Job) {
        let mut jobs = self.jobs.write().await;
        jobs.insert(job.id.clone(), job);
    }

    /// Atomically update a job's status after execution
    ///
    /// This prevents race conditions by holding the write lock while modifying the job.
    /// Returns Ok(()) if successful, Err if job not found.
    pub async fn mark_job_executed(&self, job_id: &str) -> Result<(), String> {
        let mut jobs = self.jobs.write().await;

        if let Some(job) = jobs.get_mut(job_id) {
            job.mark_executed();
            Ok(())
        } else {
            Err(format!("Job '{}' not found", job_id))
        }
    }

    /// Atomically update a job's status after failure
    ///
    /// This prevents race conditions by holding the write lock while modifying the job.
    /// Returns Ok(()) if successful, Err if job not found.
    pub async fn mark_job_failed(&self, job_id: &str, error: String) -> Result<(), String> {
        let mut jobs = self.jobs.write().await;

        if let Some(job) = jobs.get_mut(job_id) {
            job.mark_failed(error);
            Ok(())
        } else {
            Err(format!("Job '{}' not found", job_id))
        }
    }

    /// Atomically mark a job as running
    ///
    /// This prevents race conditions by holding the write lock while modifying the job.
    /// Returns Ok(()) if successful, Err if job not found.
    pub async fn mark_job_running(&self, job_id: &str) -> Result<(), String> {
        let mut jobs = self.jobs.write().await;

        if let Some(job) = jobs.get_mut(job_id) {
            job.mark_running();
            Ok(())
        } else {
            Err(format!("Job '{}' not found", job_id))
        }
    }

    /// Removes completed or failed FireAt jobs
    ///
    /// This should be called periodically to clean up old jobs.
    /// Removes jobs with status: Completed, Cancelled, or Failed (FireAt only).
    pub async fn cleanup_completed_jobs(&self) {
        let mut jobs = self.jobs.write().await;

        let to_remove: Vec<_> = jobs
            .iter()
            .filter(|(_, job)| {
                matches!(
                    job.status,
                    JobStatus::Completed | JobStatus::Cancelled | JobStatus::Failed
                )
            })
            .map(|(id, _)| id.clone())
            .collect();

        for id in to_remove {
            jobs.remove(&id);
            debug!(job_id = %id, "Removed completed/cancelled/failed job");
        }
    }

    /// Starts the scheduler loop
    ///
    /// This spawns a background task that checks for due jobs every minute
    /// and executes them. The task runs indefinitely until the program exits.
    ///
    /// # Returns
    /// A JoinHandle for the scheduler task
    pub fn start_scheduler(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));

            info!("Cron scheduler started");

            loop {
                interval.tick().await;

                debug!("Checking for due jobs");

                // Get due jobs
                let due_jobs = self.get_due_jobs().await;

                if !due_jobs.is_empty() {
                    info!(count = %due_jobs.len(), "Found due jobs");

                    // Execute each due job
                    for (job_id, job) in due_jobs {
                        // Mark as running atomically
                        if let Err(e) = self.mark_job_running(&job_id).await {
                            error!(
                                job_id = %job_id,
                                error = %e,
                                "Failed to mark job as running"
                            );
                            continue;
                        }

                        // Spawn job execution in background
                        let scheduler = self.clone();
                        tokio::spawn(async move {
                            info!(
                                job_id = %job_id,
                                command = %job.command,
                                "Executing job"
                            );

                            // Execute the command
                            match execute_job(&job).await {
                                Ok(output) => {
                                    info!(
                                        job_id = %job_id,
                                        output = %output,
                                        "Job executed successfully"
                                    );

                                    // Update job status atomically
                                    if let Err(e) = scheduler.mark_job_executed(&job_id).await {
                                        error!(
                                            job_id = %job_id,
                                            error = %e,
                                            "Failed to mark job as executed (possibly cancelled)"
                                        );
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        job_id = %job_id,
                                        error = %e,
                                        "Job execution failed"
                                    );

                                    // Update job status atomically
                                    if let Err(err) = scheduler.mark_job_failed(&job_id, e).await {
                                        error!(
                                            job_id = %job_id,
                                            error = %err,
                                            "Failed to mark job as failed (possibly cancelled)"
                                        );
                                    }
                                }
                            }
                        });
                    }
                }

                // Cleanup completed jobs periodically
                self.cleanup_completed_jobs().await;
            }
        })
    }
}

impl Default for CronScheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Executes a job command
///
/// This is a helper function that executes the job's command using
/// tokio::process::Command. It applies the same security blacklist
/// as the exec tool.
///
/// # Returns
/// The stdout output of the command, or an error message
async fn execute_job(job: &Job) -> Result<String, String> {
    use tokio::process::Command;

    // Security: Check command against blacklist
    const BLACKLIST: &[&str] = &[
        "rm", "sudo", "dd", "mkfs", "shutdown", "reboot", "passwd", "visudo",
    ];

    // Extract basename for blacklist check
    let base_cmd = std::path::Path::new(&job.command)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(&job.command)
        .to_lowercase();

    if BLACKLIST.contains(&base_cmd.as_str()) {
        return Err(format!(
            "Command '{}' is blacklisted for security reasons",
            job.command
        ));
    }

    // Additional security: Check if command contains absolute paths to blacklisted commands
    // This prevents bypassing with /bin/rm, /usr/bin/sudo, etc.
    let cmd_lower = job.command.to_lowercase();
    for blacklisted in BLACKLIST {
        if cmd_lower.contains(&format!("/{}", blacklisted)) {
            return Err(format!(
                "Command '{}' contains blacklisted command '{}' and is blocked for security reasons",
                job.command, blacklisted
            ));
        }
    }

    // Build the command
    let mut cmd = Command::new(&job.command);

    if let Some(args) = &job.args {
        cmd.args(args);
    }

    // Execute with timeout
    match tokio::time::timeout(Duration::from_secs(30), cmd.output()).await {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if output.status.success() {
                Ok(stdout.to_string())
            } else {
                Err(format!(
                    "Command failed with exit code {:?}: {}",
                    output.status.code(),
                    if stderr.is_empty() {
                        stdout.to_string()
                    } else {
                        stderr.to_string()
                    }
                ))
            }
        }
        Ok(Err(e)) => Err(format!("Failed to execute command: {}", e)),
        Err(_) => Err("Command timed out after 30 seconds".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[tokio::test]
    async fn test_scheduler_creation() {
        let scheduler = CronScheduler::new();
        let jobs = scheduler.list_jobs().await;
        assert!(jobs.jobs.is_empty());
    }

    #[tokio::test]
    async fn test_schedule_fire_at() {
        let scheduler = CronScheduler::new();
        let execute_at = (Utc::now() + Duration::hours(1)).to_rfc3339();

        let result = scheduler
            .schedule_fire_at(
                "echo".to_string(),
                execute_at,
                Some(vec!["hello".to_string()]),
            )
            .await;

        assert!(result.is_ok());
        let schedule_result = result.unwrap();
        assert!(schedule_result.success);
        assert!(schedule_result.job_id.starts_with("job_"));
    }

    #[tokio::test]
    async fn test_schedule_fire_at_past_time() {
        let scheduler = CronScheduler::new();
        let execute_at = (Utc::now() - Duration::hours(1)).to_rfc3339();

        let result = scheduler
            .schedule_fire_at("echo".to_string(), execute_at, None)
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("in the past"));
    }

    #[tokio::test]
    async fn test_schedule_interval() {
        let scheduler = CronScheduler::new();

        let result = scheduler
            .schedule_interval("echo".to_string(), 5, Some(vec!["test".to_string()]))
            .await;

        assert!(result.is_ok());
        let schedule_result = result.unwrap();
        assert!(schedule_result.success);
        assert!(schedule_result.message.contains("every 5 minutes"));
    }

    #[tokio::test]
    async fn test_schedule_interval_too_small() {
        let scheduler = CronScheduler::new();

        let result = scheduler
            .schedule_interval("echo".to_string(), 1, None)
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at least 2 minutes"));
    }

    #[tokio::test]
    async fn test_list_jobs() {
        let scheduler = CronScheduler::new();

        // Schedule FireAt job with time far in the future
        let execute_at = (Utc::now() + Duration::days(1)).to_rfc3339();
        let result1 = scheduler
            .schedule_fire_at("echo".to_string(), execute_at.clone(), None)
            .await;
        assert!(
            result1.is_ok(),
            "Failed to schedule FireAt job: {:?}",
            result1.err()
        );

        // Schedule Interval job
        let result2 = scheduler
            .schedule_interval("ls".to_string(), 10, None)
            .await;
        assert!(
            result2.is_ok(),
            "Failed to schedule Interval job: {:?}",
            result2.err()
        );

        let jobs = scheduler.list_jobs().await;
        assert_eq!(
            jobs.jobs.len(),
            2,
            "Expected 2 jobs but found {}",
            jobs.jobs.len()
        );
    }

    #[tokio::test]
    async fn test_cancel_job() {
        let scheduler = CronScheduler::new();

        let execute_at = (Utc::now() + Duration::hours(1)).to_rfc3339();
        let result = scheduler
            .schedule_fire_at("echo".to_string(), execute_at, None)
            .await
            .unwrap();

        let job_id = result.job_id;

        // Cancel the job
        let cancel_result = scheduler.cancel_job(&job_id).await;
        assert!(cancel_result.is_ok());

        let cancel = cancel_result.unwrap();
        assert!(cancel.success);

        // Verify job is no longer in active list
        let jobs = scheduler.list_jobs().await;
        assert_eq!(jobs.jobs.len(), 0);
    }

    #[tokio::test]
    async fn test_cancel_nonexistent_job() {
        let scheduler = CronScheduler::new();

        let result = scheduler.cancel_job("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_due_jobs() {
        let scheduler = CronScheduler::new();

        // Create a job in the past directly (bypass validation)
        let execute_at = Utc::now() - Duration::minutes(1);
        let job = crate::cron::types::Job::new_fire_at(
            "test-due-job".to_string(),
            "echo".to_string(),
            execute_at,
            None,
        );

        // Insert directly into scheduler storage
        {
            let mut jobs = scheduler.jobs.write().await;
            jobs.insert(job.id.clone(), job);
        }

        let due_jobs = scheduler.get_due_jobs().await;
        assert_eq!(due_jobs.len(), 1);
        assert_eq!(due_jobs[0].0, "test-due-job");
    }

    #[tokio::test]
    async fn test_cleanup_completed_jobs() {
        let scheduler = CronScheduler::new();

        // Create a completed job directly (bypass validation)
        let execute_at = Utc::now() - Duration::minutes(5);
        let mut job = crate::cron::types::Job::new_fire_at(
            "test-cleanup-job".to_string(),
            "echo".to_string(),
            execute_at,
            None,
        );
        job.mark_executed(); // This marks it as Completed

        // Insert directly into scheduler storage
        {
            let mut jobs = scheduler.jobs.write().await;
            jobs.insert(job.id.clone(), job);
        }

        // Verify job exists
        assert!(scheduler.get_job("test-cleanup-job").await.is_some());

        // Cleanup
        scheduler.cleanup_completed_jobs().await;

        // Verify job is removed
        let job = scheduler.get_job("test-cleanup-job").await;
        assert!(job.is_none());
    }

    #[tokio::test]
    async fn test_job_id_generation() {
        let id1 = CronScheduler::generate_job_id();
        let id2 = CronScheduler::generate_job_id();

        assert!(id1.starts_with("job_"));
        assert!(id2.starts_with("job_"));
        assert_ne!(id1, id2); // Should be unique even when generated in same millisecond
    }

    #[tokio::test]
    async fn test_default_impl() {
        let scheduler: CronScheduler = Default::default();
        let jobs = scheduler.list_jobs().await;
        assert!(jobs.jobs.is_empty());
    }

    #[tokio::test]
    async fn test_execute_job_success() {
        let job = crate::cron::types::Job::new_fire_at(
            "test-exec".to_string(),
            "echo".to_string(),
            Utc::now() + Duration::hours(1),
            Some(vec!["hello".to_string()]),
        );

        let result = execute_job(&job).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("hello"));
    }

    #[tokio::test]
    async fn test_execute_job_blacklisted() {
        let job = crate::cron::types::Job::new_fire_at(
            "test-blocked".to_string(),
            "rm".to_string(),
            Utc::now() + Duration::hours(1),
            Some(vec!["-rf".to_string(), "/".to_string()]),
        );

        let result = execute_job(&job).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("blacklisted"));
    }

    #[tokio::test]
    async fn test_execute_job_blacklisted_with_path() {
        let job = crate::cron::types::Job::new_fire_at(
            "test-blocked-path".to_string(),
            "/bin/rm".to_string(),
            Utc::now() + Duration::hours(1),
            Some(vec!["-rf".to_string()]),
        );

        let result = execute_job(&job).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("blacklisted"));
    }

    #[tokio::test]
    async fn test_execute_job_nonexistent_command() {
        let job = crate::cron::types::Job::new_fire_at(
            "test-nonexist".to_string(),
            "nonexistent_command_xyz123".to_string(),
            Utc::now() + Duration::hours(1),
            None,
        );

        let result = execute_job(&job).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mark_failed_preserves_interval_status() {
        let mut job = crate::cron::types::Job::new_interval(
            "test-interval-fail".to_string(),
            "echo".to_string(),
            5,
            None,
        );

        job.mark_failed("Test error".to_string());

        // Interval jobs should remain Scheduled after failure (AC#8)
        assert_eq!(job.status, JobStatus::Scheduled);
        assert_eq!(job.last_error, Some("Test error".to_string()));
    }

    #[tokio::test]
    async fn test_mark_failed_marks_fire_at_as_failed() {
        let mut job = crate::cron::types::Job::new_fire_at(
            "test-fireat-fail".to_string(),
            "echo".to_string(),
            Utc::now() + Duration::hours(1),
            None,
        );

        job.mark_failed("Test error".to_string());

        // FireAt jobs should be marked as Failed after failure (AC#8)
        assert_eq!(job.status, JobStatus::Failed);
        assert_eq!(job.last_error, Some("Test error".to_string()));
    }
}
