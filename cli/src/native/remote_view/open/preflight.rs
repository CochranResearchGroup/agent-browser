#![allow(unused_imports)]
use super::operator_route::remote_view_open_display_access_probe;
use super::planner::{
    inline_route_pool_entry_from_command, service_remote_view_acquisition_plan_from_state,
};
use super::route_lifecycle::service_remote_view_timestamp;
use super::shared::*;
use super::target::retained_readiness_component;
pub(crate) async fn handle_service_remote_view_route_preflight(
    cmd: &Value,
    daemon_state: &DaemonState,
) -> Result<Value, String> {
    let observed_at = service_remote_view_timestamp();
    let browser_id = optional_command_string(cmd, "browserId")
        .unwrap_or_else(|| service_browser_id(&daemon_state.session_id));
    let session_id = optional_command_string(cmd, "sessionName")
        .unwrap_or_else(|| daemon_state.session_id.clone());
    let repository = LockedServiceStateRepository::default_json()?;
    let mut state = repository.load_snapshot()?;
    let intent = normalize_remote_view_open_intent(cmd)?;
    let inline_route_pool_entry = inline_route_pool_entry_from_command(cmd)?;
    if let Some(entry) = inline_route_pool_entry.as_ref() {
        state.route_pool.insert(entry.id.clone(), entry.clone());
    }
    let acquisition_plan = service_remote_view_acquisition_plan_from_state(
        cmd,
        &state,
        &intent,
        inline_route_pool_entry.as_ref(),
        &browser_id,
        &session_id,
    )?;
    let route_binding = acquisition_plan.route_binding.clone();
    let fast_preflight =
        remote_view_route_fast_preflight(&route_binding, &acquisition_plan, &observed_at);
    Ok(json!(
        { "status" : "preflight_ready", "preflightStatus" : fast_preflight
        .get("status").cloned().unwrap_or(Value::Null), "observedAt" : observed_at,
        "routeId" : route_binding.route_id, "displayAllocationId" : route_binding
        .display_allocation_id, "routePoolEntryId" : route_binding
        .route_pool_entry_id, "browserId" : browser_id, "sessionName" : session_id,
        "frameUrl" : route_binding.frame_url, "externalUrl" : route_binding
        .external_url, "routeDescriptor" : route_binding.route_descriptor,
        "providerMode" : route_binding.provider_mode, "routeBinding" : route_binding,
        "acquisitionPlan" : acquisition_plan, "fastPreflight" : fast_preflight, }
    ))
}
pub(crate) fn remote_view_route_fast_preflight(
    route_binding: &super::super::super::remote_view::RemoteViewRouteBinding,
    acquisition_plan: &RemoteViewAcquisitionPlan,
    observed_at: &str,
) -> Value {
    let route_readiness = route_binding.readiness.as_ref();
    let mut components = vec![
        remote_view_preflight_component(
            "acquisition_plan",
            if acquisition_plan.blockers.is_empty() {
                "ready"
            } else {
                "blocked"
            },
            if acquisition_plan.blockers.is_empty() {
                "acquisition planner selected a route without blockers".to_string()
            } else {
                format!(
                    "acquisition planner reported {} blocker(s)",
                    acquisition_plan.blockers.len()
                )
            },
            Some(observed_at),
            json!({ "mode" : acquisition_plan.mode,
        "selectedRoutePoolEntryId" : acquisition_plan.selected_route_pool_entry_id,
        "displayAllocationId" : acquisition_plan.display_allocation_id, "blockers" :
        acquisition_plan.blockers, }),
            None,
        ),
        remote_view_route_url_preflight_component(route_binding, observed_at),
        retained_remote_view_preflight_component(
            route_readiness,
            "guacamole_web",
            &["guacamole_web", "guacamole_web_app"],
            observed_at,
            "run_rdp_gateway_readiness",
        ),
        retained_remote_view_preflight_component(
            route_readiness,
            "guacamole_login",
            &["guacamole_login"],
            observed_at,
            "repair_guacamole_admin_credentials",
        ),
        retained_remote_view_preflight_component(
            route_readiness,
            "guacamole_connection_permissions",
            &["guacamole_connection_permissions"],
            observed_at,
            "repair_guacamole_connection_permissions",
        ),
        retained_remote_view_preflight_component(
            route_readiness,
            "rdp_backend_tcp",
            &["rdp_backend_tcp", "backend_tcp"],
            observed_at,
            "repair_rdp_backend_reachability",
        ),
        remote_view_helper_status_preflight_component(observed_at),
        remote_view_display_access_preflight_component(route_binding, observed_at),
        remote_view_route_desktop_preflight_component(route_binding, observed_at),
    ];
    let blockers = components
        .iter()
        .filter(|component| {
            component
                .get("status")
                .and_then(Value::as_str)
                .is_some_and(|status| matches!(status, "blocked" | "failed"))
        })
        .cloned()
        .collect::<Vec<_>>();
    let stale = components
        .iter()
        .filter(|component| {
            component
                .get("status")
                .and_then(Value::as_str)
                .is_some_and(|status| status == "stale")
        })
        .cloned()
        .collect::<Vec<_>>();
    let not_checked = components
        .iter()
        .filter(|component| {
            component
                .get("status")
                .and_then(Value::as_str)
                .is_some_and(|status| status == "not_checked")
        })
        .cloned()
        .collect::<Vec<_>>();
    let status = if !blockers.is_empty() {
        "blocked"
    } else if !stale.is_empty() {
        "stale"
    } else if !not_checked.is_empty() {
        "partial"
    } else {
        "ready"
    };
    let next_action = blockers
        .first()
        .or_else(|| stale.first())
        .or_else(|| not_checked.first())
        .and_then(|component| component.get("nextAction"))
        .and_then(Value::as_str)
        .unwrap_or("remote_view_open");
    json!(
        { "status" : status, "observedAt" : observed_at, "noLaunch" : true, "source" :
        "service_remote_view_route_preflight", "nextAction" : next_action, "components" :
        std::mem::take(& mut components), "blockers" : blockers, "stale" : stale,
        "notChecked" : not_checked, }
    )
}
pub(crate) fn remote_view_preflight_component(
    component: &str,
    status: &str,
    evidence: String,
    observed_at: Option<&str>,
    detail: Value,
    next_action: Option<&str>,
) -> Value {
    json!(
        { "component" : component, "status" : status, "evidence" : evidence, "observedAt"
        : observed_at, "freshness" : { "state" : if observed_at.is_some() {
        "observed_now" } else { "not_timestamped" }, "observedAt" : observed_at, },
        "nextAction" : next_action.unwrap_or(if status == "ready" { "none" } else {
        "inspect_remote_view_preflight" }), "detail" : detail, }
    )
}
pub(crate) fn remote_view_route_url_preflight_component(
    route_binding: &super::super::super::remote_view::RemoteViewRouteBinding,
    observed_at: &str,
) -> Value {
    let has_route_url = route_binding
        .frame_url
        .as_deref()
        .is_some_and(|url| url.contains("#/client/"))
        || route_binding
            .external_url
            .as_deref()
            .is_some_and(|url| url.contains("#/client/"))
        || route_binding
            .route_descriptor
            .as_ref()
            .and_then(Value::as_object)
            .is_some_and(|record| {
                [
                    "localEmbedUrl",
                    "dashboardEmbedUrl",
                    "publicOperatorUrl",
                    "externalUrl",
                    "healthUrl",
                ]
                .iter()
                .any(|key| {
                    record
                        .get(*key)
                        .and_then(Value::as_str)
                        .is_some_and(|url| url.contains("#/client/"))
                })
            });
    remote_view_preflight_component(
        "guacamole_route_url",
        if has_route_url { "ready" } else { "blocked" },
        if has_route_url {
            "selected route binding has a concrete Guacamole client URL".to_string()
        } else {
            "selected route binding has no concrete Guacamole client URL".to_string()
        },
        Some(observed_at),
        json!(
            { "frameUrl" : route_binding.frame_url, "externalUrl" : route_binding
            .external_url, "routeDescriptor" : route_binding.route_descriptor, }
        ),
        Some(if has_route_url {
            "none"
        } else {
            "repair_guacamole_route_url"
        }),
    )
}
pub(crate) fn retained_remote_view_preflight_component(
    readiness: Option<&Value>,
    output_component: &str,
    component_names: &[&str],
    observed_at: &str,
    default_next_action: &str,
) -> Value {
    let Some(component) = retained_readiness_component(readiness, component_names) else {
        return remote_view_preflight_component(
            output_component,
            "not_checked",
            format!("{output_component} has no retained readiness component"),
            Some(observed_at),
            json!(
                { "source" : "route_pool_entry.readiness", "componentNames" :
                component_names, }
            ),
            Some(default_next_action),
        );
    };
    let raw_status = component
        .get("status")
        .or_else(|| component.get("state"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let status = match raw_status {
        "ready" => "ready",
        "stale" | "expired" => "stale",
        "blocked" | "failed" | "missing" | "unavailable" => "blocked",
        _ => "not_checked",
    };
    let source_observed_at = component
        .get("observedAt")
        .or_else(|| component.get("checkedAt"))
        .or_else(|| component.get("lastCheckedAt"))
        .or_else(|| component.get("lastSucceededAt"))
        .and_then(Value::as_str);
    let next_action = component
        .get("nextAction")
        .and_then(Value::as_str)
        .unwrap_or(default_next_action);
    remote_view_preflight_component(
        output_component,
        status,
        component
            .get("evidence")
            .or_else(|| component.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("retained readiness component found")
            .to_string(),
        source_observed_at,
        json!(
            { "source" : "route_pool_entry.readiness", "observedByPreflightAt" :
            observed_at, "retainedComponent" : component, }
        ),
        Some(next_action),
    )
}
pub(crate) fn remote_view_helper_status_preflight_component(observed_at: &str) -> Value {
    let helper_path = env::var("AGENT_BROWSER_PRIVILEGED_HELPER").unwrap_or_else(|_| {
        "/usr/local/libexec/agent-browser/agent-browser-privileged-helper".to_string()
    });
    let report = remote_view_helper_status_probe(&helper_path);
    let ready = remote_view_helper_status_contract_ready(&report);
    remote_view_preflight_component(
        "privileged_helper_status",
        if ready { "ready" } else { "blocked" },
        if ready {
            "installed remote-view helper reports the current route desktop and display-access capability contract"
                .to_string()
        } else {
            "installed remote-view helper does not report the current route desktop and display-access capability contract"
                .to_string()
        },
        Some(observed_at),
        json!({ "helperPath" : helper_path, "statusProbe" : report, }),
        Some(if ready {
            "none"
        } else {
            "install_privileged_helper"
        }),
    )
}
pub(crate) fn remote_view_helper_status_probe(helper_path: &str) -> Value {
    let output = Command::new("timeout")
        .args(["--kill-after=1", "2s", helper_path, "status-json"])
        .stdin(Stdio::null())
        .output();
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let mut report = json!(
                { "available" : true, "success" : output.status.success(), "timedOut" :
                matches!(output.status.code(), Some(124 | 137)), "exitCode" : output
                .status.code(), "stdout" : stdout, "stderr" : stderr, }
            );
            if !stdout.is_empty() {
                match serde_json::from_str::<Value>(&stdout) {
                    Ok(parsed) => {
                        if let Some(object) = report.as_object_mut() {
                            object.insert("parsed".to_string(), parsed);
                        }
                    }
                    Err(error) => {
                        if let Some(object) = report.as_object_mut() {
                            object.insert("parseError".to_string(), json!(error.to_string()));
                        }
                    }
                }
            }
            report
        }
        Err(error) => {
            json!(
                { "available" : false, "success" : false, "timedOut" : false, "exitCode"
                : null, "stdout" : "", "stderr" : error.to_string(), }
            )
        }
    }
}
pub(crate) fn remote_view_helper_status_contract_ready(report: &Value) -> bool {
    crate::remote_view_helper_contract::status_contract_ready(report)
}
pub(crate) fn remote_view_display_access_preflight_component(
    route_binding: &super::super::super::remote_view::RemoteViewRouteBinding,
    observed_at: &str,
) -> Value {
    let Some(display_name) = route_binding.launch_display_name.as_deref() else {
        return remote_view_preflight_component(
            "display_access",
            "blocked",
            "selected route has no launch display".to_string(),
            Some(observed_at),
            json!({ "routeId" : route_binding.route_id }),
            Some("repair_route_display_binding"),
        );
    };
    let probe = remote_view_open_display_access_probe(display_name);
    let status = if probe
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        "ready"
    } else {
        "blocked"
    };
    remote_view_preflight_component(
        "display_access",
        status,
        if status == "ready" {
            format!("display {display_name} is accessible to agent-browser")
        } else {
            format!("display {display_name} is not accessible to agent-browser")
        },
        Some(observed_at),
        json!(
            { "displayName" : display_name, "routeUser" : route_binding.route_user,
            "retainedDisplayAccess" : route_binding.display_access, "probe" : probe, }
        ),
        Some(if status == "ready" {
            "none"
        } else {
            "grant_route_display_access"
        }),
    )
}
pub(crate) fn remote_view_route_desktop_preflight_component(
    route_binding: &super::super::super::remote_view::RemoteViewRouteBinding,
    observed_at: &str,
) -> Value {
    let Some(display_name) = route_binding.launch_display_name.as_deref() else {
        return remote_view_preflight_component(
            "route_desktop",
            "blocked",
            "selected route has no launch display".to_string(),
            Some(observed_at),
            json!({ "routeId" : route_binding.route_id }),
            Some("repair_route_display_binding"),
        );
    };
    let display_content = route_display_content(display_name).unwrap_or_else(|| {
        json!(
            { "state" : "display_probe_unavailable", "displayName" : display_name,
            "windows" : [], "error" : "route display probe returned no content", }
        )
    });
    let display_state = display_content
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let status = match display_state {
        "terminal_only" | "terminal_topmost" => "blocked",
        "display_probe_unavailable" => "not_checked",
        _ => "ready",
    };
    remote_view_preflight_component(
        "route_desktop",
        status,
        format!("route display {display_name} currently reports {display_state}"),
        Some(observed_at),
        json!(
            { "displayName" : display_name, "displayState" : display_state,
            "displayContent" : display_content, }
        ),
        Some(match status {
            "ready" => "none",
            "blocked" => "clear_route_terminal_or_restart_route_desktop",
            _ => "open_or_select_single_rdp_route_display",
        }),
    )
}

#[cfg(all(test, unix))]
mod tests {
    use super::remote_view_helper_status_probe;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn helper_status_probe_executes_read_only_contract_without_sudo() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-browser-helper-status-probe-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let helper = root.join("agent-browser-privileged-helper");
        fs::write(
            &helper,
            "#!/bin/sh\n[ -z \"${SUDO_COMMAND:-}\" ] || exit 91\n[ \"${1:-}\" = status-json ] || exit 92\nprintf '%s\\n' '{\"probeMode\":\"direct\"}'\n",
        )
        .unwrap();
        fs::set_permissions(&helper, fs::Permissions::from_mode(0o755)).unwrap();

        let report = remote_view_helper_status_probe(helper.to_str().unwrap());

        assert_eq!(report["success"], true);
        assert_eq!(report["parsed"]["probeMode"], "direct");
        fs::remove_dir_all(root).unwrap();
    }
}
