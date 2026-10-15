/// A disposable production connection/router/worker around a prepared browser.
/// This qualifies dispatch, not executable startup or provider acquisition.
#[derive(Clone)]
pub(crate) struct P237NativeDispatch {
    router: RuntimeHostRouter,
    session: String,
}

impl P237NativeDispatch {
    pub(crate) async fn new(state: DaemonState) -> Self {
        // Debug test executables are much larger than installed binaries.
        // Qualify immutable producer identity during fixture setup, keeping
        // its one-time hash cost outside the command response deadline.
        let started = Instant::now();
        let identity = super::service_failure_journal::ServiceFailureRecord::new(
            super::service_failure_journal::ServiceFailureCategory::ServiceAction,
            "p237_fixture",
            "setup",
            "identity_preparation",
            "synthetic fixture identity",
        );
        println!(
            "physical-native-dispatch-identity-ready: {}",
            serde_json::json!({
                "elapsedMs":started.elapsed().as_millis(), "identity":identity.build_identity,
            })
        );
        let session = state.session_id.clone();
        let router = RuntimeHostRouter::new(
            crate::connection::get_socket_dir(),
            &session,
            None,
            None,
            None,
            RuntimeHostWorkerOptions {
                service_reconcile_interval_ms: None,
                service_job_timeout_ms: None,
                service_monitor_interval_ms: None,
            },
        )
        .unwrap();
        let initial = router.lanes.remove(&session).unwrap();
        initial.control_plane.shutdown().await;
        router
            .lanes
            .insert(
                session.clone(),
                RuntimeLane {
                    control_plane: ControlPlaneWorker::start_with_options(
                        state.with_managed_session_host(router.browser_sessions.clone()),
                        None,
                        None,
                        None,
                    ),
                    ..initial
                },
            )
            .unwrap();
        Self { router, session }
    }

    pub(crate) async fn dispatch(&self, line: &str, token: &str) -> Value {
        let (mut client, server) = tokio::io::duplex(1024 * 1024);
        let router = self.router.clone();
        let session = self.session.clone();
        let token = Arc::new(token.to_owned());
        let task = tokio::spawn(async move {
            handle_connection(
                server,
                router,
                &session,
                None,
                None,
                Arc::new(Notify::new()),
                token,
            )
            .await;
        });
        client
            .write_all(format!("{line}\n").as_bytes())
            .await
            .unwrap();
        let mut response = String::new();
        let mut reader = BufReader::new(client);
        tokio::time::timeout(Duration::from_secs(10), reader.read_line(&mut response))
            .await
            .unwrap()
            .unwrap();
        drop(reader);
        task.await.unwrap();
        serde_json::from_str(&response).unwrap()
    }

    pub(crate) async fn close(&self, token: &str) {
        let response = self
            .dispatch(
                &serde_json::json!({
                    "id":format!("fixture-close-{}", self.session), "action":"close",
                    (DAEMON_AUTH_FIELD): token,
                })
                .to_string(),
                token,
            )
            .await;
        assert_eq!(
            response["success"], true,
            "native fixture close failed: {response}"
        );
        self.router.shutdown().await;
    }
}
