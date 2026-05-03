//! Repository-backed service job operations.

use serde_json::json;

use super::service_model::{JobState, ServiceJob, ServiceState};
use super::service_store::{LockedServiceStateRepository, ServiceStateRepository};

pub const MAX_SERVICE_JOBS: usize = 200;

pub fn mutate_persisted_service_jobs(mutator: impl FnOnce(&mut ServiceState)) {
    if let Ok(repository) = LockedServiceStateRepository::default_json() {
        let _ = mutate_service_jobs_in_repository(&repository, mutator);
    }
}

pub fn mutate_service_jobs_in_repository(
    repository: &impl ServiceStateRepository,
    mutator: impl FnOnce(&mut ServiceState),
) -> Result<(), String> {
    repository.mutate(|state| {
        mutator(state);
        prune_service_jobs(state);
        Ok(())
    })
}

pub fn cancel_persisted_service_job(
    job_id: &str,
    reason: Option<&str>,
) -> Result<ServiceJob, String> {
    LockedServiceStateRepository::default_json()
        .and_then(|repository| cancel_service_job_in_repository(&repository, job_id, reason))
        .map_err(cancel_persisted_service_job_response_error)
}

pub fn cancel_service_job_in_repository(
    repository: &impl ServiceStateRepository,
    job_id: &str,
    reason: Option<&str>,
) -> Result<ServiceJob, String> {
    repository.mutate(|state| {
        let job = state
            .jobs
            .get_mut(job_id)
            .ok_or_else(|| format!("Service job not found: {}", job_id))?;

        match job.state {
            JobState::Queued | JobState::WaitingProfileLease => {
                job.state = JobState::Cancelled;
                job.completed_at = Some(current_timestamp());
                job.error = Some(
                    reason
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or("Cancelled by operator")
                        .to_string(),
                );
                job.result = Some(json!({ "success": false, "cancelled": true }));
                Ok(job.clone())
            }
            JobState::Cancelled => Ok(job.clone()),
            JobState::Running => Err(format!(
                "Service job {} is already running and cannot be cancelled safely",
                job_id
            )),
            JobState::Succeeded | JobState::Failed | JobState::TimedOut => Err(format!(
                "Service job {} is already terminal with state {}",
                job_id,
                job_state_name(job.state)
            )),
        }
    })
}

pub fn load_service_job_in_repository(
    repository: &impl ServiceStateRepository,
    id: &str,
) -> Option<ServiceJob> {
    repository.load_snapshot().ok()?.jobs.remove(id)
}

pub fn cancel_persisted_service_job_response_error(err: String) -> String {
    if err.starts_with("Failed to") || err.starts_with("Invalid service state") {
        format!("Unable to load service state: {}", err)
    } else {
        err
    }
}

fn prune_service_jobs(state: &mut ServiceState) {
    if state.jobs.len() <= MAX_SERVICE_JOBS {
        return;
    }
    let mut jobs = state
        .jobs
        .values()
        .map(|job| (job.submitted_at.clone().unwrap_or_default(), job.id.clone()))
        .collect::<Vec<_>>();
    jobs.sort();
    let excess = state.jobs.len() - MAX_SERVICE_JOBS;
    for (_, id) in jobs.into_iter().take(excess) {
        state.jobs.remove(&id);
    }
}

fn job_state_name(state: JobState) -> &'static str {
    match state {
        JobState::Queued => "queued",
        JobState::WaitingProfileLease => "waiting_profile_lease",
        JobState::Running => "running",
        JobState::Succeeded => "succeeded",
        JobState::Failed => "failed",
        JobState::Cancelled => "cancelled",
        JobState::TimedOut => "timed_out",
    }
}

fn current_timestamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}
