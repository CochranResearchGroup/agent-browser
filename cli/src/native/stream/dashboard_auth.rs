use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::Sha256;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

mod operator_focus;
pub(crate) use operator_focus::{
    issue_operator_focus, issue_operator_tab_close, verify_operator_focus,
    verify_operator_focus_scope, verify_operator_tab_close, OperatorFocusScope,
};

pub(super) fn require_current_operator_role(username: &str) -> Result<(), String> {
    if load_auth_store()?
        .users
        .iter()
        .any(|user| user.username == username && user.role == DASHBOARD_ROLE_SUPERUSER)
    {
        Ok(())
    } else {
        Err("operator_controller_authority_required: current superuser role required".into())
    }
}

const AUTH_FILE_ENV: &str = "AGENT_BROWSER_DASHBOARD_AUTH_FILE";
const AUTH_DIR_ENV: &str = "AGENT_BROWSER_DASHBOARD_AUTH_DIR";
const AUTH_FILE_NAME: &str = "dashboard-auth.json";
const BOOTSTRAP_CREDENTIAL_FILE_NAME: &str = "dashboard-auth.env";
const PBKDF2_ITERATIONS: u32 = 120_000;
const SESSION_COOKIE: &str = "agent_browser_dashboard_session";
/// Dashboard sessions persist for thirty days and are renewed by the
/// authenticated status check that runs whenever the dashboard loads.
const SESSION_TTL_SECONDS: u64 = 30 * 24 * 60 * 60;
pub(super) const DASHBOARD_ROLE_SUPERUSER: &str = "superuser";
pub(super) const DASHBOARD_ROLE_OBSERVER: &str = "observer";

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DashboardAuthStore {
    version: u8,
    created_at: String,
    session_secret: String,
    users: Vec<DashboardAuthUser>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DashboardAuthUser {
    username: String,
    display_name: String,
    role: String,
    password_hash: String,
    created_at: String,
    bootstrap: bool,
}

#[derive(Debug, Clone)]
pub(super) struct DashboardAuthIdentity {
    ingress: bool,
    pub username: String,
    pub display_name: String,
    pub role: String,
}

#[derive(Debug)]
pub(super) struct DashboardAuthPaths {
    pub auth_file: PathBuf,
    pub bootstrap_credential_file: PathBuf,
}

#[derive(Debug)]
pub(super) struct DashboardAuthResponse {
    status: String,
    content_type: &'static str,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl DashboardAuthResponse {
    pub(super) fn is_success(&self) -> bool {
        self.status.starts_with('2')
    }

    pub(super) fn into_http_bytes(self) -> Vec<u8> {
        let mut response = format!(
            "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n",
            self.status,
            self.content_type,
            self.body.len()
        );
        for (name, value) in self.headers {
            response.push_str(&name);
            response.push_str(": ");
            response.push_str(&value);
            response.push_str("\r\n");
        }
        response.push_str("\r\n");
        let mut bytes = response.into_bytes();
        bytes.extend_from_slice(&self.body);
        bytes
    }
}

/// Ensure the dashboard has a private user-scoped auth store before it serves
/// operator APIs or exposes a forward-auth endpoint for component routes.
pub(super) fn ensure_dashboard_auth_config() -> Result<DashboardAuthPaths, String> {
    let auth_dir = dashboard_auth_dir()?;
    ensure_private_dir(&auth_dir)?;
    let auth_file = dashboard_auth_file(&auth_dir);
    let bootstrap_credential_file = auth_dir.join(BOOTSTRAP_CREDENTIAL_FILE_NAME);
    let now = now_rfc3339();

    let mut generated_credentials: BTreeMap<String, String> = BTreeMap::new();
    let mut store = if auth_file.exists() {
        load_auth_store_from_path(&auth_file)?
    } else {
        let admin_password = generate_secret(24)?;
        let codex_password = generate_secret(24)?;
        generated_credentials.insert("admin".to_string(), admin_password.clone());
        generated_credentials.insert("codex".to_string(), codex_password.clone());
        DashboardAuthStore {
            version: 1,
            created_at: now.clone(),
            session_secret: generate_secret(32)?,
            users: vec![
                build_user(
                    "admin",
                    "Default superuser",
                    DASHBOARD_ROLE_SUPERUSER,
                    &admin_password,
                    &now,
                )?,
                build_user(
                    "codex",
                    "Codex observer",
                    DASHBOARD_ROLE_OBSERVER,
                    &codex_password,
                    &now,
                )?,
            ],
        }
    };

    if !store.users.iter().any(|user| user.username == "admin") {
        let password = generate_secret(24)?;
        generated_credentials.insert("admin".to_string(), password.clone());
        store.users.push(build_user(
            "admin",
            "Default superuser",
            DASHBOARD_ROLE_SUPERUSER,
            &password,
            &now,
        )?);
    }

    if !store.users.iter().any(|user| user.username == "codex") {
        let password = generate_secret(24)?;
        generated_credentials.insert("codex".to_string(), password.clone());
        store.users.push(build_user(
            "codex",
            "Codex observer",
            DASHBOARD_ROLE_OBSERVER,
            &password,
            &now,
        )?);
    }

    let migrated_roles = normalize_bootstrap_roles(&mut store);
    if migrated_roles && auth_file.exists() {
        backup_auth_store(&auth_file)?;
    }
    write_auth_store(&auth_file, &store)?;
    upsert_default_env_file(&auth_dir, &auth_file)?;
    if !generated_credentials.is_empty() || !bootstrap_credential_file.exists() {
        write_bootstrap_credential_file(&bootstrap_credential_file, &generated_credentials)?;
    }

    Ok(DashboardAuthPaths {
        auth_file,
        bootstrap_credential_file,
    })
}

pub(super) fn auth_status_response(
    headers: &[(String, String)],
    secure_cookie: bool,
) -> DashboardAuthResponse {
    let paths = match ensure_dashboard_auth_config() {
        Ok(paths) => paths,
        Err(err) => {
            return json_response(
                "500 Internal Server Error",
                json!({"success": false, "error": err}),
            )
        }
    };
    let identity = match authenticate_headers(headers) {
        Ok(identity) => identity,
        Err(err) => return authentication_error_response(&err),
    };
    let mut payload = json!({
        "authenticated": identity.is_some(),
        "user": identity.as_ref().map(identity_json),
        "cookieSecure": secure_cookie,
        "sessionPersistence": {
            "maxAgeSeconds": identity.as_ref().filter(|identity| !identity.ingress).map(|_| SESSION_TTL_SECONDS),
            "rollingRefresh": identity.as_ref().is_some_and(|identity| !identity.ingress),
        },
    });
    if identity.is_some() {
        payload["credentialStore"] = json!(paths.auth_file);
        payload["bootstrapCredentialFile"] = json!(paths.bootstrap_credential_file);
    }
    let mut response = json_response("200 OK", payload);
    if let Some(identity) = identity.as_ref().filter(|identity| !identity.ingress) {
        let store = match load_auth_store() {
            Ok(store) => store,
            Err(err) => {
                return json_response(
                    "500 Internal Server Error",
                    json!({
                        "success": false,
                        "authenticated": false,
                        "errorCode": "dashboard_auth_refresh_failed",
                        "error": err,
                    }),
                )
            }
        };
        let cookie = match create_session_cookie(&store, identity, secure_cookie) {
            Ok(cookie) => cookie,
            Err(err) => {
                return json_response(
                    "500 Internal Server Error",
                    json!({
                        "success": false,
                        "authenticated": false,
                        "errorCode": "dashboard_auth_refresh_failed",
                        "error": err,
                    }),
                )
            }
        };
        response.headers.push(("Set-Cookie".to_string(), cookie));
    }
    response
}

pub(super) fn login_response(
    headers: &[(String, String)],
    body: &str,
    secure_cookie: bool,
) -> DashboardAuthResponse {
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(_) => {
            return json_response(
                "400 Bad Request",
                json!({"success": false, "authenticated": false, "error": "Invalid login payload"}),
            )
        }
    };
    let username = parsed
        .get("username")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .trim();
    let password = parsed
        .get("password")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");

    let store = match load_auth_store() {
        Ok(store) => store,
        Err(err) => {
            return json_response(
                "500 Internal Server Error",
                json!({"success": false, "error": err}),
            )
        }
    };
    let Some(user) = store.users.iter().find(|user| user.username == username) else {
        return json_response(
            "401 Unauthorized",
            json!({"success": false, "authenticated": false, "error": "Invalid username or password"}),
        );
    };
    if !verify_password(password, &user.password_hash) {
        return json_response(
            "401 Unauthorized",
            json!({"success": false, "authenticated": false, "error": "Invalid username or password"}),
        );
    }

    let identity = DashboardAuthIdentity {
        ingress: false,
        username: user.username.clone(),
        display_name: user.display_name.clone(),
        role: user.role.clone(),
    };
    let cookie = match create_session_cookie(&store, &identity, secure_cookie) {
        Ok(cookie) => cookie,
        Err(err) => {
            return json_response(
                "500 Internal Server Error",
                json!({"success": false, "error": err}),
            )
        }
    };
    let mut response = json_response(
        "200 OK",
        json!({"success": true, "authenticated": true, "user": identity_json(&identity)}),
    );
    response
        .headers
        .push(("Set-Cookie".to_string(), cookie.to_string()));

    if let Some(next) = login_next_path(headers) {
        response
            .headers
            .push(("X-Agent-Browser-Next".to_string(), next));
    }
    response
}

pub(super) fn logout_response(secure_cookie: bool) -> DashboardAuthResponse {
    let mut response = json_response("200 OK", json!({"success": true, "authenticated": false}));
    response.headers.push((
        "Set-Cookie".to_string(),
        expired_session_cookie(secure_cookie),
    ));
    response
}

pub(super) fn verify_forward_auth_response(
    headers: &[(String, String)],
    _secure_cookie: bool,
) -> DashboardAuthResponse {
    match authenticate_headers(headers) {
        Ok(Some(identity)) => authenticated_forward_auth_response(
            headers,
            identity,
            std::env::var("AGENT_BROWSER_GUACAMOLE_HEADER_USER")
                .ok()
                .as_deref(),
        )
        .unwrap_or_else(|err| {
            json_response(
                "500 Internal Server Error",
                json!({"success": false, "error": err}),
            )
        }),
        Ok(None) => {
            let next = forwarded_request_target(headers).unwrap_or_else(|| "/".to_string());
            let location = forwarded_login_location(headers, &next);
            DashboardAuthResponse {
                status: if header_value(headers, "x-forwarded-uri").is_some() {
                    "302 Found".to_string()
                } else {
                    "401 Unauthorized".to_string()
                },
                content_type: "application/json; charset=utf-8",
                headers: vec![("Location".to_string(), location)],
                body: json!({
                    "success": false,
                    "authenticated": false,
                    "error": "Login required",
                })
                .to_string()
                .into_bytes(),
            }
        }
        Err(err) => authentication_error_response(&err),
    }
}

/// Preserve the authenticated dashboard actor while selecting the stable
/// provider principal required by Guacamole header authentication. Dashboard
/// usernames are not Guacamole authorization identities and may legitimately
/// have no provider route grants.
fn authenticated_forward_auth_response(
    headers: &[(String, String)],
    identity: DashboardAuthIdentity,
    guacamole_header_user: Option<&str>,
) -> Result<DashboardAuthResponse, String> {
    let forwarded_path = header_value(headers, "x-forwarded-uri").unwrap_or("/");
    let guacamole_request = forwarded_path == "/guacamole"
        || forwarded_path.starts_with("/guacamole/")
        || forwarded_path.starts_with("/guacamole?");
    let remote_user = if guacamole_request {
        guacamole_header_user
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(identity.username.as_str())
    } else {
        identity.username.as_str()
    };
    if !remote_user
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "._@-".contains(character))
    {
        return Err("Guacamole header user contains unsupported characters".to_string());
    }
    Ok(DashboardAuthResponse {
        status: "204 No Content".to_string(),
        content_type: "text/plain; charset=utf-8",
        headers: vec![
            ("Remote-User".to_string(), remote_user.to_string()),
            ("Remote-Name".to_string(), identity.display_name),
            ("Remote-Groups".to_string(), identity.role),
            ("X-Agent-Browser-User".to_string(), identity.username),
        ],
        body: Vec::new(),
    })
}

pub(super) fn unauthorized_api_response(secure_cookie: bool) -> DashboardAuthResponse {
    let mut response = json_response(
        "401 Unauthorized",
        json!({"success": false, "authenticated": false, "error": "Login required"}),
    );
    response.headers.push((
        "Set-Cookie".to_string(),
        expired_session_cookie(secure_cookie),
    ));
    response
}

pub(super) fn forbidden_api_response(message: &str) -> DashboardAuthResponse {
    json_response(
        "403 Forbidden",
        json!({"success": false, "authenticated": true, "authorized": false, "error": message}),
    )
}

pub(super) fn require_superuser(
    headers: &[(String, String)],
    secure_cookie: bool,
) -> Result<DashboardAuthIdentity, DashboardAuthResponse> {
    let identity = require_authenticated(headers, secure_cookie)?;
    if identity.role == DASHBOARD_ROLE_SUPERUSER {
        Ok(identity)
    } else {
        Err(forbidden_api_response("Superuser role required"))
    }
}

pub(super) fn require_authenticated(
    headers: &[(String, String)],
    secure_cookie: bool,
) -> Result<DashboardAuthIdentity, DashboardAuthResponse> {
    match authenticate_headers(headers) {
        Ok(Some(identity)) => Ok(identity),
        Ok(None) => Err(unauthorized_api_response(secure_cookie)),
        Err(err) => Err(authentication_error_response(&err)),
    }
}

pub(super) fn authenticate_headers(
    headers: &[(String, String)],
) -> Result<Option<DashboardAuthIdentity>, String> {
    if let Some(token) =
        header_value(headers, "cookie").and_then(|value| cookie_value(value, SESSION_COOKIE))
    {
        if let Some(identity) = verify_session_token(&load_auth_store()?, &token)? {
            return Ok(Some(identity));
        }
    }
    authenticate_ingress(headers)
}

/// Preserve authorization failures separately from owner/configuration failures.
/// Never return owner bodies, cookie values, or verifier URLs to clients.
pub(super) fn authentication_error_response(error: &str) -> DashboardAuthResponse {
    if error == "workspace_ingress_access_denied" {
        return json_response(
            "403 Forbidden",
            json!({"success": false, "authenticated": false, "authorized": false, "errorCode": "workspace_access_denied", "error": "Workspace access denied. Check ingress permissions and the explicit workspace role mapping."}),
        );
    }
    json_response(
        "503 Service Unavailable",
        json!({"success": false, "authenticated": false, "errorCode": "workspace_auth_unavailable", "error": "Workspace authentication service unavailable. Check the reviewed ingress verifier configuration."}),
    )
}

/// Verify the existing ingress owner's session directly. Caller identity and
/// forwarded headers never establish authority. Explicit local role mappings
/// preserve dashboard access semantics; ingress sessions never mint local cookies.
fn authenticate_ingress(
    headers: &[(String, String)],
) -> Result<Option<DashboardAuthIdentity>, String> {
    const VERIFY: &str = "AGENT_BROWSER_DASHBOARD_INGRESS_VERIFY_URL";
    const WORKSPACE: &str = "AGENT_BROWSER_DASHBOARD_INGRESS_WORKSPACE_URL";
    const USERS: &str = "AGENT_BROWSER_DASHBOARD_INGRESS_USERS";
    let values = [VERIFY, WORKSPACE, USERS].map(|key| std::env::var(key).ok());
    if values.iter().all(Option::is_none) {
        return Ok(None);
    }
    let invalid = || "workspace_ingress_configuration_invalid".to_string();
    let [Some(endpoint), Some(workspace), Some(users)] = values else {
        return Err(invalid());
    };
    let endpoint = url::Url::parse(&endpoint).map_err(|_| invalid())?;
    let workspace = url::Url::parse(&workspace).map_err(|_| invalid())?;
    let loopback = endpoint
        .host_str()
        .and_then(|host| host.parse::<std::net::IpAddr>().ok())
        .is_some_and(|address| address.is_loopback());
    if !(endpoint.scheme() == "https" || (endpoint.scheme() == "http" && loopback))
        || endpoint.host_str().is_none()
        || !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.path() != "/api/authz/forward-auth"
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
        || workspace.scheme() != "https"
        || workspace.host_str().is_none()
        || !workspace.username().is_empty()
        || workspace.password().is_some()
        || workspace.fragment().is_some()
    {
        return Err(invalid());
    }
    let users: BTreeMap<String, String> = serde_json::from_str(&users).map_err(|_| invalid())?;
    let Some(cookie) = header_value(headers, "cookie") else {
        return Ok(None);
    };
    // Bound concurrent owner requests without caching identities or allowing
    // unbounded thread creation during an owner outage.
    static IN_FLIGHT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    use std::sync::atomic::Ordering;
    struct Permit;
    impl Drop for Permit {
        fn drop(&mut self) {
            IN_FLIGHT.fetch_sub(1, Ordering::SeqCst);
        }
    }
    IN_FLIGHT
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
            (count < 8).then_some(count + 1)
        })
        .map_err(|_| "workspace_ingress_busy".to_string())?;
    let _permit = Permit;
    let cookie = cookie.to_string();
    let (status, principal) = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| "workspace_ingress_unavailable".to_string())?
            .block_on(async move {
                let client = reqwest::Client::builder()
                    .no_proxy()
                    .redirect(reqwest::redirect::Policy::none())
                    .connect_timeout(std::time::Duration::from_secs(1))
                    .read_timeout(std::time::Duration::from_secs(2))
                    .timeout(std::time::Duration::from_secs(3))
                    .build()
                    .map_err(|_| "workspace_ingress_unavailable".to_string())?;
                let response = client
                    .get(endpoint)
                    .header("Cookie", cookie)
                    .header("X-Forwarded-Method", "GET")
                    .header("X-Forwarded-Proto", workspace.scheme())
                    .header(
                        "X-Forwarded-Host",
                        &workspace[url::Position::BeforeHost..url::Position::AfterPort],
                    )
                    .header(
                        "X-Forwarded-URI",
                        &workspace[url::Position::BeforePath..url::Position::AfterQuery],
                    )
                    .send()
                    .await
                    .map_err(|_| "workspace_ingress_unavailable".to_string())?;
                let principals = response
                    .headers()
                    .get_all("Remote-User")
                    .iter()
                    .collect::<Vec<_>>();
                let principal = if principals.len() == 1 {
                    principals[0].to_str().ok().map(str::to_string)
                } else {
                    None
                };
                Ok::<_, String>((response.status().as_u16(), principal))
            })
    })
    .join()
    .map_err(|_| "workspace_ingress_unavailable".to_string())??;
    if matches!(status, 401 | 302 | 303) {
        return Ok(None);
    }
    if status == 403 {
        return Err("workspace_ingress_access_denied".into());
    }
    if status != 200 {
        return Err("workspace_ingress_unavailable".into());
    }
    let principal = principal
        .filter(|value| !value.is_empty() && value.trim() == value)
        .ok_or_else(|| "workspace_ingress_invalid_response".to_string())?;
    let username = users
        .get(&principal)
        .ok_or("workspace_ingress_access_denied")?;
    let store = load_auth_store()?;
    let user = store
        .users
        .iter()
        .find(|user| {
            &user.username == username
                && matches!(
                    user.role.as_str(),
                    DASHBOARD_ROLE_SUPERUSER | DASHBOARD_ROLE_OBSERVER
                )
        })
        .ok_or("workspace_ingress_access_denied")?;
    Ok(Some(DashboardAuthIdentity {
        ingress: true,
        username: user.username.clone(),
        display_name: user.display_name.clone(),
        role: user.role.clone(),
    }))
}

pub(super) fn request_is_secure(headers: &[(String, String)]) -> bool {
    header_value(headers, "x-forwarded-proto")
        .map(|value| value.eq_ignore_ascii_case("https"))
        .unwrap_or(false)
}

pub(super) fn parse_headers(header_str: &str) -> Vec<(String, String)> {
    header_str
        .lines()
        .skip(1)
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_ascii_lowercase(), value.trim().to_string()))
        })
        .collect()
}

fn dashboard_auth_dir() -> Result<PathBuf, String> {
    dashboard_auth_dir_from(
        std::env::var_os(AUTH_DIR_ENV).map(PathBuf::from),
        dirs::home_dir(),
    )
}

fn dashboard_auth_dir_from(
    isolated_dir: Option<PathBuf>,
    home_dir: Option<PathBuf>,
) -> Result<PathBuf, String> {
    isolated_dir
        .or_else(|| home_dir.map(|home| home.join(".agent-browser")))
        .ok_or_else(|| "Cannot resolve the user-scoped agent-browser directory".to_string())
}

fn dashboard_auth_file(auth_dir: &Path) -> PathBuf {
    std::env::var(AUTH_FILE_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|_| auth_dir.join(AUTH_FILE_NAME))
}

fn load_auth_store() -> Result<DashboardAuthStore, String> {
    let auth_dir = dashboard_auth_dir()?;
    let auth_file = dashboard_auth_file(&auth_dir);
    if !auth_file.exists() {
        let _ = ensure_dashboard_auth_config()?;
    }
    load_auth_store_from_path(&auth_file)
}

fn load_auth_store_from_path(path: &Path) -> Result<DashboardAuthStore, String> {
    let contents = fs::read_to_string(path).map_err(|err| {
        format!(
            "Failed to read dashboard auth store {}: {}",
            path.display(),
            err
        )
    })?;
    serde_json::from_str(&contents).map_err(|err| {
        format!(
            "Failed to parse dashboard auth store {}: {}",
            path.display(),
            err
        )
    })
}

fn write_auth_store(path: &Path, store: &DashboardAuthStore) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        ensure_private_dir(parent)?;
    }
    let contents = serde_json::to_string_pretty(store)
        .map_err(|err| format!("Failed to serialize dashboard auth store: {}", err))?;
    fs::write(path, format!("{}\n", contents)).map_err(|err| {
        format!(
            "Failed to write dashboard auth store {}: {}",
            path.display(),
            err
        )
    })?;
    set_private_file(path)?;
    Ok(())
}

fn build_user(
    username: &str,
    display_name: &str,
    role: &str,
    password: &str,
    created_at: &str,
) -> Result<DashboardAuthUser, String> {
    Ok(DashboardAuthUser {
        username: username.to_string(),
        display_name: display_name.to_string(),
        role: role.to_string(),
        password_hash: hash_password(password)?,
        created_at: created_at.to_string(),
        bootstrap: true,
    })
}

fn normalize_bootstrap_roles(store: &mut DashboardAuthStore) -> bool {
    let mut changed = false;
    for user in &mut store.users {
        if user.username == "admin" && user.bootstrap && user.role != DASHBOARD_ROLE_SUPERUSER {
            user.role = DASHBOARD_ROLE_SUPERUSER.to_string();
            changed = true;
        }
        if user.username == "codex"
            && user.bootstrap
            && user.display_name == "Codex observer"
            && user.role == DASHBOARD_ROLE_SUPERUSER
        {
            user.role = DASHBOARD_ROLE_OBSERVER.to_string();
            changed = true;
        }
    }
    changed
}

fn backup_auth_store(path: &Path) -> Result<(), String> {
    let timestamp = now_epoch_seconds();
    let backup = path.with_extension(format!("json.pre-role-migration-{timestamp}"));
    fs::copy(path, &backup).map_err(|err| {
        format!(
            "Failed to back up dashboard auth store {} to {}: {}",
            path.display(),
            backup.display(),
            err
        )
    })?;
    set_private_file(&backup)?;
    Ok(())
}

fn upsert_default_env_file(auth_dir: &Path, auth_file: &Path) -> Result<(), String> {
    let env_file = auth_dir.join(".env");
    let mut lines = if env_file.exists() {
        fs::read_to_string(&env_file)
            .map_err(|err| format!("Failed to read {}: {}", env_file.display(), err))?
            .lines()
            .map(str::to_string)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    upsert_env_line(
        &mut lines,
        AUTH_FILE_ENV,
        &auth_file.to_string_lossy().replace('\\', "\\\\"),
    );
    fs::write(&env_file, format!("{}\n", lines.join("\n")))
        .map_err(|err| format!("Failed to write {}: {}", env_file.display(), err))?;
    set_private_file(&env_file)?;
    Ok(())
}

fn upsert_env_line(lines: &mut Vec<String>, key: &str, value: &str) {
    let next = format!("{key}=\"{}\"", value.replace('"', "\\\""));
    for line in lines.iter_mut() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            continue;
        }
        if trimmed
            .split_once('=')
            .map(|(candidate, _)| candidate.trim() == key)
            .unwrap_or(false)
        {
            *line = next;
            return;
        }
    }
    lines.push(next);
}

fn write_bootstrap_credential_file(
    path: &Path,
    generated_credentials: &BTreeMap<String, String>,
) -> Result<(), String> {
    let mut existing = if path.exists() {
        fs::read_to_string(path).unwrap_or_default()
    } else {
        String::new()
    };
    if existing.trim().is_empty() {
        existing.push_str("# Agent Browser dashboard bootstrap superuser credentials.\n");
        existing.push_str("# Keep this file mode 0600 and rotate passwords after sharing them.\n");
    }
    for (username, password) in generated_credentials {
        let upper = username.to_ascii_uppercase();
        upsert_env_text(
            &mut existing,
            &format!("AGENT_BROWSER_DASHBOARD_{}_USERNAME", upper),
            username,
        );
        upsert_env_text(
            &mut existing,
            &format!("AGENT_BROWSER_DASHBOARD_{}_PASSWORD", upper),
            password,
        );
    }
    fs::write(path, existing)
        .map_err(|err| format!("Failed to write {}: {}", path.display(), err))?;
    set_private_file(path)?;
    Ok(())
}

fn upsert_env_text(contents: &mut String, key: &str, value: &str) {
    let mut found = false;
    let next_line = format!("{key}=\"{}\"", value.replace('"', "\\\""));
    let mut lines = contents.lines().map(str::to_string).collect::<Vec<_>>();
    for line in &mut lines {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            continue;
        }
        if trimmed
            .split_once('=')
            .map(|(candidate, _)| candidate.trim() == key)
            .unwrap_or(false)
        {
            *line = next_line.clone();
            found = true;
        }
    }
    if !found {
        lines.push(next_line);
    }
    *contents = format!("{}\n", lines.join("\n"));
}

fn hash_password(password: &str) -> Result<String, String> {
    let mut salt = [0u8; 16];
    getrandom::getrandom(&mut salt)
        .map_err(|err| format!("Failed to generate dashboard password salt: {}", err))?;
    let hash = pbkdf2_hmac_sha256(password.as_bytes(), &salt, PBKDF2_ITERATIONS);
    Ok(format!(
        "pbkdf2-sha256${}${}${}",
        PBKDF2_ITERATIONS,
        URL_SAFE_NO_PAD.encode(salt),
        URL_SAFE_NO_PAD.encode(hash)
    ))
}

fn verify_password(password: &str, encoded: &str) -> bool {
    let parts = encoded.split('$').collect::<Vec<_>>();
    if parts.len() != 4 || parts[0] != "pbkdf2-sha256" {
        return false;
    }
    let Ok(iterations) = parts[1].parse::<u32>() else {
        return false;
    };
    let Ok(salt) = URL_SAFE_NO_PAD.decode(parts[2]) else {
        return false;
    };
    let Ok(expected) = URL_SAFE_NO_PAD.decode(parts[3]) else {
        return false;
    };
    let actual = pbkdf2_hmac_sha256(password.as_bytes(), &salt, iterations);
    constant_time_eq(&actual, &expected)
}

fn pbkdf2_hmac_sha256(password: &[u8], salt: &[u8], iterations: u32) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(password).expect("HMAC accepts keys of any size");
    mac.update(salt);
    mac.update(&1u32.to_be_bytes());
    let mut u = mac.finalize().into_bytes();
    let mut output = [0u8; 32];
    output.copy_from_slice(&u);

    for _ in 1..iterations {
        let mut mac = HmacSha256::new_from_slice(password).expect("HMAC accepts keys of any size");
        mac.update(&u);
        u = mac.finalize().into_bytes();
        for (out, byte) in output.iter_mut().zip(u.iter()) {
            *out ^= *byte;
        }
    }

    output
}

fn create_session_cookie(
    store: &DashboardAuthStore,
    identity: &DashboardAuthIdentity,
    secure_cookie: bool,
) -> Result<String, String> {
    let token = create_session_token(store, identity)?;
    Ok(format!(
        "{SESSION_COOKIE}={token}; HttpOnly; SameSite=Lax; Path=/; Max-Age={SESSION_TTL_SECONDS}{}",
        if secure_cookie { "; Secure" } else { "" }
    ))
}

fn expired_session_cookie(secure_cookie: bool) -> String {
    format!(
        "{SESSION_COOKIE}=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0{}",
        if secure_cookie { "; Secure" } else { "" }
    )
}

fn create_session_token(
    store: &DashboardAuthStore,
    identity: &DashboardAuthIdentity,
) -> Result<String, String> {
    let expires = now_epoch_seconds() + SESSION_TTL_SECONDS;
    let nonce = generate_secret(16)?;
    let payload = format!(
        "v1|{}|{}|{}|{}",
        identity.username, identity.role, expires, nonce
    );
    let signature = sign_payload(store, payload.as_bytes())?;
    Ok(format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(payload.as_bytes()),
        URL_SAFE_NO_PAD.encode(signature)
    ))
}

fn verify_session_token(
    store: &DashboardAuthStore,
    token: &str,
) -> Result<Option<DashboardAuthIdentity>, String> {
    let Some((payload_b64, signature_b64)) = token.split_once('.') else {
        return Ok(None);
    };
    let payload = match URL_SAFE_NO_PAD.decode(payload_b64) {
        Ok(payload) => payload,
        Err(_) => return Ok(None),
    };
    let signature = match URL_SAFE_NO_PAD.decode(signature_b64) {
        Ok(signature) => signature,
        Err(_) => return Ok(None),
    };
    let expected = sign_payload(store, &payload)?;
    if !constant_time_eq(&expected, &signature) {
        return Ok(None);
    }
    let payload_str = match String::from_utf8(payload) {
        Ok(payload) => payload,
        Err(_) => return Ok(None),
    };
    let parts = payload_str.split('|').collect::<Vec<_>>();
    if parts.len() != 5 || parts[0] != "v1" {
        return Ok(None);
    }
    let username = parts[1];
    let role = parts[2];
    let Ok(expires) = parts[3].parse::<u64>() else {
        return Ok(None);
    };
    if expires <= now_epoch_seconds() {
        return Ok(None);
    }
    let Some(user) = store
        .users
        .iter()
        .find(|user| user.username == username && user.role == role)
    else {
        return Ok(None);
    };
    Ok(Some(DashboardAuthIdentity {
        ingress: false,
        username: user.username.clone(),
        display_name: user.display_name.clone(),
        role: user.role.clone(),
    }))
}

fn sign_payload(store: &DashboardAuthStore, payload: &[u8]) -> Result<Vec<u8>, String> {
    let secret = URL_SAFE_NO_PAD
        .decode(&store.session_secret)
        .map_err(|err| format!("Invalid dashboard session secret: {}", err))?;
    let mut mac = HmacSha256::new_from_slice(&secret)
        .map_err(|err| format!("Invalid dashboard session secret: {}", err))?;
    mac.update(payload);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn cookie_value(cookie_header: &str, name: &str) -> Option<String> {
    cookie_header.split(';').find_map(|part| {
        let (candidate, value) = part.trim().split_once('=')?;
        (candidate == name).then(|| value.to_string())
    })
}

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(candidate, _)| candidate.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

fn identity_json(identity: &DashboardAuthIdentity) -> serde_json::Value {
    json!({
        "username": identity.username.clone(),
        "displayName": identity.display_name.clone(),
        "role": identity.role.clone(),
        "authenticationSource": if identity.ingress { "ingress" } else { "local" },
    })
}

fn json_response(status: &str, value: serde_json::Value) -> DashboardAuthResponse {
    DashboardAuthResponse {
        status: status.to_string(),
        content_type: "application/json; charset=utf-8",
        headers: Vec::new(),
        body: value.to_string().into_bytes(),
    }
}

fn forwarded_request_target(headers: &[(String, String)]) -> Option<String> {
    let uri = header_value(headers, "x-forwarded-uri")?;
    Some(uri.to_string())
}

fn forwarded_login_location(headers: &[(String, String)], next: &str) -> String {
    let path = format!("/login?next={}", urlencoding::encode(next));
    let Some(host) =
        header_value(headers, "x-forwarded-host").or_else(|| header_value(headers, "host"))
    else {
        return path;
    };
    let proto = header_value(headers, "x-forwarded-proto").unwrap_or("http");
    format!("{proto}://{host}{path}")
}

fn login_next_path(headers: &[(String, String)]) -> Option<String> {
    header_value(headers, "referer")
        .and_then(|referer| url::Url::parse(referer).ok())
        .and_then(|url| {
            url.query_pairs()
                .find_map(|(key, value)| (key == "next").then(|| value.to_string()))
        })
}

fn generate_secret(bytes_len: usize) -> Result<String, String> {
    let mut bytes = vec![0u8; bytes_len];
    getrandom::getrandom(&mut bytes)
        .map_err(|err| format!("Failed to generate dashboard secret: {}", err))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in left.iter().zip(right.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn now_rfc3339() -> String {
    let now = time::OffsetDateTime::now_utc();
    now.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn ensure_private_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path)
        .map_err(|err| format!("Failed to create {}: {}", path.display(), err))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|err| format!("Failed to secure {}: {}", path.display(), err))?;
    }
    Ok(())
}

fn set_private_file(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|err| format!("Failed to secure {}: {}", path.display(), err))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingress_owner_verification_preserves_roles_and_ignores_spoofed_headers() {
        use std::io::{Read, Write};
        let scratch = std::env::temp_dir().join(format!("p524-auth-{}", std::process::id()));
        fs::create_dir_all(&scratch).unwrap();
        let original = [
            AUTH_FILE_ENV,
            AUTH_DIR_ENV,
            "AGENT_BROWSER_DASHBOARD_INGRESS_VERIFY_URL",
            "AGENT_BROWSER_DASHBOARD_INGRESS_WORKSPACE_URL",
            "AGENT_BROWSER_DASHBOARD_INGRESS_USERS",
        ]
        .map(|name| (name, std::env::var_os(name)));
        struct Restore(Vec<(&'static str, Option<std::ffi::OsString>)>, PathBuf);
        impl Drop for Restore {
            fn drop(&mut self) {
                for (name, value) in &self.0 {
                    match value {
                        Some(value) => std::env::set_var(name, value),
                        None => std::env::remove_var(name),
                    }
                }
                let _ = fs::remove_dir_all(&self.1);
            }
        }
        let _restore = Restore(original.to_vec(), scratch.clone());
        std::env::set_var(AUTH_DIR_ENV, &scratch);
        std::env::remove_var(AUTH_FILE_ENV);
        ensure_dashboard_auth_config().unwrap();
        let owner = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        std::env::set_var(
            "AGENT_BROWSER_DASHBOARD_INGRESS_VERIFY_URL",
            format!(
                "http://{}/api/authz/forward-auth",
                owner.local_addr().unwrap()
            ),
        );
        std::env::set_var(
            "AGENT_BROWSER_DASHBOARD_INGRESS_WORKSPACE_URL",
            "https://workspace.example.test/",
        );
        std::env::set_var(
            "AGENT_BROWSER_DASHBOARD_INGRESS_USERS",
            r#"{"operator":"admin","reader":"codex"}"#,
        );
        let server = std::thread::spawn(move || {
            for (status, user) in [
                ("200 OK", "operator"),
                ("200 OK", "reader"),
                ("200 OK", "reader"),
                ("403 Forbidden", "operator"),
                ("200 OK", "unknown"),
                ("200 OK", ""),
                ("302 Found", "operator"),
                ("401 Unauthorized", ""),
                ("200 OK", "operator"),
                ("200 OK", "operator"),
            ] {
                let (mut stream, _) = owner.accept().unwrap();
                let mut bytes = [0; 8192];
                let n = stream.read(&mut bytes).unwrap();
                let request = String::from_utf8_lossy(&bytes[..n]).to_ascii_lowercase();
                assert!(request.contains("cookie: cooper_authelia_session=synthetic"));
                assert!(request.contains("x-forwarded-proto: https"));
                assert!(request.contains("x-forwarded-host: workspace.example.test"));
                assert!(request.contains("x-forwarded-uri: /"));
                assert!(!request.contains("spoofed"));
                let principal = if user.is_empty() {
                    String::new()
                } else {
                    format!("Remote-User: {user}\r\n")
                };
                let location = if status.starts_with("302") {
                    "Location: https://spoofed.example/sign-in\r\n"
                } else {
                    ""
                };
                write!(stream, "HTTP/1.1 {status}\r\n{principal}{location}Content-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
            }
        });
        fn exchange(path: &str, method: &str, cookie: &str, body: &str) -> String {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            listener.set_nonblocking(true).unwrap();
            let handler = std::thread::spawn(move || {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(async {
                        let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                        let (stream, _) = listener.accept().await.unwrap();
                        super::super::dashboard::handle_dashboard_connection(stream).await;
                    });
            });
            let mut client = std::net::TcpStream::connect(address).unwrap();
            client
                .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .unwrap();
            let request = format!("{method} {path} HTTP/1.1\r\nHost: spoofed.example\r\nRemote-User: spoofed\r\nRemote-Groups: superuser\r\nCookie: {cookie}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            client.write_all(request.as_bytes()).unwrap();
            let mut result = String::new();
            client.read_to_string(&mut result).unwrap();
            handler.join().unwrap();
            result
        }
        let cookie = "cooper_authelia_session=synthetic";
        let status = || exchange("/api/dashboard-auth/status", "GET", cookie, "");
        let response = status();
        assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
        assert!(response.contains(r#""username":"admin""#));
        assert!(response.contains(r#""authenticationSource":"ingress""#));
        assert!(
            !response.contains("Set-Cookie"),
            "ingress identity must not become a local session"
        );
        let denied = exchange("/api/app-intelligence/operator/turn", "POST", cookie, "{}");
        assert!(denied.starts_with("HTTP/1.1 403 Forbidden"), "{denied}");
        assert!(status().starts_with("HTTP/1.1 403 Forbidden"));
        assert!(status().starts_with("HTTP/1.1 403 Forbidden"));
        assert!(status().starts_with("HTTP/1.1 503 Service Unavailable"));
        let redirect = status();
        assert!(
            redirect.starts_with("HTTP/1.1 200 OK")
                && redirect.contains(r#""authenticated":false"#)
        );
        assert!(!redirect.contains("Location:"));
        let anonymous = status();
        assert!(anonymous.starts_with("HTTP/1.1 200 OK"));
        assert!(anonymous.contains(r#""authenticated":false"#));
        // A mapped operator reaches request validation rather than the role gate.
        let operator = exchange("/api/app-intelligence/operator/turn", "POST", cookie, "{}");
        assert!(
            operator.starts_with("HTTP/1.1 400 Bad Request")
                && operator.contains("Missing operator prompt"),
            "{operator}"
        );
        server.join().unwrap();
        assert!(status().starts_with("HTTP/1.1 503 Service Unavailable"));
        // Existing signed local sessions remain valid even while ingress is unavailable.
        let credentials = fs::read_to_string(scratch.join(BOOTSTRAP_CREDENTIAL_FILE_NAME)).unwrap();
        let password = credentials
            .lines()
            .find_map(|line| {
                line.strip_prefix("AGENT_BROWSER_DASHBOARD_ADMIN_PASSWORD=\"")
                    .and_then(|value| value.strip_suffix('"'))
            })
            .unwrap();
        let login = exchange(
            "/api/dashboard-auth/login",
            "POST",
            "",
            &json!({"username":"admin", "password": password}).to_string(),
        );
        assert!(login.starts_with("HTTP/1.1 200 OK"));
        let local_cookie = login
            .lines()
            .find_map(|line| {
                line.strip_prefix("Set-Cookie: ")
                    .and_then(|value| value.split(';').next())
            })
            .unwrap();
        let local = exchange("/api/dashboard-auth/status", "GET", local_cookie, "");
        assert!(local.starts_with("HTTP/1.1 200 OK") && local.contains("Set-Cookie"));
        assert!(local.contains(r#""authenticationSource":"local""#));
        assert!(
            authenticate_headers(&[("remote-user".into(), "admin".into())])
                .unwrap()
                .is_none()
        );
        std::env::remove_var("AGENT_BROWSER_DASHBOARD_INGRESS_WORKSPACE_URL");
        assert!(status().starts_with("HTTP/1.1 503 Service Unavailable"));
        std::env::set_var(
            "AGENT_BROWSER_DASHBOARD_INGRESS_WORKSPACE_URL",
            "https://workspace.example.test/",
        );
        std::env::set_var(
            "AGENT_BROWSER_DASHBOARD_INGRESS_VERIFY_URL",
            "http://unreviewed.example/api/authz/forward-auth",
        );
        assert!(status().starts_with("HTTP/1.1 503 Service Unavailable"));
    }

    #[test]
    fn isolated_dashboard_auth_directory_does_not_resolve_through_user_home() {
        let isolated = PathBuf::from("/tmp/agent-browser-dashboard-auth-fixture");
        let home = PathBuf::from("/home/operator");

        assert_eq!(
            dashboard_auth_dir_from(Some(isolated.clone()), Some(home)).unwrap(),
            isolated
        );
    }

    #[test]
    fn password_hash_round_trips() {
        let hash = hash_password("correct horse battery staple").unwrap();

        assert!(verify_password("correct horse battery staple", &hash));
        assert!(!verify_password("wrong", &hash));
    }

    #[test]
    fn session_token_requires_valid_signature_and_user() {
        let password = "secret";
        let user = build_user(
            "admin",
            "Default superuser",
            DASHBOARD_ROLE_SUPERUSER,
            password,
            "2026-01-01T00:00:00Z",
        )
        .unwrap();
        let store = DashboardAuthStore {
            version: 1,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            session_secret: URL_SAFE_NO_PAD.encode([7u8; 32]),
            users: vec![user],
        };
        let identity = DashboardAuthIdentity {
            ingress: false,
            username: "admin".to_string(),
            display_name: "Default superuser".to_string(),
            role: "superuser".to_string(),
        };
        let token = create_session_token(&store, &identity).unwrap();

        assert!(verify_session_token(&store, &token).unwrap().is_some());
        assert!(verify_session_token(&store, &format!("{token}x"))
            .unwrap()
            .is_none());
    }

    #[test]
    fn session_cookie_is_persistent_for_thirty_days() {
        let user = build_user(
            "admin",
            "Default superuser",
            DASHBOARD_ROLE_SUPERUSER,
            "secret",
            "2026-01-01T00:00:00Z",
        )
        .unwrap();
        let store = DashboardAuthStore {
            version: 1,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            session_secret: URL_SAFE_NO_PAD.encode([7u8; 32]),
            users: vec![user],
        };
        let identity = DashboardAuthIdentity {
            ingress: false,
            username: "admin".to_string(),
            display_name: "Default superuser".to_string(),
            role: DASHBOARD_ROLE_SUPERUSER.to_string(),
        };

        let cookie = create_session_cookie(&store, &identity, true).unwrap();

        assert!(cookie.contains("Max-Age=2592000"));
        assert!(cookie.contains("Path=/"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.ends_with("; Secure"));
    }

    #[test]
    fn cookie_parser_finds_named_cookie() {
        assert_eq!(
            cookie_value(
                "foo=1; agent_browser_dashboard_session=abc.def; theme=dark",
                SESSION_COOKIE
            ),
            Some("abc.def".to_string())
        );
    }

    #[test]
    fn generated_users_have_distinct_roles() {
        let admin = build_user(
            "admin",
            "Default superuser",
            DASHBOARD_ROLE_SUPERUSER,
            "admin-secret",
            "2026-01-01T00:00:00Z",
        )
        .unwrap();
        let codex = build_user(
            "codex",
            "Codex observer",
            DASHBOARD_ROLE_OBSERVER,
            "codex-secret",
            "2026-01-01T00:00:00Z",
        )
        .unwrap();

        assert_eq!(admin.role, DASHBOARD_ROLE_SUPERUSER);
        assert_eq!(codex.role, DASHBOARD_ROLE_OBSERVER);
    }

    #[test]
    fn normalize_bootstrap_roles_downgrades_generated_codex_observer() {
        let mut store = DashboardAuthStore {
            version: 1,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            session_secret: URL_SAFE_NO_PAD.encode([8u8; 32]),
            users: vec![DashboardAuthUser {
                username: "codex".to_string(),
                display_name: "Codex observer".to_string(),
                role: DASHBOARD_ROLE_SUPERUSER.to_string(),
                password_hash: "hash".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                bootstrap: true,
            }],
        };

        assert!(normalize_bootstrap_roles(&mut store));
        assert_eq!(store.users[0].role, DASHBOARD_ROLE_OBSERVER);
    }

    #[test]
    fn normalize_bootstrap_roles_preserves_non_bootstrap_codex_superuser() {
        let mut store = DashboardAuthStore {
            version: 1,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            session_secret: URL_SAFE_NO_PAD.encode([8u8; 32]),
            users: vec![DashboardAuthUser {
                username: "codex".to_string(),
                display_name: "Codex operator".to_string(),
                role: DASHBOARD_ROLE_SUPERUSER.to_string(),
                password_hash: "hash".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                bootstrap: false,
            }],
        };

        assert!(!normalize_bootstrap_roles(&mut store));
        assert_eq!(store.users[0].role, DASHBOARD_ROLE_SUPERUSER);
    }

    #[test]
    fn forwarded_verify_redirects_to_login_when_missing_cookie() {
        let headers = vec![("x-forwarded-uri".to_string(), "/guacamole/".to_string())];
        let response = verify_forward_auth_response(&headers, false);

        assert_eq!(response.status, "302 Found");
        assert!(response
            .headers
            .iter()
            .any(|(name, value)| name == "Location" && value == "/login?next=%2Fguacamole%2F"));
        assert!(!response
            .headers
            .iter()
            .any(|(name, _)| name == "Set-Cookie"));
    }

    #[test]
    fn forwarded_verify_redirect_uses_forwarded_host() {
        let headers = vec![
            ("x-forwarded-uri".to_string(), "/guacamole/".to_string()),
            (
                "x-forwarded-host".to_string(),
                "agent-browser.localhost".to_string(),
            ),
            ("x-forwarded-proto".to_string(), "http".to_string()),
        ];
        let response = verify_forward_auth_response(&headers, false);

        assert!(response.headers.iter().any(|(name, value)| {
            name == "Location"
                && value == "http://agent-browser.localhost/login?next=%2Fguacamole%2F"
        }));
    }

    #[test]
    fn guacamole_forward_auth_uses_provider_principal_and_preserves_dashboard_actor() {
        let identity = DashboardAuthIdentity {
            ingress: false,
            username: "codex".to_string(),
            display_name: "Codex observer".to_string(),
            role: DASHBOARD_ROLE_OBSERVER.to_string(),
        };
        let headers = vec![(
            "x-forwarded-uri".to_string(),
            "/guacamole/#/client/opaque".to_string(),
        )];

        let response =
            authenticated_forward_auth_response(&headers, identity, Some("provider-operator"))
                .unwrap();

        assert!(response
            .headers
            .iter()
            .any(|(name, value)| { name == "Remote-User" && value == "provider-operator" }));
        assert!(response
            .headers
            .iter()
            .any(|(name, value)| name == "X-Agent-Browser-User" && value == "codex"));
    }

    #[test]
    fn non_guacamole_forward_auth_keeps_dashboard_actor_as_remote_user() {
        let identity = DashboardAuthIdentity {
            ingress: false,
            username: "codex".to_string(),
            display_name: "Codex observer".to_string(),
            role: DASHBOARD_ROLE_OBSERVER.to_string(),
        };
        let headers = vec![(
            "x-forwarded-uri".to_string(),
            "/api/runtime/health".to_string(),
        )];

        let response =
            authenticated_forward_auth_response(&headers, identity, Some("provider-operator"))
                .unwrap();

        assert!(response
            .headers
            .iter()
            .any(|(name, value)| name == "Remote-User" && value == "codex"));
    }

    #[test]
    fn guacamole_forward_auth_rejects_invalid_provider_principal() {
        let identity = DashboardAuthIdentity {
            ingress: false,
            username: "codex".to_string(),
            display_name: "Codex observer".to_string(),
            role: DASHBOARD_ROLE_OBSERVER.to_string(),
        };
        let headers = vec![("x-forwarded-uri".to_string(), "/guacamole/".to_string())];

        let error = authenticated_forward_auth_response(
            &headers,
            identity,
            Some("provider operator\nforged"),
        )
        .unwrap_err();

        assert_eq!(
            error,
            "Guacamole header user contains unsupported characters"
        );
    }

    #[test]
    fn require_superuser_accepts_admin_and_rejects_observer() {
        let admin = build_user(
            "admin",
            "Default superuser",
            DASHBOARD_ROLE_SUPERUSER,
            "admin-secret",
            "2026-01-01T00:00:00Z",
        )
        .unwrap();
        let codex = build_user(
            "codex",
            "Codex observer",
            DASHBOARD_ROLE_OBSERVER,
            "codex-secret",
            "2026-01-01T00:00:00Z",
        )
        .unwrap();
        let store = DashboardAuthStore {
            version: 1,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            session_secret: URL_SAFE_NO_PAD.encode([9u8; 32]),
            users: vec![admin, codex],
        };
        let admin_identity = DashboardAuthIdentity {
            ingress: false,
            username: "admin".to_string(),
            display_name: "Default superuser".to_string(),
            role: DASHBOARD_ROLE_SUPERUSER.to_string(),
        };
        let observer_identity = DashboardAuthIdentity {
            ingress: false,
            username: "codex".to_string(),
            display_name: "Codex observer".to_string(),
            role: DASHBOARD_ROLE_OBSERVER.to_string(),
        };
        let admin_token = create_session_token(&store, &admin_identity).unwrap();
        let observer_token = create_session_token(&store, &observer_identity).unwrap();

        let admin_verified = verify_session_token(&store, &admin_token).unwrap().unwrap();
        let observer_verified = verify_session_token(&store, &observer_token)
            .unwrap()
            .unwrap();

        assert_eq!(admin_verified.role, DASHBOARD_ROLE_SUPERUSER);
        assert_eq!(observer_verified.role, DASHBOARD_ROLE_OBSERVER);
        assert_eq!(
            forbidden_api_response("Superuser role required").status,
            "403 Forbidden"
        );
    }
}
