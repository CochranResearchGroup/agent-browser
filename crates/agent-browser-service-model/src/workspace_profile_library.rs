//! Intended profile context extends existing profiles and SitePolicies. It owns no
//! account registry, credential store, login verification or browser effects.
use crate::{BrowserProfile, ServiceEntitySource, ServiceState};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A descriptive intended identity scoped to one existing SitePolicy. Equality
/// compares the entire exact tuple; an email label alone is never identity proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileSiteAssociation {
    pub site_policy_id: String,
    pub account: String,
    pub organization: String,
}

/// Compare-and-set only intended metadata. Existing authentication, observed
/// identity, profile identity and browser ownership are preserved byte-for-byte.
pub fn replace_profile_site_associations(
    state: &mut ServiceState,
    profile_id: &str,
    associations: Vec<ProfileSiteAssociation>,
    expected: &[ProfileSiteAssociation],
    now: &str,
) -> Result<BrowserProfile, String> {
    if associations.len() > 100 {
        return Err("At most 100 intended associations are supported".into());
    }
    let mut distinct = BTreeSet::new();
    for association in &associations {
        if !state
            .site_policies
            .contains_key(&association.site_policy_id)
        {
            return Err(format!(
                "Unknown SitePolicy: {}",
                association.site_policy_id
            ));
        }
        for value in [&association.account, &association.organization] {
            if value.trim().is_empty() || value.len() > 500 {
                return Err("Account and organization labels must contain 1 to 500 bytes".into());
            }
        }
        if !distinct.insert((
            &association.site_policy_id,
            &association.account,
            &association.organization,
        )) {
            return Err("Duplicate intended association".into());
        }
    }
    let profile = state
        .profiles
        .get_mut(profile_id)
        .ok_or("Unknown profile")?;
    if profile.site_associations != expected {
        return Err("Profile associations changed; reload before saving".into());
    }
    profile.site_associations = associations;
    profile.site_associations_updated_at = Some(now.into());
    state
        .entity_sources
        .profiles
        .insert(profile_id.into(), ServiceEntitySource::PersistedState);
    Ok(profile.clone())
}

/// Requested identity context and optional deliberate profile/build/mode choice.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ProfileLibrarySelectionRequest {
    pub site_policy_id: String,
    pub account: String,
    pub organization: String,
    pub profile_id: Option<String>,
    pub browser_build: Option<crate::BrowserBuild>,
    pub cdp_connected: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileLibrarySelection {
    pub state: String,
    pub profile_id: Option<String>,
    pub candidates: Vec<String>,
    pub reason: String,
    /// Even an exact intended match requires current independent identity checks.
    pub identity_verification_required: bool,
}

fn occupied_browsers<'a>(
    state: &'a ServiceState,
    id: &'a str,
) -> impl Iterator<Item = &'a crate::BrowserProcess> {
    state.browsers.values().filter(move |browser| {
        browser.profile_id.as_deref() == Some(id)
            && !matches!(
                browser.health,
                crate::BrowserHealth::ProcessExited | crate::BrowserHealth::NotStarted
            )
    })
}

fn compatible(
    state: &ServiceState,
    profile: &BrowserProfile,
    request: &ProfileLibrarySelectionRequest,
) -> bool {
    if request.browser_build.is_some()
        && profile.browser_build.is_some()
        && request.browser_build != profile.browser_build
    {
        return false;
    }
    if state
        .site_policies
        .get(&request.site_policy_id)
        .is_some_and(|site| site.requires_cdp_free)
        && request.cdp_connected == Some(true)
    {
        return false;
    }
    for browser in occupied_browsers(state, &profile.id) {
        // Retained profile preference is not exact current-build proof. Leave
        // occupied unknown build cases to deliberate owner acquisition choices.
        if request.browser_build.is_some() && profile.browser_build.is_none() {
            return false;
        }
        let connected = browser
            .cdp_endpoint
            .as_deref()
            .is_some_and(|value| !value.is_empty());
        if request
            .cdp_connected
            .is_some_and(|requested| requested != connected)
            || state
                .site_policies
                .get(&request.site_policy_id)
                .is_some_and(|site| site.requires_cdp_free && connected)
        {
            return false;
        }
    }
    true
}

/// Pure selection, never an acquisition or login assertion. Explicit selection
/// wins if compatible. Automatic routing compares the complete exact requested
/// tuple and refuses ambiguity, missing context and conflicting constraints.
pub fn select_profile_for_site_identity(
    state: &ServiceState,
    request: &ProfileLibrarySelectionRequest,
) -> ProfileLibrarySelection {
    let result = |status: &str, id: Option<String>, candidates: Vec<String>, reason: &str| {
        ProfileLibrarySelection {
            state: status.into(),
            profile_id: id,
            candidates,
            reason: reason.into(),
            identity_verification_required: true,
        }
    };
    if !(state.site_policies.contains_key(&request.site_policy_id)
        || (request.site_policy_id.is_empty() && request.profile_id.is_some()))
    {
        return result("conflict", None, vec![], "Requested SitePolicy is unknown");
    }
    if let Some(id) = request.profile_id.as_ref() {
        return match state.profiles.get(id) {
            Some(profile) if compatible(state, profile, request) => result("selected", Some(id.clone()), vec![id.clone()], "Explicit profile preserved; independently verify the requested website identity before effects"),
            Some(_) => result("conflict", None, vec![id.clone()], "Explicit profile conflicts with requested build or mode; choose deliberately"),
            None => result("conflict", None, vec![], "Explicit profile is unknown; no alternate profile was selected"),
        };
    }
    if request.account.trim().is_empty() || request.organization.trim().is_empty() {
        return result(
            "choice_required",
            None,
            vec![],
            "Requested account and organization are required for automatic selection",
        );
    }
    let matches: Vec<_> = state
        .profiles
        .values()
        .filter(|profile| {
            profile.site_associations.iter().any(|association| {
                association.site_policy_id == request.site_policy_id
                    && association.account == request.account
                    && association.organization == request.organization
            })
        })
        .collect();
    let candidates: Vec<_> = matches
        .iter()
        .filter(|profile| compatible(state, profile, request))
        .map(|profile| profile.id.clone())
        .collect();
    match candidates.as_slice() {
        [id] => result("selected", Some(id.clone()), candidates.clone(), "Unique compatible intended identity match; independently verify current authentication"),
        [] if !matches.is_empty() => result("conflict", None, matches.iter().map(|profile| profile.id.clone()).collect(), "Intended matches conflict with requested build or mode"),
        [] => result("choice_required", None, vec![], "No compatible intended identity match; choose a profile"),
        _ => result("choice_required", None, candidates, "Multiple compatible profiles match; choose a profile"),
    }
}

/// No-launch manager projection of existing profile, readiness and occupancy
/// records. No current account identity probe exists here: label metadata and
/// historical freshness must not be promoted into observed account identity.
pub fn profile_library_snapshot(state: &ServiceState) -> serde_json::Value {
    let profiles: Vec<_> = state.profiles.values().map(|profile| {
        let current_use: Vec<_> = occupied_browsers(state, &profile.id).map(|browser| serde_json::json!({
            "browserId": browser.id,
            "mode": if browser.cdp_endpoint.as_deref().is_some_and(|value| !value.is_empty()) { "automation_connected" } else { "cdp_free" },
            "sessionIds": browser.active_session_ids,
        })).collect();
        let sites: BTreeSet<_> = profile.site_associations.iter().map(|association| association.site_policy_id.clone()).chain(profile.site_policy_ids.iter().cloned()).collect();
        serde_json::json!({"profile":profile,"currentUse":current_use,"observedIdentity":{"state":"unknown","account":null},"relevantSitePolicyIds":sites})
    }).collect();
    serde_json::json!({"profiles":profiles,"sitePolicies":state.site_policies.values().map(|site|serde_json::json!({"id":site.id,"name":site.name})).collect::<Vec<_>>()})
}

/// The enclosing trusted adapter derives assurance from authenticated context,
/// never a caller-supplied assurance label. Self-declared labels stay untrusted.
#[derive(Debug, Clone)]
pub struct ProfileLibraryActor {
    pub subject_id: Option<String>,
    pub assurance: crate::ProfileIdentityAssurance,
}

fn permitted(
    state: &ServiceState,
    id: &str,
    actor: &ProfileLibraryActor,
    permission: crate::ProfilePermission,
) -> bool {
    let Some(profile) = state.profiles.get(id) else {
        return false;
    };
    crate::evaluate_profile_access(crate::ProfileAccessEvaluation {
        profile_id: id,
        explicit_policy: profile.access_policy.as_ref(),
        subject_id: actor.subject_id.clone(),
        assurance: actor.assurance,
        connection_instance_id: None,
        permission,
        operation: "workspace_profile_library",
        incompatible_occupancy: vec![],
    })
    .1
    .allowed
}

/// Public no-launch library owner seam, including existing per-profile policy.
/// Read/selection projections never reveal denied profiles. A denied or unknown
/// mutation has the same error and creates no profile or association record.
pub fn profile_library_request(
    state: &mut ServiceState,
    cmd: &serde_json::Value,
    actor: &ProfileLibraryActor,
    now: &str,
) -> Result<serde_json::Value, String> {
    use crate::ProfilePermission;
    use serde_json::{json, Value};
    match cmd
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or("list")
    {
        "list" | "select" => {
            let selection = cmd.get("operation").and_then(Value::as_str) == Some("select");
            let mut visible = state.clone();
            visible.profiles.retain(|id, _| {
                permitted(state, id, actor, ProfilePermission::PolicyRead)
                    && (!selection || permitted(state, id, actor, ProfilePermission::ProfileUse))
            });
            if selection {
                let request: ProfileLibrarySelectionRequest =
                    serde_json::from_value(cmd.clone())
                        .map_err(|error| format!("Invalid profile selection: {error}"))?;
                serde_json::to_value(select_profile_for_site_identity(&visible, &request))
                    .map_err(|error| error.to_string())
            } else {
                visible.refresh_profile_readiness();
                Ok(profile_library_snapshot(&visible))
            }
        }
        "replace_associations" => {
            let id = cmd
                .get("profileId")
                .and_then(Value::as_str)
                .ok_or("Missing profileId")?;
            if !permitted(state, id, actor, ProfilePermission::PolicyWrite)
                || !permitted(state, id, actor, ProfilePermission::PolicyRead)
            {
                return Err("Profile unavailable or permission denied".into());
            }
            let decode = |field: &str| -> Result<Vec<ProfileSiteAssociation>, String> {
                serde_json::from_value(
                    cmd.get(field)
                        .cloned()
                        .ok_or_else(|| format!("Missing {field}"))?,
                )
                .map_err(|error| format!("Invalid {field}: {error}"))
            };
            replace_profile_site_associations(
                state,
                id,
                decode("associations")?,
                &decode("expectedAssociations")?,
                now,
            )
            .map(|profile| json!({"profile":profile}))
        }
        _ => Err("Unsupported profile library operation".into()),
    }
}
