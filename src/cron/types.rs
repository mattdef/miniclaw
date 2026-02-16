//! Types for the cron scheduling system
//!
//! This module defines the core types for job scheduling including:
//! - Job struct for scheduled tasks
//! - JobType enum for FireAt and Interval jobs
//! - JobStatus enum for tracking job execution state

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Minimum interval in minutes for recurring jobs
pub const MIN_INTERVAL_MINUTES: u32 = 2;

/// Type of scheduled job
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    /// One-time job that executes at a specific time
    FireAt {
        /// The time when the job should execute
        execute_at: DateTime<Utc>,
    },
    /// Recurring job that executes at regular intervals
    Interval {
        /// Interval in minutes (minimum 2)
        minutes: u32,
        /// The last time this job was executed
        last_executed: Option<DateTime<Utc>>,
        /// The next scheduled execution time
        next_execution: DateTime<Utc>,
    },
}

impl JobType {
    /// Get the type name as a string
    pub fn type_name(&self) -> &'static str {
        match self {
            JobType::FireAt { .. } => "fire_at",
            JobType::Interval { .. } => "interval",
        }
    }

    /// Calculate the next execution time for this job type
    pub fn calculate_next_execution(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            JobType::FireAt { execute_at } => {
                if *execute_at > now {
                    Some(*execute_at)
                } else {
                    None // Already executed or passed
                }
            }
            JobType::Interval {
                minutes,
                next_execution,
                ..
            } => {
                if *next_execution > now {
                    Some(*next_execution)
                } else {
                    // Calculate next execution based on interval
                    Some(now + chrono::Duration::minutes(*minutes as i64))
                }
            }
        }
    }
}

/// Status of a job in the scheduler
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// Job is scheduled and waiting to execute
    Scheduled,
    /// Job is currently executing
    Running,
    /// Job has completed successfully (FireAt only)
    Completed,
    /// Job failed during execution
    Failed,
    /// Job was cancelled by user
    Cancelled,
}

/// A scheduled job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique identifier for the job
    pub id: String,
    /// Type of job (FireAt or Interval)
    pub job_type: JobType,
    /// Command to execute
    pub command: String,
    /// Arguments for the command (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// Current status of the job
    #[serde(skip_serializing)]
    pub status: JobStatus,
    /// When the job was created
    pub created_at: DateTime<Utc>,
    /// Number of times the job has been executed
    pub execution_count: u32,
    /// Last error message if job failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

impl Job {
    /// Creates a new FireAt job
    ///
    /// # Arguments
    /// * `id` - Unique job identifier
    /// * `command` - Command to execute
    /// * `execute_at` - When to execute the job
    /// * `args` - Optional command arguments
    ///
    /// # Returns
    /// A new Job configured as a FireAt job
    pub fn new_fire_at(
        id: String,
        command: String,
        execute_at: DateTime<Utc>,
        args: Option<Vec<String>>,
    ) -> Self {
        Self {
            id,
            job_type: JobType::FireAt { execute_at },
            command,
            args,
            status: JobStatus::Scheduled,
            created_at: Utc::now(),
            execution_count: 0,
            last_error: None,
        }
    }

    /// Creates a new Interval job
    ///
    /// # Arguments
    /// * `id` - Unique job identifier
    /// * `command` - Command to execute
    /// * `minutes` - Interval in minutes (must be >= 2)
    /// * `args` - Optional command arguments
    ///
    /// # Returns
    /// A new Job configured as an Interval job
    ///
    /// # Panics
    /// Panics if minutes < MIN_INTERVAL_MINUTES
    pub fn new_interval(
        id: String,
        command: String,
        minutes: u32,
        args: Option<Vec<String>>,
    ) -> Self {
        assert!(
            minutes >= MIN_INTERVAL_MINUTES,
            "Interval must be at least {} minutes",
            MIN_INTERVAL_MINUTES
        );

        let now = Utc::now();
        let next_execution = now + chrono::Duration::minutes(minutes as i64);

        Self {
            id,
            job_type: JobType::Interval {
                minutes,
                last_executed: None,
                next_execution,
            },
            command,
            args,
            status: JobStatus::Scheduled,
            created_at: now,
            execution_count: 0,
            last_error: None,
        }
    }

    /// Get the next execution time for this job
    pub fn next_execution(&self) -> Option<DateTime<Utc>> {
        match &self.job_type {
            JobType::FireAt { execute_at } => Some(*execute_at),
            JobType::Interval { next_execution, .. } => Some(*next_execution),
        }
    }

    /// Check if this job is due for execution
    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        match self.next_execution() {
            Some(exec_time) => exec_time <= now,
            None => false,
        }
    }

    /// Mark the job as executed and update state
    ///
    /// For Interval jobs, updates last_executed and calculates next_execution.
    /// For FireAt jobs, marks as Completed.
    pub fn mark_executed(&mut self) {
        self.execution_count += 1;
        self.last_error = None;

        match &mut self.job_type {
            JobType::FireAt { .. } => {
                self.status = JobStatus::Completed;
            }
            JobType::Interval {
                minutes,
                last_executed,
                next_execution,
            } => {
                let now = Utc::now();
                *last_executed = Some(now);
                *next_execution = now + chrono::Duration::minutes(*minutes as i64);
                self.status = JobStatus::Scheduled;
            }
        }
    }

    /// Mark the job as failed
    ///
    /// For Interval jobs, preserves Scheduled status so they continue executing.
    /// For FireAt jobs, marks as Failed permanently.
    pub fn mark_failed(&mut self, error: String) {
        self.last_error = Some(error);

        match &self.job_type {
            JobType::FireAt { .. } => {
                self.status = JobStatus::Failed;
            }
            JobType::Interval { .. } => {
                // Keep status as Scheduled so interval jobs continue
                // (AC#8: Interval jobs continue, don't stop on failure)
                self.status = JobStatus::Scheduled;
            }
        }
    }

    /// Mark the job as running
    pub fn mark_running(&mut self) {
        self.status = JobStatus::Running;
    }

    /// Get the interval in minutes (if this is an Interval job)
    pub fn interval_minutes(&self) -> Option<u32> {
        match &self.job_type {
            JobType::Interval { minutes, .. } => Some(*minutes),
            _ => None,
        }
    }
}

/// Information about a job for listing purposes
#[derive(Debug, Clone, Serialize)]
pub struct JobInfo {
    pub id: String,
    pub job_type: String,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    pub next_execution: DateTime<Utc>,
    pub execution_count: u32,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

impl From<&Job> for JobInfo {
    fn from(job: &Job) -> Self {
        // For completed/failed FireAt jobs, show their original execution time
        // For interval jobs or pending jobs, show next_execution
        let next_execution = match &job.job_type {
            JobType::FireAt { execute_at } => *execute_at,
            JobType::Interval { next_execution, .. } => *next_execution,
        };

        Self {
            id: job.id.clone(),
            job_type: job.job_type.type_name().to_string(),
            command: job.command.clone(),
            args: job.args.clone(),
            next_execution,
            execution_count: job.execution_count,
            status: format!("{:?}", job.status).to_lowercase(),
            last_error: job.last_error.clone(),
        }
    }
}

/// Result of scheduling a job
#[derive(Debug, Clone, Serialize)]
pub struct ScheduleResult {
    pub success: bool,
    pub job_id: String,
    pub message: String,
    pub next_execution: DateTime<Utc>,
}

/// Result of cancelling a job
#[derive(Debug, Clone, Serialize)]
pub struct CancelResult {
    pub success: bool,
    pub message: String,
}

/// Result of listing jobs
#[derive(Debug, Clone, Serialize)]
pub struct ListResult {
    pub jobs: Vec<JobInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_type_fire_at() {
        let execute_at = Utc::now() + chrono::Duration::hours(1);
        let job_type = JobType::FireAt { execute_at };

        assert_eq!(job_type.type_name(), "fire_at");
    }

    #[test]
    fn test_job_type_interval() {
        let now = Utc::now();
        let job_type = JobType::Interval {
            minutes: 5,
            last_executed: None,
            next_execution: now + chrono::Duration::minutes(5),
        };

        assert_eq!(job_type.type_name(), "interval");
    }

    #[test]
    fn test_job_fire_at_creation() {
        let execute_at = Utc::now() + chrono::Duration::hours(1);
        let job = Job::new_fire_at(
            "test-1".to_string(),
            "echo".to_string(),
            execute_at,
            Some(vec!["hello".to_string()]),
        );

        assert_eq!(job.id, "test-1");
        assert_eq!(job.command, "echo");
        assert_eq!(job.args, Some(vec!["hello".to_string()]));
        assert_eq!(job.execution_count, 0);
        assert_eq!(job.status, JobStatus::Scheduled);
    }

    #[test]
    fn test_job_interval_creation() {
        let job = Job::new_interval(
            "test-2".to_string(),
            "echo".to_string(),
            5,
            Some(vec!["world".to_string()]),
        );

        assert_eq!(job.id, "test-2");
        assert_eq!(job.command, "echo");
        assert_eq!(job.interval_minutes(), Some(5));
        assert_eq!(job.execution_count, 0);
    }

    #[test]
    #[should_panic(expected = "Interval must be at least")]
    fn test_job_interval_min_too_small() {
        Job::new_interval(
            "test-3".to_string(),
            "echo".to_string(),
            1, // Less than MIN_INTERVAL_MINUTES
            None,
        );
    }

    #[test]
    fn test_job_is_due() {
        let execute_at = Utc::now() - chrono::Duration::minutes(1);
        let job = Job::new_fire_at("test-4".to_string(), "echo".to_string(), execute_at, None);

        assert!(job.is_due(Utc::now()));
    }

    #[test]
    fn test_job_is_not_due() {
        let execute_at = Utc::now() + chrono::Duration::hours(1);
        let job = Job::new_fire_at("test-5".to_string(), "echo".to_string(), execute_at, None);

        assert!(!job.is_due(Utc::now()));
    }

    #[test]
    fn test_job_mark_executed_fire_at() {
        let execute_at = Utc::now() + chrono::Duration::hours(1);
        let mut job = Job::new_fire_at("test-6".to_string(), "echo".to_string(), execute_at, None);

        job.mark_executed();

        assert_eq!(job.execution_count, 1);
        assert_eq!(job.status, JobStatus::Completed);
    }

    #[test]
    fn test_job_mark_executed_interval() {
        let mut job = Job::new_interval("test-7".to_string(), "echo".to_string(), 5, None);

        let original_next = job.next_execution().unwrap();
        job.mark_executed();

        assert_eq!(job.execution_count, 1);
        assert_eq!(job.status, JobStatus::Scheduled);
        // Next execution should be updated to the future
        assert!(job.next_execution().unwrap() > original_next);
    }

    #[test]
    fn test_job_mark_failed() {
        // Test with Interval job - should remain Scheduled per AC#8
        let mut interval_job =
            Job::new_interval("test-8-interval".to_string(), "echo".to_string(), 5, None);

        interval_job.mark_failed("Command not found".to_string());

        // Interval jobs remain Scheduled (AC#8: Interval jobs continue, don't stop on failure)
        assert_eq!(interval_job.status, JobStatus::Scheduled);
        assert_eq!(
            interval_job.last_error,
            Some("Command not found".to_string())
        );

        // Test with FireAt job - should be marked Failed
        let mut fireat_job = Job::new_fire_at(
            "test-8-fireat".to_string(),
            "echo".to_string(),
            Utc::now() + chrono::Duration::hours(1),
            None,
        );

        fireat_job.mark_failed("Command not found".to_string());

        // FireAt jobs are marked Failed
        assert_eq!(fireat_job.status, JobStatus::Failed);
        assert_eq!(fireat_job.last_error, Some("Command not found".to_string()));
    }

    #[test]
    fn test_job_info_from_job() {
        let job = Job::new_interval("test-9".to_string(), "echo hello".to_string(), 10, None);

        let info: JobInfo = (&job).into();

        assert_eq!(info.id, "test-9");
        assert_eq!(info.job_type, "interval");
        assert_eq!(info.command, "echo hello");
        assert_eq!(info.execution_count, 0);
    }

    #[test]
    fn test_min_interval_constant() {
        assert_eq!(MIN_INTERVAL_MINUTES, 2);
    }
}
