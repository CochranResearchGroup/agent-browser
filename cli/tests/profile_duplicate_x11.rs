#![cfg(target_os = "linux")]
//! Stale native windows must produce a bounded result, never terminate an owner.
#[allow(dead_code)]
#[path = "../src/native/x11_scene.rs"]
mod scene;
use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
struct Children(Vec<Child>);
impl Drop for Children {
    fn drop(&mut self) {
        for child in self.0.iter_mut().rev() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
#[test]
#[ignore = "requires isolated X11 fixture"]
fn vanished_window_returns_error_without_terminating_owner() {
    if std::env::var_os("AB403_PRIVATE_X11").is_none() {
        let token = uuid::Uuid::new_v4().to_string();
        let status = Command::new("unshare")
            .args([
                "--user",
                "--map-current-user",
                "--keep-caps",
                "--mount",
                "--net",
                "--propagation",
                "private",
            ])
            .arg(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "vanished_window_returns_error_without_terminating_owner",
                "--ignored",
                "--nocapture",
            ])
            .env("AB403_PRIVATE_X11", "1")
            .env("AB403_X11_TEST_TOKEN", &token)
            .status()
            .unwrap();
        // The old fatal Xlib handler bypasses child destructors. Cleanup stays
        // bounded to this one random fixture token, even on the expected RED.
        for entry in std::fs::read_dir("/proc").unwrap().flatten() {
            let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|value| value.parse::<i32>().ok())
            else {
                continue;
            };
            let Ok(environment) = std::fs::read(entry.path().join("environ")) else {
                continue;
            };
            if environment
                .split(|byte| *byte == 0)
                .any(|field| field == format!("AB403_X11_TEST_TOKEN={token}").as_bytes())
            {
                unsafe {
                    libc::kill(pid, libc::SIGTERM);
                }
            }
        }
        assert!(
            status.success(),
            "stale-window subprocess terminated instead of returning a typed result"
        );
        return;
    }
    let root = std::env::temp_dir().join(format!("ab403-stale-x11-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join("sockets")).unwrap();
    assert!(Command::new("mount")
        .args(["--bind"])
        .arg(root.join("sockets"))
        .arg("/tmp/.X11-unix")
        .status()
        .unwrap()
        .success());
    let mut children = Children(vec![]);
    let mut server = Command::new("Xvfb")
        .args([
            "-displayfd",
            "1",
            "-screen",
            "0",
            "800x600x24",
            "-nolisten",
            "tcp",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut display = String::new();
    BufReader::new(server.stdout.take().unwrap())
        .read_line(&mut display)
        .unwrap();
    let display = format!(":{}", display.trim());
    children.0.push(server);
    let command = |program: &str| {
        let mut command = Command::new(program);
        command.env("DISPLAY", &display).env("HOME", &root);
        command
    };
    children.0.push(
        command("openbox")
            .args(["--sm-disable"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let out = command("xprop")
            .args(["-root", "_NET_SUPPORTING_WM_CHECK"])
            .output()
            .unwrap();
        if String::from_utf8_lossy(&out.stdout).contains("window id #") {
            break;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(20));
    }
    let title = format!("ab403-window-{}", uuid::Uuid::new_v4());
    let app = command("xmessage")
        .args(["-title", &title, "-buttons", "OK:0", "synthetic"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let pid = app.id();
    children.0.push(app);
    let until = Instant::now() + Duration::from_secs(5);
    let window = loop {
        let out = command("xdotool")
            .args(["search", "--name", &title])
            .output()
            .unwrap();
        let id = String::from_utf8_lossy(&out.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .to_owned();
        if !id.is_empty() {
            break id;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(command("xprop")
        .args([
            "-id",
            &window,
            "-f",
            "_NET_WM_PID",
            "32c",
            "-set",
            "_NET_WM_PID",
            &pid.to_string()
        ])
        .status()
        .unwrap()
        .success());
    assert!(command("xdotool")
        .args(["windowactivate", "--sync", &window])
        .status()
        .unwrap()
        .success());
    unsafe {libc::kill(children.0[1].id() as i32,libc::SIGSTOP);}
    for property in ["_NET_CLIENT_LIST_STACKING","_NET_ACTIVE_WINDOW"] {
        assert!(command("xprop").args(["-root","-f",property,"32x","-set",property,&window]).status().unwrap().success());
    }
    let until=Instant::now()+Duration::from_secs(5);
    let snapshot=loop {match scene::snapshot_browser_scene(pid,&display){Ok(snapshot)=>break snapshot,Err(error)=>{assert!(Instant::now()<until,"scene did not become ready: {error}");std::thread::sleep(Duration::from_millis(20));}}};
    let mut app = children.0.pop().unwrap();
    app.kill().unwrap();
    app.wait().unwrap();
    assert!(
        scene::stage_browser_scene(&snapshot).is_err(),
        "a vanished target was reported staged"
    );
    drop(children);
    std::fs::remove_dir_all(root).unwrap();
}
