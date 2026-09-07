use std::fs;
use std::path::Path;

/// Ensure `packages/dashboard/out/` exists so `rust-embed` does not fail during
/// Rust-only development builds where the dashboard has not been built. The
/// placeholder is written only when the directory is completely absent.
fn ensure_dashboard_dir() {
    let dashboard_out = Path::new("../packages/dashboard/out");
    println!("cargo:rerun-if-changed=../packages/dashboard/out");
    if !dashboard_out.join("index.html").exists() {
        let _ = fs::create_dir_all(dashboard_out);
        let _ = fs::write(
            dashboard_out.join("index.html"),
            "<!DOCTYPE html><html><body><p>Dashboard not built. Run: cd packages/dashboard &amp;&amp; pnpm build</p></body></html>\n",
        );
    }
}

fn main() {
    ensure_dashboard_dir();
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=../crates");
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .current_dir("..")
            .output()
            .ok()
            .filter(|out| out.status.success())
            .and_then(|out| String::from_utf8(out.stdout).ok())
            .map(|text| text.trim().to_string())
    };
    if let Some(revision) = git(&["rev-parse", "HEAD"])
        .filter(|value| value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        println!("cargo:rustc-env=AGENT_BROWSER_BUILD_SOURCE_REVISION={revision}");
    }
    let tree = git(&["status", "--porcelain"])
        .map(|status| if status.is_empty() { "clean" } else { "dirty" })
        .unwrap_or("unknown");
    println!("cargo:rustc-env=AGENT_BROWSER_BUILD_SOURCE_TREE_STATE={tree}");
    let mut paths = vec!["HEAD".to_string(), "packed-refs".to_string()];
    if let Some(reference) = git(&["symbolic-ref", "-q", "HEAD"]) {
        paths.push(reference);
    }
    for path in paths {
        if let Some(path) = git(&["rev-parse", "--path-format=absolute", "--git-path", &path]) {
            println!("cargo:rerun-if-changed={path}");
        }
    }
}
