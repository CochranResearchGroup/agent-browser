#![cfg(target_os = "linux")]

#[path = "support/argv_probe.rs"]
mod argv_probe;

use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        self.0.stdin.take();
        let _ = self.0.wait();
    }
}

#[test]
fn argv_probe_accepts_only_exact_zombie_or_positive_absence_after_observation_race() {
    let mut child = OwnedChild(
        Command::new("/bin/sh")
            .args(["-c", "read line"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let pid = child.0.id();
    let path = format!("/proc/{pid}/stat");
    let stat = std::fs::read_to_string(&path).unwrap();
    let ticks = stat
        .rsplit_once(')')
        .unwrap()
        .1
        .split_whitespace()
        .nth(19)
        .unwrap();
    let boot = std::fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap();
    let expected = format!("linux:{}:{ticks}", boot.trim());
    assert!(!argv_probe::original_has_ended(pid, &expected, Err(())));
    child.0.stdin.take();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let stat = std::fs::read_to_string(&path).unwrap();
        if stat.rsplit_once(')').unwrap().1.split_whitespace().next() == Some("Z") {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "owned child must exit before probing"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        argv_probe::original_has_ended(pid, &expected, Err(())),
        "argv probe must recognize its exact zombie after identity observation became unavailable"
    );
    assert!(!argv_probe::original_has_ended(
        pid,
        "linux:other:0",
        Err(())
    ));
    assert!(!argv_probe::original_has_ended(
        pid,
        &expected,
        Ok(Some("changed"))
    ));
    child.0.wait().unwrap();
    assert!(argv_probe::original_has_ended(pid, &expected, Ok(None)));
}
