#![allow(unused_imports)]
use super::shared::*;
pub(crate) fn remote_view_open_visible_window_proof(
    route_binding: &super::super::super::remote_view::RemoteViewRouteBinding,
) -> Result<Value, String> {
    let display_name = route_binding
        .launch_display_name
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            format!(
                "route_display_missing: route '{}' has no launch display",
                route_binding.route_id
            )
        })?;
    if env::var("AGENT_BROWSER_REMOTE_VIEW_FORCE_PROOF_FAILURE")
        .ok()
        .is_some_and(|value| value.trim() == "1")
    {
        return Err(
            format!(
                "forced_visible_window_proof_failure: route '{}' display '{}' proof failure requested by AGENT_BROWSER_REMOTE_VIEW_FORCE_PROOF_FAILURE",
                route_binding.route_id, display_name
            ),
        );
    }
    let timeout = Duration::from_secs(10);
    let interval = Duration::from_millis(500);
    let started_at = Instant::now();
    let mut attempts = 0_u32;
    loop {
        attempts += 1;
        let display_content = route_bound_display_content(display_name).unwrap_or_else(|| {
            json!(
                { "state" : "display_probe_unavailable", "displayName" :
                display_name, "windows" : [], "error" :
                "route display probe returned no content", }
            )
        });
        match visible_browser_window_proof(
            &route_binding.route_id,
            display_name,
            display_content.clone(),
        ) {
            Ok(proof) => return Ok(proof),
            Err(error) => {
                let state = remote_view_visible_window_proof_state(&display_content);
                if !remote_view_visible_window_proof_retryable_state(state)
                    || started_at.elapsed() >= timeout
                {
                    return Err(format!(
                        "{error}; visible_window_proof_attempts={attempts}; timeoutMs={}",
                        timeout.as_millis()
                    ));
                }
                std::thread::sleep(interval);
            }
        }
    }
}
pub(crate) fn remote_view_visible_window_proof_state(display_content: &Value) -> &str {
    display_content
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
}
pub(crate) fn remote_view_visible_window_proof_retryable_state(state: &str) -> bool {
    matches!(
        state,
        "display_probe_unavailable" | "empty_display" | "non_browser_windows" | "unknown"
    )
}
pub(crate) fn command_object_with_action(cmd: &Value, action: &str) -> Map<String, Value> {
    let mut command = cmd.as_object().cloned().unwrap_or_default();
    command.insert("action".to_string(), Value::String(action.to_string()));
    command.remove("dryRun");
    command
}
