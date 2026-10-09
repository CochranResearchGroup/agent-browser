//! Mutation custody and exact public outcomes. Persistence stays in adapters.
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    RemoteViewApplicationActivation, RemoteViewApplicationAdapter,
    RemoteViewApplicationAdapterError, RemoteViewApplicationCleanupPermit,
    RemoteViewApplicationEnvelope, RemoteViewApplicationGrant, RemoteViewApplicationRequest,
    RemoteViewApplicationResponseError, RemoteViewApplicationTransport,
    RemoteViewApplicationViewCapability, RemoteViewApplicationViewIssuance,
    RemoteViewApplicationViewRevocation, RemoteViewAssignmentRecord, RemoteViewAssignmentState,
    RemoteViewJoinedReleaseOutcome,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "response",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RemoteViewApplicationMutationOutcome {
    Acquire(RemoteViewAssignmentRecord),
    Activate(RemoteViewApplicationActivation),
    IssueView(RemoteViewApplicationViewIssuance),
    /// Provider proved the exact retained grant terminal; no usable view remains.
    IssueViewTerminal,
    RevokeView(RemoteViewApplicationViewRevocation),
    Release(RemoteViewJoinedReleaseOutcome),
    /// Exact public response proves the guarded release made no effects.
    ReleaseIdleDeferred,
}

/// A pending record survives ambiguous transport or process interruption.
/// It contains only the exact public request and a qualified public outcome.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewApplicationMutationRecord {
    pub schema_version: u32,
    pub envelope: RemoteViewApplicationEnvelope,
    pub outcome: Option<RemoteViewApplicationMutationOutcome>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RemoteViewApplicationMutationClaim {
    New,
    Existing(Box<RemoteViewApplicationMutationRecord>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemoteViewApplicationMutationStoreError {
    Unavailable,
    Conflict,
    InvalidRecord,
}

/// Implementations atomically and durably install the exact pending request
/// before returning New. A concurrent caller receives Existing, never New.
/// Completion compares the entire pending request before committing. Failed
/// or ambiguous completion leaves the pending request intact for readback.
pub trait RemoteViewApplicationMutationStore {
    /// Read the immutable intent by mutation identity without changing custody.
    /// A returned record may have a different payload and must be qualified.
    fn read(
        &mut self,
        envelope: &RemoteViewApplicationEnvelope,
    ) -> Result<Option<RemoteViewApplicationMutationRecord>, RemoteViewApplicationMutationStoreError>;
    fn claim(
        &mut self,
        envelope: &RemoteViewApplicationEnvelope,
    ) -> Result<RemoteViewApplicationMutationClaim, RemoteViewApplicationMutationStoreError>;
    fn complete(
        &mut self,
        envelope: &RemoteViewApplicationEnvelope,
        outcome: &RemoteViewApplicationMutationOutcome,
    ) -> Result<(), RemoteViewApplicationMutationStoreError>;
}

impl RemoteViewApplicationEnvelope {
    /// Stable client mutation identity. It is not a Remote View operation ID.
    /// Changing the payload under this identity is a conflict, including a
    /// generation change. Revocation is naturally identified by opaque route.
    pub fn mutation_key(&self) -> Result<String, RemoteViewApplicationMutationStoreError> {
        if self.application.is_empty()
            || self.application.len() > 64
            || !self
                .application
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(RemoteViewApplicationMutationStoreError::InvalidRecord);
        }
        let key = match &self.request {
            RemoteViewApplicationRequest::Acquire {
                idempotency_key, ..
            }
            | RemoteViewApplicationRequest::Activate {
                idempotency_key, ..
            }
            | RemoteViewApplicationRequest::IssueView {
                idempotency_key, ..
            }
            | RemoteViewApplicationRequest::Release {
                idempotency_key, ..
            }
            | RemoteViewApplicationRequest::ReleaseIdle {
                idempotency_key, ..
            } => {
                if idempotency_key.is_empty()
                    || idempotency_key.len() > 128
                    || !idempotency_key.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')
                    })
                {
                    return Err(RemoteViewApplicationMutationStoreError::InvalidRecord);
                }
                format!("key:{idempotency_key}")
            }
            RemoteViewApplicationRequest::RevokeView { route_id, .. } => {
                if !crate::remote_view_application_response::viewing_route_id(route_id) {
                    return Err(RemoteViewApplicationMutationStoreError::InvalidRecord);
                }
                format!("revoke:{route_id}")
            }
            _ => return Err(RemoteViewApplicationMutationStoreError::InvalidRecord),
        };
        Ok(format!("{}:{key}", self.application))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteViewApplicationViewOptions {
    pub audience: String,
    pub capability: RemoteViewApplicationViewCapability,
    pub lifetime_seconds: u32,
    pub idempotency_key: String,
}

/// Complete expected retirement set from Agent Browser's retained public join.
/// This does not prove application references or process absence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteViewApplicationReleaseTarget {
    pub assignment: RemoteViewAssignmentRecord,
    pub route_ids: Vec<String>,
    pub viewer_session_ids: Vec<String>,
}

impl RemoteViewApplicationReleaseTarget {
    pub fn validate(&self) -> Result<(), RemoteViewApplicationResponseError> {
        let assignment = &self.assignment;
        if assignment.assignment_id.is_empty()
            || assignment.registration_id.is_empty()
            || assignment.pool_id.is_empty()
            || assignment.generation == 0
            || !crate::remote_view_application_response::uuid(&assignment.desktop_id)
            || !same_set(&self.route_ids, &self.route_ids)
            || !same_set(&self.viewer_session_ids, &self.viewer_session_ids)
            || self
                .route_ids
                .iter()
                .any(|route| !crate::remote_view_application_response::viewing_route_id(route))
        {
            return Err(RemoteViewApplicationResponseError::InvalidTarget);
        }
        Ok(())
    }

    pub fn validate_outcome(
        &self,
        outcome: &RemoteViewJoinedReleaseOutcome,
    ) -> Result<(), RemoteViewApplicationResponseError> {
        self.validate()?;
        let assignment = &self.assignment;
        let released = &outcome.assignment;
        let retirement = &outcome.retirement;
        if released.assignment_id != assignment.assignment_id
            || released.registration_id != assignment.registration_id
            || released.pool_id != assignment.pool_id
            || released.desktop_id != assignment.desktop_id
            || released.generation != assignment.generation
            || released.state != RemoteViewAssignmentState::Released
            || retirement.schema_version != 1
            || retirement.desktop_id != assignment.desktop_id
            || retirement.generation != assignment.generation
            || !same_set(&self.route_ids, &retirement.routes)
            || !same_set(&self.viewer_session_ids, &retirement.sessions)
        {
            return Err(RemoteViewApplicationResponseError::InvalidTarget);
        }
        Ok(())
    }
}

fn same_set(expected: &[String], actual: &[String]) -> bool {
    let expected_set: BTreeSet<_> = expected.iter().collect();
    let actual_set: BTreeSet<_> = actual.iter().collect();
    expected_set.len() == expected.len()
        && actual_set.len() == actual.len()
        && expected.iter().chain(actual).all(|value| !value.is_empty())
        && expected_set == actual_set
}

fn parse_outcome(
    request: &RemoteViewApplicationRequest,
    value: Value,
) -> Result<RemoteViewApplicationMutationOutcome, RemoteViewApplicationResponseError> {
    let outcome = match request {
        RemoteViewApplicationRequest::Acquire { .. } => {
            serde_json::from_value(value).map(RemoteViewApplicationMutationOutcome::Acquire)
        }
        RemoteViewApplicationRequest::Activate { .. } => {
            serde_json::from_value(value).map(RemoteViewApplicationMutationOutcome::Activate)
        }
        RemoteViewApplicationRequest::IssueView { .. } => {
            serde_json::from_value(value).map(RemoteViewApplicationMutationOutcome::IssueView)
        }
        RemoteViewApplicationRequest::RevokeView { .. } => {
            serde_json::from_value(value).map(RemoteViewApplicationMutationOutcome::RevokeView)
        }
        RemoteViewApplicationRequest::ReleaseIdle {
            assignment_id,
            expected_generation,
            ..
        } if value.get("state").and_then(Value::as_str) == Some("retained") => {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase", deny_unknown_fields)]
            struct Deferred {
                schema_version: u32,
                operation: String,
                state: String,
                assignment_id: String,
                generation: u64,
            }
            let retained: Deferred = serde_json::from_value(value)
                .map_err(|_| RemoteViewApplicationResponseError::InvalidShape)?;
            if retained.schema_version != 1
                || retained.operation != "release_idle"
                || retained.state != "retained"
                || &retained.assignment_id != assignment_id
                || retained.generation != *expected_generation
            {
                return Err(RemoteViewApplicationResponseError::InvalidTarget);
            }
            return Ok(RemoteViewApplicationMutationOutcome::ReleaseIdleDeferred);
        }
        RemoteViewApplicationRequest::Release { .. }
        | RemoteViewApplicationRequest::ReleaseIdle { .. } => {
            serde_json::from_value(value).map(RemoteViewApplicationMutationOutcome::Release)
        }
        _ => return Err(RemoteViewApplicationResponseError::InvalidShape),
    };
    outcome.map_err(|_| RemoteViewApplicationResponseError::InvalidShape)
}

impl<T: RemoteViewApplicationTransport> RemoteViewApplicationAdapter<T> {
    /// Resume only an exact fenced, idempotent idle release. A lost response
    /// cannot authorize a different target or a second provider retirement.
    pub fn reconcile_idle_release(
        &mut self,
        record: &RemoteViewApplicationMutationRecord,
        target: &RemoteViewApplicationReleaseTarget,
        store: &mut impl RemoteViewApplicationMutationStore,
    ) -> Result<RemoteViewApplicationMutationOutcome, RemoteViewApplicationAdapterError> {
        if record.schema_version != 1
            || record.envelope.application != self.application
            || record.outcome.is_some()
            || !matches!(&record.envelope.request,RemoteViewApplicationRequest::ReleaseIdle { assignment_id,expected_generation,expected_retirement,.. }
                if assignment_id == &target.assignment.assignment_id && *expected_generation == target.assignment.generation
                && expected_retirement.routes == target.route_ids && expected_retirement.sessions == target.viewer_session_ids
                && expected_retirement.desktop_id == target.assignment.desktop_id && expected_retirement.generation == target.assignment.generation)
        {
            return Err(RemoteViewApplicationResponseError::InvalidTarget.into());
        }
        match store
            .claim(&record.envelope)
            .map_err(RemoteViewApplicationAdapterError::MutationStore)?
        {
            RemoteViewApplicationMutationClaim::Existing(current) if *current == *record => (),
            _ => return Err(RemoteViewApplicationAdapterError::MutationReadbackRequired),
        }
        let response = self.request(record.envelope.request.clone())?;
        let outcome = parse_outcome(&record.envelope.request, response)?;
        match &outcome {
            RemoteViewApplicationMutationOutcome::Release(released) => {
                target.validate_outcome(released)?
            }
            RemoteViewApplicationMutationOutcome::ReleaseIdleDeferred => (),
            _ => return Err(RemoteViewApplicationResponseError::InvalidShape.into()),
        }
        store
            .complete(&record.envelope, &outcome)
            .map_err(RemoteViewApplicationAdapterError::MutationStore)?;
        Ok(outcome)
    }
    /// Consume current application cleanup admission and request owner-guarded
    /// idle release. The provider rechecks viewers while holding its native lock.
    pub fn release_idle(
        &mut self,
        assignment: &RemoteViewAssignmentRecord,
        observation: &crate::RemoteViewIdleAssignmentObservation,
        cleanup: RemoteViewApplicationCleanupPermit,
        idempotency_key: String,
        store: &mut impl RemoteViewApplicationMutationStore,
    ) -> Result<RemoteViewJoinedReleaseOutcome, RemoteViewApplicationAdapterError> {
        let target = observation.release_target(assignment)?;
        if !observation.idle {
            return Err(RemoteViewApplicationResponseError::InvalidTarget.into());
        }
        let cleanup = cleanup
            .into_acknowledgement(&target)
            .map_err(|_| RemoteViewApplicationResponseError::InvalidTarget)?;
        let request = RemoteViewApplicationRequest::ReleaseIdle {
            assignment_id: assignment.assignment_id.clone(),
            expected_generation: assignment.generation,
            expected_viewing_generation: observation.target.viewing_generation,
            idempotency_key,
            cleanup,
            expected_retirement: observation.retirement.clone(),
        };
        let outcome = self.mutate(request, store, |outcome, _| match outcome {
            RemoteViewApplicationMutationOutcome::Release(released) => {
                target.validate_outcome(released)
            }
            RemoteViewApplicationMutationOutcome::ReleaseIdleDeferred => Ok(()),
            _ => Err(RemoteViewApplicationResponseError::InvalidShape),
        })?;
        match outcome {
            RemoteViewApplicationMutationOutcome::Release(released) => Ok(released),
            RemoteViewApplicationMutationOutcome::ReleaseIdleDeferred => {
                Err(RemoteViewApplicationAdapterError::IdleReleaseDeferred)
            }
            _ => unreachable!(),
        }
    }
    /// Resolve an interrupted acquisition using its original provider idempotency
    /// key. Only acquisitions support this replay; all other uncertain effects
    /// retain their existing readback requirement.
    pub fn reconcile_acquisition(
        &mut self,
        record: &RemoteViewApplicationMutationRecord,
        store: &mut impl RemoteViewApplicationMutationStore,
    ) -> Result<(), RemoteViewApplicationAdapterError> {
        let RemoteViewApplicationRequest::Acquire { pool_name, .. } = &record.envelope.request
        else {
            return Err(RemoteViewApplicationResponseError::InvalidShape.into());
        };
        if record.schema_version != 1 || record.envelope.application != self.application {
            return Err(RemoteViewApplicationResponseError::InvalidTarget.into());
        }
        match store
            .claim(&record.envelope)
            .map_err(RemoteViewApplicationAdapterError::MutationStore)?
        {
            RemoteViewApplicationMutationClaim::Existing(current)
                if *current == *record && current.outcome.is_none() => {}
            _ => return Err(RemoteViewApplicationAdapterError::MutationReadbackRequired),
        }
        let outcome = parse_outcome(
            &record.envelope.request,
            self.request(record.envelope.request.clone())?,
        )?;
        let RemoteViewApplicationMutationOutcome::Acquire(assignment) = &outcome else {
            return Err(RemoteViewApplicationResponseError::InvalidShape.into());
        };
        let inventory = self.inventory()?;
        inventory.validate_acquisition(pool_name, assignment)?;
        if !inventory.assignments.contains(assignment) {
            return Err(RemoteViewApplicationResponseError::StaleTarget.into());
        }
        store
            .complete(&record.envelope, &outcome)
            .map_err(RemoteViewApplicationAdapterError::MutationStore)
    }

    fn mutate(
        &mut self,
        request: RemoteViewApplicationRequest,
        store: &mut impl RemoteViewApplicationMutationStore,
        validate: impl Fn(
            &RemoteViewApplicationMutationOutcome,
            bool,
        ) -> Result<(), RemoteViewApplicationResponseError>,
    ) -> Result<RemoteViewApplicationMutationOutcome, RemoteViewApplicationAdapterError> {
        let envelope = RemoteViewApplicationEnvelope {
            application: self.application.clone(),
            request,
        };
        envelope
            .mutation_key()
            .map_err(RemoteViewApplicationAdapterError::MutationStore)?;
        match store
            .claim(&envelope)
            .map_err(RemoteViewApplicationAdapterError::MutationStore)?
        {
            RemoteViewApplicationMutationClaim::Existing(record) => {
                if record.schema_version != 1 || record.envelope != envelope {
                    return Err(RemoteViewApplicationAdapterError::MutationStore(
                        RemoteViewApplicationMutationStoreError::Conflict,
                    ));
                }
                let outcome = record
                    .outcome
                    .ok_or(RemoteViewApplicationAdapterError::MutationReadbackRequired)?;
                validate(&outcome, true)?;
                Ok(outcome)
            }
            RemoteViewApplicationMutationClaim::New => {
                // Custody is durable before transport. Any subsequent error
                // retains it; repeated calls cannot automatically resubmit.
                let response = self.request(envelope.request.clone())?;
                let outcome = parse_outcome(&envelope.request, response)?;
                validate(&outcome, false)?;
                store
                    .complete(&envelope, &outcome)
                    .map_err(RemoteViewApplicationAdapterError::MutationStore)?;
                Ok(outcome)
            }
        }
    }

    pub fn acquire(
        &mut self,
        pool_name: String,
        idempotency_key: String,
        store: &mut impl RemoteViewApplicationMutationStore,
    ) -> Result<RemoteViewAssignmentRecord, RemoteViewApplicationAdapterError> {
        let inventory = self.inventory()?;
        if !inventory.pools.iter().any(|pool| pool.name == pool_name) {
            return Err(RemoteViewApplicationResponseError::InvalidTarget.into());
        }
        let request = RemoteViewApplicationRequest::Acquire {
            pool_name: pool_name.clone(),
            idempotency_key,
        };
        let outcome = self.mutate(request, store, |outcome, replay| match outcome {
            RemoteViewApplicationMutationOutcome::Acquire(assignment) => {
                if replay && !inventory.assignments.contains(assignment) {
                    return Err(RemoteViewApplicationResponseError::StaleTarget);
                }
                inventory.validate_acquisition(&pool_name, assignment)
            }
            _ => Err(RemoteViewApplicationResponseError::InvalidShape),
        })?;
        match outcome {
            RemoteViewApplicationMutationOutcome::Acquire(assignment) => Ok(assignment),
            _ => unreachable!(),
        }
    }

    pub fn activate(
        &mut self,
        assignment: &RemoteViewAssignmentRecord,
        window_id: u32,
        idempotency_key: String,
        store: &mut impl RemoteViewApplicationMutationStore,
    ) -> Result<RemoteViewApplicationActivation, RemoteViewApplicationAdapterError> {
        if window_id == 0 {
            return Err(RemoteViewApplicationResponseError::InvalidTarget.into());
        }
        let observed = self.observe_assignment(assignment)?;
        let request = RemoteViewApplicationRequest::Activate {
            assignment_id: assignment.assignment_id.clone(),
            expected_generation: observed.target.lifecycle_generation,
            expected_viewing_generation: observed.target.viewing_generation,
            window_id,
            idempotency_key,
        };
        let outcome = self.mutate(request, store, |outcome, _| match outcome {
            RemoteViewApplicationMutationOutcome::Activate(receipt) => {
                receipt.validate_target(&observed.target, window_id)
            }
            _ => Err(RemoteViewApplicationResponseError::InvalidShape),
        })?;
        let after = self.observe_assignment(assignment)?;
        if after.target != observed.target {
            return Err(RemoteViewApplicationResponseError::StaleTarget.into());
        }
        match outcome {
            RemoteViewApplicationMutationOutcome::Activate(receipt) => Ok(receipt),
            _ => unreachable!(),
        }
    }

    pub fn issue_view(
        &mut self,
        assignment: &RemoteViewAssignmentRecord,
        options: RemoteViewApplicationViewOptions,
        now_ms: impl Fn() -> u64,
        store: &mut impl RemoteViewApplicationMutationStore,
    ) -> Result<RemoteViewApplicationViewIssuance, RemoteViewApplicationAdapterError> {
        if options.lifetime_seconds == 0 {
            return Err(RemoteViewApplicationResponseError::InvalidShape.into());
        }
        let observed = self.observe_assignment(assignment)?;
        let application = self.application.clone();
        let request = RemoteViewApplicationRequest::IssueView {
            assignment_id: assignment.assignment_id.clone(),
            expected_generation: observed.target.lifecycle_generation,
            expected_viewing_generation: observed.target.viewing_generation,
            audience: options.audience.clone(),
            capability: options.capability,
            lifetime_seconds: options.lifetime_seconds,
            idempotency_key: options.idempotency_key,
        };
        let envelope = RemoteViewApplicationEnvelope {
            application: application.clone(),
            request,
        };
        envelope
            .mutation_key()
            .map_err(RemoteViewApplicationAdapterError::MutationStore)?;
        // A completed grant can outlive publication of its issuance into the
        // handoff. Read its original intent before claiming a new generation's
        // payload under the same key. Only qualified expiry permits renewal;
        // pending or malformed records retain their custody and conflict.
        if let Some(record) = store
            .read(&envelope)
            .map_err(RemoteViewApplicationAdapterError::MutationStore)?
        {
            if let Some(RemoteViewApplicationMutationOutcome::IssueView(view)) = &record.outcome {
                let mut prior_target = observed.target.clone();
                prior_target.lifecycle_generation = view.grant.request.target.lifecycle_generation;
                prior_target.viewing_generation = view.grant.request.target.viewing_generation;
                let mut prior_envelope = envelope.clone();
                if let RemoteViewApplicationRequest::IssueView {
                    expected_generation,
                    expected_viewing_generation,
                    ..
                } = &mut prior_envelope.request
                {
                    *expected_generation = prior_target.lifecycle_generation;
                    *expected_viewing_generation = prior_target.viewing_generation;
                }
                if record.schema_version == 1
                    && record.envelope == prior_envelope
                    && view.grant.expires_at <= now_ms()
                    && view
                        .validate_target(
                            &prior_target,
                            &application,
                            &options.audience,
                            options.capability,
                            options.lifetime_seconds,
                            view.grant.issued_at,
                        )
                        .is_ok()
                {
                    return Err(RemoteViewApplicationAdapterError::ViewGrantTerminal);
                }
            }
        }
        let outcome = match store
            .claim(&envelope)
            .map_err(RemoteViewApplicationAdapterError::MutationStore)?
        {
            RemoteViewApplicationMutationClaim::Existing(record) => {
                if record.schema_version != 1 || record.envelope != envelope {
                    return Err(RemoteViewApplicationAdapterError::MutationStore(
                        RemoteViewApplicationMutationStoreError::Conflict,
                    ));
                }
                record.outcome
            }
            RemoteViewApplicationMutationClaim::New => None,
        };
        let outcome = if let Some(outcome) = outcome {
            outcome
        } else {
            // Replay only this exact durable intent. Provider idempotency keeps
            // a lost reply from creating a second grant. Ambiguity retains custody.
            let outcome = match self.request(envelope.request.clone()) {
                Ok(response) => parse_outcome(&envelope.request, response)?,
                Err(RemoteViewApplicationAdapterError::Transport(
                    crate::RemoteViewApplicationTransportError::ViewGrantTerminal,
                )) => RemoteViewApplicationMutationOutcome::IssueViewTerminal,
                Err(error) => return Err(error),
            };
            if let RemoteViewApplicationMutationOutcome::IssueView(view) = &outcome {
                view.validate_target(
                    &observed.target,
                    &application,
                    &options.audience,
                    options.capability,
                    options.lifetime_seconds,
                    now_ms(),
                )?;
            }
            store
                .complete(&envelope, &outcome)
                .map_err(RemoteViewApplicationAdapterError::MutationStore)?;
            outcome
        };
        match outcome {
            RemoteViewApplicationMutationOutcome::IssueView(view) => {
                view.validate_target(
                    &observed.target,
                    &application,
                    &options.audience,
                    options.capability,
                    options.lifetime_seconds,
                    now_ms(),
                )?;
                Ok(view)
            }
            RemoteViewApplicationMutationOutcome::IssueViewTerminal => {
                Err(RemoteViewApplicationAdapterError::ViewGrantTerminal)
            }
            _ => Err(RemoteViewApplicationResponseError::InvalidShape.into()),
        }
    }

    pub fn revoke_view(
        &mut self,
        grant: &RemoteViewApplicationGrant,
        store: &mut impl RemoteViewApplicationMutationStore,
    ) -> Result<RemoteViewApplicationViewRevocation, RemoteViewApplicationAdapterError> {
        if grant.request.application != self.application {
            return Err(RemoteViewApplicationResponseError::InvalidTarget.into());
        }
        let request = RemoteViewApplicationRequest::RevokeView {
            route_id: grant.route_id.clone(),
            audience: grant.request.audience.clone(),
        };
        let outcome = self.mutate(request, store, |outcome, _| match outcome {
            RemoteViewApplicationMutationOutcome::RevokeView(receipt)
                if receipt.schema_version == 1
                    && receipt.route_id == grant.route_id
                    && receipt.state == "revoked" =>
            {
                Ok(())
            }
            _ => Err(RemoteViewApplicationResponseError::InvalidTarget),
        })?;
        match outcome {
            RemoteViewApplicationMutationOutcome::RevokeView(receipt) => Ok(receipt),
            _ => unreachable!(),
        }
    }

    pub fn release(
        &mut self,
        target: &RemoteViewApplicationReleaseTarget,
        cleanup: RemoteViewApplicationCleanupPermit,
        idempotency_key: String,
        store: &mut impl RemoteViewApplicationMutationStore,
    ) -> Result<RemoteViewJoinedReleaseOutcome, RemoteViewApplicationAdapterError> {
        target.validate()?;
        let cleanup = cleanup
            .into_acknowledgement(target)
            .map_err(|_| RemoteViewApplicationResponseError::InvalidTarget)?;
        let assignment = &target.assignment;
        if !cleanup.validates_target(&assignment.assignment_id, assignment.generation) {
            return Err(RemoteViewApplicationResponseError::InvalidTarget.into());
        }
        let request = RemoteViewApplicationRequest::Release {
            assignment_id: assignment.assignment_id.clone(),
            expected_generation: assignment.generation,
            idempotency_key,
            cleanup,
        };
        let outcome = self.mutate(request, store, |outcome, _| match outcome {
            RemoteViewApplicationMutationOutcome::Release(released) => {
                target.validate_outcome(released)
            }
            _ => Err(RemoteViewApplicationResponseError::InvalidShape),
        })?;
        match outcome {
            RemoteViewApplicationMutationOutcome::Release(released) => Ok(released),
            _ => unreachable!(),
        }
    }
}
