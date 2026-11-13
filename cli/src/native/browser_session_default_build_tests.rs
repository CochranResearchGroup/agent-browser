//! Managed-host regressions at the real startup and process-launch boundary.
use super::*;
use crate::test_utils::EnvGuard;
use std::fs;
use std::os::unix::fs::PermissionsExt;

struct Fixture {
    root: PathBuf,
    _environment: EnvGuard<'static>,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("managed-build-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let environment = EnvGuard::new(&[
            "HOME",
            "AGENT_BROWSER_CONFIG",
            "AGENT_BROWSER_EXECUTABLE_PATH",
            "AGENT_BROWSER_STEALTHCDP_CHROMIUM_MANIFEST_PATH",
            "AGENT_BROWSER_STEALTHCDP_CHROMIUM_INSTALL_ROOT",
            "AGENT_BROWSER_REMOTE_VIEW_ORIGIN",
            "AGENT_BROWSER_REMOTE_VIEW_POOL",
            "AGENT_BROWSER_REMOTE_VIEW_APPLICATION",
            "AGENT_BROWSER_REMOTE_VIEW_DESKTOP_COUNT",
            "AGENT_BROWSER_SESSION_DISPLAY",
            "AGENT_BROWSER_TEST_ALLOW_LIVE_HOME",
            "AGENT_BROWSER_DISPOSABLE_PROFILE_ROOT",
            "AGENT_BROWSER_HOSTS_FILE",
        ]);
        for name in [
            "AGENT_BROWSER_EXECUTABLE_PATH",
            "AGENT_BROWSER_REMOTE_VIEW_ORIGIN",
            "AGENT_BROWSER_REMOTE_VIEW_POOL",
            "AGENT_BROWSER_REMOTE_VIEW_APPLICATION",
            "AGENT_BROWSER_REMOTE_VIEW_DESKTOP_COUNT",
            "AGENT_BROWSER_SESSION_DISPLAY",
            "AGENT_BROWSER_STEALTHCDP_CHROMIUM_INSTALL_ROOT",
            "AGENT_BROWSER_DISPOSABLE_PROFILE_ROOT",
            "AGENT_BROWSER_HOSTS_FILE",
        ] {
            environment.remove(name);
        }
        environment.set("HOME", root.to_str().unwrap());
        // The explicitly isolated HOME above is the fixture's only state root.
        environment.set("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1");
        let artifact = root.join("artifact");
        fs::create_dir_all(&artifact).unwrap();
        Self::recorder(&artifact.join("chrome"), "STEALTHCDP_SELECTED");
        fs::write(
            artifact.join("smoke.json"),
            r#"{"success":true,"checks":{"navigatorWebdriver":"false"}}"#,
        )
        .unwrap();
        let manifest = artifact.join("manifest.json");
        fs::write(&manifest, r#"{"schema":"chromium-stealthcdp.artifact.v1","executable":{"relativePath":"chrome"},"smoke":{"relativePath":"smoke.json"}}"#).unwrap();
        environment.set(
            "AGENT_BROWSER_STEALTHCDP_CHROMIUM_MANIFEST_PATH",
            manifest.to_str().unwrap(),
        );
        let config = root.join("config.json");
        fs::write(
            &config,
            r#"{"service":{"defaultBrowserBuild":"stealthcdp_chromium"}}"#,
        )
        .unwrap();
        environment.set("AGENT_BROWSER_CONFIG", config.to_str().unwrap());
        let cache = root.join(".agent-browser/browsers/chrome-999.0.0.0");
        fs::create_dir_all(&cache).unwrap();
        Self::recorder(&cache.join("chrome"), "TESTING_CHROME_SELECTED");
        Self {
            root,
            _environment: environment,
        }
    }
    fn recorder(path: &Path, marker: &str) {
        fs::write(path, format!("#!/bin/sh\necho {marker} >&2\nexit 73\n")).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn managed_disposable_launch_honors_required_stealthcdp_manifest() {
    let _fixture = Fixture::new();
    let mut host = load_default_browser_session_host().unwrap();
    let error = host
        .open(OpenBrowserSession::disposable("probe", "default", 1_000))
        .unwrap_err();
    assert!(
        error.contains("STEALTHCDP_SELECTED"),
        "managed launch ignored configured build: {error}"
    );
    assert!(
        !error.contains("TESTING_CHROME_SELECTED"),
        "managed launch substituted testing Chrome: {error}"
    );
}

#[test]
fn managed_host_refuses_missing_required_stealthcdp_instead_of_testing_cache() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.root.join("artifact/manifest.json")).unwrap();
    let error = load_default_browser_session_host()
        .err()
        .expect("required StealthCDP must not permit generic Chrome discovery");
    assert!(
        error.contains("stealthcdp_chromium"),
        "missing build must identify its remedy: {error}"
    );
}
