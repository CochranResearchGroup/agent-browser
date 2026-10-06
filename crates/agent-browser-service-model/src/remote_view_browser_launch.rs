//! Generation-bound browser launch with durable intent and private environment.
use crate::{
    BrowserLaunch, BrowserLaunchCustodyRecord, BrowserLaunchCustodyStore, BrowserLaunchIntent,
    BrowserProfileCatalogEntry, BrowserSessionState, LaunchCustodyAdmission,
    LaunchCustodyStoreError, RemoteViewApplicationAdapter, RemoteViewApplicationAdapterError,
    RemoteViewApplicationTransport, RemoteViewAssignmentObservation,
    RemoteViewPrivateLaunchEnvironment,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemoteViewBrowserProcessError {
    Rejected,
    OutcomeUnknown,
}

/// Process adapters consume the complete private environment once. Implementors
/// must validate the supplied observation when consuming it and keep environment
/// values out of public output, durable custody and diagnostic errors.
pub trait RemoteViewBrowserProcessEffects {
    /// Positive OS process and exact profile census; unavailable evidence fails closed.
    fn prove_recovery_absence(
        &mut self,
        _browser: &crate::ManagedBrowserInstance,
        _profile: &BrowserProfileCatalogEntry,
    ) -> Result<(), RemoteViewBrowserProcessError> {
        Err(RemoteViewBrowserProcessError::Rejected)
    }
    /// Repeat absence qualification immediately before launching the retained logical ID.
    fn recover_launch(
        &mut self,
        _browser: &crate::ManagedBrowserInstance,
        _profile: &BrowserProfileCatalogEntry,
        _intent: &BrowserLaunchIntent,
        _environment: RemoteViewPrivateLaunchEnvironment,
        _observation: &RemoteViewAssignmentObservation,
    ) -> Result<BrowserLaunch, RemoteViewBrowserProcessError> {
        Err(RemoteViewBrowserProcessError::Rejected)
    }

    fn launch(
        &mut self,
        profile: &BrowserProfileCatalogEntry,
        intent: &BrowserLaunchIntent,
        environment: RemoteViewPrivateLaunchEnvironment,
        observation: &RemoteViewAssignmentObservation,
    ) -> Result<BrowserLaunch, RemoteViewBrowserProcessError>;
}

#[derive(Debug, Eq, PartialEq)]
pub enum RemoteViewBrowserLaunchError {
    InvalidProfile,
    Store(LaunchCustodyStoreError),
    ReadbackRequired,
    Provider(RemoteViewApplicationAdapterError),
    Process(RemoteViewBrowserProcessError),
    InvalidLaunchIdentity,
}

/// Persist intent before acquiring fresh private launch inputs or starting a
/// process. Existing custody never authorizes another launch. Any later failure
/// retains that claim for explicit reconciliation. Session publication remains
/// a separate atomic custody-and-session transaction after the returned launch.
pub fn launch_remote_view_browser<T: RemoteViewApplicationTransport>(
    adapter: &mut RemoteViewApplicationAdapter<T>,
    store: &mut impl BrowserLaunchCustodyStore,
    process: &mut impl RemoteViewBrowserProcessEffects,
    intent: &BrowserLaunchIntent,
    expected: &BrowserSessionState,
    profile: &BrowserProfileCatalogEntry,
) -> Result<BrowserLaunch, RemoteViewBrowserLaunchError> {
    use RemoteViewBrowserLaunchError as Error;
    if profile.id != intent.profile_id {
        return Err(Error::InvalidProfile);
    }
    match store
        .admit_launch_intent(intent, expected)
        .map_err(Error::Store)?
    {
        LaunchCustodyAdmission::New => (),
        LaunchCustodyAdmission::Existing(_) => return Err(Error::ReadbackRequired),
    }
    let environment = adapter
        .launch_environment(&intent.assignment)
        .map_err(Error::Provider)?;
    let observation = adapter
        .observe_assignment(&intent.assignment)
        .map_err(Error::Provider)?;
    environment
        .validate_observation(&observation, &intent.assignment)
        .map_err(|_| Error::InvalidLaunchIdentity)?;
    let launch = process
        .launch(profile, intent, environment, &observation)
        .map_err(Error::Process)?;
    let mut qualified = BrowserLaunchCustodyRecord::pending(intent.clone())
        .map_err(|_| Error::InvalidLaunchIdentity)?;
    qualified
        .observe(&launch)
        .map_err(|_| Error::InvalidLaunchIdentity)?;
    store
        .observe_launch_intent(intent, &launch)
        .map_err(Error::Store)?;
    Ok(launch)
}
