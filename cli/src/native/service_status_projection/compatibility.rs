use crate::native::service_model::ServiceState;
use crate::runtime_profile::ManualRuntimeBrowser;

use super::StatusObservationSnapshot;

pub(super) fn join_manual_browsers(
    state: &ServiceState,
    mut manual_browsers: Vec<ManualRuntimeBrowser>,
) -> Vec<ManualRuntimeBrowser> {
    for manual_browser in &mut manual_browsers {
        let route = state.remote_view_routes.values().find(|route| {
            let Some(display) = manual_browser.display.as_deref() else {
                return false;
            };
            route
                .display_allocation_id
                .as_deref()
                .and_then(|allocation_id| state.display_allocations.get(allocation_id))
                .and_then(|allocation| allocation.display_name.as_deref())
                == Some(display)
        });
        if let Some(route) = route {
            manual_browser.remote_view_route_id = Some(route.id.clone());
            manual_browser.remote_view_url = route
                .frame_url
                .clone()
                .or_else(|| route.external_url.clone());
            manual_browser.remote_control_available =
                !route.read_only && route.control_input.is_some();
            if manual_browser.remote_view_url.is_some() {
                manual_browser.next_safe_action =
                    "open_remote_view_or_finish_login_then_close".to_string();
            }
        }
    }
    manual_browsers.sort_by(|left, right| left.id.cmp(&right.id));
    manual_browsers
}

pub(super) fn apply_legacy_observation_mirrors(
    state: &ServiceState,
    observations: &StatusObservationSnapshot,
) -> Result<serde_json::Value, super::ServiceStatusProjectionError> {
    let mut value = serde_json::to_value(state)
        .map_err(|error| super::ServiceStatusProjectionError::Serialization(error.to_string()))?;
    let Some(browsers) = value
        .get_mut("browsers")
        .and_then(serde_json::Value::as_object_mut)
    else {
        return Ok(value);
    };
    for (browser_id, stats) in &observations.browser_process_stats {
        if let Some(browser) = browsers.get_mut(browser_id) {
            browser["processStats"] = stats.clone();
        }
    }
    for observation in &observations.view_streams {
        let Some(browser) = browsers.get_mut(&observation.browser_id) else {
            continue;
        };
        if browser.get("host").and_then(serde_json::Value::as_str) != Some("remote_headed") {
            continue;
        }
        let Some(stream) = browser
            .get_mut("viewStreams")
            .and_then(serde_json::Value::as_array_mut)
            .and_then(|streams| {
                streams.iter_mut().find(|stream| {
                    stream.get("id").and_then(serde_json::Value::as_str)
                        == Some(observation.stream_id.as_str())
                })
            })
        else {
            continue;
        };
        if stream.get("provider").and_then(serde_json::Value::as_str) != Some("rdp_gateway") {
            continue;
        }
        if let Some(presentation) = &observation.route_presentation {
            if stream
                .get("frameUrl")
                .and_then(serde_json::Value::as_str)
                .is_none()
            {
                stream["frameUrl"] = serde_json::Value::String(presentation.frame_url.clone());
            }
            if stream
                .get("externalUrl")
                .and_then(serde_json::Value::as_str)
                .is_none()
            {
                stream["externalUrl"] =
                    serde_json::Value::String(presentation.external_url.clone());
            }
        }
        if stream.get("displayContent").is_none() {
            if let Some(content) = &observation.display_content {
                stream["displayContent"] = content.clone();
            }
        }
    }
    Ok(value)
}
