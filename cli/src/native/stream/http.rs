use rust_embed::Embed;
use serde_json::{json, Value};
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;

#[cfg(windows)]
use crate::connection::resolve_port;
use crate::connection::{attach_daemon_auth_token, get_socket_dir};
use crate::flags::parse_flags;
use crate::native::service_access::{
    parse_service_access_plan_query, service_access_plan_for_state,
};
use crate::native::service_contracts::{
    service_contracts_metadata, SERVICE_REQUEST_ACTIONS, SERVICE_REQUEST_HTTP_ROUTE,
};
use crate::native::service_lifecycle::{
    select_service_profile_for_request, ProfileSelectionRequest,
};
use crate::native::service_model::{
    service_profile_allocations, service_profile_seeding_handoff, service_profile_sources,
    service_site_policy_sources, BrowserProfile, ProfileSelectionReason, ServiceEntitySource,
    ServiceState,
};
use crate::native::service_monitors::{
    parse_monitor_state, service_monitors_response, MonitorCollectionFilters,
};

use super::chat::{chat_status_json, handle_chat_request, handle_models_request};
use super::dashboard::spawn_session;
use super::discovery::discover_sessions;

const SERVICE_REQUEST_ALLOWED_ACTIONS: &[&str] = SERVICE_REQUEST_ACTIONS;

#[derive(Embed)]
#[folder = "../packages/dashboard/out/"]
struct DashboardAssets;

pub(super) const CORS_HEADERS: &str = "Access-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, DELETE, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\n";

/// Build CORS headers that reflect the request origin only when it passes
/// `is_allowed_origin`. Used for sensitive endpoints (chat, models) so the
/// API key is not accessible from arbitrary web pages.
pub(super) fn cors_headers_for_origin(origin: Option<&str>) -> String {
    let allowed_origin = match origin {
        Some(o) if super::is_allowed_origin(Some(o)) => o,
        _ => "http://localhost",
    };
    format!(
        "Access-Control-Allow-Origin: {}\r\nAccess-Control-Allow-Methods: GET, POST, DELETE, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\n",
        allowed_origin
    )
}

fn parse_origin(peeked: &[u8]) -> Option<String> {
    let header_str = std::str::from_utf8(peeked).ok()?;
    for line in header_str.lines() {
        if line.len() > 8 && line[..8].eq_ignore_ascii_case("origin: ") {
            return Some(line[8..].trim().to_string());
        }
    }
    None
}

pub(super) async fn handle_http_request(
    mut stream: tokio::net::TcpStream,
    peeked: &[u8],
    last_tabs: &Arc<RwLock<Vec<Value>>>,
    last_engine: &Arc<RwLock<String>>,
    session_name: &str,
) {
    let peeked_len = peeked.len();
    let mut discard = vec![0u8; peeked_len];
    let _ = stream.read_exact(&mut discard).await;

    let request = String::from_utf8_lossy(peeked);
    let first_line = request.lines().next().unwrap_or("");
    let method = first_line.split_whitespace().next().unwrap_or("GET");
    let raw_path = first_line.split_whitespace().nth(1).unwrap_or("/");
    let (path, query) = split_path_query(raw_path);
    let origin = parse_origin(peeked);

    if method == "OPTIONS" {
        let response = format!(
            "HTTP/1.1 204 No Content\r\n{CORS_HEADERS}Access-Control-Max-Age: 86400\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        let _ = stream.write_all(response.as_bytes()).await;
        return;
    }

    if method == "POST" {
        let full_body = read_full_body(&mut stream, peeked).await;
        if full_body.is_none()
            && (path == "/api/chat"
                || path == "/api/sessions"
                || path == "/api/command"
                || path.starts_with("/api/browser/"))
        {
            let body = r#"{"error":"Request body too large"}"#;
            let response = format!(
                "HTTP/1.1 413 Payload Too Large\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{CORS_HEADERS}\r\n",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes()).await;
            let _ = stream.write_all(body.as_bytes()).await;
            return;
        }
        let body_str = full_body.as_deref().unwrap_or("");

        if path == "/api/sessions" {
            let result = spawn_session(body_str).await;
            let (status, resp_body) = match result {
                Ok(msg) => ("200 OK", msg),
                Err(e) => (
                    "400 Bad Request",
                    format!(
                        r#"{{"success":false,"error":{}}}"#,
                        serde_json::to_string(&e).unwrap_or_else(|_| format!("\"{}\"", e))
                    ),
                ),
            };
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n{CORS_HEADERS}\r\n",
                resp_body.len()
            );
            let _ = stream.write_all(response.as_bytes()).await;
            let _ = stream.write_all(resp_body.as_bytes()).await;
            return;
        }

        if path == "/api/command" {
            let result = relay_command_to_daemon(session_name, body_str).await;
            let (status, resp_body) = match result {
                Ok(resp) => ("200 OK", resp),
                Err(e) => (
                    "502 Bad Gateway",
                    format!(
                        r#"{{"success":false,"error":{}}}"#,
                        serde_json::to_string(&e).unwrap_or_else(|_| format!("\"{}\"", e))
                    ),
                ),
            };
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n{CORS_HEADERS}\r\n",
                resp_body.len()
            );
            let _ = stream.write_all(response.as_bytes()).await;
            let _ = stream.write_all(resp_body.as_bytes()).await;
            return;
        }

        if let Some(command) = browser_api_command(method, path, query, body_str) {
            match command {
                Ok(cmd) => {
                    let result = relay_service_command(session_name, cmd).await;
                    write_json_result(&mut stream, result, "502 Bad Gateway").await;
                }
                Err(err) => write_json_result(&mut stream, Err(err), "400 Bad Request").await,
            }
            return;
        }

        if path == "/api/service/reconcile" {
            let result = relay_service_command(session_name, service_reconcile_command()).await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if path == SERVICE_REQUEST_HTTP_ROUTE {
            let cmd = match service_request_command(body_str) {
                Ok(cmd) => cmd,
                Err(err) => {
                    write_json_result(&mut stream, Err(err), "400 Bad Request").await;
                    return;
                }
            };
            let result = relay_service_command(session_name, cmd).await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if let Some(job_id) = service_job_cancel_id(path) {
            let result =
                relay_service_command(session_name, service_job_cancel_command(job_id)).await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if let Some(browser_id) = service_browser_retry_id(path) {
            let cmd = service_browser_retry_command(browser_id, query);
            let result = relay_service_command(session_name, cmd).await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if path == "/api/service/remedies/apply" {
            let cmd = match service_remedies_apply_command(query) {
                Ok(cmd) => cmd,
                Err(err) => {
                    write_json_result(&mut stream, Err(err), "400 Bad Request").await;
                    return;
                }
            };
            let result = relay_service_command(session_name, cmd).await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if let Some(incident_id) = service_incident_action_id(path, "/acknowledge") {
            let incident_id = match decode_path_segment(incident_id, "service incident id") {
                Ok(incident_id) => incident_id,
                Err(err) => {
                    write_json_result(&mut stream, Err(err), "400 Bad Request").await;
                    return;
                }
            };
            let cmd = service_incident_mutation_command(
                "service_incident_acknowledge",
                &incident_id,
                query,
            );
            let result = relay_service_command(session_name, cmd).await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if let Some(incident_id) = service_incident_action_id(path, "/resolve") {
            let incident_id = match decode_path_segment(incident_id, "service incident id") {
                Ok(incident_id) => incident_id,
                Err(err) => {
                    write_json_result(&mut stream, Err(err), "400 Bad Request").await;
                    return;
                }
            };
            let cmd =
                service_incident_mutation_command("service_incident_resolve", &incident_id, query);
            let result = relay_service_command(session_name, cmd).await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if let Some(profile_id) = service_profile_id(path) {
            let cmd = match service_profile_upsert_command(profile_id, body_str) {
                Ok(cmd) => cmd,
                Err(err) => {
                    write_json_result(&mut stream, Err(err), "400 Bad Request").await;
                    return;
                }
            };
            let result = relay_service_command(session_name, cmd).await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if let Some(profile_id) = service_profile_freshness_id(path) {
            let cmd = match service_profile_freshness_command(profile_id, body_str) {
                Ok(cmd) => cmd,
                Err(err) => {
                    write_json_result(&mut stream, Err(err), "400 Bad Request").await;
                    return;
                }
            };
            let result = relay_service_command(session_name, cmd).await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if let Some(service_session_id) = service_session_id(path) {
            let cmd = match service_session_upsert_command(service_session_id, body_str) {
                Ok(cmd) => cmd,
                Err(err) => {
                    write_json_result(&mut stream, Err(err), "400 Bad Request").await;
                    return;
                }
            };
            let result = relay_service_command(session_name, cmd).await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if let Some(site_policy_id) = service_site_policy_id(path) {
            let cmd = match service_site_policy_upsert_command(site_policy_id, body_str) {
                Ok(cmd) => cmd,
                Err(err) => {
                    write_json_result(&mut stream, Err(err), "400 Bad Request").await;
                    return;
                }
            };
            let result = relay_service_command(session_name, cmd).await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if path == "/api/service/monitors/run-due" {
            let result =
                relay_service_command(session_name, service_monitors_run_due_command()).await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if let Some(monitor_id) = service_monitor_action_id(path, "/pause") {
            let result = relay_service_command(
                session_name,
                service_monitor_state_command(monitor_id, "service_monitor_pause"),
            )
            .await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if let Some(monitor_id) = service_monitor_action_id(path, "/resume") {
            let result = relay_service_command(
                session_name,
                service_monitor_state_command(monitor_id, "service_monitor_resume"),
            )
            .await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if let Some(monitor_id) = service_monitor_action_id(path, "/reset-failures") {
            let result = relay_service_command(
                session_name,
                service_monitor_state_command(monitor_id, "service_monitor_reset_failures"),
            )
            .await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if let Some(monitor_id) = service_monitor_action_id(path, "/triage") {
            let result = relay_service_command(
                session_name,
                service_monitor_triage_command(monitor_id, query),
            )
            .await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if let Some(monitor_id) = service_monitor_id(path) {
            let cmd = match service_monitor_upsert_command(monitor_id, body_str) {
                Ok(cmd) => cmd,
                Err(err) => {
                    write_json_result(&mut stream, Err(err), "400 Bad Request").await;
                    return;
                }
            };
            let result = relay_service_command(session_name, cmd).await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if let Some(provider_id) = service_provider_id(path) {
            let cmd = match service_provider_upsert_command(provider_id, body_str) {
                Ok(cmd) => cmd,
                Err(err) => {
                    write_json_result(&mut stream, Err(err), "400 Bad Request").await;
                    return;
                }
            };
            let result = relay_service_command(session_name, cmd).await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if path == "/api/chat" {
            handle_chat_request(&mut stream, body_str, origin.as_deref()).await;
            return;
        }
    }

    if method == "DELETE" {
        if let Some(profile_id) = service_profile_id(path) {
            let result =
                relay_service_command(session_name, service_profile_delete_command(profile_id))
                    .await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if let Some(service_session_id) = service_session_id(path) {
            let result = relay_service_command(
                session_name,
                service_session_delete_command(service_session_id),
            )
            .await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if let Some(site_policy_id) = service_site_policy_id(path) {
            let result = relay_service_command(
                session_name,
                service_site_policy_delete_command(site_policy_id),
            )
            .await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if let Some(monitor_id) = service_monitor_id(path) {
            let result =
                relay_service_command(session_name, service_monitor_delete_command(monitor_id))
                    .await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }

        if let Some(provider_id) = service_provider_id(path) {
            let result =
                relay_service_command(session_name, service_provider_delete_command(provider_id))
                    .await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }
    }

    if method == "GET" && path == "/api/service/contracts" {
        write_json_value(
            &mut stream,
            "200 OK",
            json!({
                "success": true,
                "data": service_contracts_metadata(),
            }),
        )
        .await;
        return;
    }

    if method == "GET" && path == "/api/service/status" {
        let result = relay_service_command(session_name, service_status_command()).await;
        write_json_result(&mut stream, result, "502 Bad Gateway").await;
        return;
    }

    if method == "GET" && path == "/api/service/profiles/lookup" {
        match service_profile_lookup_response(query) {
            Ok(data) => {
                write_json_value(
                    &mut stream,
                    "200 OK",
                    json!({
                        "success": true,
                        "data": data,
                    }),
                )
                .await;
            }
            Err(err) => {
                write_json_result(&mut stream, Err(err), "400 Bad Request").await;
            }
        }
        return;
    }

    if method == "GET" && path == "/api/service/access-plan" {
        match service_access_plan_response(query) {
            Ok(data) => {
                write_json_value(
                    &mut stream,
                    "200 OK",
                    json!({
                        "success": true,
                        "data": data,
                    }),
                )
                .await;
            }
            Err(err) => {
                write_json_result(&mut stream, Err(err), "400 Bad Request").await;
            }
        }
        return;
    }

    if method == "GET" {
        if let Some(profile_id) = service_profile_seeding_handoff_id(path) {
            let mut service_state = load_service_state();
            service_state.refresh_profile_readiness();
            let target_service_id = query_params(query).into_iter().find_map(|(key, value)| {
                matches!(
                    key.as_str(),
                    "targetServiceId"
                        | "target_service_id"
                        | "target-service-id"
                        | "targetService"
                        | "target_service"
                        | "target-service"
                        | "siteId"
                        | "site_id"
                        | "site-id"
                        | "loginId"
                        | "login_id"
                        | "login-id"
                )
                .then_some(value)
            });
            match service_profile_seeding_handoff(
                &service_state,
                profile_id,
                target_service_id.as_deref(),
            ) {
                Ok(data) => {
                    write_json_value(
                        &mut stream,
                        "200 OK",
                        json!({"success": true, "data": data}),
                    )
                    .await;
                }
                Err(err) => {
                    write_json_result(&mut stream, Err(err), "404 Not Found").await;
                }
            }
            return;
        }

        if let Some(profile_id) = service_profile_readiness_id(path) {
            let service_state = load_service_state();
            let profile = service_state.profiles.get(profile_id);
            match profile {
                Some(profile) => {
                    write_json_value(
                        &mut stream,
                        "200 OK",
                        json!({
                            "success": true,
                            "data": {
                                "profileId": profile.id.clone(),
                                "targetReadiness": profile.target_readiness.clone(),
                                "count": profile.target_readiness.len(),
                            },
                        }),
                    )
                    .await;
                }
                None => {
                    write_json_result(
                        &mut stream,
                        Err(format!("Profile readiness not found: {profile_id}")),
                        "404 Not Found",
                    )
                    .await;
                }
            }
            return;
        }

        if let Some(profile_id) = service_profile_allocation_id(path) {
            let service_state = load_service_state();
            let profile_allocation = service_profile_allocations(&service_state)
                .into_iter()
                .find(|allocation| allocation.profile_id == profile_id);
            match profile_allocation {
                Some(profile_allocation) => {
                    write_json_value(
                        &mut stream,
                        "200 OK",
                        json!({
                            "success": true,
                            "data": {
                                "profileAllocation": profile_allocation,
                            },
                        }),
                    )
                    .await;
                }
                None => {
                    write_json_result(
                        &mut stream,
                        Err(format!("Profile allocation not found: {profile_id}")),
                        "404 Not Found",
                    )
                    .await;
                }
            }
            return;
        }
    }

    if method == "GET" {
        if let Some(command) = browser_api_command(method, path, query, "") {
            match command {
                Ok(cmd) => {
                    let result = relay_service_command(session_name, cmd).await;
                    write_json_result(&mut stream, result, "502 Bad Gateway").await;
                }
                Err(err) => write_json_result(&mut stream, Err(err), "400 Bad Request").await,
            }
            return;
        }
    }

    if method == "GET" {
        if let Some(contents) = service_collection_contents(path, query) {
            write_json_value(
                &mut stream,
                "200 OK",
                json!({
                    "success": true,
                    "data": contents,
                }),
            )
            .await;
            return;
        }
    }

    if method == "GET" && path == "/api/service/trace" {
        let cmd = match service_trace_command(query) {
            Ok(cmd) => cmd,
            Err(err) => {
                write_json_result(&mut stream, Err(err), "400 Bad Request").await;
                return;
            }
        };
        let result = relay_service_command(session_name, cmd).await;
        write_json_result(&mut stream, result, "502 Bad Gateway").await;
        return;
    }

    if method == "GET" && path == "/api/service/events" {
        let cmd = match service_events_command(query) {
            Ok(cmd) => cmd,
            Err(err) => {
                write_json_result(&mut stream, Err(err), "400 Bad Request").await;
                return;
            }
        };
        let result = relay_service_command(session_name, cmd).await;
        write_json_result(&mut stream, result, "502 Bad Gateway").await;
        return;
    }

    if method == "GET" && path == "/api/service/incidents" {
        let cmd = match service_incidents_command(query) {
            Ok(cmd) => cmd,
            Err(err) => {
                write_json_result(&mut stream, Err(err), "400 Bad Request").await;
                return;
            }
        };
        let result = relay_service_command(session_name, cmd).await;
        write_json_result(&mut stream, result, "502 Bad Gateway").await;
        return;
    }

    if method == "GET" {
        if let Some(incident_id) = service_incident_action_id(path, "/activity") {
            let incident_id = match decode_path_segment(incident_id, "service incident id") {
                Ok(incident_id) => incident_id,
                Err(err) => {
                    write_json_result(&mut stream, Err(err), "400 Bad Request").await;
                    return;
                }
            };
            let result = relay_service_command(
                session_name,
                service_incident_activity_command(&incident_id),
            )
            .await;
            write_json_result(&mut stream, result, "502 Bad Gateway").await;
            return;
        }
    }

    if method == "GET" && path.starts_with("/api/service/incidents/") {
        let Some(incident_id) = path
            .strip_prefix("/api/service/incidents/")
            .filter(|id| !id.is_empty())
        else {
            write_json_result(
                &mut stream,
                Err("Missing service incident id".to_string()),
                "400 Bad Request",
            )
            .await;
            return;
        };
        let incident_id = match decode_path_segment(incident_id, "service incident id") {
            Ok(incident_id) => incident_id,
            Err(err) => {
                write_json_result(&mut stream, Err(err), "400 Bad Request").await;
                return;
            }
        };
        let mut cmd = match service_incidents_command(query) {
            Ok(cmd) => cmd,
            Err(err) => {
                write_json_result(&mut stream, Err(err), "400 Bad Request").await;
                return;
            }
        };
        cmd["incidentId"] = json!(incident_id);
        let result = relay_service_command(session_name, cmd).await;
        write_json_result(&mut stream, result, "502 Bad Gateway").await;
        return;
    }

    if method == "GET" && path.starts_with("/api/service/jobs/") {
        let Some(job_id) = path
            .strip_prefix("/api/service/jobs/")
            .filter(|id| !id.is_empty())
        else {
            write_json_result(
                &mut stream,
                Err("Missing service job id".to_string()),
                "400 Bad Request",
            )
            .await;
            return;
        };
        let mut cmd = match service_jobs_command(query) {
            Ok(cmd) => cmd,
            Err(err) => {
                write_json_result(&mut stream, Err(err), "400 Bad Request").await;
                return;
            }
        };
        cmd["jobId"] = json!(job_id);
        let result = relay_service_command(session_name, cmd).await;
        write_json_result(&mut stream, result, "502 Bad Gateway").await;
        return;
    }

    if method == "GET" && path == "/api/service/jobs" {
        let cmd = match service_jobs_command(query) {
            Ok(cmd) => cmd,
            Err(err) => {
                write_json_result(&mut stream, Err(err), "400 Bad Request").await;
                return;
            }
        };
        let result = relay_service_command(session_name, cmd).await;
        write_json_result(&mut stream, result, "502 Bad Gateway").await;
        return;
    }

    if method == "GET" && path == "/api/models" {
        handle_models_request(&mut stream, origin.as_deref()).await;
        return;
    }

    let (status, content_type, body): (&str, &str, Vec<u8>) = if path == "/api/sessions" {
        (
            "200 OK",
            "application/json; charset=utf-8",
            discover_sessions().into_bytes(),
        )
    } else if path == "/api/tabs" {
        let tabs = last_tabs.read().await;
        (
            "200 OK",
            "application/json; charset=utf-8",
            serde_json::to_string(&*tabs)
                .unwrap_or_else(|_| "[]".to_string())
                .into_bytes(),
        )
    } else if path == "/api/status" {
        let engine = last_engine.read().await;
        (
            "200 OK",
            "application/json; charset=utf-8",
            format!(r#"{{"engine":"{}"}}"#, *engine).into_bytes(),
        )
    } else if path == "/api/chat/status" {
        (
            "200 OK",
            "application/json; charset=utf-8",
            chat_status_json().into_bytes(),
        )
    } else {
        serve_embedded_file(path)
    };

    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n{CORS_HEADERS}\r\n",
        status,
        content_type,
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.write_all(&body).await;
}

fn split_path_query(raw_path: &str) -> (&str, Option<&str>) {
    match raw_path.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (raw_path, None),
    }
}

async fn write_json_result(
    stream: &mut tokio::net::TcpStream,
    result: Result<String, String>,
    error_status: &str,
) {
    let (status, resp_body) = match result {
        Ok(resp) => ("200 OK", resp),
        Err(e) => (
            error_status,
            format!(
                r#"{{"success":false,"error":{}}}"#,
                serde_json::to_string(&e).unwrap_or_else(|_| format!("\"{}\"", e))
            ),
        ),
    };
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n{CORS_HEADERS}\r\n",
        resp_body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.write_all(resp_body.as_bytes()).await;
}

async fn write_json_value(stream: &mut tokio::net::TcpStream, status: &str, value: Value) {
    let resp_body = serde_json::to_string(&value).unwrap_or_else(|_| {
        r#"{"success":false,"error":"Failed to serialize JSON response"}"#.to_string()
    });
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n{CORS_HEADERS}\r\n",
        resp_body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.write_all(resp_body.as_bytes()).await;
}

fn service_status_command() -> Value {
    json!({
        "action": "service_status",
        "serviceState": load_service_state_snapshot(),
    })
}

fn service_reconcile_command() -> Value {
    json!({
        "action": "service_reconcile",
        "serviceState": load_service_state_snapshot(),
    })
}

fn browser_api_command(
    method: &str,
    path: &str,
    query: Option<&str>,
    body: &str,
) -> Option<Result<Value, String>> {
    match (method, path) {
        ("GET", "/api/browser/url") => {
            Some(browser_read_api_command("url", "http-browser-url", query))
        }
        ("GET", "/api/browser/title") => Some(browser_read_api_command(
            "title",
            "http-browser-title",
            query,
        )),
        ("GET", "/api/browser/tabs") => Some(browser_tabs_api_command(query)),
        ("POST", "/api/browser/navigate") => Some(browser_body_command(
            "navigate",
            "http-browser-navigate",
            body,
        )),
        ("POST", "/api/browser/back") => {
            Some(browser_body_command("back", "http-browser-back", body))
        }
        ("POST", "/api/browser/forward") => Some(browser_body_command(
            "forward",
            "http-browser-forward",
            body,
        )),
        ("POST", "/api/browser/reload") => {
            Some(browser_body_command("reload", "http-browser-reload", body))
        }
        ("POST", "/api/browser/new-tab") => Some(browser_body_command(
            "tab_new",
            "http-browser-new-tab",
            body,
        )),
        ("POST", "/api/browser/switch-tab") => Some(browser_body_command(
            "tab_switch",
            "http-browser-switch-tab",
            body,
        )),
        ("POST", "/api/browser/close-tab") => Some(browser_body_command(
            "tab_close",
            "http-browser-close-tab",
            body,
        )),
        ("POST", "/api/browser/viewport") => Some(browser_body_command(
            "viewport",
            "http-browser-viewport",
            body,
        )),
        ("POST", "/api/browser/user-agent") => Some(browser_body_command(
            "user_agent",
            "http-browser-user-agent",
            body,
        )),
        ("POST", "/api/browser/media") => Some(browser_body_command(
            "emulatemedia",
            "http-browser-media",
            body,
        )),
        ("POST", "/api/browser/timezone") => Some(browser_body_command(
            "timezone",
            "http-browser-timezone",
            body,
        )),
        ("POST", "/api/browser/locale") => {
            Some(browser_body_command("locale", "http-browser-locale", body))
        }
        ("POST", "/api/browser/geolocation") => Some(browser_body_command(
            "geolocation",
            "http-browser-geolocation",
            body,
        )),
        ("POST", "/api/browser/permissions") => Some(browser_body_command(
            "permissions",
            "http-browser-permissions",
            body,
        )),
        ("POST", "/api/browser/cookies/get") => Some(browser_body_command(
            "cookies_get",
            "http-browser-cookies-get",
            body,
        )),
        ("POST", "/api/browser/cookies/set") => Some(browser_body_command(
            "cookies_set",
            "http-browser-cookies-set",
            body,
        )),
        ("POST", "/api/browser/cookies/clear") => Some(browser_body_command(
            "cookies_clear",
            "http-browser-cookies-clear",
            body,
        )),
        ("POST", "/api/browser/storage/get") => Some(browser_body_command(
            "storage_get",
            "http-browser-storage-get",
            body,
        )),
        ("POST", "/api/browser/storage/set") => Some(browser_body_command(
            "storage_set",
            "http-browser-storage-set",
            body,
        )),
        ("POST", "/api/browser/storage/clear") => Some(browser_body_command(
            "storage_clear",
            "http-browser-storage-clear",
            body,
        )),
        ("POST", "/api/browser/console") => Some(browser_body_command(
            "console",
            "http-browser-console",
            body,
        )),
        ("POST", "/api/browser/errors") => {
            Some(browser_body_command("errors", "http-browser-errors", body))
        }
        ("POST", "/api/browser/set-content") => Some(browser_body_command(
            "setcontent",
            "http-browser-set-content",
            body,
        )),
        ("POST", "/api/browser/headers") => Some(browser_body_command(
            "headers",
            "http-browser-headers",
            body,
        )),
        ("POST", "/api/browser/offline") => Some(browser_body_command(
            "offline",
            "http-browser-offline",
            body,
        )),
        ("POST", "/api/browser/dialog") => {
            Some(browser_body_command("dialog", "http-browser-dialog", body))
        }
        ("POST", "/api/browser/clipboard") => Some(browser_body_command(
            "clipboard",
            "http-browser-clipboard",
            body,
        )),
        ("POST", "/api/browser/upload") => {
            Some(browser_body_command("upload", "http-browser-upload", body))
        }
        ("POST", "/api/browser/download") => Some(browser_body_command(
            "download",
            "http-browser-download",
            body,
        )),
        ("POST", "/api/browser/wait-for-download") => Some(browser_body_command(
            "waitfordownload",
            "http-browser-wait-for-download",
            body,
        )),
        ("POST", "/api/browser/pdf") => Some(browser_body_command("pdf", "http-browser-pdf", body)),
        ("POST", "/api/browser/response-body") => Some(browser_body_command(
            "responsebody",
            "http-browser-response-body",
            body,
        )),
        ("POST", "/api/browser/har/start") => Some(browser_body_command(
            "har_start",
            "http-browser-har-start",
            body,
        )),
        ("POST", "/api/browser/har/stop") => Some(browser_body_command(
            "har_stop",
            "http-browser-har-stop",
            body,
        )),
        ("POST", "/api/browser/route") => {
            Some(browser_body_command("route", "http-browser-route", body))
        }
        ("POST", "/api/browser/unroute") => Some(browser_body_command(
            "unroute",
            "http-browser-unroute",
            body,
        )),
        ("POST", "/api/browser/requests") => Some(browser_body_command(
            "requests",
            "http-browser-requests",
            body,
        )),
        ("POST", "/api/browser/request-detail") => Some(browser_body_command(
            "request_detail",
            "http-browser-request-detail",
            body,
        )),
        ("POST", "/api/browser/snapshot") => Some(browser_body_command(
            "snapshot",
            "http-browser-snapshot",
            body,
        )),
        ("POST", "/api/browser/screenshot") => Some(browser_body_command(
            "screenshot",
            "http-browser-screenshot",
            body,
        )),
        ("POST", "/api/browser/click") => {
            Some(browser_body_command("click", "http-browser-click", body))
        }
        ("POST", "/api/browser/fill") => {
            Some(browser_body_command("fill", "http-browser-fill", body))
        }
        ("POST", "/api/browser/wait") => {
            Some(browser_body_command("wait", "http-browser-wait", body))
        }
        ("POST", "/api/browser/type") => {
            Some(browser_body_command("type", "http-browser-type", body))
        }
        ("POST", "/api/browser/press") => {
            Some(browser_body_command("press", "http-browser-press", body))
        }
        ("POST", "/api/browser/hover") => {
            Some(browser_body_command("hover", "http-browser-hover", body))
        }
        ("POST", "/api/browser/select") => {
            Some(browser_body_command("select", "http-browser-select", body))
        }
        ("POST", "/api/browser/get-text") => Some(browser_body_command(
            "gettext",
            "http-browser-get-text",
            body,
        )),
        ("POST", "/api/browser/get-value") => Some(browser_body_command(
            "inputvalue",
            "http-browser-get-value",
            body,
        )),
        ("POST", "/api/browser/is-visible") => Some(browser_body_command(
            "isvisible",
            "http-browser-is-visible",
            body,
        )),
        ("POST", "/api/browser/get-attribute") => Some(browser_body_command(
            "getattribute",
            "http-browser-get-attribute",
            body,
        )),
        ("POST", "/api/browser/get-html") => Some(browser_body_command(
            "innerhtml",
            "http-browser-get-html",
            body,
        )),
        ("POST", "/api/browser/get-styles") => Some(browser_body_command(
            "styles",
            "http-browser-get-styles",
            body,
        )),
        ("POST", "/api/browser/count") => {
            Some(browser_body_command("count", "http-browser-count", body))
        }
        ("POST", "/api/browser/get-box") => Some(browser_body_command(
            "boundingbox",
            "http-browser-get-box",
            body,
        )),
        ("POST", "/api/browser/is-enabled") => Some(browser_body_command(
            "isenabled",
            "http-browser-is-enabled",
            body,
        )),
        ("POST", "/api/browser/is-checked") => Some(browser_body_command(
            "ischecked",
            "http-browser-is-checked",
            body,
        )),
        ("POST", "/api/browser/check") => {
            Some(browser_body_command("check", "http-browser-check", body))
        }
        ("POST", "/api/browser/uncheck") => Some(browser_body_command(
            "uncheck",
            "http-browser-uncheck",
            body,
        )),
        ("POST", "/api/browser/scroll") => {
            Some(browser_body_command("scroll", "http-browser-scroll", body))
        }
        ("POST", "/api/browser/scroll-into-view") => Some(browser_body_command(
            "scrollintoview",
            "http-browser-scroll-into-view",
            body,
        )),
        ("POST", "/api/browser/focus") => {
            Some(browser_body_command("focus", "http-browser-focus", body))
        }
        ("POST", "/api/browser/clear") => {
            Some(browser_body_command("clear", "http-browser-clear", body))
        }
        _ => None,
    }
}

fn browser_get_command(action: &str, id_prefix: &str) -> Value {
    json!({
        "id": format!("{}-{}", id_prefix, uuid::Uuid::new_v4()),
        "action": action,
    })
}

fn browser_read_api_command(
    action: &str,
    id_prefix: &str,
    query: Option<&str>,
) -> Result<Value, String> {
    let mut command = browser_get_command(action, id_prefix);
    apply_browser_common_query(&mut command, query, "browser read")?;
    Ok(command)
}

fn browser_tabs_api_command(query: Option<&str>) -> Result<Value, String> {
    let mut command = browser_get_command("tab_list", "http-browser-tabs");

    for (key, value) in query_params(query) {
        match key.as_str() {
            "verbose" => {
                command["verbose"] = json!(parse_query_bool("verbose", &value)?);
            }
            "jobTimeoutMs" | "job_timeout_ms" | "job-timeout-ms" => {
                command["jobTimeoutMs"] = json!(parse_positive_query_u64("jobTimeoutMs", &value)?)
            }
            "serviceName" | "service_name" | "service-name" => {
                command["serviceName"] = json!(value)
            }
            "agentName" | "agent_name" | "agent-name" => command["agentName"] = json!(value),
            "taskName" | "task_name" | "task-name" => command["taskName"] = json!(value),
            "" => {}
            _ => return Err(format!("Unknown browser tabs query parameter: {}", key)),
        }
    }

    Ok(command)
}

fn apply_browser_common_query(
    command: &mut Value,
    query: Option<&str>,
    label: &str,
) -> Result<(), String> {
    for (key, value) in query_params(query) {
        match key.as_str() {
            "jobTimeoutMs" | "job_timeout_ms" | "job-timeout-ms" => {
                command["jobTimeoutMs"] = json!(parse_positive_query_u64("jobTimeoutMs", &value)?);
            }
            "serviceName" | "service_name" | "service-name" => {
                command["serviceName"] = json!(value);
            }
            "agentName" | "agent_name" | "agent-name" => {
                command["agentName"] = json!(value);
            }
            "taskName" | "task_name" | "task-name" => {
                command["taskName"] = json!(value);
            }
            "" => {}
            _ => return Err(format!("Unknown {} query parameter: {}", label, key)),
        }
    }
    Ok(())
}

fn browser_body_command(action: &str, id_prefix: &str, body: &str) -> Result<Value, String> {
    let mut command = if body.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str::<Value>(body).map_err(|err| format!("Invalid JSON: {}", err))?
    };

    if !command.is_object() {
        return Err("Browser API request body must be a JSON object".to_string());
    }
    if command.get("id").is_none() {
        command["id"] = json!(format!("{}-{}", id_prefix, uuid::Uuid::new_v4()));
    }
    command["action"] = json!(action);
    Ok(command)
}

fn service_request_command(body: &str) -> Result<Value, String> {
    let request = if body.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str::<Value>(body).map_err(|err| format!("Invalid JSON: {}", err))?
    };

    let request = request
        .as_object()
        .ok_or_else(|| "Service request body must be a JSON object".to_string())?;
    let action = request
        .get("action")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "service request requires action".to_string())?;
    if !SERVICE_REQUEST_ALLOWED_ACTIONS.contains(&action) {
        return Err(format!(
            "service request action '{}' is not supported",
            action
        ));
    }
    let mut command = json!({
        "id": format!("http-service-request-{}-{}", action, uuid::Uuid::new_v4()),
        "action": action,
    });
    if let Some(params) = request.get("params") {
        let params = params
            .as_object()
            .ok_or_else(|| "service request params must be a JSON object".to_string())?;
        for (key, value) in params {
            if key != "id" && key != "action" {
                command[key] = value.clone();
            }
        }
    }
    for key in [
        "jobTimeoutMs",
        "profileLeasePolicy",
        "profileLeaseWaitTimeoutMs",
        "serviceName",
        "agentName",
        "taskName",
        "targetServiceId",
        "targetService",
        "targetServiceIds",
        "targetServices",
        "siteId",
        "siteIds",
        "loginId",
        "loginIds",
        "profile",
        "runtimeProfile",
    ] {
        if let Some(value) = request.get(key) {
            command[key] = value.clone();
        }
    }
    Ok(command)
}

fn parse_query_bool(name: &str, value: &str) -> Result<bool, String> {
    match value {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        _ => Err(format!("Invalid {} value: {}", name, value)),
    }
}

fn parse_positive_query_u64(name: &str, value: &str) -> Result<u64, String> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| format!("Invalid {} value: {}", name, value))?;
    if parsed == 0 {
        return Err(format!("{} must be a positive integer", name));
    }
    Ok(parsed)
}

fn service_collection_contents(path: &str, query: Option<&str>) -> Option<Value> {
    let service_state = load_service_state();
    match path {
        "/api/service/profiles" => {
            let profile_allocations = service_profile_allocations(&service_state);
            let profile_sources = service_profile_sources(&service_state);
            let profiles = service_state.profiles.values().cloned().collect::<Vec<_>>();
            Some(json!({
                "profiles": profiles,
                "profileSources": profile_sources,
                "profileAllocations": profile_allocations,
                "count": profiles.len(),
            }))
        }
        "/api/service/sessions" => {
            let sessions = service_state.sessions.values().cloned().collect::<Vec<_>>();
            Some(json!({
                "sessions": sessions,
                "count": sessions.len(),
            }))
        }
        "/api/service/browsers" => {
            let browsers = service_state.browsers.values().cloned().collect::<Vec<_>>();
            Some(json!({
                "browsers": browsers,
                "count": browsers.len(),
            }))
        }
        "/api/service/tabs" => {
            let tabs = service_state.tabs.values().cloned().collect::<Vec<_>>();
            Some(json!({
                "tabs": tabs,
                "count": tabs.len(),
            }))
        }
        "/api/service/monitors" => Some(service_monitors_response(
            &service_state,
            parse_monitor_collection_filters(query),
        )),
        "/api/service/site-policies" => {
            let site_policies = service_state
                .site_policies
                .values()
                .cloned()
                .collect::<Vec<_>>();
            let site_policy_sources = service_site_policy_sources(&service_state);
            Some(json!({
                "sitePolicies": site_policies,
                "sitePolicySources": site_policy_sources,
                "count": site_policies.len(),
            }))
        }
        "/api/service/providers" => {
            let providers = service_state
                .providers
                .values()
                .cloned()
                .collect::<Vec<_>>();
            Some(json!({
                "providers": providers,
                "count": providers.len(),
            }))
        }
        "/api/service/challenges" => {
            let challenges = service_state
                .challenges
                .values()
                .cloned()
                .collect::<Vec<_>>();
            Some(json!({
                "challenges": challenges,
                "count": challenges.len(),
            }))
        }
        _ => None,
    }
}

fn service_profile_lookup_response(query: Option<&str>) -> Result<Value, String> {
    let service_state = load_service_state();
    service_profile_lookup_response_for_state(query, &service_state)
}

fn service_access_plan_response(query: Option<&str>) -> Result<Value, String> {
    let service_state = load_service_state();
    service_access_plan_response_for_state(query, &service_state)
}

fn parse_monitor_collection_filters(query: Option<&str>) -> MonitorCollectionFilters {
    let mut filters = MonitorCollectionFilters::default();
    for (key, value) in query_params(query) {
        match key.as_str() {
            "state" => {
                filters.state = parse_monitor_state(value.trim());
            }
            "failed" | "failedOnly" | "failed-only" => {
                filters.failed_only = matches!(value.as_str(), "1" | "true" | "yes");
            }
            "summary" => {
                filters.summary = matches!(value.as_str(), "1" | "true" | "yes");
            }
            _ => {}
        }
    }
    filters
}

fn service_access_plan_response_for_state(
    query: Option<&str>,
    service_state: &ServiceState,
) -> Result<Value, String> {
    let request = parse_service_access_plan_query(query_params(query))?;
    Ok(service_access_plan_for_state(service_state, request))
}

fn service_profile_lookup_response_for_state(
    query: Option<&str>,
    service_state: &ServiceState,
) -> Result<Value, String> {
    let mut service_name = None;
    let mut target_service_ids = Vec::new();
    let mut readiness_profile_id = None;

    for (key, value) in query_params(query) {
        match key.as_str() {
            "serviceName" | "service_name" | "service-name" => service_name = non_empty(value),
            "targetServiceId" | "target_service_id" | "target-service-id" | "targetService"
            | "target_service" | "target-service" | "siteId" | "site_id" | "site-id"
            | "loginId" | "login_id" | "login-id" => {
                append_identity_values(&mut target_service_ids, &value);
            }
            "targetServiceIds" | "target_service_ids" | "target-service-ids" | "targetServices"
            | "target_services" | "target-services" | "siteIds" | "site_ids" | "site-ids"
            | "loginIds" | "login_ids" | "login-ids" => {
                append_identity_values(&mut target_service_ids, &value);
            }
            "readinessProfileId" | "readiness_profile_id" | "readiness-profile-id" => {
                readiness_profile_id = non_empty(value);
            }
            "" => {}
            _ => {
                return Err(format!(
                    "Unknown service profile lookup query parameter: {}",
                    key
                ))
            }
        }
    }

    target_service_ids.sort();
    target_service_ids.dedup();

    let request = ProfileSelectionRequest {
        service_name: service_name.clone(),
        target_service_ids: target_service_ids.clone(),
    };
    let selection = select_service_profile_for_request(service_state, &request);
    let selected_profile = selection
        .as_ref()
        .and_then(|selection| service_state.profiles.get(&selection.profile_id))
        .cloned();
    let readiness_id = readiness_profile_id.clone().or_else(|| {
        selection
            .as_ref()
            .map(|selection| selection.profile_id.clone())
    });
    let readiness_profile = readiness_id
        .as_deref()
        .and_then(|profile_id| service_state.profiles.get(profile_id));
    let target_readiness = readiness_profile
        .map(|profile| profile.target_readiness.clone())
        .unwrap_or_default();
    let target_readiness_count = target_readiness.len();
    let readiness = readiness_id.map(|profile_id| {
        json!({
            "profileId": profile_id,
            "targetReadiness": target_readiness,
            "count": target_readiness_count,
        })
    });

    Ok(json!({
        "query": {
            "serviceName": service_name,
            "targetServiceIds": target_service_ids,
            "readinessProfileId": readiness_profile_id,
        },
        "selectedProfile": selected_profile.clone(),
        "selectedProfileSource": selection.as_ref().map(|selection| {
            profile_source_value(service_state, &selection.profile_id)
        }),
        "selectedProfileMatch": selection.as_ref().map(|selection| {
            let (matched_field, matched_identity) = selected_profile
                .as_ref()
                .map(|profile| service_profile_match_details(profile, &request, selection.reason))
                .unwrap_or((None, None));
            json!({
                "profileId": selection.profile_id,
                "profile": selected_profile.clone(),
                "reason": selection.reason,
                "matchedField": matched_field,
                "matchedIdentity": matched_identity,
            })
        }),
        "readiness": readiness,
        "readinessSummary": readiness_summary(readiness.as_ref()),
    }))
}

fn profile_source_value(service_state: &ServiceState, profile_id: &str) -> Value {
    let source = service_state
        .profile_source(profile_id)
        .unwrap_or(ServiceEntitySource::PersistedState);
    json!({
        "id": profile_id,
        "source": source.as_str(),
        "overrideable": source.overrideable(),
        "precedence": ["config", "runtime_observed", "persisted_state"],
    })
}

fn service_profile_match_details(
    profile: &BrowserProfile,
    request: &ProfileSelectionRequest,
    reason: ProfileSelectionReason,
) -> (Option<&'static str>, Option<String>) {
    match reason {
        ProfileSelectionReason::AuthenticatedTarget => (
            Some("authenticatedServiceIds"),
            first_matching_identity(
                &request.target_service_ids,
                &profile.authenticated_service_ids,
            ),
        ),
        ProfileSelectionReason::TargetMatch => (
            Some("targetServiceIds"),
            first_matching_identity(&request.target_service_ids, &profile.target_service_ids),
        ),
        ProfileSelectionReason::ServiceAllowList => (
            Some("sharedServiceIds"),
            request
                .service_name
                .as_ref()
                .filter(|service_name| {
                    profile
                        .shared_service_ids
                        .iter()
                        .any(|allowed| allowed == *service_name)
                })
                .cloned(),
        ),
        ProfileSelectionReason::ExplicitProfile => (None, None),
    }
}

fn first_matching_identity(requested: &[String], candidates: &[String]) -> Option<String> {
    requested
        .iter()
        .find(|requested| candidates.iter().any(|candidate| candidate == *requested))
        .cloned()
}

fn append_identity_values(target_service_ids: &mut Vec<String>, value: &str) {
    for item in value.split(',') {
        if let Some(item) = non_empty(item.to_string()) {
            target_service_ids.push(item);
        }
    }
}

fn non_empty(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn readiness_summary(readiness: Option<&Value>) -> Value {
    let manual_rows = readiness
        .and_then(|readiness| readiness["targetReadiness"].as_array())
        .map(|rows| {
            rows.iter()
                .filter(|row| {
                    row["state"] == "needs_manual_seeding"
                        || row["manualSeedingRequired"].as_bool() == Some(true)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let target_service_ids = manual_rows
        .iter()
        .filter_map(|row| row["targetServiceId"].as_str())
        .collect::<Vec<_>>();
    let mut recommended_actions = manual_rows
        .iter()
        .filter_map(|row| row["recommendedAction"].as_str())
        .filter(|action| !action.is_empty())
        .collect::<Vec<_>>();
    recommended_actions.sort();
    recommended_actions.dedup();

    json!({
        "needsManualSeeding": manual_rows.iter().any(|row| row["state"] == "needs_manual_seeding"),
        "manualSeedingRequired": !manual_rows.is_empty(),
        "targetServiceIds": target_service_ids,
        "recommendedActions": recommended_actions,
    })
}

fn service_site_policy_id(path: &str) -> Option<&str> {
    path.strip_prefix("/api/service/site-policies/")
        .filter(|id| !id.is_empty() && !id.contains('/'))
}

fn service_profile_id(path: &str) -> Option<&str> {
    path.strip_prefix("/api/service/profiles/")
        .filter(|id| !id.is_empty() && !id.contains('/'))
}

fn service_profile_allocation_id(path: &str) -> Option<&str> {
    path.strip_prefix("/api/service/profiles/")
        .and_then(|suffix| suffix.strip_suffix("/allocation"))
        .filter(|id| !id.is_empty() && !id.contains('/'))
}

fn service_profile_readiness_id(path: &str) -> Option<&str> {
    path.strip_prefix("/api/service/profiles/")
        .and_then(|suffix| suffix.strip_suffix("/readiness"))
        .filter(|id| !id.is_empty() && !id.contains('/'))
}

fn service_profile_seeding_handoff_id(path: &str) -> Option<&str> {
    path.strip_prefix("/api/service/profiles/")
        .and_then(|suffix| suffix.strip_suffix("/seeding-handoff"))
        .filter(|id| !id.is_empty() && !id.contains('/'))
}

fn service_profile_freshness_id(path: &str) -> Option<&str> {
    path.strip_prefix("/api/service/profiles/")
        .and_then(|suffix| suffix.strip_suffix("/freshness"))
        .filter(|id| !id.is_empty() && !id.contains('/'))
}

fn service_session_id(path: &str) -> Option<&str> {
    path.strip_prefix("/api/service/sessions/")
        .filter(|id| !id.is_empty() && !id.contains('/'))
}

fn service_provider_id(path: &str) -> Option<&str> {
    path.strip_prefix("/api/service/providers/")
        .filter(|id| !id.is_empty() && !id.contains('/'))
}

fn service_monitor_id(path: &str) -> Option<&str> {
    path.strip_prefix("/api/service/monitors/")
        .filter(|id| !id.is_empty() && !id.contains('/') && *id != "run-due")
}

fn service_monitor_action_id<'a>(path: &'a str, suffix: &str) -> Option<&'a str> {
    path.strip_prefix("/api/service/monitors/")
        .and_then(|rest| rest.strip_suffix(suffix))
        .filter(|id| !id.is_empty() && !id.contains('/'))
}

fn service_job_cancel_id(path: &str) -> Option<&str> {
    path.strip_prefix("/api/service/jobs/")
        .and_then(|rest| rest.strip_suffix("/cancel"))
        .filter(|id| !id.is_empty())
}

fn service_browser_retry_id(path: &str) -> Option<&str> {
    path.strip_prefix("/api/service/browsers/")
        .and_then(|rest| rest.strip_suffix("/retry"))
        .filter(|id| !id.is_empty())
}

fn service_incident_action_id<'a>(path: &'a str, suffix: &str) -> Option<&'a str> {
    path.strip_prefix("/api/service/incidents/")
        .and_then(|rest| rest.strip_suffix(suffix))
        .filter(|id| !id.is_empty())
}

fn decode_path_segment(value: &str, label: &str) -> Result<String, String> {
    urlencoding::decode(value)
        .map(|decoded| decoded.into_owned())
        .map_err(|err| format!("Invalid encoded {}: {}", label, err))
}

fn service_job_cancel_command(job_id: &str) -> Value {
    json!({
        "action": "service_job_cancel",
        "jobId": job_id,
    })
}

fn service_profile_upsert_command(profile_id: &str, body: &str) -> Result<Value, String> {
    let profile = parse_service_config_body(body, "profile")?;
    Ok(json!({
        "id": format!("http-service-profile-upsert-{}", uuid::Uuid::new_v4()),
        "action": "service_profile_upsert",
        "profileId": profile_id,
        "profile": profile,
    }))
}

fn service_profile_freshness_command(profile_id: &str, body: &str) -> Result<Value, String> {
    let freshness = parse_service_config_body(body, "profile freshness")?;
    Ok(json!({
        "id": format!("http-service-profile-freshness-{}", uuid::Uuid::new_v4()),
        "action": "service_profile_freshness_update",
        "profileId": profile_id,
        "freshness": freshness,
    }))
}

fn service_profile_delete_command(profile_id: &str) -> Value {
    json!({
        "id": format!("http-service-profile-delete-{}", uuid::Uuid::new_v4()),
        "action": "service_profile_delete",
        "profileId": profile_id,
    })
}

fn service_session_upsert_command(session_id: &str, body: &str) -> Result<Value, String> {
    let session = parse_service_config_body(body, "session")?;
    Ok(json!({
        "id": format!("http-service-session-upsert-{}", uuid::Uuid::new_v4()),
        "action": "service_session_upsert",
        "sessionId": session_id,
        "session": session,
    }))
}

fn service_session_delete_command(session_id: &str) -> Value {
    json!({
        "id": format!("http-service-session-delete-{}", uuid::Uuid::new_v4()),
        "action": "service_session_delete",
        "sessionId": session_id,
    })
}

fn service_site_policy_upsert_command(site_policy_id: &str, body: &str) -> Result<Value, String> {
    let site_policy = parse_service_config_body(body, "site policy")?;
    Ok(json!({
        "id": format!("http-service-site-policy-upsert-{}", uuid::Uuid::new_v4()),
        "action": "service_site_policy_upsert",
        "sitePolicyId": site_policy_id,
        "sitePolicy": site_policy,
    }))
}

fn service_site_policy_delete_command(site_policy_id: &str) -> Value {
    json!({
        "id": format!("http-service-site-policy-delete-{}", uuid::Uuid::new_v4()),
        "action": "service_site_policy_delete",
        "sitePolicyId": site_policy_id,
    })
}

fn service_provider_upsert_command(provider_id: &str, body: &str) -> Result<Value, String> {
    let provider = parse_service_config_body(body, "provider")?;
    Ok(json!({
        "id": format!("http-service-provider-upsert-{}", uuid::Uuid::new_v4()),
        "action": "service_provider_upsert",
        "providerId": provider_id,
        "provider": provider,
    }))
}

fn service_monitor_upsert_command(monitor_id: &str, body: &str) -> Result<Value, String> {
    let monitor = parse_service_config_body(body, "monitor")?;
    Ok(json!({
        "id": format!("http-service-monitor-upsert-{}", uuid::Uuid::new_v4()),
        "action": "service_monitor_upsert",
        "monitorId": monitor_id,
        "monitor": monitor,
    }))
}

fn service_monitor_delete_command(monitor_id: &str) -> Value {
    json!({
        "id": format!("http-service-monitor-delete-{}", uuid::Uuid::new_v4()),
        "action": "service_monitor_delete",
        "monitorId": monitor_id,
    })
}

fn service_monitors_run_due_command() -> Value {
    json!({
        "id": format!("http-service-monitors-run-due-{}", uuid::Uuid::new_v4()),
        "action": "service_monitors_run_due",
    })
}

fn service_monitor_state_command(monitor_id: &str, action: &str) -> Value {
    json!({
        "id": format!("http-{action}-{}", uuid::Uuid::new_v4()),
        "action": action,
        "monitorId": monitor_id,
    })
}

fn service_monitor_triage_command(monitor_id: &str, query: Option<&str>) -> Value {
    let mut cmd = json!({
        "id": format!("http-service-monitor-triage-{}", uuid::Uuid::new_v4()),
        "action": "service_monitor_triage",
        "monitorId": monitor_id,
    });

    for (key, value) in query_params(query) {
        match key.as_str() {
            "by" => cmd["by"] = json!(value),
            "note" => cmd["note"] = json!(value),
            "serviceName" | "service_name" | "service-name" => {
                cmd["serviceName"] = json!(value);
            }
            "agentName" | "agent_name" | "agent-name" => {
                cmd["agentName"] = json!(value);
            }
            "taskName" | "task_name" | "task-name" => {
                cmd["taskName"] = json!(value);
            }
            "" => {}
            _ => {}
        }
    }

    cmd
}

fn service_provider_delete_command(provider_id: &str) -> Value {
    json!({
        "id": format!("http-service-provider-delete-{}", uuid::Uuid::new_v4()),
        "action": "service_provider_delete",
        "providerId": provider_id,
    })
}

fn parse_service_config_body(body: &str, label: &str) -> Result<Value, String> {
    serde_json::from_str::<Value>(body).map_err(|err| format!("Invalid {label} JSON: {err}"))
}

fn service_browser_retry_command(browser_id: &str, query: Option<&str>) -> Value {
    let mut cmd = json!({
        "action": "service_browser_retry",
        "browserId": browser_id,
    });

    for (key, value) in query_params(query) {
        match key.as_str() {
            "by" => cmd["by"] = json!(value),
            "note" => cmd["note"] = json!(value),
            "serviceName" | "service_name" | "service-name" => {
                cmd["serviceName"] = json!(value);
            }
            "agentName" | "agent_name" | "agent-name" => {
                cmd["agentName"] = json!(value);
            }
            "taskName" | "task_name" | "task-name" => {
                cmd["taskName"] = json!(value);
            }
            "" => {}
            _ => {}
        }
    }

    cmd
}

fn service_incident_mutation_command(
    action: &str,
    incident_id: &str,
    query: Option<&str>,
) -> Value {
    let mut cmd = json!({
        "action": action,
        "incidentId": incident_id,
    });

    for (key, value) in query_params(query) {
        match key.as_str() {
            "by" => cmd["by"] = json!(value),
            "note" => cmd["note"] = json!(value),
            "" => {}
            _ => {}
        }
    }

    cmd
}

fn service_incident_activity_command(incident_id: &str) -> Value {
    json!({
        "action": "service_incident_activity",
        "incidentId": incident_id,
        "serviceState": load_service_state_snapshot(),
    })
}

fn service_trace_command(query: Option<&str>) -> Result<Value, String> {
    let mut cmd = json!({
        "action": "service_trace",
        "serviceState": load_service_state_snapshot(),
    });

    for (key, value) in query_params(query) {
        match key.as_str() {
            "limit" => {
                let limit = value
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid limit value: {}", value))?;
                cmd["limit"] = json!(limit);
            }
            "browserId" | "browser_id" | "browser-id" => {
                cmd["browserId"] = json!(value);
            }
            "profileId" | "profile_id" | "profile-id" => {
                cmd["profileId"] = json!(value);
            }
            "sessionId" | "session_id" | "session-id" => {
                cmd["sessionId"] = json!(value);
            }
            "serviceName" | "service_name" | "service-name" => {
                cmd["serviceName"] = json!(value);
            }
            "agentName" | "agent_name" | "agent-name" => {
                cmd["agentName"] = json!(value);
            }
            "taskName" | "task_name" | "task-name" => {
                cmd["taskName"] = json!(value);
            }
            "since" => {
                cmd["since"] = json!(value);
            }
            "" => {}
            _ => return Err(format!("Unknown service trace query parameter: {}", key)),
        }
    }

    Ok(cmd)
}

fn service_events_command(query: Option<&str>) -> Result<Value, String> {
    let mut cmd = json!({
        "action": "service_events",
        "serviceState": load_service_state_snapshot(),
    });

    for (key, value) in query_params(query) {
        match key.as_str() {
            "limit" => {
                let limit = value
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid limit value: {}", value))?;
                cmd["limit"] = json!(limit);
            }
            "kind" => match value.as_str() {
                "reconciliation"
                | "browser_launch_recorded"
                | "browser_health_changed"
                | "browser_recovery_started"
                | "browser_recovery_override"
                | "tab_lifecycle_changed"
                | "profile_lease_wait_started"
                | "profile_lease_wait_ended"
                | "reconciliation_error"
                | "incident_acknowledged"
                | "incident_resolved" => {
                    cmd["kind"] = json!(value);
                }
                _ => return Err(format!("Invalid kind value: {}", value)),
            },
            "browserId" | "browser_id" | "browser-id" => {
                cmd["browserId"] = json!(value);
            }
            "profileId" | "profile_id" | "profile-id" => {
                cmd["profileId"] = json!(value);
            }
            "sessionId" | "session_id" | "session-id" => {
                cmd["sessionId"] = json!(value);
            }
            "serviceName" | "service_name" | "service-name" => {
                cmd["serviceName"] = json!(value);
            }
            "agentName" | "agent_name" | "agent-name" => {
                cmd["agentName"] = json!(value);
            }
            "taskName" | "task_name" | "task-name" => {
                cmd["taskName"] = json!(value);
            }
            "since" => {
                cmd["since"] = json!(value);
            }
            "" => {}
            _ => return Err(format!("Unknown service events query parameter: {}", key)),
        }
    }

    Ok(cmd)
}

fn service_incidents_command(query: Option<&str>) -> Result<Value, String> {
    let mut cmd = json!({
        "action": "service_incidents",
        "serviceState": load_service_state_snapshot(),
    });

    for (key, value) in query_params(query) {
        match key.as_str() {
            "summary" => {
                cmd["summary"] = json!(parse_query_bool("summary", &value)?);
            }
            "remedies" | "remediesOnly" | "remedies_only" | "remedies-only" => {
                let enabled = parse_query_bool(&key, &value)?;
                cmd["remediesOnly"] = json!(enabled);
                if enabled {
                    cmd["summary"] = json!(true);
                    cmd["state"] = json!("active");
                }
            }
            "limit" => {
                let limit = value
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid limit value: {}", value))?;
                cmd["limit"] = json!(limit);
            }
            "id" | "incidentId" | "incident_id" | "incident-id" => {
                cmd["incidentId"] = json!(value);
            }
            "state" => match value.as_str() {
                "active" | "recovered" | "service" => {
                    cmd["state"] = json!(value);
                }
                _ => return Err(format!("Invalid state value: {}", value)),
            },
            "severity" => match value.as_str() {
                "info" | "warning" | "error" | "critical" => {
                    cmd["severity"] = json!(value);
                }
                _ => return Err(format!("Invalid severity value: {}", value)),
            },
            "escalation" => match value.as_str() {
                "none"
                | "browser_degraded"
                | "browser_recovery"
                | "job_attention"
                | "monitor_attention"
                | "service_triage"
                | "os_degraded_possible" => {
                    cmd["escalation"] = json!(value);
                }
                _ => return Err(format!("Invalid escalation value: {}", value)),
            },
            "handlingState" | "handling_state" | "handling-state" => match value.as_str() {
                "unacknowledged" | "acknowledged" | "resolved" => {
                    cmd["handlingState"] = json!(value);
                }
                _ => return Err(format!("Invalid handlingState value: {}", value)),
            },
            "kind" => match value.as_str() {
                "browser_health_changed"
                | "reconciliation_error"
                | "service_job_timeout"
                | "service_job_cancelled" => {
                    cmd["kind"] = json!(value);
                }
                _ => return Err(format!("Invalid kind value: {}", value)),
            },
            "browserId" | "browser_id" | "browser-id" => {
                cmd["browserId"] = json!(value);
            }
            "profileId" | "profile_id" | "profile-id" => {
                cmd["profileId"] = json!(value);
            }
            "sessionId" | "session_id" | "session-id" => {
                cmd["sessionId"] = json!(value);
            }
            "serviceName" | "service_name" | "service-name" => {
                cmd["serviceName"] = json!(value);
            }
            "agentName" | "agent_name" | "agent-name" => {
                cmd["agentName"] = json!(value);
            }
            "taskName" | "task_name" | "task-name" => {
                cmd["taskName"] = json!(value);
            }
            "since" => {
                cmd["since"] = json!(value);
            }
            "" => {}
            _ => {
                return Err(format!(
                    "Unknown service incidents query parameter: {}",
                    key
                ))
            }
        }
    }

    Ok(cmd)
}

fn service_remedies_apply_command(query: Option<&str>) -> Result<Value, String> {
    let mut cmd = json!({
        "action": "service_remedies_apply",
        "escalation": "monitor_attention",
    });

    for (key, value) in query_params(query) {
        match key.as_str() {
            "escalation" => match value.as_str() {
                "browser_degraded" | "monitor_attention" | "os_degraded_possible" => {
                    cmd["escalation"] = json!(value)
                }
                _ => return Err(format!("Invalid escalation value: {}", value)),
            },
            "by" => cmd["by"] = json!(value),
            "note" => cmd["note"] = json!(value),
            "serviceName" | "service_name" | "service-name" => {
                cmd["serviceName"] = json!(value);
            }
            "agentName" | "agent_name" | "agent-name" => {
                cmd["agentName"] = json!(value);
            }
            "taskName" | "task_name" | "task-name" => {
                cmd["taskName"] = json!(value);
            }
            "" => {}
            _ => {
                return Err(format!(
                    "Unknown service remedies apply query parameter: {}",
                    key
                ))
            }
        }
    }

    Ok(cmd)
}

fn service_jobs_command(query: Option<&str>) -> Result<Value, String> {
    let mut cmd = json!({
        "action": "service_jobs",
        "serviceState": load_service_state_snapshot(),
    });

    for (key, value) in query_params(query) {
        match key.as_str() {
            "limit" => {
                let limit = value
                    .parse::<usize>()
                    .map_err(|_| format!("Invalid limit value: {}", value))?;
                cmd["limit"] = json!(limit);
            }
            "id" | "jobId" | "job_id" | "job-id" => {
                cmd["jobId"] = json!(value);
            }
            "state" => match value.as_str() {
                "queued"
                | "waiting_profile_lease"
                | "running"
                | "succeeded"
                | "failed"
                | "cancelled"
                | "timed_out" => {
                    cmd["state"] = json!(value);
                }
                _ => return Err(format!("Invalid state value: {}", value)),
            },
            "action" | "jobAction" | "job_action" | "job-action" => {
                cmd["jobAction"] = json!(value);
            }
            "profileId" | "profile_id" | "profile-id" => {
                cmd["profileId"] = json!(value);
            }
            "sessionId" | "session_id" | "session-id" => {
                cmd["sessionId"] = json!(value);
            }
            "serviceName" | "service_name" | "service-name" => {
                cmd["serviceName"] = json!(value);
            }
            "agentName" | "agent_name" | "agent-name" => {
                cmd["agentName"] = json!(value);
            }
            "taskName" | "task_name" | "task-name" => {
                cmd["taskName"] = json!(value);
            }
            "since" => {
                cmd["since"] = json!(value);
            }
            "" => {}
            _ => return Err(format!("Unknown service jobs query parameter: {}", key)),
        }
    }

    Ok(cmd)
}

fn query_params(query: Option<&str>) -> Vec<(String, String)> {
    query
        .map(|query| {
            url::form_urlencoded::parse(query.as_bytes())
                .into_owned()
                .collect()
        })
        .unwrap_or_default()
}

fn load_service_state_snapshot() -> Value {
    let args = vec!["service".to_string(), "status".to_string()];
    serde_json::to_value(parse_flags(&args).service_state).unwrap_or_else(|_| json!({}))
}

fn load_service_state() -> ServiceState {
    let args = vec!["service".to_string(), "status".to_string()];
    let mut service_state = parse_flags(&args).service_state;
    service_state.refresh_profile_readiness();
    service_state
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|p| p + 4)
        .or_else(|| buf.windows(2).position(|w| w == b"\n\n").map(|p| p + 2))
}

fn parse_content_length_bytes(headers: &[u8]) -> Option<usize> {
    let header_str = std::str::from_utf8(headers).ok()?;
    for line in header_str.lines() {
        if line.len() > 16 && line[..16].eq_ignore_ascii_case("content-length: ") {
            return line[16..].trim().parse().ok();
        }
    }
    None
}

const MAX_BODY_SIZE: usize = 10 * 1024 * 1024;

async fn read_full_body(stream: &mut tokio::net::TcpStream, peeked: &[u8]) -> Option<String> {
    let body_offset = find_header_end(peeked)?;
    let content_length = parse_content_length_bytes(&peeked[..body_offset])?;
    if content_length == 0 {
        return Some(String::new());
    }
    if content_length > MAX_BODY_SIZE {
        return None;
    }

    let peeked_body = &peeked[body_offset..];
    let peeked_body_len = peeked_body.len().min(content_length);

    let mut body = Vec::with_capacity(content_length);
    body.extend_from_slice(&peeked_body[..peeked_body_len]);

    let remaining = content_length - peeked_body_len;
    if remaining > 0 {
        let mut rest = vec![0u8; remaining];
        if stream.read_exact(&mut rest).await.is_err() {
            return String::from_utf8(body).ok();
        }
        body.extend_from_slice(&rest);
    }

    String::from_utf8(body).ok()
}

pub(super) async fn relay_command_to_daemon(
    session_name: &str,
    body: &str,
) -> Result<String, String> {
    let mut cmd: Value = serde_json::from_str(body).map_err(|e| format!("Invalid JSON: {}", e))?;

    if cmd.get("id").is_none() {
        let id = format!(
            "dash-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );
        cmd["id"] = json!(id);
    }

    let authenticated_cmd = attach_daemon_auth_token(&cmd, session_name)?;
    let mut json_str = serde_json::to_string(&authenticated_cmd).map_err(|e| e.to_string())?;
    json_str.push('\n');

    #[cfg(unix)]
    let stream = {
        let socket_path = get_socket_dir().join(format!("{}.sock", session_name));
        tokio::net::UnixStream::connect(&socket_path)
            .await
            .map_err(|e| format!("Failed to connect to daemon: {}", e))?
    };

    #[cfg(windows)]
    let stream = {
        let port = resolve_port(session_name);
        tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .map_err(|e| format!("Failed to connect to daemon: {}", e))?
    };

    let (reader, mut writer) = tokio::io::split(stream);

    writer
        .write_all(json_str.as_bytes())
        .await
        .map_err(|e| format!("Failed to send command: {}", e))?;

    let mut buf_reader = tokio::io::BufReader::new(reader);
    let mut response_line = String::new();
    tokio::io::AsyncBufReadExt::read_line(&mut buf_reader, &mut response_line)
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    Ok(response_line.trim().to_string())
}

async fn relay_service_command(session_name: &str, cmd: Value) -> Result<String, String> {
    let body = serde_json::to_string(&cmd).map_err(|e| e.to_string())?;
    relay_command_to_daemon(session_name, &body).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::actions::{execute_command, DaemonState};
    use crate::native::service_model::{
        assert_service_event_record_contract, assert_service_incident_record_contract,
        assert_service_job_naming_warning_contract, service_job_naming_warning_values,
        BrowserProfile, Challenge, ChallengeKind, ChallengePolicy, ChallengeState, MonitorState,
        MonitorTarget, ProfileReadinessState, ProfileTargetReadiness, ProviderKind,
        ServiceProvider, SiteMonitor, SitePolicy,
    };

    #[test]
    fn split_path_query_returns_path_and_query() {
        assert_eq!(
            split_path_query("/api/service/events?limit=2&kind=reconciliation"),
            ("/api/service/events", Some("limit=2&kind=reconciliation"))
        );
        assert_eq!(
            split_path_query("/api/service/status"),
            ("/api/service/status", None)
        );
    }

    #[test]
    fn service_profile_lookup_response_prefers_authenticated_target_profile() {
        let mut service_state = ServiceState::default();
        service_state.profiles.insert(
            "target-only".to_string(),
            BrowserProfile {
                id: "target-only".to_string(),
                name: "Target-only ACS profile".to_string(),
                target_service_ids: vec!["acs".to_string()],
                shared_service_ids: vec!["JournalDownloader".to_string()],
                persistent: true,
                ..BrowserProfile::default()
            },
        );
        service_state.profiles.insert(
            "authenticated".to_string(),
            BrowserProfile {
                id: "authenticated".to_string(),
                name: "Authenticated ACS profile".to_string(),
                target_service_ids: vec!["acs".to_string()],
                authenticated_service_ids: vec!["acs".to_string()],
                shared_service_ids: vec!["JournalDownloader".to_string()],
                target_readiness: vec![ProfileTargetReadiness {
                    target_service_id: "acs".to_string(),
                    state: ProfileReadinessState::Fresh,
                    evidence: "seeded smoke fixture".to_string(),
                    recommended_action: "Use this profile for ACS".to_string(),
                    ..ProfileTargetReadiness::default()
                }],
                persistent: true,
                ..BrowserProfile::default()
            },
        );
        service_state.profiles.insert(
            "other-service".to_string(),
            BrowserProfile {
                id: "other-service".to_string(),
                name: "Other service ACS profile".to_string(),
                target_service_ids: vec!["acs".to_string()],
                authenticated_service_ids: vec!["acs".to_string()],
                shared_service_ids: vec!["OtherService".to_string()],
                persistent: true,
                ..BrowserProfile::default()
            },
        );

        let response = service_profile_lookup_response_for_state(
            Some("service-name=JournalDownloader&login-id=acs"),
            &service_state,
        )
        .expect("profile lookup response should be built");

        assert_eq!(response["query"]["serviceName"], "JournalDownloader");
        assert_eq!(response["query"]["targetServiceIds"][0], "acs");
        assert_eq!(response["selectedProfile"]["id"], "authenticated");
        assert_eq!(
            response["selectedProfileMatch"]["profileId"],
            "authenticated"
        );
        assert_eq!(
            response["selectedProfileMatch"]["reason"],
            "authenticated_target"
        );
        assert_eq!(
            response["selectedProfileMatch"]["matchedField"],
            "authenticatedServiceIds"
        );
        assert_eq!(response["selectedProfileMatch"]["matchedIdentity"], "acs");
        assert_eq!(response["readiness"]["profileId"], "authenticated");
        assert_eq!(
            response["readiness"]["targetReadiness"][0]["state"],
            "fresh"
        );
        assert_eq!(response["readinessSummary"]["needsManualSeeding"], false);
        assert_ne!(response["selectedProfile"]["id"], "target-only");
        assert_ne!(response["selectedProfile"]["id"], "other-service");
    }

    #[test]
    fn service_profile_lookup_response_reports_target_match_details() {
        let mut service_state = ServiceState::default();
        service_state.profiles.insert(
            "target-only".to_string(),
            BrowserProfile {
                id: "target-only".to_string(),
                name: "Target-only Google profile".to_string(),
                target_service_ids: vec!["google".to_string(), "acs".to_string()],
                shared_service_ids: vec!["JournalDownloader".to_string()],
                persistent: true,
                ..BrowserProfile::default()
            },
        );

        let response = service_profile_lookup_response_for_state(
            Some("service-name=JournalDownloader&login-id=acs&target-service-id=google"),
            &service_state,
        )
        .expect("profile lookup response should be built");

        assert_eq!(response["selectedProfile"]["id"], "target-only");
        assert_eq!(response["selectedProfileMatch"]["reason"], "target_match");
        assert_eq!(
            response["selectedProfileMatch"]["matchedField"],
            "targetServiceIds"
        );
        assert_eq!(response["selectedProfileMatch"]["matchedIdentity"], "acs");
    }

    #[test]
    fn service_profile_lookup_response_reports_service_allow_list_details() {
        let mut service_state = ServiceState::default();
        service_state.profiles.insert(
            "service-shared".to_string(),
            BrowserProfile {
                id: "service-shared".to_string(),
                name: "Journal Downloader shared profile".to_string(),
                shared_service_ids: vec!["JournalDownloader".to_string()],
                persistent: true,
                ..BrowserProfile::default()
            },
        );

        let response = service_profile_lookup_response_for_state(
            Some("service-name=JournalDownloader&login-id=unknown"),
            &service_state,
        )
        .expect("profile lookup response should be built");

        assert_eq!(response["selectedProfile"]["id"], "service-shared");
        assert_eq!(
            response["selectedProfileMatch"]["reason"],
            "service_allow_list"
        );
        assert_eq!(
            response["selectedProfileMatch"]["matchedField"],
            "sharedServiceIds"
        );
        assert_eq!(
            response["selectedProfileMatch"]["matchedIdentity"],
            "JournalDownloader"
        );
    }

    #[test]
    fn service_access_plan_response_combines_profile_policy_provider_and_challenge() {
        let mut service_state = ServiceState::default();
        service_state.profiles.insert(
            "work".to_string(),
            BrowserProfile {
                id: "work".to_string(),
                name: "Work".to_string(),
                target_service_ids: vec!["google".to_string()],
                credential_provider_ids: vec!["manual".to_string()],
                target_readiness: vec![ProfileTargetReadiness {
                    target_service_id: "google".to_string(),
                    state: ProfileReadinessState::NeedsManualSeeding,
                    manual_seeding_required: true,
                    evidence: "manual_seed_required_without_authenticated_hint".to_string(),
                    recommended_action:
                        "launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable"
                            .to_string(),
                    ..ProfileTargetReadiness::default()
                }],
                ..BrowserProfile::default()
            },
        );
        service_state.site_policies.insert(
            "google".to_string(),
            SitePolicy {
                id: "google".to_string(),
                origin_pattern: "https://accounts.google.com".to_string(),
                manual_login_preferred: true,
                profile_required: true,
                auth_providers: vec!["manual".to_string()],
                challenge_policy: ChallengePolicy::ManualOnly,
                ..SitePolicy::default()
            },
        );
        service_state.providers.insert(
            "manual".to_string(),
            ServiceProvider {
                id: "manual".to_string(),
                kind: ProviderKind::ManualApproval,
                display_name: "Manual approval".to_string(),
                ..ServiceProvider::default()
            },
        );
        service_state.challenges.insert(
            "challenge-1".to_string(),
            Challenge {
                id: "challenge-1".to_string(),
                kind: ChallengeKind::TwoFactor,
                state: ChallengeState::WaitingForHuman,
                provider_id: Some("manual".to_string()),
                ..Challenge::default()
            },
        );

        let response = service_access_plan_response_for_state(
            Some("service-name=JournalDownloader&login-id=google&site-policy-id=google&challenge-id=challenge-1"),
            &service_state,
        )
        .expect("access plan response should be built");

        assert_eq!(response["query"]["serviceName"], "JournalDownloader");
        assert_eq!(response["selectedProfile"]["id"], "work");
        assert_eq!(response["sitePolicy"]["id"], "google");
        assert_eq!(response["providers"][0]["id"], "manual");
        assert_eq!(response["challenges"][0]["id"], "challenge-1");
        assert_eq!(response["readinessSummary"]["manualSeedingRequired"], true);
        assert_eq!(
            response["decision"]["recommendedAction"],
            "launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable"
        );
    }

    #[test]
    fn service_events_command_maps_query_filters() {
        let cmd = service_events_command(Some(
            "limit=7&kind=browser_health_changed&browser-id=browser-1&profile-id=work&session-id=session-1&service-name=JournalDownloader&agent-name=codex&task-name=probeACSwebsite&since=2026-04-22T00%3A00%3A00Z",
        ))
        .unwrap();

        assert_eq!(cmd["action"], "service_events");
        assert_eq!(cmd["limit"], 7);
        assert_eq!(cmd["kind"], "browser_health_changed");
        assert_eq!(cmd["browserId"], "browser-1");
        assert_eq!(cmd["profileId"], "work");
        assert_eq!(cmd["sessionId"], "session-1");
        assert_eq!(cmd["serviceName"], "JournalDownloader");
        assert_eq!(cmd["agentName"], "codex");
        assert_eq!(cmd["taskName"], "probeACSwebsite");
        assert_eq!(cmd["since"], "2026-04-22T00:00:00Z");
        assert!(cmd.get("serviceState").is_some());
    }

    #[tokio::test]
    async fn service_events_http_command_returns_record_contract_fields() {
        let mut cmd = service_events_command(Some("kind=browser_health_changed")).unwrap();
        cmd["serviceState"] = json!({
            "events": [
                {
                    "id": "event-1",
                    "timestamp": "2026-04-22T00:01:00Z",
                    "kind": "browser_health_changed",
                    "message": "Browser crashed",
                    "browserId": "browser-1",
                    "profileId": "work",
                    "sessionId": "session-1",
                    "serviceName": "JournalDownloader",
                    "agentName": "codex",
                    "taskName": "probeACSwebsite",
                    "previousHealth": "ready",
                    "currentHealth": "process_exited",
                    "details": {"reasonKind": "process_exited"}
                }
            ]
        });
        let mut state = DaemonState::new();

        let result = execute_command(&cmd, &mut state).await;

        assert_eq!(result["success"], true);
        assert_eq!(result["data"]["count"], 1);
        assert_eq!(result["data"]["events"][0]["id"], "event-1");
        assert_service_event_record_contract(&result["data"]["events"][0]);
    }

    #[test]
    fn service_events_command_accepts_tab_lifecycle_kind() {
        let cmd = service_events_command(Some("kind=tab_lifecycle_changed")).unwrap();

        assert_eq!(cmd["kind"], "tab_lifecycle_changed");
    }

    #[test]
    fn service_events_command_accepts_profile_lease_wait_kinds() {
        let started = service_events_command(Some("kind=profile_lease_wait_started")).unwrap();
        let ended = service_events_command(Some("kind=profile_lease_wait_ended")).unwrap();

        assert_eq!(started["kind"], "profile_lease_wait_started");
        assert_eq!(ended["kind"], "profile_lease_wait_ended");
    }

    #[test]
    fn service_events_command_accepts_browser_launch_kind() {
        let cmd = service_events_command(Some("kind=browser_launch_recorded")).unwrap();

        assert_eq!(cmd["kind"], "browser_launch_recorded");
    }

    #[test]
    fn service_events_command_accepts_browser_recovery_kind() {
        let cmd = service_events_command(Some("kind=browser_recovery_started")).unwrap();

        assert_eq!(cmd["kind"], "browser_recovery_started");
    }

    #[test]
    fn service_events_command_accepts_browser_recovery_override_kind() {
        let cmd = service_events_command(Some("kind=browser_recovery_override")).unwrap();

        assert_eq!(cmd["kind"], "browser_recovery_override");
    }

    #[test]
    fn service_events_command_accepts_incident_handling_kinds() {
        let acknowledged = service_events_command(Some("kind=incident_acknowledged")).unwrap();
        let resolved = service_events_command(Some("kind=incident_resolved")).unwrap();

        assert_eq!(acknowledged["kind"], "incident_acknowledged");
        assert_eq!(resolved["kind"], "incident_resolved");
    }

    #[test]
    fn service_incidents_command_maps_query_filters() {
        let cmd = service_incidents_command(Some(
            "summary=true&remedies=true&id=incident-1&limit=7&state=active&severity=critical&escalation=os_degraded_possible&handling-state=unacknowledged&kind=service_job_timeout&browser-id=browser-1&profile-id=work&session-id=session-1&service-name=JournalDownloader&agent-name=codex&task-name=probeACSwebsite&since=2026-04-22T00%3A00%3A00Z",
        ))
        .unwrap();

        assert_eq!(cmd["action"], "service_incidents");
        assert_eq!(cmd["summary"], true);
        assert_eq!(cmd["remediesOnly"], true);
        assert_eq!(cmd["incidentId"], "incident-1");
        assert_eq!(cmd["limit"], 7);
        assert_eq!(cmd["state"], "active");
        assert_eq!(cmd["severity"], "critical");
        assert_eq!(cmd["escalation"], "os_degraded_possible");
        assert_eq!(cmd["handlingState"], "unacknowledged");
        assert_eq!(cmd["kind"], "service_job_timeout");
        assert_eq!(cmd["browserId"], "browser-1");
        assert_eq!(cmd["profileId"], "work");
        assert_eq!(cmd["sessionId"], "session-1");
        assert_eq!(cmd["serviceName"], "JournalDownloader");
        assert_eq!(cmd["agentName"], "codex");
        assert_eq!(cmd["taskName"], "probeACSwebsite");
        assert_eq!(cmd["since"], "2026-04-22T00:00:00Z");
        assert!(cmd.get("serviceState").is_some());
    }

    #[test]
    fn split_path_query_handles_service_incident_detail() {
        assert_eq!(
            split_path_query("/api/service/incidents/incident-123?since=2026-04-22T00%3A00%3A00Z"),
            (
                "/api/service/incidents/incident-123",
                Some("since=2026-04-22T00%3A00%3A00Z")
            )
        );
    }

    #[test]
    fn service_remedies_apply_command_maps_query() {
        let cmd = service_remedies_apply_command(Some(
            "escalation=monitor_attention&by=operator&note=reviewed&service-name=JournalDownloader",
        ))
        .unwrap();

        assert_eq!(cmd["action"], "service_remedies_apply");
        assert_eq!(cmd["escalation"], "monitor_attention");
        assert_eq!(cmd["by"], "operator");
        assert_eq!(cmd["note"], "reviewed");
        assert_eq!(cmd["serviceName"], "JournalDownloader");
    }

    #[test]
    fn service_remedies_apply_command_accepts_os_degraded_possible() {
        let cmd =
            service_remedies_apply_command(Some("escalation=os_degraded_possible&by=operator"))
                .unwrap();

        assert_eq!(cmd["action"], "service_remedies_apply");
        assert_eq!(cmd["escalation"], "os_degraded_possible");
        assert_eq!(cmd["by"], "operator");
    }

    #[test]
    fn service_remedies_apply_command_accepts_browser_degraded() {
        let cmd = service_remedies_apply_command(Some("escalation=browser_degraded&by=operator"))
            .unwrap();

        assert_eq!(cmd["action"], "service_remedies_apply");
        assert_eq!(cmd["escalation"], "browser_degraded");
        assert_eq!(cmd["by"], "operator");
    }

    #[test]
    fn service_jobs_command_maps_query_filters() {
        let cmd = service_jobs_command(Some(
            "limit=7&state=waiting_profile_lease&action=navigate&profile-id=work&session-id=session-1&service-name=JournalDownloader&agent-name=codex&task-name=probeACSwebsite&since=2026-04-22T00%3A00%3A00Z",
        ))
        .unwrap();

        assert_eq!(cmd["action"], "service_jobs");
        assert_eq!(cmd["limit"], 7);
        assert_eq!(cmd["state"], "waiting_profile_lease");
        assert_eq!(cmd["jobAction"], "navigate");
        assert_eq!(cmd["profileId"], "work");
        assert_eq!(cmd["sessionId"], "session-1");
        assert_eq!(cmd["serviceName"], "JournalDownloader");
        assert_eq!(cmd["agentName"], "codex");
        assert_eq!(cmd["taskName"], "probeACSwebsite");
        assert_eq!(cmd["since"], "2026-04-22T00:00:00Z");
        assert!(cmd.get("serviceState").is_some());
    }

    #[test]
    fn service_jobs_command_maps_id_filter() {
        let cmd = service_jobs_command(Some("id=job-123")).unwrap();

        assert_eq!(cmd["action"], "service_jobs");
        assert_eq!(cmd["jobId"], "job-123");
    }

    #[tokio::test]
    async fn service_jobs_http_command_returns_naming_warning_contract_fields() {
        let mut cmd = service_jobs_command(Some("id=job-unnamed")).unwrap();
        cmd["serviceState"] = json!({
            "jobs": {
                "job-unnamed": {
                    "id": "job-unnamed",
                    "action": "navigate",
                    "state": "succeeded",
                    "namingWarnings": service_job_naming_warning_values(),
                    "hasNamingWarning": true,
                    "submittedAt": "2026-04-22T00:00:00Z"
                }
            }
        });
        let mut state = DaemonState::new();

        let result = execute_command(&cmd, &mut state).await;

        assert_eq!(result["success"], true);
        assert_eq!(result["data"]["count"], 1);
        assert_eq!(result["data"]["job"]["id"], "job-unnamed");
        assert_eq!(result["data"]["jobs"][0]["id"], "job-unnamed");
        for job in [&result["data"]["job"], &result["data"]["jobs"][0]] {
            assert_service_job_naming_warning_contract(job);
        }
    }

    #[tokio::test]
    async fn service_incidents_http_command_returns_record_contract_fields() {
        let mut cmd = service_incidents_command(Some("id=browser-1")).unwrap();
        cmd["serviceState"] = json!({
            "incidents": [
                {
                    "id": "browser-1",
                    "browserId": "browser-1",
                    "label": "browser-1",
                    "state": "active",
                    "severity": "error",
                    "escalation": "browser_recovery",
                    "recommendedAction": "Review recovery trace and retry or relaunch the affected browser.",
                    "latestTimestamp": "2026-04-22T00:01:00Z",
                    "latestMessage": "Browser crashed",
                    "latestKind": "browser_health_changed",
                    "currentHealth": "process_exited",
                    "eventIds": ["event-1"],
                    "jobIds": ["job-1"]
                }
            ]
        });
        let mut state = DaemonState::new();

        let result = execute_command(&cmd, &mut state).await;

        assert_eq!(result["success"], true);
        assert_eq!(result["data"]["count"], 1);
        assert_eq!(result["data"]["incident"]["id"], "browser-1");
        assert_eq!(result["data"]["incidents"][0]["id"], "browser-1");
        for incident in [&result["data"]["incident"], &result["data"]["incidents"][0]] {
            assert_service_incident_record_contract(incident);
        }
    }

    #[test]
    fn service_job_cancel_id_maps_path() {
        assert_eq!(
            service_job_cancel_id("/api/service/jobs/job-123/cancel"),
            Some("job-123")
        );
        assert_eq!(service_job_cancel_id("/api/service/jobs//cancel"), None);
        assert_eq!(service_job_cancel_id("/api/service/jobs/job-123"), None);
    }

    #[test]
    fn service_browser_retry_id_maps_path() {
        assert_eq!(
            service_browser_retry_id("/api/service/browsers/browser-123/retry"),
            Some("browser-123")
        );
        assert_eq!(
            service_browser_retry_id("/api/service/browsers//retry"),
            None
        );
        assert_eq!(
            service_browser_retry_id("/api/service/browsers/browser-123"),
            None
        );
    }

    #[test]
    fn service_browser_retry_command_maps_query() {
        let cmd = service_browser_retry_command(
            "browser-123",
            Some("by=operator&note=approved&service-name=JournalDownloader&agent-name=codex&task-name=probeACSwebsite"),
        );

        assert_eq!(cmd["action"], "service_browser_retry");
        assert_eq!(cmd["browserId"], "browser-123");
        assert_eq!(cmd["by"], "operator");
        assert_eq!(cmd["note"], "approved");
        assert_eq!(cmd["serviceName"], "JournalDownloader");
        assert_eq!(cmd["agentName"], "codex");
        assert_eq!(cmd["taskName"], "probeACSwebsite");
    }

    #[test]
    fn service_browser_retry_command_maps_trace_query_aliases() {
        let camel_case = service_browser_retry_command(
            "browser-123",
            Some("serviceName=JournalDownloader&agentName=codex&taskName=probeACSwebsite"),
        );
        assert_eq!(camel_case["serviceName"], "JournalDownloader");
        assert_eq!(camel_case["agentName"], "codex");
        assert_eq!(camel_case["taskName"], "probeACSwebsite");

        let snake_case = service_browser_retry_command(
            "browser-123",
            Some("service_name=JournalDownloader&agent_name=codex&task_name=probeACSwebsite"),
        );
        assert_eq!(snake_case["serviceName"], "JournalDownloader");
        assert_eq!(snake_case["agentName"], "codex");
        assert_eq!(snake_case["taskName"], "probeACSwebsite");
    }

    #[test]
    fn service_incident_action_id_maps_path() {
        assert_eq!(
            service_incident_action_id(
                "/api/service/incidents/incident-123/acknowledge",
                "/acknowledge"
            ),
            Some("incident-123")
        );
        assert_eq!(
            service_incident_action_id("/api/service/incidents/incident-123/resolve", "/resolve"),
            Some("incident-123")
        );
        assert_eq!(
            service_incident_action_id("/api/service/incidents/incident-123/activity", "/activity"),
            Some("incident-123")
        );
        assert_eq!(
            service_incident_action_id("/api/service/incidents//resolve", "/resolve"),
            None
        );
    }

    #[test]
    fn decode_path_segment_decodes_reserved_incident_id_characters() {
        assert_eq!(
            decode_path_segment("session%3Asip-123", "service incident id").unwrap(),
            "session:sip-123"
        );
    }

    #[test]
    fn service_incident_activity_command_maps_id() {
        let cmd = service_incident_activity_command("incident-123");

        assert_eq!(cmd["action"], "service_incident_activity");
        assert_eq!(cmd["incidentId"], "incident-123");
        assert!(cmd["serviceState"].is_object());
    }

    #[test]
    fn service_trace_command_maps_query_filters() {
        let cmd = service_trace_command(Some(
            "limit=7&browser-id=browser-1&profile-id=work&session-id=session-1&service-name=JournalDownloader&agent-name=codex&task-name=probeACSwebsite&since=2026-04-22T00%3A00%3A00Z",
        ))
        .unwrap();

        assert_eq!(cmd["action"], "service_trace");
        assert_eq!(cmd["limit"], 7);
        assert_eq!(cmd["browserId"], "browser-1");
        assert_eq!(cmd["profileId"], "work");
        assert_eq!(cmd["sessionId"], "session-1");
        assert_eq!(cmd["serviceName"], "JournalDownloader");
        assert_eq!(cmd["agentName"], "codex");
        assert_eq!(cmd["taskName"], "probeACSwebsite");
        assert_eq!(cmd["since"], "2026-04-22T00:00:00Z");
        assert!(cmd["serviceState"].is_object());
    }

    #[test]
    fn service_collection_contents_maps_known_resource_routes() {
        let profiles = service_collection_contents("/api/service/profiles", None).unwrap();
        let sessions = service_collection_contents("/api/service/sessions", None).unwrap();
        let browsers = service_collection_contents("/api/service/browsers", None).unwrap();
        let tabs = service_collection_contents("/api/service/tabs", None).unwrap();
        let monitors = service_collection_contents("/api/service/monitors", None).unwrap();
        let site_policies =
            service_collection_contents("/api/service/site-policies", None).unwrap();
        let providers = service_collection_contents("/api/service/providers", None).unwrap();
        let challenges = service_collection_contents("/api/service/challenges", None).unwrap();

        assert!(profiles["profiles"].is_array());
        assert!(profiles["profileSources"].is_array());
        assert!(profiles["profileAllocations"].is_array());
        assert!(sessions["sessions"].is_array());
        assert!(browsers["browsers"].is_array());
        assert!(tabs["tabs"].is_array());
        assert!(monitors["monitors"].is_array());
        assert!(site_policies["sitePolicies"].is_array());
        assert!(providers["providers"].is_array());
        assert!(challenges["challenges"].is_array());
        assert_eq!(
            service_collection_contents("/api/service/unknown", None),
            None
        );
    }

    #[test]
    fn service_collection_contents_filters_monitor_failures() {
        let mut service_state = ServiceState::default();
        service_state.monitors.insert(
            "healthy".to_string(),
            SiteMonitor {
                id: "healthy".to_string(),
                state: MonitorState::Active,
                target: MonitorTarget::SitePolicy("google".to_string()),
                last_checked_at: Some("2026-05-07T00:00:00Z".to_string()),
                last_succeeded_at: Some("2026-05-07T00:00:00Z".to_string()),
                ..SiteMonitor::default()
            },
        );
        service_state.monitors.insert(
            "faulted".to_string(),
            SiteMonitor {
                id: "faulted".to_string(),
                state: MonitorState::Faulted,
                target: MonitorTarget::SitePolicy("google".to_string()),
                last_checked_at: Some("2026-05-07T00:01:00Z".to_string()),
                last_failed_at: Some("2026-05-07T00:01:00Z".to_string()),
                consecutive_failures: 2,
                ..SiteMonitor::default()
            },
        );
        let response = service_monitors_response(
            &service_state,
            parse_monitor_collection_filters(Some("failed=true&summary=true&state=faulted")),
        );

        assert_eq!(response["count"], 1);
        assert_eq!(response["total"], 2);
        assert_eq!(response["monitors"][0]["id"], "faulted");
        assert_eq!(response["summary"]["failing"], 1);
        assert_eq!(response["summary"]["repeatedFailures"], 1);
    }

    #[test]
    fn service_config_entity_ids_map_mutation_paths() {
        assert_eq!(
            service_profile_id("/api/service/profiles/journal-downloader"),
            Some("journal-downloader")
        );
        assert_eq!(
            service_profile_allocation_id("/api/service/profiles/journal-downloader/allocation"),
            Some("journal-downloader")
        );
        assert_eq!(
            service_profile_readiness_id("/api/service/profiles/journal-downloader/readiness"),
            Some("journal-downloader")
        );
        assert_eq!(
            service_profile_seeding_handoff_id(
                "/api/service/profiles/journal-downloader/seeding-handoff"
            ),
            Some("journal-downloader")
        );
        assert_eq!(
            service_profile_freshness_id("/api/service/profiles/journal-downloader/freshness"),
            Some("journal-downloader")
        );
        assert_eq!(
            service_session_id("/api/service/sessions/journal-run"),
            Some("journal-run")
        );
        assert_eq!(
            service_site_policy_id("/api/service/site-policies/google"),
            Some("google")
        );
        assert_eq!(
            service_provider_id("/api/service/providers/manual"),
            Some("manual")
        );
        assert_eq!(
            service_monitor_id("/api/service/monitors/google-login-freshness"),
            Some("google-login-freshness")
        );
        assert_eq!(service_monitor_id("/api/service/monitors/run-due"), None);
        assert_eq!(
            service_monitors_run_due_command()["action"],
            "service_monitors_run_due"
        );
        assert_eq!(
            service_monitor_action_id(
                "/api/service/monitors/google-login-freshness/pause",
                "/pause"
            ),
            Some("google-login-freshness")
        );
        assert_eq!(
            service_monitor_action_id(
                "/api/service/monitors/google-login-freshness/resume",
                "/resume"
            ),
            Some("google-login-freshness")
        );
        assert_eq!(
            service_monitor_action_id(
                "/api/service/monitors/google-login-freshness/reset-failures",
                "/reset-failures"
            ),
            Some("google-login-freshness")
        );
        assert_eq!(
            service_monitor_action_id(
                "/api/service/monitors/google-login-freshness/triage",
                "/triage"
            ),
            Some("google-login-freshness")
        );
        assert_eq!(
            service_monitor_state_command("google-login-freshness", "service_monitor_pause")
                ["action"],
            "service_monitor_pause"
        );
        assert_eq!(
            service_monitor_state_command(
                "google-login-freshness",
                "service_monitor_reset_failures"
            )["action"],
            "service_monitor_reset_failures"
        );
        let triage = service_monitor_triage_command(
            "google-login-freshness",
            Some("by=operator&note=reviewed&service-name=JournalDownloader"),
        );
        assert_eq!(triage["action"], "service_monitor_triage");
        assert_eq!(triage["monitorId"], "google-login-freshness");
        assert_eq!(triage["by"], "operator");
        assert_eq!(triage["note"], "reviewed");
        assert_eq!(triage["serviceName"], "JournalDownloader");
        assert_eq!(service_profile_id("/api/service/profiles/"), None);
        assert_eq!(
            service_profile_allocation_id("/api/service/profiles/journal/extra/allocation"),
            None
        );
        assert_eq!(
            service_profile_readiness_id("/api/service/profiles/journal/extra/readiness"),
            None
        );
        assert_eq!(
            service_profile_seeding_handoff_id(
                "/api/service/profiles/journal/extra/seeding-handoff"
            ),
            None
        );
        assert_eq!(
            service_profile_allocation_id("/api/service/profiles/journal-downloader"),
            None
        );
        assert_eq!(
            service_profile_readiness_id("/api/service/profiles/journal-downloader"),
            None
        );
        assert_eq!(
            service_profile_freshness_id("/api/service/profiles/journal-downloader"),
            None
        );
        assert_eq!(
            service_profile_id("/api/service/profiles/journal/extra"),
            None
        );
        assert_eq!(service_session_id("/api/service/sessions/"), None);
        assert_eq!(
            service_session_id("/api/service/sessions/journal/extra"),
            None
        );
        assert_eq!(service_site_policy_id("/api/service/site-policies/"), None);
        assert_eq!(
            service_site_policy_id("/api/service/site-policies/google/extra"),
            None
        );
        assert_eq!(service_provider_id("/api/service/providers/"), None);
        assert_eq!(
            service_provider_id("/api/service/providers/manual/extra"),
            None
        );
        assert_eq!(service_monitor_id("/api/service/monitors/"), None);
        assert_eq!(
            service_monitor_id("/api/service/monitors/google/extra"),
            None
        );
    }

    #[test]
    fn browser_api_command_maps_named_get_routes() {
        let url = browser_api_command(
            "GET",
            "/api/browser/url",
            Some("service-name=JournalDownloader&agent-name=codex&task-name=probeACSwebsite&job-timeout-ms=1000"),
            "",
        )
        .unwrap()
        .unwrap();
        let title = browser_api_command("GET", "/api/browser/title", None, "")
            .unwrap()
            .unwrap();
        let tabs = browser_api_command(
            "GET",
            "/api/browser/tabs",
            Some("verbose=true&service-name=JournalDownloader&agent-name=codex&task-name=probeACSwebsite&job-timeout-ms=1000"),
            "",
        )
        .unwrap()
        .unwrap();

        assert_eq!(url["action"], "url");
        assert_eq!(url["serviceName"], "JournalDownloader");
        assert_eq!(url["agentName"], "codex");
        assert_eq!(url["taskName"], "probeACSwebsite");
        assert_eq!(url["jobTimeoutMs"], 1000);
        assert_eq!(title["action"], "title");
        assert_eq!(tabs["action"], "tab_list");
        assert_eq!(tabs["verbose"], true);
        assert_eq!(tabs["serviceName"], "JournalDownloader");
        assert_eq!(tabs["agentName"], "codex");
        assert_eq!(tabs["taskName"], "probeACSwebsite");
        assert_eq!(tabs["jobTimeoutMs"], 1000);
    }

    #[test]
    fn service_request_command_maps_request_object() {
        let command = service_request_command(
            r##"{"action":"navigate","params":{"url":"https://example.com","action":"ignored","id":"ignored"},"serviceName":"JournalDownloader","agentName":"codex","taskName":"probeACSwebsite","siteId":"acs","loginIds":["orcid"],"jobTimeoutMs":1000,"profileLeasePolicy":"wait","profileLeaseWaitTimeoutMs":2500}"##,
        )
        .unwrap();

        assert_eq!(command["action"], "navigate");
        assert!(command["id"]
            .as_str()
            .is_some_and(|id| id.starts_with("http-service-request-navigate-")));
        assert_eq!(command["url"], "https://example.com");
        assert_eq!(command["serviceName"], "JournalDownloader");
        assert_eq!(command["agentName"], "codex");
        assert_eq!(command["taskName"], "probeACSwebsite");
        assert_eq!(command["siteId"], "acs");
        assert_eq!(command["loginIds"][0], "orcid");
        assert_eq!(command["jobTimeoutMs"], 1000);
        assert_eq!(command["profileLeasePolicy"], "wait");
        assert_eq!(command["profileLeaseWaitTimeoutMs"], 2500);
    }

    #[test]
    fn service_request_command_accepts_contract_actions() {
        for action in SERVICE_REQUEST_ACTIONS {
            let command = service_request_command(&format!(
                r##"{{"action":"{}","params":{{"action":"ignored","id":"ignored"}},"serviceName":"JournalDownloader","agentName":"codex","taskName":"probeACSwebsite"}}"##,
                action
            ))
            .unwrap_or_else(|err| panic!("service request should accept {action}: {err}"));

            assert_eq!(command["action"], *action);
            let expected_id_prefix = format!("http-service-request-{action}-");
            assert!(command["id"]
                .as_str()
                .is_some_and(|id| id.starts_with(&expected_id_prefix)));
            assert_eq!(command["serviceName"], "JournalDownloader");
            assert_eq!(command["agentName"], "codex");
            assert_eq!(command["taskName"], "probeACSwebsite");
        }
    }

    #[test]
    fn browser_api_command_maps_named_post_routes() {
        let navigate = browser_api_command(
            "POST",
            "/api/browser/navigate",
            None,
            r##"{"url":"https://example.com","waitUntil":"load","serviceName":"JournalDownloader"}"##,
        )
        .unwrap()
        .unwrap();
        let back = browser_api_command("POST", "/api/browser/back", None, "{}")
            .unwrap()
            .unwrap();
        let forward = browser_api_command("POST", "/api/browser/forward", None, "{}")
            .unwrap()
            .unwrap();
        let reload = browser_api_command("POST", "/api/browser/reload", None, "{}")
            .unwrap()
            .unwrap();
        let new_tab = browser_api_command(
            "POST",
            "/api/browser/new-tab",
            None,
            r##"{"url":"about:blank"}"##,
        )
        .unwrap()
        .unwrap();
        let switch_tab =
            browser_api_command("POST", "/api/browser/switch-tab", None, r##"{"index":0}"##)
                .unwrap()
                .unwrap();
        let close_tab =
            browser_api_command("POST", "/api/browser/close-tab", None, r##"{"index":1}"##)
                .unwrap()
                .unwrap();
        let viewport = browser_api_command(
            "POST",
            "/api/browser/viewport",
            None,
            r##"{"width":800,"height":600,"deviceScaleFactor":2,"mobile":true}"##,
        )
        .unwrap()
        .unwrap();
        let user_agent = browser_api_command(
            "POST",
            "/api/browser/user-agent",
            None,
            r##"{"userAgent":"TestBot/1.0"}"##,
        )
        .unwrap()
        .unwrap();
        let media = browser_api_command(
            "POST",
            "/api/browser/media",
            None,
            r##"{"colorScheme":"dark","reducedMotion":"reduce"}"##,
        )
        .unwrap()
        .unwrap();
        let timezone = browser_api_command(
            "POST",
            "/api/browser/timezone",
            None,
            r##"{"timezoneId":"America/Chicago"}"##,
        )
        .unwrap()
        .unwrap();
        let locale = browser_api_command(
            "POST",
            "/api/browser/locale",
            None,
            r##"{"locale":"en-US"}"##,
        )
        .unwrap()
        .unwrap();
        let geolocation = browser_api_command(
            "POST",
            "/api/browser/geolocation",
            None,
            r##"{"latitude":41.8781,"longitude":-87.6298,"accuracy":10}"##,
        )
        .unwrap()
        .unwrap();
        let permissions = browser_api_command(
            "POST",
            "/api/browser/permissions",
            None,
            r##"{"permissions":["geolocation"]}"##,
        )
        .unwrap()
        .unwrap();
        let cookies_get = browser_api_command("POST", "/api/browser/cookies/get", None, "{}")
            .unwrap()
            .unwrap();
        let cookies_set = browser_api_command(
            "POST",
            "/api/browser/cookies/set",
            None,
            r##"{"name":"smoke","value":"ok"}"##,
        )
        .unwrap()
        .unwrap();
        let cookies_clear = browser_api_command("POST", "/api/browser/cookies/clear", None, "{}")
            .unwrap()
            .unwrap();
        let storage_get = browser_api_command(
            "POST",
            "/api/browser/storage/get",
            None,
            r##"{"type":"local","key":"smoke"}"##,
        )
        .unwrap()
        .unwrap();
        let storage_set = browser_api_command(
            "POST",
            "/api/browser/storage/set",
            None,
            r##"{"type":"local","key":"smoke","value":"ok"}"##,
        )
        .unwrap()
        .unwrap();
        let storage_clear = browser_api_command(
            "POST",
            "/api/browser/storage/clear",
            None,
            r##"{"type":"local"}"##,
        )
        .unwrap()
        .unwrap();
        let console =
            browser_api_command("POST", "/api/browser/console", None, r##"{"clear":true}"##)
                .unwrap()
                .unwrap();
        let errors = browser_api_command("POST", "/api/browser/errors", None, "{}")
            .unwrap()
            .unwrap();
        let set_content = browser_api_command(
            "POST",
            "/api/browser/set-content",
            None,
            r##"{"html":"<h1>Ready</h1>"}"##,
        )
        .unwrap()
        .unwrap();
        let headers = browser_api_command(
            "POST",
            "/api/browser/headers",
            None,
            r##"{"headers":{"X-Smoke":"ok"}}"##,
        )
        .unwrap()
        .unwrap();
        let offline = browser_api_command(
            "POST",
            "/api/browser/offline",
            None,
            r##"{"offline":true}"##,
        )
        .unwrap()
        .unwrap();
        let dialog = browser_api_command(
            "POST",
            "/api/browser/dialog",
            None,
            r##"{"response":"status"}"##,
        )
        .unwrap()
        .unwrap();
        let clipboard = browser_api_command(
            "POST",
            "/api/browser/clipboard",
            None,
            r##"{"operation":"write","text":"ok"}"##,
        )
        .unwrap()
        .unwrap();
        let upload = browser_api_command(
            "POST",
            "/api/browser/upload",
            None,
            r##"{"selector":"#file","files":["/tmp/file.txt"]}"##,
        )
        .unwrap()
        .unwrap();
        let download = browser_api_command(
            "POST",
            "/api/browser/download",
            None,
            r##"{"selector":"#download","path":"/tmp/download.txt"}"##,
        )
        .unwrap()
        .unwrap();
        let wait_for_download = browser_api_command(
            "POST",
            "/api/browser/wait-for-download",
            None,
            r##"{"path":"/tmp/download.txt"}"##,
        )
        .unwrap()
        .unwrap();
        let pdf = browser_api_command(
            "POST",
            "/api/browser/pdf",
            None,
            r##"{"path":"/tmp/page.pdf"}"##,
        )
        .unwrap()
        .unwrap();
        let response_body = browser_api_command(
            "POST",
            "/api/browser/response-body",
            None,
            r##"{"url":"/api/data"}"##,
        )
        .unwrap()
        .unwrap();
        let har_start = browser_api_command("POST", "/api/browser/har/start", None, "{}")
            .unwrap()
            .unwrap();
        let har_stop = browser_api_command(
            "POST",
            "/api/browser/har/stop",
            None,
            r##"{"path":"/tmp/capture.har"}"##,
        )
        .unwrap()
        .unwrap();
        let route = browser_api_command(
            "POST",
            "/api/browser/route",
            None,
            r##"{"url":"**/api/*","abort":true}"##,
        )
        .unwrap()
        .unwrap();
        let unroute = browser_api_command(
            "POST",
            "/api/browser/unroute",
            None,
            r##"{"url":"**/api/*"}"##,
        )
        .unwrap()
        .unwrap();
        let requests = browser_api_command(
            "POST",
            "/api/browser/requests",
            None,
            r##"{"filter":"/pixel","method":"GET","status":"2xx"}"##,
        )
        .unwrap()
        .unwrap();
        let request_detail = browser_api_command(
            "POST",
            "/api/browser/request-detail",
            None,
            r##"{"requestId":"request-1"}"##,
        )
        .unwrap()
        .unwrap();
        let click = browser_api_command(
            "POST",
            "/api/browser/click",
            None,
            r##"{"selector":"#ready","serviceName":"JournalDownloader"}"##,
        )
        .unwrap()
        .unwrap();
        let fill = browser_api_command(
            "POST",
            "/api/browser/fill",
            None,
            r##"{"selector":"#name","value":"Ada"}"##,
        )
        .unwrap()
        .unwrap();
        let snapshot = browser_api_command(
            "POST",
            "/api/browser/snapshot",
            None,
            r#"{"selector":"main","interactive":true}"#,
        )
        .unwrap()
        .unwrap();
        let screenshot = browser_api_command(
            "POST",
            "/api/browser/screenshot",
            None,
            r#"{"selector":"main","fullPage":true}"#,
        )
        .unwrap()
        .unwrap();
        let wait = browser_api_command(
            "POST",
            "/api/browser/wait",
            None,
            r##"{"selector":"#ready","state":"visible","timeoutMs":1000}"##,
        )
        .unwrap()
        .unwrap();
        let type_text = browser_api_command(
            "POST",
            "/api/browser/type",
            None,
            r##"{"selector":"#name","text":" Jr","delayMs":1}"##,
        )
        .unwrap()
        .unwrap();
        let press = browser_api_command("POST", "/api/browser/press", None, r#"{"key":"Enter"}"#)
            .unwrap()
            .unwrap();
        let hover = browser_api_command(
            "POST",
            "/api/browser/hover",
            None,
            r##"{"selector":"#ready"}"##,
        )
        .unwrap()
        .unwrap();
        let select = browser_api_command(
            "POST",
            "/api/browser/select",
            None,
            r##"{"selector":"#choice","values":["b"]}"##,
        )
        .unwrap()
        .unwrap();
        let get_text = browser_api_command(
            "POST",
            "/api/browser/get-text",
            None,
            r##"{"selector":"#ready"}"##,
        )
        .unwrap()
        .unwrap();
        let get_value = browser_api_command(
            "POST",
            "/api/browser/get-value",
            None,
            r##"{"selector":"#name"}"##,
        )
        .unwrap()
        .unwrap();
        let is_visible = browser_api_command(
            "POST",
            "/api/browser/is-visible",
            None,
            r##"{"selector":"#ready"}"##,
        )
        .unwrap()
        .unwrap();
        let get_attribute = browser_api_command(
            "POST",
            "/api/browser/get-attribute",
            None,
            r##"{"selector":"#ready","attribute":"id"}"##,
        )
        .unwrap()
        .unwrap();
        let get_html = browser_api_command(
            "POST",
            "/api/browser/get-html",
            None,
            r##"{"selector":"#box"}"##,
        )
        .unwrap()
        .unwrap();
        let get_styles = browser_api_command(
            "POST",
            "/api/browser/get-styles",
            None,
            r##"{"selector":"#box","properties":["display","width"]}"##,
        )
        .unwrap()
        .unwrap();
        let count = browser_api_command(
            "POST",
            "/api/browser/count",
            None,
            r##"{"selector":".item"}"##,
        )
        .unwrap()
        .unwrap();
        let get_box = browser_api_command(
            "POST",
            "/api/browser/get-box",
            None,
            r##"{"selector":"#box"}"##,
        )
        .unwrap()
        .unwrap();
        let is_enabled = browser_api_command(
            "POST",
            "/api/browser/is-enabled",
            None,
            r##"{"selector":"#name"}"##,
        )
        .unwrap()
        .unwrap();
        let is_checked = browser_api_command(
            "POST",
            "/api/browser/is-checked",
            None,
            r##"{"selector":"#remember"}"##,
        )
        .unwrap()
        .unwrap();
        let check = browser_api_command(
            "POST",
            "/api/browser/check",
            None,
            r##"{"selector":"#remember"}"##,
        )
        .unwrap()
        .unwrap();
        let uncheck = browser_api_command(
            "POST",
            "/api/browser/uncheck",
            None,
            r##"{"selector":"#remember"}"##,
        )
        .unwrap()
        .unwrap();
        let scroll = browser_api_command(
            "POST",
            "/api/browser/scroll",
            None,
            r##"{"direction":"down","amount":200}"##,
        )
        .unwrap()
        .unwrap();
        let scroll_into_view = browser_api_command(
            "POST",
            "/api/browser/scroll-into-view",
            None,
            r##"{"selector":"#box"}"##,
        )
        .unwrap()
        .unwrap();
        let focus = browser_api_command(
            "POST",
            "/api/browser/focus",
            None,
            r##"{"selector":"#name"}"##,
        )
        .unwrap()
        .unwrap();
        let clear = browser_api_command(
            "POST",
            "/api/browser/clear",
            None,
            r##"{"selector":"#name"}"##,
        )
        .unwrap()
        .unwrap();

        assert_eq!(navigate["action"], "navigate");
        assert_eq!(navigate["url"], "https://example.com");
        assert_eq!(navigate["waitUntil"], "load");
        assert_eq!(navigate["serviceName"], "JournalDownloader");
        assert_eq!(back["action"], "back");
        assert_eq!(forward["action"], "forward");
        assert_eq!(reload["action"], "reload");
        assert_eq!(new_tab["action"], "tab_new");
        assert_eq!(new_tab["url"], "about:blank");
        assert_eq!(switch_tab["action"], "tab_switch");
        assert_eq!(switch_tab["index"], 0);
        assert_eq!(close_tab["action"], "tab_close");
        assert_eq!(close_tab["index"], 1);
        assert_eq!(viewport["action"], "viewport");
        assert_eq!(viewport["width"], 800);
        assert_eq!(viewport["height"], 600);
        assert_eq!(viewport["deviceScaleFactor"], 2);
        assert_eq!(viewport["mobile"], true);
        assert_eq!(user_agent["action"], "user_agent");
        assert_eq!(user_agent["userAgent"], "TestBot/1.0");
        assert_eq!(media["action"], "emulatemedia");
        assert_eq!(media["colorScheme"], "dark");
        assert_eq!(timezone["action"], "timezone");
        assert_eq!(timezone["timezoneId"], "America/Chicago");
        assert_eq!(locale["action"], "locale");
        assert_eq!(locale["locale"], "en-US");
        assert_eq!(geolocation["action"], "geolocation");
        assert_eq!(geolocation["latitude"], 41.8781);
        assert_eq!(permissions["action"], "permissions");
        assert_eq!(permissions["permissions"][0], "geolocation");
        assert_eq!(cookies_get["action"], "cookies_get");
        assert_eq!(cookies_set["action"], "cookies_set");
        assert_eq!(cookies_set["name"], "smoke");
        assert_eq!(cookies_clear["action"], "cookies_clear");
        assert_eq!(storage_get["action"], "storage_get");
        assert_eq!(storage_get["type"], "local");
        assert_eq!(storage_set["action"], "storage_set");
        assert_eq!(storage_set["key"], "smoke");
        assert_eq!(storage_clear["action"], "storage_clear");
        assert_eq!(console["action"], "console");
        assert_eq!(console["clear"], true);
        assert_eq!(errors["action"], "errors");
        assert_eq!(set_content["action"], "setcontent");
        assert_eq!(set_content["html"], "<h1>Ready</h1>");
        assert_eq!(headers["action"], "headers");
        assert_eq!(headers["headers"]["X-Smoke"], "ok");
        assert_eq!(offline["action"], "offline");
        assert_eq!(offline["offline"], true);
        assert_eq!(dialog["action"], "dialog");
        assert_eq!(dialog["response"], "status");
        assert_eq!(clipboard["action"], "clipboard");
        assert_eq!(clipboard["operation"], "write");
        assert_eq!(upload["action"], "upload");
        assert_eq!(upload["selector"], "#file");
        assert_eq!(download["action"], "download");
        assert_eq!(download["path"], "/tmp/download.txt");
        assert_eq!(wait_for_download["action"], "waitfordownload");
        assert_eq!(wait_for_download["path"], "/tmp/download.txt");
        assert_eq!(pdf["action"], "pdf");
        assert_eq!(pdf["path"], "/tmp/page.pdf");
        assert_eq!(response_body["action"], "responsebody");
        assert_eq!(response_body["url"], "/api/data");
        assert_eq!(har_start["action"], "har_start");
        assert_eq!(har_stop["action"], "har_stop");
        assert_eq!(har_stop["path"], "/tmp/capture.har");
        assert_eq!(route["action"], "route");
        assert_eq!(route["url"], "**/api/*");
        assert_eq!(unroute["action"], "unroute");
        assert_eq!(unroute["url"], "**/api/*");
        assert_eq!(requests["action"], "requests");
        assert_eq!(requests["status"], "2xx");
        assert_eq!(request_detail["action"], "request_detail");
        assert_eq!(request_detail["requestId"], "request-1");
        assert_eq!(click["action"], "click");
        assert_eq!(click["selector"], "#ready");
        assert_eq!(click["serviceName"], "JournalDownloader");
        assert_eq!(fill["action"], "fill");
        assert_eq!(fill["value"], "Ada");
        assert_eq!(snapshot["action"], "snapshot");
        assert_eq!(snapshot["interactive"], true);
        assert_eq!(screenshot["action"], "screenshot");
        assert_eq!(screenshot["fullPage"], true);
        assert_eq!(wait["action"], "wait");
        assert_eq!(wait["state"], "visible");
        assert_eq!(type_text["action"], "type");
        assert_eq!(type_text["text"], " Jr");
        assert_eq!(press["action"], "press");
        assert_eq!(press["key"], "Enter");
        assert_eq!(hover["action"], "hover");
        assert_eq!(select["action"], "select");
        assert_eq!(select["values"][0], "b");
        assert_eq!(get_text["action"], "gettext");
        assert_eq!(get_value["action"], "inputvalue");
        assert_eq!(is_visible["action"], "isvisible");
        assert_eq!(get_attribute["action"], "getattribute");
        assert_eq!(get_attribute["attribute"], "id");
        assert_eq!(get_html["action"], "innerhtml");
        assert_eq!(get_styles["action"], "styles");
        assert_eq!(count["action"], "count");
        assert_eq!(get_box["action"], "boundingbox");
        assert_eq!(is_enabled["action"], "isenabled");
        assert_eq!(is_checked["action"], "ischecked");
        assert_eq!(check["action"], "check");
        assert_eq!(uncheck["action"], "uncheck");
        assert_eq!(scroll["action"], "scroll");
        assert_eq!(scroll["direction"], "down");
        assert_eq!(scroll_into_view["action"], "scrollintoview");
        assert_eq!(focus["action"], "focus");
        assert_eq!(clear["action"], "clear");
    }

    #[test]
    fn browser_api_command_rejects_invalid_arguments() {
        let tabs_err = browser_api_command("GET", "/api/browser/tabs", Some("verbose=maybe"), "")
            .unwrap()
            .unwrap_err();
        let body_err = browser_api_command("POST", "/api/browser/click", None, "[]")
            .unwrap()
            .unwrap_err();

        assert!(tabs_err.contains("Invalid verbose value"));
        assert!(body_err.contains("JSON object"));
        assert!(browser_api_command("GET", "/api/browser/unknown", None, "").is_none());
    }

    #[test]
    fn service_incident_mutation_command_maps_query() {
        let cmd = service_incident_mutation_command(
            "service_incident_acknowledge",
            "incident-123",
            Some("by=operator&note=triaged"),
        );

        assert_eq!(cmd["action"], "service_incident_acknowledge");
        assert_eq!(cmd["incidentId"], "incident-123");
        assert_eq!(cmd["by"], "operator");
        assert_eq!(cmd["note"], "triaged");
    }

    #[test]
    fn service_job_cancel_command_maps_id() {
        let cmd = service_job_cancel_command("job-123");

        assert_eq!(cmd["action"], "service_job_cancel");
        assert_eq!(cmd["jobId"], "job-123");
    }

    #[test]
    fn service_jobs_command_rejects_invalid_state() {
        let err = service_jobs_command(Some("state=broken")).unwrap_err();

        assert!(err.contains("Invalid state value"));
    }

    #[test]
    fn service_events_command_rejects_unknown_query() {
        let err = service_events_command(Some("bogus=true")).unwrap_err();

        assert!(err.contains("Unknown service events query parameter"));
    }

    #[test]
    fn service_events_command_rejects_invalid_kind() {
        let err = service_events_command(Some("kind=crash")).unwrap_err();

        assert!(err.contains("Invalid kind value"));
    }

    #[test]
    fn service_incidents_command_rejects_invalid_state() {
        let err = service_incidents_command(Some("state=failed")).unwrap_err();

        assert!(err.contains("Invalid state value"));
    }

    #[test]
    fn service_incidents_command_rejects_invalid_severity() {
        let err = service_incidents_command(Some("severity=panic")).unwrap_err();

        assert!(err.contains("Invalid severity value"));
    }

    #[test]
    fn service_incidents_command_rejects_invalid_escalation() {
        let err = service_incidents_command(Some("escalation=panic")).unwrap_err();

        assert!(err.contains("Invalid escalation value"));
    }

    #[test]
    fn service_incidents_command_rejects_invalid_handling_state() {
        let err = service_incidents_command(Some("handling-state=active")).unwrap_err();

        assert!(err.contains("Invalid handlingState value"));
    }

    #[test]
    fn service_incidents_command_rejects_invalid_summary() {
        let err = service_incidents_command(Some("summary=yes")).unwrap_err();

        assert!(err.contains("Invalid summary value"));
    }

    #[test]
    fn service_incidents_command_rejects_unknown_query() {
        let err = service_incidents_command(Some("bogus=true")).unwrap_err();

        assert!(err.contains("Unknown service incidents query parameter"));
    }
}

pub(super) fn serve_embedded_file(url_path: &str) -> (&'static str, &'static str, Vec<u8>) {
    let clean = url_path.trim_start_matches('/');
    let key = if clean.is_empty() {
        "index.html"
    } else {
        clean
    };

    let file = DashboardAssets::get(key).or_else(|| DashboardAssets::get("index.html"));

    match file {
        Some(content) => {
            let ext = key.rsplit('.').next().unwrap_or("");
            let ct = match ext {
                "html" => "text/html; charset=utf-8",
                "js" => "application/javascript; charset=utf-8",
                "css" => "text/css; charset=utf-8",
                "json" => "application/json; charset=utf-8",
                "svg" => "image/svg+xml",
                "png" => "image/png",
                "ico" => "image/x-icon",
                "woff2" => "font/woff2",
                "woff" => "font/woff",
                "txt" => "text/plain; charset=utf-8",
                _ => "application/octet-stream",
            };
            ("200 OK", ct, content.data.to_vec())
        }
        None => (
            "404 Not Found",
            "text/html; charset=utf-8",
            b"<html><body><p>404 Not Found</p></body></html>".to_vec(),
        ),
    }
}
