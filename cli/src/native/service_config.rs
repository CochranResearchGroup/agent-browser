//! Persistent service configuration mutations.
//!
//! These helpers keep HTTP and MCP mutation surfaces aligned while preserving
//! the JSON-backed service state as the current durable store.

use serde_json::Value;

use super::service_model::{ServiceProvider, ServiceState, SitePolicy};

fn validate_entity_id(id: &str, label: &str) -> Result<(), String> {
    if id.trim().is_empty() {
        return Err(format!("{label} id must not be empty"));
    }
    if id.contains('/') {
        return Err(format!("{label} id must not contain '/'"));
    }
    Ok(())
}

fn object_body_with_path_id(mut value: Value, id: &str, label: &str) -> Result<Value, String> {
    validate_entity_id(id, label)?;
    let Some(object) = value.as_object_mut() else {
        return Err(format!("{label} body must be a JSON object"));
    };
    if let Some(body_id) = object
        .get("id")
        .and_then(|value| value.as_str())
        .filter(|body_id| !body_id.is_empty())
    {
        if body_id != id {
            return Err(format!(
                "{label} body id '{body_id}' does not match path id '{id}'"
            ));
        }
    }
    object.insert("id".to_string(), Value::String(id.to_string()));
    Ok(value)
}

/// Upsert one service site-policy record into persisted service state.
pub fn upsert_site_policy(
    state: &mut ServiceState,
    id: &str,
    body: Value,
) -> Result<SitePolicy, String> {
    let body = object_body_with_path_id(body, id, "site policy")?;
    let policy = serde_json::from_value::<SitePolicy>(body)
        .map_err(|err| format!("Invalid site policy: {err}"))?;
    state.site_policies.insert(id.to_string(), policy.clone());
    Ok(policy)
}

/// Delete one service site-policy record from persisted service state.
pub fn delete_site_policy(
    state: &mut ServiceState,
    id: &str,
) -> Result<Option<SitePolicy>, String> {
    validate_entity_id(id, "site policy")?;
    Ok(state.site_policies.remove(id))
}

/// Upsert one service provider record into persisted service state.
pub fn upsert_provider(
    state: &mut ServiceState,
    id: &str,
    body: Value,
) -> Result<ServiceProvider, String> {
    let body = object_body_with_path_id(body, id, "provider")?;
    let provider = serde_json::from_value::<ServiceProvider>(body)
        .map_err(|err| format!("Invalid provider: {err}"))?;
    state.providers.insert(id.to_string(), provider.clone());
    Ok(provider)
}

/// Delete one service provider record from persisted service state.
pub fn delete_provider(
    state: &mut ServiceState,
    id: &str,
) -> Result<Option<ServiceProvider>, String> {
    validate_entity_id(id, "provider")?;
    Ok(state.providers.remove(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn upsert_site_policy_sets_path_id_and_defaults() {
        let mut state = ServiceState::default();

        let policy = upsert_site_policy(
            &mut state,
            "google",
            json!({
                "originPattern": "https://accounts.google.com",
                "interactionMode": "human_like_input"
            }),
        )
        .unwrap();

        assert_eq!(policy.id, "google");
        assert_eq!(policy.origin_pattern, "https://accounts.google.com");
        assert!(state.site_policies.contains_key("google"));
    }

    #[test]
    fn upsert_provider_rejects_body_id_mismatch() {
        let mut state = ServiceState::default();

        let err = upsert_provider(
            &mut state,
            "manual",
            json!({
                "id": "other",
                "kind": "manual_approval"
            }),
        )
        .unwrap_err();

        assert!(err.contains("does not match path id"));
    }

    #[test]
    fn delete_provider_reports_removed_record() {
        let mut state = ServiceState::default();
        upsert_provider(
            &mut state,
            "manual",
            json!({
                "displayName": "Dashboard approval",
                "kind": "manual_approval"
            }),
        )
        .unwrap();

        let removed = delete_provider(&mut state, "manual").unwrap();

        assert!(removed.is_some());
        assert!(!state.providers.contains_key("manual"));
    }
}
