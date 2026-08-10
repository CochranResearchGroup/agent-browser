#![allow(unused_imports)]
use super::shared::*;
pub(crate) fn remote_view_open_ensure_display_access(
    route_binding: &super::super::super::remote_view::RemoteViewRouteBinding,
) -> Result<Value, String> {
    let Some(display_name) = route_binding
        .launch_display_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Err(format!(
            "route_display_missing: route '{}' has no launch display",
            route_binding.route_id
        ));
    };
    let initial_probe = remote_view_open_display_access_probe(display_name);
    if initial_probe
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(json!(
            { "state" : "already_ready", "displayName" : display_name, "probe" :
            initial_probe, }
        ));
    }
    let route_user = route_binding
        .route_user
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!(
                "x11_auth_denied: route '{}' display '{}' is not accessible and no route user was reported",
                route_binding.route_id, display_name
            )
        })?;
    let operator_user = env::var("AGENT_BROWSER_RDP_DISPLAY_ACCESS_USER")
        .or_else(|_| env::var("USER"))
        .map(|value| value.trim().to_string())
        .ok()
        .filter(|value| !value.is_empty() && value != "root")
        .ok_or_else(|| {
            format!(
                "display_access_grant_failed: route '{}' display '{}' cannot infer non-root operator user",
                route_binding.route_id, display_name
            )
        })?;
    let helper_path = env::var("AGENT_BROWSER_PRIVILEGED_HELPER").unwrap_or_else(|_| {
        "/usr/local/libexec/agent-browser/agent-browser-privileged-helper".to_string()
    });
    let status = Command::new("timeout")
        .args([
            "--kill-after=1",
            REMOTE_VIEW_DISPLAY_ACCESS_GRANT_TIMEOUT_SECONDS,
            "sudo",
            "-n",
            &helper_path,
            "grant-display-access",
            "--operator-user",
            &operator_user,
            "--route-user",
            route_user,
            "--display",
            display_name,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|err| {
            format!(
                "display_access_grant_failed: route '{}' display '{}' bounded helper could not start: {}",
                route_binding.route_id, display_name, err
            )
        })?;
    if !status.success() {
        return Err(remote_view_display_access_grant_error(
            &route_binding.route_id,
            display_name,
            status.code().unwrap_or(-1),
            "",
        ));
    }
    let final_probe = remote_view_open_display_access_probe(display_name);
    if final_probe
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(json!(
            { "state" : "granted", "displayName" : display_name, "operatorUser" :
            operator_user, "routeUser" : route_user, "helperPath" : helper_path,
            "helperTimeout" : REMOTE_VIEW_DISPLAY_ACCESS_GRANT_TIMEOUT_SECONDS,
            "probe" : final_probe, }
        ));
    }
    Err(format!(
        "x11_auth_denied: route '{}' display '{}' remained inaccessible after display access grant",
        route_binding.route_id, display_name
    ))
}
pub(crate) fn remote_view_display_access_grant_error(
    route_id: &str,
    display_name: &str,
    exit_code: i32,
    stderr: &str,
) -> String {
    let stderr_suffix = if stderr.is_empty() {
        String::new()
    } else {
        format!(": {stderr}")
    };
    if matches!(exit_code, 124 | 137) {
        return format!(
            "display_access_grant_timeout: route '{}' display '{}' helper exceeded {}{}",
            route_id, display_name, REMOTE_VIEW_DISPLAY_ACCESS_GRANT_TIMEOUT_SECONDS, stderr_suffix
        );
    }
    format!(
        "display_access_grant_failed: route '{}' display '{}' helper exited with {}{}",
        route_id, display_name, exit_code, stderr_suffix
    )
}
pub(crate) fn remote_view_open_display_access_probe(display_name: &str) -> Value {
    match Command::new("timeout")
        .args(["--kill-after=1", "2", "xdpyinfo"])
        .env("DISPLAY", display_name)
        .output()
    {
        Ok(output) => {
            json!(
                { "available" : true, "success" : output.status.success(), "exitCode" :
                output.status.code(), "stdout" : String::from_utf8_lossy(& output.stdout)
                .lines().find(| line | line.trim_start().starts_with("name of display:"))
                .unwrap_or("").trim(), "stderr" : String::from_utf8_lossy(& output
                .stderr).trim().chars().take(240).collect::< String > (), }
            )
        }
        Err(error) => {
            json!(
                { "available" : false, "success" : false, "exitCode" : null, "stdout" :
                "", "stderr" : error.to_string(), }
            )
        }
    }
}
pub(crate) fn remote_view_open_dry_run(cmd: &Value) -> bool {
    cmd.get("dryRun")
        .and_then(Value::as_bool)
        .or_else(|| {
            cmd.get("params")
                .and_then(|params| params.get("dryRun"))
                .and_then(Value::as_bool)
        })
        .unwrap_or(false)
}
pub(crate) async fn remote_view_open_operator_access_readiness(
    route_binding: &RemoteViewRouteBinding,
) -> Option<Value> {
    let probe_url = remote_view_operator_access_probe_url(route_binding)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
        .ok()?;
    let started_at = Instant::now();
    let response = client.get(&probe_url).send().await;
    let elapsed_ms = started_at.elapsed().as_millis() as u64;
    Some(match response {
        Ok(response) => {
            let status = response.status().as_u16();
            let state = if response.status().is_success() || response.status().is_redirection() {
                "ready"
            } else if matches!(status, 401 | 403) {
                "auth_expired"
            } else if matches!(status, 500 | 502 | 503 | 504) {
                "proxy_failed"
            } else {
                "public_operator_unavailable"
            };
            json!(
                { "state" : state, "url" : probe_url, "httpStatus" : status,
                "elapsedMs" : elapsed_ms, "reason" : if state == "ready" {
                "public operator URL responded" } else {
                "public operator URL did not return a usable response" }, }
            )
        }
        Err(error) => {
            let state = if error.is_timeout() {
                "timed_out"
            } else if error.is_connect() {
                "proxy_failed"
            } else {
                "public_operator_unavailable"
            };
            json!(
                { "state" : state, "url" : probe_url, "httpStatus" : null,
                "elapsedMs" : elapsed_ms, "reason" : error.to_string(), }
            )
        }
    })
}
pub(crate) fn route_binding_with_operator_access(
    mut route_binding: RemoteViewRouteBinding,
    operator_access: Option<Value>,
) -> RemoteViewRouteBinding {
    let Some(operator_access) = operator_access else {
        return route_binding;
    };
    let mut readiness = route_binding
        .readiness
        .take()
        .unwrap_or_else(|| route_binding_readiness(&route_binding));
    if !readiness.is_object() {
        readiness = json!(
            { "state" : readiness_state(& readiness).unwrap_or_else(|| "ready"
            .to_string()), "previous" : readiness, }
        );
    }
    if let Some(record) = readiness.as_object_mut() {
        record.insert("operatorAccess".to_string(), operator_access);
    }
    route_binding.readiness = Some(readiness);
    route_binding
}
pub(crate) fn remote_view_operator_access_probe_url(
    route_binding: &RemoteViewRouteBinding,
) -> Option<String> {
    for key in [
        "dashboardEmbedUrl",
        "publicOperatorUrl",
        "externalUrl",
        "healthUrl",
    ] {
        if let Some(url) = route_descriptor_string(route_binding, key)
            .and_then(|value| remote_view_http_probe_url(&value))
        {
            return Some(url);
        }
    }
    None
}
pub(crate) fn route_descriptor_string(
    route_binding: &RemoteViewRouteBinding,
    key: &str,
) -> Option<String> {
    route_binding
        .route_descriptor
        .as_ref()
        .and_then(|descriptor| descriptor.get(key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}
pub(crate) fn remote_view_http_probe_url(value: &str) -> Option<String> {
    let mut url = reqwest::Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    url.set_fragment(None);
    Some(url.to_string())
}
