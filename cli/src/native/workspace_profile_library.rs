//! Public no-launch workspace adapter over the existing durable profile owner.
use super::service_store::{LockedServiceStateRepository, ServiceStateRepository};
use agent_browser_service_model::{
    profile_library_request, ProfileIdentityAssurance, ProfileLibraryActor,
};
use serde_json::Value;

/// HTTP/MCP share one owner handler. The transport must strip caller assertions
/// and supply servicePrincipalId only from its authenticated principal context.
/// Role checks remain operation-specific; per-profile policy is enforced here.
pub(crate) async fn handle_workspace_profile_library(cmd: &Value) -> Result<Value, String> {
    let repository = LockedServiceStateRepository::default_json()?;
    workspace_profile_library_in_repository(&repository, cmd)
}

pub(crate) fn workspace_profile_library_in_repository(
    repository: &impl ServiceStateRepository,
    cmd: &Value,
) -> Result<Value, String> {
    let authenticated = cmd
        .get("servicePrincipalId")
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty());
    let subject = authenticated.or_else(|| {
        cmd.get("clientSubjectId")
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty())
    });
    let actor = ProfileLibraryActor {
        subject_id: subject.map(str::to_string),
        assurance: if authenticated.is_some() {
            ProfileIdentityAssurance::RegisteredCapability
        } else if subject.is_some() {
            ProfileIdentityAssurance::SelfDeclared
        } else {
            ProfileIdentityAssurance::Unknown
        },
    };
    let configured = crate::flags::load_config(&[])?.service_state_snapshot();
    workspace_profile_library_with_config(repository, cmd, &actor, configured)
}

fn workspace_profile_library_with_config(
    repository: &impl ServiceStateRepository,
    cmd: &Value,
    actor: &ProfileLibraryActor,
    configured: agent_browser_service_model::ServiceState,
) -> Result<Value, String> {
    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|error| error.to_string())?;
    match cmd
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or("list")
    {
        "replace_associations" => repository.mutate(|state| {
            state.overlay_configured_entities(configured.clone());
            state.apply_builtin_site_policies();
            profile_library_request(state, cmd, actor, &now)
        }),
        _ => {
            let mut state = repository.load_snapshot()?;
            state.overlay_configured_entities(configured);
            state.apply_builtin_site_policies();
            profile_library_request(&mut state, cmd, actor, &now)
        }
    }
}
