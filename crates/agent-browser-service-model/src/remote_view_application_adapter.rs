//! Injectable revision 3 transport coordination. No live transport is owned here.
use serde_json::Value;

use crate::{
    RemoteViewApplicationEnvelope, RemoteViewApplicationRequest,
    RemoteViewApplicationResponseError, RemoteViewAssignmentObservation,
    RemoteViewAssignmentRecord, RemoteViewPrivateLaunchEnvironment,
};

/// Transport errors contain no response bodies, environment values or secrets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemoteViewApplicationTransportError {
    Rejected,
    Unavailable,
    OutcomeUnknown,
}

pub trait RemoteViewApplicationTransport {
    fn request(
        &mut self,
        envelope: &RemoteViewApplicationEnvelope,
    ) -> Result<Value, RemoteViewApplicationTransportError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemoteViewApplicationAdapterError {
    InvalidApplication,
    Transport(RemoteViewApplicationTransportError),
    Response(RemoteViewApplicationResponseError),
}

impl From<RemoteViewApplicationTransportError> for RemoteViewApplicationAdapterError {
    fn from(error: RemoteViewApplicationTransportError) -> Self {
        Self::Transport(error)
    }
}

impl From<RemoteViewApplicationResponseError> for RemoteViewApplicationAdapterError {
    fn from(error: RemoteViewApplicationResponseError) -> Self {
        Self::Response(error)
    }
}

/// Application is configured resource context. Transport implementations and
/// runtime adapters retain effect authority and durable operation custody.
pub struct RemoteViewApplicationAdapter<T> {
    application: String,
    transport: T,
}

impl<T: RemoteViewApplicationTransport> RemoteViewApplicationAdapter<T> {
    pub fn new(
        application: String,
        transport: T,
    ) -> Result<Self, RemoteViewApplicationAdapterError> {
        if application.is_empty()
            || application.len() > 64
            || !application
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(RemoteViewApplicationAdapterError::InvalidApplication);
        }
        Ok(Self {
            application,
            transport,
        })
    }

    fn request(
        &mut self,
        request: RemoteViewApplicationRequest,
    ) -> Result<Value, RemoteViewApplicationAdapterError> {
        Ok(self.transport.request(&RemoteViewApplicationEnvelope {
            application: self.application.clone(),
            request,
        })?)
    }

    pub fn observe_assignment(
        &mut self,
        assignment: &RemoteViewAssignmentRecord,
    ) -> Result<RemoteViewAssignmentObservation, RemoteViewApplicationAdapterError> {
        let response = self.request(RemoteViewApplicationRequest::ObserveAssignment {
            assignment_id: assignment.assignment_id.clone(),
            expected_generation: assignment.generation,
        })?;
        let observation: RemoteViewAssignmentObservation = serde_json::from_value(response)
            .map_err(|_| RemoteViewApplicationResponseError::InvalidShape)?;
        observation.validate_live_assignment(assignment)?;
        Ok(observation)
    }

    /// Refresh both generations before reading the full private environment,
    /// then observe again. A changed target invalidates the environment instead
    /// of attempting launch with cached context or inferring another display.
    pub fn launch_environment(
        &mut self,
        assignment: &RemoteViewAssignmentRecord,
    ) -> Result<RemoteViewPrivateLaunchEnvironment, RemoteViewApplicationAdapterError> {
        let before = self.observe_assignment(assignment)?;
        let response = self.request(RemoteViewApplicationRequest::LaunchEnvironment {
            assignment_id: assignment.assignment_id.clone(),
            expected_generation: before.target.lifecycle_generation,
            expected_viewing_generation: before.target.viewing_generation,
        })?;
        let environment =
            RemoteViewPrivateLaunchEnvironment::from_response(response, &before, assignment)?;
        let after = self.observe_assignment(assignment)?;
        // Recheck the exact target, preserving the non-serializable wrapper.
        environment.validate_observation(&after, assignment)?;
        Ok(environment)
    }
}
