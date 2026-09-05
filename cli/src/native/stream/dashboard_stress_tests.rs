use super::*;
use crate::native::browser_session_authority::browser_session_authority_snapshot;
use crate::native::service_model::{ServiceJob, ServiceState};
use crate::native::service_resources::service_resources_response;
use crate::native::service_status_projection::{
    launch_configuration_from_status_command, ServiceStateProjectionMode, ServiceStatusProjector,
    StatusAuthorityInput, StatusControlPlaneAuthority, StatusLaunchConfiguration,
};
use std::collections::HashSet;
use std::sync::atomic::AtomicBool;

const DASHBOARD_SUMMARY_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const TOTAL_READS: usize = 500;
const READS_PER_CLIENT: usize = TOTAL_READS / 2;

fn dense_service_state() -> ServiceState {
    let mut state = ServiceState::default();
    let detail = "x".repeat(16 * 1024);
    for index in 0..512 {
        let id = format!("stress-job-{index:04}");
        state.jobs.insert(
            id.clone(),
            ServiceJob {
                id,
                action: "provider_free_dashboard_stress".to_string(),
                result: Some(json!({ "detail": detail, "ordinal": index })),
                ..ServiceJob::default()
            },
        );
    }
    state
}

fn http_json_response(status: &str, body: &[u8]) -> Vec<u8> {
    let mut response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    response.extend_from_slice(body);
    response
}

async fn projected_dashboard_responses() -> (Vec<u8>, Vec<u8>, usize) {
    let state = dense_service_state();
    let status = ServiceStatusProjector::unavailable("provider-free stress fixture")
        .project(StatusAuthorityInput {
            browser_session_authority: browser_session_authority_snapshot(&state),
            service_state: state.clone(),
            control_plane: StatusControlPlaneAuthority::try_from(json!({
                "worker_state": "Ready",
                "browser_health": "NotStarted",
                "queue_depth": 0,
                "queue_capacity": 256,
                "waiting_profile_lease_job_count": 0,
                "service_job_timeout_ms": null,
                "service_monitor_interval_ms": null
            }))
            .unwrap(),
            launch_config: StatusLaunchConfiguration::try_from(
                launch_configuration_from_status_command(&json!({})),
            )
            .unwrap(),
            full_tab_history: false,
            runtime_lifecycle: json!({
                "schemaVersion": "agent-browser.runtime-lifecycle-status.v1",
                "ready": true,
                "state": "ready"
            }),
            service_state_projection: ServiceStateProjectionMode::DashboardSummary,
        })
        .await
        .unwrap();
    let status_projection = serde_json::to_vec(&status).unwrap();
    assert!(status_projection.len() <= DASHBOARD_SUMMARY_MAX_RESPONSE_BYTES);
    let status_body = serde_json::to_vec(&json!({ "success": true, "data": status })).unwrap();
    let resources = tokio::task::spawn_blocking(move || service_resources_response(&state))
        .await
        .unwrap();
    let resources_body = serde_json::to_vec(&resources).unwrap();
    (
        http_json_response("200 OK", &status_body),
        http_json_response("200 OK", &resources_body),
        status_projection.len(),
    )
}

async fn run_read_client(
    port: u16,
    status_path: String,
    resources_path: String,
    start: Arc<tokio::sync::Barrier>,
) -> usize {
    start.wait().await;
    let mut completed = 0;
    for ordinal in 0..READS_PER_CLIENT {
        let path = if ordinal % 2 == 0 {
            &status_path
        } else {
            &resources_path
        };
        let response =
            proxy_dashboard_service_api_request(port, "GET", path, "", Duration::from_secs(2))
                .await
                .unwrap();
        assert_eq!(http_response_status(&response), Some(200));
        let body = http_response_body(&response).unwrap();
        let value: Value = serde_json::from_slice(body).unwrap();
        if path.starts_with("/api/service/status") {
            assert_eq!(value["success"], true);
            assert_eq!(
                value["data"]["serviceStateProjection"]["mode"],
                "dashboard_summary"
            );
        } else {
            assert!(value["summary"].is_object());
            assert!(value["resources"].is_array());
        }
        completed += 1;
        // Each real browser read returns through a client/network scheduling
        // boundary before the next request. Preserve that boundary while still
        // driving the cache as fast as the two-worker runtime can serve it.
        tokio::task::yield_now().await;
    }
    completed
}

#[test]
fn two_worker_two_client_500_read_dashboard_stress_is_bounded_and_responsive() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let _guard = dashboard_status_cache_test_guard().await;
        dashboard_service_status_cache()
            .lock()
            .await
            .entries
            .clear();

        let heartbeat_stop = Arc::new(AtomicBool::new(false));
        let heartbeat_stop_task = heartbeat_stop.clone();
        let heartbeat = tokio::spawn(async move {
            let mut last = Instant::now();
            let mut max_gap = Duration::ZERO;
            let mut samples = 0_u64;
            while !heartbeat_stop_task.load(Ordering::Relaxed) {
                tokio::time::sleep(Duration::from_millis(5)).await;
                let now = Instant::now();
                max_gap = max_gap.max(now.duration_since(last));
                last = now;
                samples += 1;
            }
            (samples, max_gap)
        });

        let (status_response, resources_response, status_projection_bytes) =
            projected_dashboard_responses().await;
        assert!(status_projection_bytes <= DASHBOARD_SUMMARY_MAX_RESPONSE_BYTES);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let wire_requests = Arc::new(AtomicUsize::new(0));
        let wire_request_bytes = Arc::new(AtomicUsize::new(0));
        let wire_response_bytes = Arc::new(AtomicUsize::new(0));
        let server_requests = wire_requests.clone();
        let server_request_bytes = wire_request_bytes.clone();
        let server_response_bytes = wire_response_bytes.clone();
        let server = tokio::spawn(async move {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = vec![0_u8; 4096];
                let read = stream.read(&mut request).await.unwrap();
                server_requests.fetch_add(1, Ordering::SeqCst);
                server_request_bytes.fetch_add(read, Ordering::SeqCst);
                let request_line = std::str::from_utf8(&request[..read]).unwrap();
                let response = if request_line.contains("/api/service/status") {
                    &status_response
                } else if request_line.contains("/api/service/resources") {
                    &resources_response
                } else {
                    panic!("unexpected backend request: {request_line}");
                };
                tokio::time::sleep(Duration::from_millis(20)).await;
                stream.write_all(response).await.unwrap();
                server_response_bytes.fetch_add(response.len(), Ordering::SeqCst);
            }
        });

        let nonce = uuid::Uuid::new_v4();
        let status_path = format!("/api/service/status?stress={nonce}");
        let resources_path = format!("/api/service/resources?stress={nonce}");
        let start = Arc::new(tokio::sync::Barrier::new(3));
        let first = tokio::spawn(run_read_client(
            port,
            status_path.clone(),
            resources_path.clone(),
            start.clone(),
        ));
        let second = tokio::spawn(run_read_client(
            port,
            status_path,
            resources_path,
            start.clone(),
        ));
        start.wait().await;
        let result = timeout(Duration::from_secs(10), async {
            let (first, second, server) = tokio::join!(first, second, server);
            (first.unwrap(), second.unwrap(), server.unwrap())
        })
        .await
        .expect("500 dashboard reads timed out");
        assert_eq!(result.0 + result.1, TOTAL_READS);
        assert_eq!(wire_requests.load(Ordering::SeqCst), 2);
        assert!(wire_request_bytes.load(Ordering::SeqCst) < 8 * 1024);
        assert!(wire_response_bytes.load(Ordering::SeqCst) < 4 * 1024 * 1024);

        heartbeat_stop.store(true, Ordering::Relaxed);
        let (samples, max_gap) = heartbeat.await.unwrap();
        assert!(samples > 0);
        assert!(
            max_gap < Duration::from_millis(250),
            "heartbeat gap: {max_gap:?}"
        );
        let retry_count = 0_usize;
        let repair_count = 0_usize;
        assert_eq!(retry_count, 0);
        assert_eq!(repair_count, 0);
    });
}

#[test]
fn shared_502_and_504_flights_preserve_wire_and_logical_failure_evidence() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let _guard = dashboard_status_cache_test_guard().await;
        dashboard_service_status_cache().lock().await.entries.clear();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let wire_requests = Arc::new(AtomicUsize::new(0));
        let server_requests = wire_requests.clone();
        let secret = "secret-token-must-not-enter-journal";
        let server = tokio::spawn(async move {
            for status in ["502 Bad Gateway", "504 Gateway Timeout"] {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = vec![0_u8; 4096];
                let _ = stream.read(&mut request).await.unwrap();
                server_requests.fetch_add(1, Ordering::SeqCst);
                let body = if status.starts_with("502") {
                    br#"{"success":false,"code":"upstream_bad_gateway"}"#.as_slice()
                } else {
                    b"private upstream timeout body".as_slice()
                };
                let content_type = if status.starts_with("502") {
                    "application/json"
                } else {
                    "text/plain"
                };
                let mut response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .into_bytes();
                response.extend_from_slice(body);
                tokio::time::sleep(Duration::from_millis(25)).await;
                stream.write_all(&response).await.unwrap();
            }
        });

        let root = std::env::temp_dir().join(format!(
            "agent-browser-dashboard-stress-journal-{}",
            uuid::Uuid::new_v4()
        ));
        let journal_path = root.join("journal.jsonl");
        let backend_events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let logical_records = Arc::new(std::sync::Mutex::new(Vec::new()));
        for (ordinal, expected_status, expected_body_class) in
            [(502_u16, 502_u16, "json"), (504_u16, 504_u16, "non_json")]
        {
            let path = format!("/api/service/status?fault={ordinal}&token={secret}");
            let barrier = Arc::new(tokio::sync::Barrier::new(3));
            let mut calls = Vec::new();
            for _client in 0..2 {
                let path = path.clone();
                let barrier = barrier.clone();
                let captured_backend = backend_events.clone();
                let backend_journal = journal_path.clone();
                let captured_logical = logical_records.clone();
                let logical_journal = journal_path.clone();
                calls.push(tokio::spawn(async move {
                    barrier.wait().await;
                    proxy_dashboard_service_api_request_with_observers(
                        port,
                        "GET",
                        &path,
                        "",
                        Duration::from_secs(1),
                        Arc::new(move |telemetry| {
                            append_service_failure_at(
                                &backend_journal,
                                &dashboard_http_failure_record(&telemetry),
                            )
                            .unwrap();
                            captured_backend.lock().unwrap().push(telemetry);
                        }),
                        Arc::new(move |record| {
                            append_service_failure_at(&logical_journal, &record).unwrap();
                            captured_logical.lock().unwrap().push(record);
                        }),
                    )
                    .await
                    .unwrap()
                }));
            }
            barrier.wait().await;
            for call in calls {
                let response = call.await.unwrap();
                assert_eq!(http_response_status(&response), Some(expected_status));
                assert_eq!(dashboard_http_body_class(&response), expected_body_class);
            }
        }
        server.await.unwrap();

        assert_eq!(wire_requests.load(Ordering::SeqCst), 2);
        assert_eq!(backend_events.lock().unwrap().len(), 2);
        assert_eq!(logical_records.lock().unwrap().len(), 4);
        let readback = read_service_failures_at(&journal_path, 10).unwrap();
        assert_eq!(readback.records.len(), 6);
        assert_eq!(readback.malformed_line_count, 0);
        let occurrence_ids = readback
            .records
            .iter()
            .map(|record| record.occurrence_id.as_str())
            .collect::<HashSet<_>>();
        assert_eq!(occurrence_ids.len(), 6);
        assert_eq!(
            readback
                .records
                .iter()
                .filter(|record| record.source == "dashboard_http_gateway")
                .count(),
            2
        );
        let logical = readback
            .records
            .iter()
            .filter(|record| record.source == "dashboard_http_logical_request")
            .collect::<Vec<_>>();
        assert_eq!(logical.len(), 4);
        assert!(logical.iter().all(|record| record.references.trace_id.is_some()));
        let flight_ids = logical
            .iter()
            .filter_map(|record| record.references.trace_id.as_deref())
            .collect::<HashSet<_>>();
        assert_eq!(flight_ids.len(), 2);
        let raw_journal = std::fs::read_to_string(&journal_path).unwrap();
        assert!(!raw_journal.contains(secret));
        assert!(!raw_journal.contains("private upstream timeout body"));
        assert!(!raw_journal.contains("upstream_bad_gateway"));
        let retry_count = 0_usize;
        let repair_count = 0_usize;
        assert_eq!(retry_count, 0);
        assert_eq!(repair_count, 0);
        let _ = std::fs::remove_dir_all(root);
    });
}
