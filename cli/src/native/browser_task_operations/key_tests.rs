use super::*;
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

const ROOT_ENV: &str = "AGENT_BROWSER_KEY_PUBLICATION_TEST_ROOT";
const ROLE_ENV: &str = "AGENT_BROWSER_KEY_PUBLICATION_TEST_ROLE";
const HISTORY_ENV: &str = "AGENT_BROWSER_KEY_PUBLICATION_TEST_HISTORY";

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("key-publication-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join(".agent-browser/service")).unwrap();
        Self(root)
    }

    fn child(&self, role: &str) -> OwnedChild {
        self.child_with_history(role, false)
    }

    fn child_with_history(&self, role: &str, has_history: bool) -> OwnedChild {
        OwnedChild(Some(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "native::browser_task_operations::key_tests::key_publication_child",
                    "--ignored",
                    "--nocapture",
                ])
                .env("HOME", &self.0)
                .env("AGENT_BROWSER_HOME", self.0.join(".agent-browser"))
                .env("AGENT_BROWSER_TEST_ALLOW_LIVE_HOME", "1")
                .env(ROOT_ENV, &self.0)
                .env(ROLE_ENV, role)
                .env(HISTORY_ENV, if has_history { "1" } else { "0" })
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        ))
    }

    fn wait(&self, name: &str) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !self.0.join(name).exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            self.0.join(name).exists(),
            "child rendezvous timed out: {name}"
        );
    }

    fn result(&self, role: &str) -> String {
        std::fs::read_to_string(self.0.join(format!("{role}.result"))).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct OwnedChild(Option<Child>);

impl OwnedChild {
    fn finish(mut self) -> Output {
        self.0.take().unwrap().wait_with_output().unwrap()
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub(super) fn after_create(path: &Path) -> Result<(), String> {
    if std::env::var(ROLE_ENV).as_deref() != Ok("creator") {
        return Ok(());
    }
    let root = PathBuf::from(std::env::var_os(ROOT_ENV).unwrap());
    if path != root.join(".agent-browser/service/task-operation-mac.key") {
        return Err("key_publication_test_scope_mismatch".into());
    }
    std::fs::write(root.join("created"), [])
        .map_err(|_| "key_publication_test_rendezvous_failed")?;
    let deadline = Instant::now() + Duration::from_secs(10);
    while !root.join("release").exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    if !root.join("release").exists() {
        return Err("key_publication_test_writer_timeout".into());
    }
    Ok(())
}

#[test]
#[ignore = "subprocess helper invoked by key publication regressions"]
fn key_publication_child() {
    let root = PathBuf::from(std::env::var_os(ROOT_ENV).unwrap());
    let role = std::env::var(ROLE_ENV).unwrap();
    std::fs::write(root.join(format!("{role}.started")), []).unwrap();
    let result = match key(std::env::var(HISTORY_ENV).as_deref() == Ok("1")) {
        Ok(key) => format!("ok:{}", hex::encode(Sha256::digest(key))),
        Err(error) => format!("error:{error}"),
    };
    std::fs::write(root.join(format!("{role}.result")), result.as_bytes()).unwrap();
    assert!(result.starts_with("ok:"), "{result}");
}

#[test]
fn key_publication_serializes_independent_first_writers() {
    let fixture = Fixture::new();
    let creator = fixture.child("creator");
    fixture.wait("created");
    let metadata = std::fs::metadata(
        fixture
            .0
            .join(".agent-browser/service/task-operation-mac.key"),
    )
    .unwrap();
    assert_eq!(metadata.len(), 0, "writer is paused before writing its key");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    }
    let reader = fixture.child("reader");
    fixture.wait("reader.started");
    let deadline = Instant::now() + Duration::from_millis(300);
    while !fixture.0.join("reader.result").exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    let returned_before_publication = fixture.0.join("reader.result").exists();
    std::fs::write(fixture.0.join("release"), []).unwrap();
    let creator_output = creator.finish();
    let reader_output = reader.finish();
    assert!(creator_output.status.success(), "creator failed");
    assert!(
        reader_output.status.success(),
        "parallel key reader failed: {}{}",
        String::from_utf8_lossy(&reader_output.stdout),
        String::from_utf8_lossy(&reader_output.stderr)
    );
    assert!(
        !returned_before_publication,
        "reader must wait for the initializing process to publish a complete key"
    );
    assert!(
        fixture.result("creator") == fixture.result("reader"),
        "independent first writers must retain the same key"
    );
}

#[test]
fn key_publication_preserves_existing_and_damaged_keys() {
    let fixture = Fixture::new();
    assert!(fixture.child("initial").finish().status.success());
    let original = fixture.result("initial");
    let path = fixture
        .0
        .join(".agent-browser/service/task-operation-mac.key");
    let bytes = std::fs::read(&path).unwrap();
    assert!(fixture
        .child_with_history("existing", true)
        .finish()
        .status
        .success());
    assert!(
        fixture.result("existing") == original,
        "existing key must not change"
    );
    assert!(std::fs::read(&path).unwrap() == bytes);

    std::fs::write(&path, [0u8; 7]).unwrap();
    assert!(!fixture.child("damaged").finish().status.success());
    assert_eq!(
        fixture.result("damaged"),
        "error:browser_task_operation_key_invalid"
    );
    assert_eq!(std::fs::metadata(&path).unwrap().len(), 7);

    std::fs::remove_file(&path).unwrap();
    assert!(!fixture
        .child_with_history("missing", true)
        .finish()
        .status
        .success());
    assert!(fixture
        .result("missing")
        .contains("browser_task_operation_key_unavailable"));
    assert!(
        !path.exists(),
        "original history must not regenerate a missing key"
    );
}

#[test]
fn key_publication_refuses_a_crashed_initialization() {
    let fixture = Fixture::new();
    let creator = fixture.child("creator");
    fixture.wait("created");
    drop(creator);
    assert!(!fixture.child("after-crash").finish().status.success());
    assert_eq!(
        fixture.result("after-crash"),
        "error:browser_task_operation_key_invalid"
    );
    assert_eq!(
        std::fs::metadata(
            fixture
                .0
                .join(".agent-browser/service/task-operation-mac.key")
        )
        .unwrap()
        .len(),
        0,
        "a crashed partial key must remain refused rather than replaced"
    );
}

#[cfg(unix)]
#[test]
fn key_publication_refuses_substituted_or_public_files() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let fixture = Fixture::new();
    let path = fixture
        .0
        .join(".agent-browser/service/task-operation-mac.key");
    let target = fixture.0.join("other-private-file");
    std::fs::write(&target, [9u8; 32]).unwrap();
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600)).unwrap();
    symlink(&target, &path).unwrap();
    assert!(!fixture.child("symlink").finish().status.success());
    assert!(fixture
        .result("symlink")
        .contains("browser_task_operation_key_unavailable"));
    assert!(std::fs::symlink_metadata(&path)
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(std::fs::read(&target).unwrap() == [9u8; 32]);
    std::fs::remove_file(&path).unwrap();

    std::fs::write(&path, [7u8; 32]).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(!fixture.child("public").finish().status.success());
    assert_eq!(
        fixture.result("public"),
        "error:browser_task_operation_key_permissions_invalid"
    );
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o644
    );

    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let lock = path.with_extension("key.lock");
    std::fs::remove_file(&lock).unwrap();
    symlink(&target, &lock).unwrap();
    assert!(!fixture.child("lock-symlink").finish().status.success());
    assert_eq!(
        fixture.result("lock-symlink"),
        "error:browser_task_operation_key_publication_failed"
    );
    assert!(std::fs::read(&path).unwrap() == [7u8; 32]);
    assert!(std::fs::read(&target).unwrap() == [9u8; 32]);
}
