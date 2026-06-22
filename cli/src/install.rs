use crate::color;
use crate::connection::{cleanup_stale_files, get_socket_dir};
use crate::flags::{launch_config_status, Flags};
use crate::native::stream::runtime_manifest_json;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{exit, Command, Stdio};
use std::time::Duration;

const LAST_KNOWN_GOOD_URL: &str =
    "https://googlechromelabs.github.io/chrome-for-testing/last-known-good-versions-with-downloads.json";
const STEALTHCDP_CHROMIUM_RELEASE_TAG: &str = "v150.0.7835.0-stealthcdp.6b6558b55a1d";
const STEALTHCDP_CHROMIUM_RELEASE_ASSET: &str =
    "chromium-stealthcdp_150.0.7835.0+stealthcdp.6b6558b55a1d_win64.zip";
const STEALTHCDP_CHROMIUM_RELEASE_ASSET_SHA256: &str =
    "1e3878f270b383acdc99d0e6b82c687f1ffc4f8d27b86cc4bba1dd334946fae3";
const STEALTHCDP_CHROMIUM_EXECUTABLE_SHA256: &str =
    "5d4cb9a996df941885cf29beefc53e154a46750eed37b3d83b25e6c423c70f2c";
const STEALTHCDP_CHROMIUM_VERSION: &str = "150.0.7835.0";
const STEALTHCDP_CHROMIUM_ARTIFACT_NAME: &str = "150.0.7835.0+stealthcdp.6b6558b55a1d";
const STEALTHCDP_CHROMIUM_SOURCE_SHA: &str = "24ecda02e97db6fa730a7ccf8747776a4d21e4b9";
const STEALTHCDP_CHROMIUM_UPSTREAM_REVISION: &str = "d421c3af8268e2e6227b7fe4461183e69b64bc61";
const STEALTHCDP_CHROMIUM_PATCHSET_SHA: &str = "fcf2d964f9070cc9acf7aabbfb2c576f36107bbe";
const STEALTHCDP_CHROMIUM_PATCH_QUEUE_SHA256: &str =
    "6b6558b55a1d3b0dc081871e2d76cd6dc74665d28d0e5789846b456388afd3cf";

pub fn get_browsers_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".agent-browser")
        .join("browsers")
}

pub fn find_installed_chrome() -> Option<PathBuf> {
    let browsers_dir = get_browsers_dir();
    let debug = std::env::var("AGENT_BROWSER_DEBUG").is_ok();

    if debug {
        let _ = writeln!(
            io::stderr(),
            "[chrome-search] home_dir={:?} browsers_dir={}",
            dirs::home_dir(),
            browsers_dir.display()
        );
    }

    if !browsers_dir.exists() {
        if debug {
            let _ = writeln!(io::stderr(), "[chrome-search] browsers_dir does not exist");
        }
        return None;
    }

    let entries = match fs::read_dir(&browsers_dir) {
        Ok(entries) => entries,
        Err(e) => {
            let _ = writeln!(
                io::stderr(),
                "Warning: cannot read Chrome cache directory {}: {}",
                browsers_dir.display(),
                e
            );
            return None;
        }
    };

    let mut versions: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let matches = e
                .file_name()
                .to_str()
                .is_some_and(|n| n.starts_with("chrome-"));
            if debug {
                let _ = writeln!(
                    io::stderr(),
                    "[chrome-search] entry {:?} matches={}",
                    e.file_name(),
                    matches
                );
            }
            matches
        })
        .collect();

    versions.sort_by_key(|b| std::cmp::Reverse(b.file_name()));

    for entry in versions {
        let dir = entry.path();
        if let Some(bin) = chrome_binary_in_dir(&dir) {
            let exists = bin.exists();
            if debug {
                let _ = writeln!(
                    io::stderr(),
                    "[chrome-search] candidate {} exists={}",
                    bin.display(),
                    exists
                );
            }
            if exists {
                return Some(bin);
            }
        } else if debug {
            let _ = writeln!(
                io::stderr(),
                "[chrome-search] no binary found in {}",
                dir.display()
            );
        }
    }

    if debug {
        let _ = writeln!(io::stderr(), "[chrome-search] no installed Chrome found");
    }
    None
}

fn chrome_binary_in_dir(dir: &Path) -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let app =
            dir.join("Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing");
        if app.exists() {
            return Some(app);
        }
        let inner = dir.join("chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing");
        if inner.exists() {
            return Some(inner);
        }
        let inner_x64 = dir.join(
            "chrome-mac-x64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
        );
        if inner_x64.exists() {
            return Some(inner_x64);
        }
        None
    }

    #[cfg(target_os = "linux")]
    {
        let bin = dir.join("chrome");
        if bin.exists() {
            return Some(bin);
        }
        let inner = dir.join("chrome-linux64/chrome");
        if inner.exists() {
            return Some(inner);
        }
        None
    }

    #[cfg(target_os = "windows")]
    {
        let bin = dir.join("chrome.exe");
        if bin.exists() {
            return Some(bin);
        }
        let inner = dir.join("chrome-win64/chrome.exe");
        if inner.exists() {
            return Some(inner);
        }
        None
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

fn platform_key() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "mac-arm64"
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        "mac-x64"
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "linux64"
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "win64"
    }
    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    {
        // Compiles on unsupported platforms (e.g. linux aarch64) so the binary
        // can still be used for other commands like `connect`. The install path
        // guards against this at runtime before calling platform_key().
        panic!("Unsupported platform for Chrome for Testing download")
    }
}

async fn fetch_download_url() -> Result<(String, String), String> {
    let client = http_client()?;
    let resp = client
        .get(LAST_KNOWN_GOOD_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch version info: {}", format_reqwest_error(&e)))?;

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse version info: {}", e))?;

    let channel = body
        .get("channels")
        .and_then(|c| c.get("Stable"))
        .ok_or("No Stable channel found in version info")?;

    let version = channel
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or("No version string found")?
        .to_string();

    let platform = platform_key();

    let url = channel
        .get("downloads")
        .and_then(|d| d.get("chrome"))
        .and_then(|c| c.as_array())
        .and_then(|arr| {
            arr.iter().find_map(|entry| {
                if entry.get("platform")?.as_str()? == platform {
                    Some(entry.get("url")?.as_str()?.to_string())
                } else {
                    None
                }
            })
        })
        .ok_or_else(|| format!("No download URL found for platform: {}", platform))?;

    Ok((version, url))
}

fn format_reqwest_error(e: &reqwest::Error) -> String {
    let mut msg = e.to_string();
    let mut source = std::error::Error::source(e);
    while let Some(cause) = source {
        msg.push_str(&format!(": {}", cause));
        source = std::error::Error::source(cause);
    }
    msg
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(format!("agent-browser/{}", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(120))
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", format_reqwest_error(&e)))
}

async fn download_bytes(url: &str) -> Result<Vec<u8>, String> {
    download_bytes_with_retry_backoff(url, true).await
}

async fn download_bytes_with_retry_backoff(
    url: &str,
    wait_between_retries: bool,
) -> Result<Vec<u8>, String> {
    let client = http_client()?;
    let max_retries = 3;
    let mut last_err = String::new();

    for attempt in 0..max_retries {
        if attempt > 0 {
            eprintln!(
                "  Retrying download (attempt {}/{})",
                attempt + 1,
                max_retries
            );
            if wait_between_retries {
                tokio::time::sleep(std::time::Duration::from_secs(1 << attempt)).await;
            }
        }

        let resp = match client.get(url).send().await {
            Ok(r) => r,
            Err(e) => {
                last_err = format!("Download failed: {}", format_reqwest_error(&e));
                if e.is_connect() || e.is_timeout() {
                    continue;
                }
                return Err(last_err);
            }
        };

        let status = resp.status();
        if !status.is_success() {
            last_err = format!(
                "Download failed: server returned HTTP {} for {}",
                status, url
            );
            if status.is_server_error() {
                continue;
            }
            return Err(last_err);
        }

        let total = resp.content_length();
        let mut bytes = Vec::new();
        let mut stream = resp;
        let mut downloaded: u64 = 0;
        let mut last_pct: u64 = 0;

        let mut chunk_err = None;
        loop {
            let chunk = stream
                .chunk()
                .await
                .map_err(|e| format!("Download error: {}", format_reqwest_error(&e)));
            match chunk {
                Ok(Some(data)) => {
                    downloaded += data.len() as u64;
                    bytes.extend_from_slice(&data);

                    if let Some(total) = total {
                        let pct = (downloaded * 100) / total;
                        if pct >= last_pct + 5 {
                            last_pct = pct;
                            let mb = downloaded as f64 / 1_048_576.0;
                            let total_mb = total as f64 / 1_048_576.0;
                            eprint!("\r  {:.0}/{:.0} MB ({pct}%)", mb, total_mb);
                            let _ = io::stderr().flush();
                        }
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    chunk_err = Some(e);
                    break;
                }
            }
        }

        eprintln!();

        if let Some(e) = chunk_err {
            last_err = e;
            continue;
        }

        return Ok(bytes);
    }

    Err(last_err)
}

fn extract_zip(bytes: Vec<u8>, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| format!("Failed to create directory: {}", e))?;

    let cursor = io::Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| format!("Failed to read zip archive: {}", e))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry: {}", e))?;

        let enclosed = match file.enclosed_name() {
            Some(name) => name.to_owned(),
            None => continue,
        };
        let raw_name = enclosed.to_string_lossy().to_string();
        // Strip the top-level "chrome-<platform>/" directory from zip entries.
        // On Windows, enclosed_name() normalizes paths to backslashes, so we
        // must split on either separator.
        let rel_path = raw_name
            .strip_prefix("chrome-")
            .and_then(|s| s.find(['/', '\\']).map(|i| &s[i + 1..]))
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or(raw_name.clone());

        if rel_path.is_empty() {
            continue;
        }

        let out_path = dest.join(&rel_path);

        // Defense-in-depth: ensure the resolved path is inside dest
        if !out_path.starts_with(dest) {
            continue;
        }

        if file.is_dir() {
            fs::create_dir_all(&out_path)
                .map_err(|e| format!("Failed to create dir {}: {}", out_path.display(), e))?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    format!("Failed to create parent dir {}: {}", parent.display(), e)
                })?;
            }
            let mut out_file = fs::File::create(&out_path)
                .map_err(|e| format!("Failed to create file {}: {}", out_path.display(), e))?;
            io::copy(&mut file, &mut out_file)
                .map_err(|e| format!("Failed to write {}: {}", out_path.display(), e))?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    let _ = fs::set_permissions(&out_path, fs::Permissions::from_mode(mode));
                }
            }
        }
    }

    Ok(())
}

fn sha256_bytes(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn stealthcdp_chromium_release_url() -> String {
    format!(
        "https://github.com/CochranResearchGroup/chromium-stealthcdp/releases/download/{STEALTHCDP_CHROMIUM_RELEASE_TAG}/{STEALTHCDP_CHROMIUM_RELEASE_ASSET}"
    )
}

fn stealthcdp_chromium_install_root() -> Result<PathBuf, String> {
    if let Ok(root) = std::env::var("AGENT_BROWSER_STEALTHCDP_CHROMIUM_INSTALL_ROOT") {
        let trimmed = root.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }

    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        let trimmed = local_app_data.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed).join("chromium-stealthcdp"));
        }
    }

    if let Some(local_app_data) = find_wsl_windows_local_app_data() {
        return Ok(local_app_data.join("chromium-stealthcdp"));
    }

    dirs::data_local_dir()
        .map(|path| path.join("chromium-stealthcdp"))
        .ok_or_else(|| {
            "could not determine an install root; set AGENT_BROWSER_STEALTHCDP_CHROMIUM_INSTALL_ROOT"
                .to_string()
        })
}

fn find_wsl_windows_local_app_data() -> Option<PathBuf> {
    let users_dir = Path::new("/mnt/c/Users");
    let entries = fs::read_dir(users_dir).ok()?;
    let mut candidates = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().to_string();
        if matches!(
            name.as_str(),
            "All Users" | "Default" | "Default User" | "Public"
        ) {
            continue;
        }
        let local_app_data = entry.path().join("AppData").join("Local");
        if local_app_data.is_dir() {
            candidates.push(local_app_data);
        }
    }
    candidates.sort();
    candidates.into_iter().next()
}

fn write_stealthcdp_manifest(artifact_dir: &Path, executable_relative: &str) -> Result<(), String> {
    let smoke_relative = if artifact_dir.join("smoke.json").exists() {
        "smoke.json"
    } else if artifact_dir.join("smoke-win.json").exists() {
        "smoke-win.json"
    } else {
        "smoke.json"
    };
    let smoke_path = artifact_dir.join(smoke_relative);
    if !smoke_path.exists() {
        fs::write(
            &smoke_path,
            serde_json::to_vec_pretty(&json!({
                "success": false,
                "reason": "live_smoke_not_run_after_install",
                "checks": {
                    "navigatorWebdriver": null
                }
            }))
            .map_err(|err| format!("Failed to serialize smoke placeholder: {err}"))?,
        )
        .map_err(|err| format!("Failed to write {}: {err}", smoke_path.display()))?;
    }

    let manifest_path = artifact_dir.join("manifest.json");
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&json!({
            "schema": "chromium-stealthcdp.artifact.v1",
            "artifactName": STEALTHCDP_CHROMIUM_ARTIFACT_NAME,
            "chromeVersion": format!("Chromium {STEALTHCDP_CHROMIUM_VERSION}"),
            "chromium": {
                "sourceSha": STEALTHCDP_CHROMIUM_SOURCE_SHA,
                "upstreamRevision": STEALTHCDP_CHROMIUM_UPSTREAM_REVISION
            },
            "patchset": {
                "repoSha": STEALTHCDP_CHROMIUM_PATCHSET_SHA,
                "patchQueueSha256": STEALTHCDP_CHROMIUM_PATCH_QUEUE_SHA256
            },
            "release": {
                "repo": "CochranResearchGroup/chromium-stealthcdp",
                "tag": STEALTHCDP_CHROMIUM_RELEASE_TAG,
                "asset": STEALTHCDP_CHROMIUM_RELEASE_ASSET,
                "assetSha256": STEALTHCDP_CHROMIUM_RELEASE_ASSET_SHA256
            },
            "executable": {
                "relativePath": executable_relative,
                "sha256": STEALTHCDP_CHROMIUM_EXECUTABLE_SHA256
            },
            "smoke": {
                "relativePath": smoke_relative
            }
        }))
        .map_err(|err| format!("Failed to serialize manifest: {err}"))?,
    )
    .map_err(|err| format!("Failed to write {}: {err}", manifest_path.display()))?;

    Ok(())
}

fn flatten_stealthcdp_artifact_root(extracted_dir: &Path) -> Result<(), String> {
    if extracted_dir.join("chrome.exe").is_file() {
        return Ok(());
    }

    let nested_dir = extracted_dir.join("chromium-stealthcdp");
    if !nested_dir.join("chrome.exe").is_file() {
        return Ok(());
    }

    let entries = fs::read_dir(&nested_dir)
        .map_err(|err| format!("Failed to read {}: {err}", nested_dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("Failed to read artifact entry: {err}"))?;
        let dest = extracted_dir.join(entry.file_name());
        fs::rename(entry.path(), &dest).map_err(|err| {
            format!(
                "Failed to move {} to {}: {err}",
                entry.path().display(),
                dest.display()
            )
        })?;
    }
    fs::remove_dir(&nested_dir)
        .map_err(|err| format!("Failed to remove {}: {err}", nested_dir.display()))?;

    Ok(())
}

fn create_current_pointer(install_root: &Path, artifact_dir: &Path) -> Result<PathBuf, String> {
    let current = install_root.join("current");
    if let Ok(metadata) = fs::symlink_metadata(&current) {
        if metadata.file_type().is_symlink() || metadata.is_file() {
            fs::remove_file(&current)
                .map_err(|err| format!("Failed to replace {}: {err}", current.display()))?;
        } else if metadata.is_dir() {
            fs::remove_dir_all(&current)
                .map_err(|err| format!("Failed to replace {}: {err}", current.display()))?;
        }
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(artifact_dir, &current).map_err(|err| {
            format!(
                "Failed to create current symlink {} -> {}: {err}",
                current.display(),
                artifact_dir.display()
            )
        })?;
    }

    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(artifact_dir, &current).map_err(|err| {
            format!(
                "Failed to create current symlink {} -> {}: {err}",
                current.display(),
                artifact_dir.display()
            )
        })?;
    }

    Ok(current)
}

fn install_stealthcdp_chromium_archive_bytes(
    bytes: Vec<u8>,
    install_root: &Path,
    force: bool,
    expected_archive_sha256: &str,
    expected_executable_sha256: &str,
) -> Result<PathBuf, String> {
    let archive_sha256 = sha256_bytes(&bytes);
    if archive_sha256 != expected_archive_sha256 {
        return Err(format!(
            "downloaded archive sha256 mismatch: expected {expected_archive_sha256}, got {archive_sha256}"
        ));
    }

    fs::create_dir_all(install_root).map_err(|err| {
        format!(
            "Failed to create install root {}: {err}",
            install_root.display()
        )
    })?;
    let artifact_dir = install_root.join(STEALTHCDP_CHROMIUM_ARTIFACT_NAME);
    if artifact_dir.exists() {
        if force {
            fs::remove_dir_all(&artifact_dir).map_err(|err| {
                format!(
                    "Failed to replace existing artifact {}: {err}",
                    artifact_dir.display()
                )
            })?;
        } else {
            let chrome_exe = artifact_dir.join("chrome.exe");
            if !chrome_exe.is_file() {
                return Err(format!(
                    "existing artifact {} does not contain chrome.exe; rerun with --force",
                    artifact_dir.display()
                ));
            }
            let executable_sha256 = file_sha256(&chrome_exe)?;
            if executable_sha256 != expected_executable_sha256 {
                return Err(format!(
                    "existing chrome.exe sha256 mismatch: expected {expected_executable_sha256}, got {executable_sha256}; rerun with --force"
                ));
            }
            write_stealthcdp_manifest(&artifact_dir, "chrome.exe")?;
            return create_current_pointer(install_root, &artifact_dir);
        }
    }

    let tmp_dir = install_root.join(format!(".tmp-{STEALTHCDP_CHROMIUM_ARTIFACT_NAME}"));
    if tmp_dir.exists() {
        fs::remove_dir_all(&tmp_dir).map_err(|err| {
            format!(
                "Failed to clear temporary install directory {}: {err}",
                tmp_dir.display()
            )
        })?;
    }

    extract_zip(bytes, &tmp_dir)?;
    flatten_stealthcdp_artifact_root(&tmp_dir)?;
    let chrome_exe = tmp_dir.join("chrome.exe");
    if !chrome_exe.is_file() {
        let _ = fs::remove_dir_all(&tmp_dir);
        return Err("release archive did not contain chrome.exe at the artifact root".to_string());
    }

    let executable_sha256 = file_sha256(&chrome_exe)?;
    if executable_sha256 != expected_executable_sha256 {
        let _ = fs::remove_dir_all(&tmp_dir);
        return Err(format!(
            "chrome.exe sha256 mismatch: expected {expected_executable_sha256}, got {executable_sha256}"
        ));
    }

    write_stealthcdp_manifest(&tmp_dir, "chrome.exe")?;
    fs::rename(&tmp_dir, &artifact_dir).map_err(|err| {
        format!(
            "Failed to move {} to {}: {err}",
            tmp_dir.display(),
            artifact_dir.display()
        )
    })?;

    create_current_pointer(install_root, &artifact_dir)
}

pub fn run_install_stealthcdp_chromium(force: bool) {
    println!("{}", color::cyan("Installing chromium-stealthcdp..."));

    let install_root = match stealthcdp_chromium_install_root() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("{} {}", color::error_indicator(), err);
            exit(1);
        }
    };
    let current = install_root.join("current");
    if current.join("chrome.exe").is_file() && current.join("manifest.json").is_file() && !force {
        println!(
            "{} chromium-stealthcdp is already installed",
            color::success_indicator()
        );
        println!("  Location: {}", current.display());
        println!("  Manifest: {}", current.join("manifest.json").display());
        return;
    }

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|e| {
            eprintln!(
                "{} Failed to create runtime: {}",
                color::error_indicator(),
                e
            );
            exit(1);
        });
    let url = stealthcdp_chromium_release_url();
    println!("  Downloading {STEALTHCDP_CHROMIUM_RELEASE_ASSET}");
    println!("  {url}");

    let bytes = match rt.block_on(download_bytes(&url)) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("{} {}", color::error_indicator(), err);
            exit(1);
        }
    };

    match install_stealthcdp_chromium_archive_bytes(
        bytes,
        &install_root,
        force,
        STEALTHCDP_CHROMIUM_RELEASE_ASSET_SHA256,
        STEALTHCDP_CHROMIUM_EXECUTABLE_SHA256,
    ) {
        Ok(current) => {
            println!(
                "{} chromium-stealthcdp installed successfully",
                color::success_indicator()
            );
            println!("  Location: {}", current.display());
            println!("  Manifest: {}", current.join("manifest.json").display());
            println!(
                "  Run a live smoke before relying on it as ready: agent-browser install doctor"
            );
        }
        Err(err) => {
            eprintln!("{} {}", color::error_indicator(), err);
            exit(1);
        }
    }
}

const REMOTE_VIEW_PRIVILEGE_INSTALLER: &str =
    include_str!("../../scripts/install-agent-browser-privileges.sh");
const REMOTE_VIEW_PRIVILEGED_HELPER: &str =
    include_str!("../../scripts/libexec/agent-browser-privileged-helper");

pub fn run_install(with_deps: bool, with_remote_view_privileges: bool) {
    if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        eprintln!(
            "{} Chrome for Testing does not provide Linux ARM64 builds.",
            color::error_indicator()
        );
        eprintln!("  Install Chromium from your system package manager instead:");
        eprintln!("    sudo apt install chromium-browser   # Debian/Ubuntu");
        eprintln!("    sudo dnf install chromium            # Fedora");
        eprintln!("  Then use: agent-browser --executable-path /usr/bin/chromium");
        exit(1);
    }

    let is_linux = cfg!(target_os = "linux");

    if is_linux {
        if with_remote_view_privileges {
            install_remote_view_privileges();
        }

        if with_deps {
            install_linux_deps();
        } else {
            println!(
                "{} Linux detected. If browser fails to launch, run:",
                color::warning_indicator()
            );
            println!("  agent-browser install --with-deps");
            println!();
        }
    } else if with_remote_view_privileges {
        eprintln!(
            "{} --with-remote-view-privileges is only supported on Linux.",
            color::error_indicator()
        );
        exit(1);
    }

    println!("{}", color::cyan("Installing Chrome..."));

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|e| {
            eprintln!(
                "{} Failed to create runtime: {}",
                color::error_indicator(),
                e
            );
            exit(1);
        });

    let (version, url) = match rt.block_on(fetch_download_url()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{} {}", color::error_indicator(), e);
            exit(1);
        }
    };

    let dest = get_browsers_dir().join(format!("chrome-{}", version));

    if let Some(bin) = chrome_binary_in_dir(&dest) {
        if bin.exists() {
            println!(
                "{} Chrome {} is already installed",
                color::success_indicator(),
                version
            );
            return;
        }
    }

    println!("  Downloading Chrome {} for {}", version, platform_key());
    println!("  {}", url);

    let bytes = match rt.block_on(download_bytes(&url)) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{} {}", color::error_indicator(), e);
            exit(1);
        }
    };

    match extract_zip(bytes, &dest) {
        Ok(()) => {
            println!(
                "{} Chrome {} installed successfully",
                color::success_indicator(),
                version
            );
            println!("  Location: {}", dest.display());

            if is_linux && !with_deps {
                println!();
                println!(
                    "{} If you see \"shared library\" errors when running, use:",
                    color::yellow("Note:")
                );
                println!("  agent-browser install --with-deps");
            }
        }
        Err(e) => {
            let _ = fs::remove_dir_all(&dest);
            eprintln!("{} {}", color::error_indicator(), e);
            exit(1);
        }
    }
}

/// Inspect the user-scoped command and package binaries without launching a browser.
pub fn run_install_doctor(flags: &Flags) {
    let report = install_doctor_report(flags);
    if flags.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| {
                r#"{"success":false,"error":"Failed to serialize install doctor report"}"#
                    .to_string()
            })
        );
        if !report
            .get("success")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
        {
            exit(1);
        }
        return;
    }

    let success = report
        .get("success")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let indicator = if success {
        color::success_indicator()
    } else {
        color::warning_indicator()
    };
    println!("{indicator} agent-browser install doctor");
    print_doctor_field("version", report.pointer("/data/version"));
    print_doctor_field("path command", report.pointer("/data/pathCommand/path"));
    print_doctor_field(
        "current executable",
        report.pointer("/data/currentExecutable/path"),
    );
    print_doctor_field(
        "pnpm package binary",
        report.pointer("/data/pnpmPackageBinary/path"),
    );
    print_doctor_field(
        "workspace binary",
        report.pointer("/data/workspaceBinary/path"),
    );
    print_doctor_field(
        "launch config source",
        report.pointer("/data/launchConfig/executablePathSource"),
    );
    print_doctor_field(
        "launch config ready",
        report.pointer("/data/launchConfig/stealthCdpChromiumReady"),
    );
    print_doctor_field(
        "launch executable",
        report.pointer("/data/launchConfig/executablePath"),
    );
    print_doctor_field("service ready", report.pointer("/data/service/ready"));
    print_doctor_field(
        "service no-launch",
        report.pointer("/data/service/noLaunch"),
    );
    print_doctor_field(
        "resource candidates",
        report.pointer("/data/serviceResources/candidateCount"),
    );
    print_doctor_field(
        "readiness resource candidates",
        report.pointer("/data/serviceResources/readinessImpactingCandidates"),
    );
    print_doctor_field(
        "remote-view privileges ready",
        report.pointer("/data/remoteViewPrivileges/ready"),
    );
    print_doctor_field(
        "dashboard runtime contract",
        report.pointer("/data/dashboardRuntime/serviceContractVersion"),
    );
    print_doctor_field(
        "dashboard runtime sha",
        report.pointer("/data/dashboardRuntime/dashboard/sha256"),
    );
    print_doctor_field(
        "dashboard runtime executable",
        report.pointer("/data/dashboardRuntime/executable/sha256"),
    );
    print_doctor_field(
        "live dashboard runtime",
        report.pointer("/data/liveDashboardRuntime/state"),
    );
    print_doctor_field(
        "live dashboard executable",
        report.pointer("/data/liveDashboardRuntime/executable/sha256"),
    );
    print_doctor_field(
        "runtime convergence",
        report.pointer("/data/runtimeConvergence/status"),
    );
    print_doctor_field(
        "runtime inventory",
        report.pointer("/data/runtimeInventory/status"),
    );
    print_doctor_field(
        "stale runtimes",
        report.pointer("/data/runtimeInventory/staleCount"),
    );
    print_doctor_field(
        "remote-view helper",
        report.pointer("/data/remoteViewPrivileges/helperPath"),
    );
    print_doctor_field(
        "remote-view sudoers",
        report.pointer("/data/remoteViewPrivileges/sudoersPath"),
    );

    let issues = report
        .pointer("/data/issues")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    if issues.is_empty() {
        println!("No install drift detected.");
    } else {
        println!("Issues:");
        for issue in issues {
            let code = issue
                .get("code")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown");
            let message = issue
                .get("message")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            println!("  - {code}: {message}");
        }
        exit(1);
    }
}

fn print_doctor_field(label: &str, value: Option<&serde_json::Value>) {
    let rendered = match value {
        Some(value) if value.is_string() => value.as_str().unwrap_or("").to_string(),
        Some(value) if value.is_boolean() => value.as_bool().unwrap_or(false).to_string(),
        Some(value) if !value.is_null() => value.to_string(),
        _ => "not found".to_string(),
    };
    println!("{label}: {rendered}");
}

fn install_doctor_report(flags: &Flags) -> serde_json::Value {
    let current_executable = binary_fingerprint(std::env::current_exe().ok());
    let path_command = binary_fingerprint(find_path_command(command_name()));
    let pnpm_package_binary = binary_fingerprint(find_pnpm_package_binary());
    let workspace_binary = binary_fingerprint(find_workspace_binary());
    let launch_config = launch_config_status(flags);
    let remote_view_privileges = remote_view_privilege_status();
    let dashboard_runtime = runtime_manifest_json();
    let live_dashboard_runtime = live_dashboard_runtime_probe(
        current_executable
            .get("sha256")
            .and_then(|value| value.as_str()),
    );
    let service = service_status_probe();
    let service_resources = service_resources_probe();
    let runtime_inventory = active_runtime_inventory(
        current_executable
            .get("sha256")
            .and_then(|value| value.as_str()),
    );
    let runtime_convergence =
        runtime_convergence_summary(&live_dashboard_runtime, &runtime_inventory);
    let issues = install_doctor_issues(InstallDoctorIssueInputs {
        current_executable: &current_executable,
        path_command: &path_command,
        pnpm_package_binary: &pnpm_package_binary,
        workspace_binary: &workspace_binary,
        launch_config: &launch_config,
        service: &service,
        service_resources: &service_resources,
        live_dashboard_runtime: &live_dashboard_runtime,
        runtime_inventory: &runtime_inventory,
    });

    json!({
        "success": issues.is_empty(),
        "data": {
            "version": env!("CARGO_PKG_VERSION"),
            "currentExecutable": current_executable,
            "pathCommand": path_command,
            "pnpmPackageBinary": pnpm_package_binary,
            "workspaceBinary": workspace_binary,
            "launchConfig": launch_config,
            "service": service,
            "serviceResources": service_resources,
            "remoteViewPrivileges": remote_view_privileges,
            "dashboardRuntime": dashboard_runtime,
            "liveDashboardRuntime": live_dashboard_runtime,
            "runtimeInventory": runtime_inventory,
            "runtimeConvergence": runtime_convergence,
            "issues": issues,
        }
    })
}

struct InstallDoctorIssueInputs<'a> {
    current_executable: &'a serde_json::Value,
    path_command: &'a serde_json::Value,
    pnpm_package_binary: &'a serde_json::Value,
    workspace_binary: &'a serde_json::Value,
    launch_config: &'a serde_json::Value,
    service: &'a serde_json::Value,
    service_resources: &'a serde_json::Value,
    live_dashboard_runtime: &'a serde_json::Value,
    runtime_inventory: &'a serde_json::Value,
}

fn install_doctor_issues(inputs: InstallDoctorIssueInputs<'_>) -> Vec<serde_json::Value> {
    let mut issues = Vec::new();
    let current_executable = inputs.current_executable;
    let path_command = inputs.path_command;
    let pnpm_package_binary = inputs.pnpm_package_binary;
    let workspace_binary = inputs.workspace_binary;
    let launch_config = inputs.launch_config;
    let service = inputs.service;
    let service_resources = inputs.service_resources;
    let live_dashboard_runtime = inputs.live_dashboard_runtime;
    let runtime_inventory = inputs.runtime_inventory;

    if path_command
        .get("path")
        .and_then(|value| value.as_str())
        .is_none()
    {
        issues.push(json!({
            "code": "path_command_missing",
            "message": "agent-browser was not found on PATH"
        }));
    }

    if let (Some(current_hash), Some(path_hash)) = (
        current_executable
            .get("sha256")
            .and_then(|value| value.as_str()),
        path_command.get("sha256").and_then(|value| value.as_str()),
    ) {
        if current_hash != path_hash {
            issues.push(json!({
                "code": "current_executable_path_command_mismatch",
                "message": "the running executable does not match the agent-browser command on PATH"
            }));
        }
    }

    if let (Some(path_hash), Some(package_hash)) = (
        path_command.get("sha256").and_then(|value| value.as_str()),
        pnpm_package_binary
            .get("sha256")
            .and_then(|value| value.as_str()),
    ) {
        if path_hash != package_hash {
            issues.push(json!({
                "code": "path_command_pnpm_binary_mismatch",
                "message": "the agent-browser command on PATH does not match the pnpm global package binary"
            }));
        }
    }

    if let (Some(path_hash), Some(workspace_hash)) = (
        path_command.get("sha256").and_then(|value| value.as_str()),
        workspace_binary
            .get("sha256")
            .and_then(|value| value.as_str()),
    ) {
        if path_hash != workspace_hash {
            issues.push(json!({
                "code": "path_command_workspace_binary_mismatch",
                "message": "the agent-browser command on PATH does not match the binary in the current workspace"
            }));
        }
    }

    let stealth_required = launch_config
        .get("stealthCdpChromiumRequired")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let stealth_ready = launch_config
        .get("stealthCdpChromiumReady")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    if stealth_required && !stealth_ready {
        issues.push(json!({
            "code": "launch_config_not_ready",
            "message": "configured stealthcdp_chromium launch posture is not ready"
        }));
    }

    if !service
        .get("ready")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
    {
        issues.push(json!({
            "code": "service_status_not_ready",
            "message": "agent-browser service status no-launch probe did not report ready"
        }));
    }

    let readiness_candidates = service_resources
        .get("readinessImpactingCandidates")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    if readiness_candidates > 0 {
        issues.push(json!({
            "code": "service_resource_candidates_ready",
            "message": "service resource monitor found readiness-impacting stale resource candidates"
        }));
    }

    let duplicate_pressure_warnings = service_resources
        .get("duplicateProfilePressureWarnings")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    if duplicate_pressure_warnings > 0 {
        issues.push(json!({
            "code": "service_duplicate_profile_pressure",
            "message": "service resource monitor found duplicate live browser or profile lease pressure for the same retained profile"
        }));
    }

    if live_dashboard_runtime
        .get("available")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
        && !live_dashboard_runtime
            .get("ready")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    {
        issues.push(json!({
            "code": "dashboard_runtime_stale_or_unreadable",
            "message": "the running local dashboard service did not serve a runtime manifest matching the current executable",
            "nextAction": "converge_local_runtime",
            "remedy": {
                "kind": "operator_command",
                "command": "pnpm converge:local-runtime -- --apply --json",
                "argv": ["pnpm", "converge:local-runtime", "--", "--apply", "--json"],
                "requiresInteractiveSudo": false,
                "why": "Republish and restart the local dashboard runtime, then rerun install doctor."
            },
            "dashboardUrl": live_dashboard_runtime.get("url").cloned().unwrap_or(serde_json::Value::Null),
            "state": live_dashboard_runtime.get("state").cloned().unwrap_or(serde_json::Value::Null)
        }));
    }

    if let Some(rows) = runtime_inventory
        .get("runtimes")
        .and_then(|value| value.as_array())
    {
        for row in rows {
            if row
                .get("state")
                .and_then(|value| value.as_str())
                .is_some_and(|state| state == "stale")
            {
                let session = row
                    .get("session")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown");
                issues.push(json!({
                    "code": "active_runtime_stale_executable",
                    "message": format!("active daemon session {session} was started by stale or incomplete executable metadata"),
                    "session": session,
                    "nextAction": "restart_stale_daemon_session",
                    "remedy": {
                        "kind": "operator_command",
                        "command": "agent-browser close --session <session>",
                        "argv": ["agent-browser", "close", "--session", session],
                        "requiresInteractiveSudo": false,
                        "why": "Stop the stale daemon session so the next command relaunches it with the current executable."
                    }
                }));
            }
        }
    }

    issues
}

fn active_runtime_inventory(expected_sha256: Option<&str>) -> serde_json::Value {
    let socket_dir = get_socket_dir();
    let mut sessions: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    if let Ok(entries) = fs::read_dir(&socket_dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let Some((session, extension)) = file_name.rsplit_once('.') else {
                continue;
            };
            match extension {
                "pid" | "version" | "sha256" | "stream" | "sock" | "port" => {
                    sessions
                        .entry(session.to_string())
                        .or_default()
                        .insert(extension.to_string());
                }
                _ => {}
            }
        }
    }

    let mut rows = Vec::new();
    let mut converged_count = 0usize;
    let mut stale_count = 0usize;
    for (session, extensions) in sessions {
        let pid_path = socket_dir.join(format!("{session}.pid"));
        let version_path = socket_dir.join(format!("{session}.version"));
        let sha_path = socket_dir.join(format!("{session}.sha256"));
        let stream_path = socket_dir.join(format!("{session}.stream"));
        let pid = fs::read_to_string(&pid_path)
            .ok()
            .and_then(|value| value.trim().parse::<u32>().ok());
        let package_version = fs::read_to_string(&version_path)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let executable_sha256 = fs::read_to_string(&sha_path)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let stream_port = fs::read_to_string(&stream_path)
            .ok()
            .and_then(|value| value.trim().parse::<u16>().ok());
        let pid_running = pid.is_some_and(pid_is_running);
        let addressable = extensions.contains("sock")
            || extensions.contains("port")
            || extensions.contains("stream");
        let package_version_matches = package_version
            .as_deref()
            .map(|value| value == env!("CARGO_PKG_VERSION"))
            .unwrap_or(false);
        let executable_sha256_matches = match (expected_sha256, executable_sha256.as_deref()) {
            (Some(expected), Some(actual)) => expected == actual,
            (None, _) => true,
            (_, None) => false,
        };
        let state =
            if pid_running && addressable && package_version_matches && executable_sha256_matches {
                converged_count += 1;
                "converged"
            } else if pid_running && addressable {
                stale_count += 1;
                "stale"
            } else if pid_running {
                "diagnostic"
            } else {
                "inactive"
            };
        rows.push(json!({
            "kind": "daemon_session",
            "session": session,
            "state": state,
            "pid": pid,
            "pidRunning": pid_running,
            "packageVersion": package_version,
            "packageVersionMatches": package_version_matches,
            "executableSha256": executable_sha256,
            "expectedExecutableSha256": expected_sha256,
            "executableSha256Matches": executable_sha256_matches,
            "streamPort": stream_port,
            "metadata": {
                "socketDir": socket_dir.display().to_string(),
                "hasPid": extensions.contains("pid"),
                "hasVersion": extensions.contains("version"),
                "hasExecutableSha256": extensions.contains("sha256"),
                "hasSocket": extensions.contains("sock"),
                "hasPort": extensions.contains("port"),
                "hasStream": extensions.contains("stream"),
                "addressable": addressable,
            }
        }));
    }
    let status = if stale_count > 0 {
        "stale"
    } else if converged_count > 0 {
        "converged"
    } else {
        "none"
    };
    json!({
        "schemaVersion": "agent-browser.runtime-inventory.v1",
        "status": status,
        "socketDir": socket_dir.display().to_string(),
        "expectedExecutableSha256": expected_sha256,
        "runtimeCount": rows.len(),
        "convergedCount": converged_count,
        "staleCount": stale_count,
        "runtimes": rows,
    })
}

fn pid_is_running(pid: u32) -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

fn live_dashboard_runtime_probe(expected_executable_sha256: Option<&str>) -> serde_json::Value {
    let (url, host_header, port) = match local_dashboard_probe_target() {
        Ok(target) => target,
        Err(reason) => {
            return json!({
                "available": false,
                "ready": true,
                "state": "unsupported_dashboard_url",
                "reason": reason,
            });
        }
    };
    let address = format!("127.0.0.1:{port}");
    let timeout = Duration::from_millis(12_000);
    let mut stream = match TcpStream::connect_timeout(
        &address
            .parse()
            .unwrap_or_else(|_| "127.0.0.1:4848".parse().unwrap()),
        timeout,
    ) {
        Ok(stream) => stream,
        Err(error) => {
            return json!({
                "available": false,
                "ready": true,
                "state": "not_running",
                "url": url,
                "reason": redact_doctor_text(error.to_string()),
            });
        }
    };
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));
    let request = format!(
        "GET /api/runtime/manifest HTTP/1.1\r\nHost: {host_header}\r\nConnection: close\r\n\r\n"
    );
    if let Err(error) = stream.write_all(request.as_bytes()) {
        return json!({
            "available": true,
            "ready": false,
            "state": "unreadable_manifest",
            "url": url,
            "reason": redact_doctor_text(error.to_string()),
        });
    }
    let mut response_bytes = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => response_bytes.extend_from_slice(&buffer[..n]),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) && !response_bytes.is_empty() =>
            {
                break;
            }
            Err(error) => {
                return json!({
                    "available": true,
                    "ready": false,
                    "state": "unreadable_manifest",
                    "url": url,
                    "reason": redact_doctor_text(error.to_string()),
                });
            }
        }
    }
    let response = String::from_utf8_lossy(&response_bytes);
    let status_ok = response.starts_with("HTTP/1.1 200") || response.starts_with("HTTP/1.0 200");
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or("");
    let manifest = serde_json::from_str::<serde_json::Value>(body).ok();
    let executable_sha256 = manifest
        .as_ref()
        .and_then(|value| value.pointer("/executable/sha256"))
        .and_then(|value| value.as_str());
    let executable_matches = match (expected_executable_sha256, executable_sha256) {
        (Some(expected), Some(actual)) => expected == actual,
        (None, Some(_)) => true,
        (_, None) => false,
    };
    let ready = status_ok && manifest.is_some() && executable_matches;
    let state = if ready {
        "ready"
    } else if !status_ok {
        "manifest_http_error"
    } else if manifest.is_none() {
        "unreadable_manifest"
    } else {
        "stale_executable"
    };
    json!({
        "available": true,
        "ready": ready,
        "state": state,
        "url": url,
        "expectedExecutableSha256": expected_executable_sha256,
        "executable": {
            "sha256": executable_sha256,
            "matchesExpected": executable_matches,
        },
        "serviceContractVersion": manifest
            .as_ref()
            .and_then(|value| value.get("serviceContractVersion"))
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        "dashboard": manifest
            .as_ref()
            .and_then(|value| value.get("dashboard"))
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    })
}

fn local_dashboard_probe_target() -> Result<(String, String, u16), String> {
    if let Ok(url) = std::env::var("AGENT_BROWSER_DASHBOARD_URL") {
        let trimmed = url.trim().trim_end_matches('/');
        if !trimmed.is_empty() {
            let Some(rest) = trimmed.strip_prefix("http://") else {
                return Err(
                    "AGENT_BROWSER_DASHBOARD_URL must use http:// for the local no-launch probe"
                        .to_string(),
                );
            };
            let host_port = rest.split('/').next().unwrap_or("");
            let (host, port) = parse_local_dashboard_host_port(host_port)?;
            let origin = format!("http://{host}:{port}");
            return Ok((
                format!("{origin}/api/runtime/manifest"),
                format!("{host}:{port}"),
                port,
            ));
        }
    }
    let port = std::env::var("AGENT_BROWSER_DASHBOARD_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(4848);
    Ok((
        format!("http://127.0.0.1:{port}/api/runtime/manifest"),
        format!("127.0.0.1:{port}"),
        port,
    ))
}

fn parse_local_dashboard_host_port(host_port: &str) -> Result<(String, u16), String> {
    let (host, port) = host_port
        .rsplit_once(':')
        .ok_or_else(|| "local dashboard URL must include an explicit port".to_string())?;
    if host != "127.0.0.1" && host != "localhost" {
        return Err("local dashboard URL must target 127.0.0.1 or localhost".to_string());
    }
    let port = port
        .parse::<u16>()
        .map_err(|_| "local dashboard URL port is invalid".to_string())?;
    Ok((host.to_string(), port))
}

fn runtime_convergence_summary(
    live_dashboard_runtime: &serde_json::Value,
    runtime_inventory: &serde_json::Value,
) -> serde_json::Value {
    let stale_runtime_count = runtime_inventory
        .get("staleCount")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let diagnostic_runtime_count = runtime_inventory
        .get("runtimes")
        .and_then(|value| value.as_array())
        .map(|rows| {
            rows.iter()
                .filter(|row| {
                    row.get("state")
                        .and_then(|value| value.as_str())
                        .is_some_and(|state| state == "diagnostic")
                })
                .count()
        })
        .unwrap_or(0);
    let live_dashboard_available = live_dashboard_runtime
        .get("available")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let live_dashboard_ready = live_dashboard_runtime
        .get("ready")
        .and_then(|value| value.as_bool())
        .unwrap_or(true);
    let live_dashboard_state = live_dashboard_runtime
        .get("state")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown");
    let live_dashboard_stale = live_dashboard_available
        && !live_dashboard_ready
        && live_dashboard_state == "stale_executable";
    let manual_review_required = live_dashboard_available
        && !live_dashboard_ready
        && live_dashboard_state != "stale_executable";
    let status = if manual_review_required {
        "manual_review_required"
    } else if stale_runtime_count > 0 || live_dashboard_stale {
        "stale"
    } else if diagnostic_runtime_count > 0 {
        "partial"
    } else {
        "converged"
    };
    json!({
        "schemaVersion": "agent-browser.runtime-convergence.v1",
        "status": status,
        "staleRuntimeCount": stale_runtime_count,
        "diagnosticRuntimeCount": diagnostic_runtime_count,
        "liveDashboardAvailable": live_dashboard_available,
        "liveDashboardReady": live_dashboard_ready,
        "liveDashboardState": live_dashboard_state,
    })
}

fn service_status_probe() -> serde_json::Value {
    let temp_home = std::env::temp_dir().join(format!(
        "agent-browser-install-doctor-service-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_micros())
            .unwrap_or(0)
    ));
    let probe_session = format!(
        "install-doctor-service-probe-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_micros())
            .unwrap_or(0)
    );
    let current_exe = std::env::current_exe().ok();
    let Some(current_exe) = current_exe else {
        return json!({
            "available": false,
            "ready": false,
            "success": false,
            "exitCode": null,
            "command": "agent-browser --json service status",
            "noLaunch": true,
            "browserHealth": null,
            "statePathExists": false,
            "stderr": "current executable path is unavailable",
        });
    };

    let output = Command::new(&current_exe)
        .args(["--json", "--session", &probe_session, "service", "status"])
        .env("AGENT_BROWSER_HOME", &temp_home)
        .env("AGENT_BROWSER_ARGS", "--no-sandbox")
        .output();
    let state_path = temp_home.join("service").join("state.json");

    let result = match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let data = serde_json::from_str::<serde_json::Value>(stdout.trim()).ok();
            let command_success = output.status.success();
            let response_success = data
                .as_ref()
                .and_then(|value| value.get("success"))
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            let browser_health = data
                .as_ref()
                .and_then(|value| value.pointer("/data/control_plane/browser_health"))
                .and_then(|value| value.as_str())
                .map(ToString::to_string);
            let no_launch = browser_health.as_deref() == Some("NotStarted") || !state_path.exists();
            json!({
                "available": true,
                "ready": command_success && response_success && no_launch,
                "success": command_success,
                "exitCode": output.status.code(),
                "command": format!(
                    "{} --json --session {} service status",
                    current_exe.display(),
                    probe_session
                ),
                "noLaunch": no_launch,
                "browserHealth": browser_health,
                "statePathExists": state_path.exists(),
                "stderr": redact_doctor_text(stderr.trim()),
            })
        }
        Err(error) => json!({
            "available": false,
            "ready": false,
            "success": false,
            "exitCode": null,
            "command": format!(
                "{} --json --session {} service status",
                current_exe.display(),
                probe_session
            ),
            "noLaunch": true,
            "browserHealth": null,
            "statePathExists": state_path.exists(),
            "stderr": redact_doctor_text(error.to_string()),
        }),
    };

    let _ = Command::new(&current_exe)
        .args(["--json", "--session", &probe_session, "close"])
        .env("AGENT_BROWSER_HOME", &temp_home)
        .env("AGENT_BROWSER_ARGS", "--no-sandbox")
        .output();
    terminate_session_process_from_metadata(&probe_session);
    cleanup_stale_files(&probe_session);
    let _ = fs::remove_dir_all(&temp_home);
    result
}

fn terminate_session_process_from_metadata(session: &str) {
    let pid_path = get_socket_dir().join(format!("{session}.pid"));
    let pid = fs::read_to_string(pid_path)
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok());
    let Some(pid) = pid else {
        return;
    };
    #[cfg(unix)]
    unsafe {
        libc::kill(pid as i32, libc::SIGKILL);
    }
    #[cfg(windows)]
    {
        let _ = pid;
    }
}

fn service_resources_probe() -> serde_json::Value {
    let current_exe = std::env::current_exe().ok();
    let Some(current_exe) = current_exe else {
        return json!({
            "available": false,
            "success": false,
            "candidateCount": 0,
            "readinessImpactingCandidates": 0,
            "duplicateProfilePressureWarnings": 0,
            "stderr": "current executable path is unavailable",
        });
    };

    let output = Command::new(&current_exe)
        .args(["--json", "service", "gc", "--dry-run"])
        .env("AGENT_BROWSER_ARGS", "--no-sandbox")
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let data = serde_json::from_str::<serde_json::Value>(stdout.trim()).ok();
            let candidates = data
                .as_ref()
                .and_then(|value| value.pointer("/data/actions/terminateProcess"))
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default();
            let readiness_impacting = candidates
                .iter()
                .filter(|candidate| service_resource_candidate_is_readiness_impacting(candidate))
                .count();
            let warnings = data
                .as_ref()
                .and_then(|value| value.pointer("/data/warnings"))
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default();
            let duplicate_profile_pressure_warnings = warnings
                .iter()
                .filter(|warning| {
                    matches!(
                        warning.get("code").and_then(|value| value.as_str()),
                        Some(
                            "duplicate_live_browsers_for_profile"
                                | "duplicate_active_profile_leases"
                        )
                    )
                })
                .count();
            json!({
                "available": true,
                "success": output.status.success()
                    && data
                        .as_ref()
                        .and_then(|value| value.get("success"))
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false),
                "candidateCount": candidates.len(),
                "readinessImpactingCandidates": readiness_impacting,
                "duplicateProfilePressureWarnings": duplicate_profile_pressure_warnings,
                "stderr": redact_doctor_text(stderr.trim()),
            })
        }
        Err(error) => json!({
            "available": false,
            "success": false,
            "candidateCount": 0,
            "readinessImpactingCandidates": 0,
            "duplicateProfilePressureWarnings": 0,
            "stderr": redact_doctor_text(error.to_string()),
        }),
    }
}

fn service_resource_candidate_is_readiness_impacting(candidate: &serde_json::Value) -> bool {
    let kind = candidate.get("kind").and_then(|value| value.as_str());
    let has_display_reason = candidate
        .get("reasons")
        .and_then(|value| value.as_array())
        .is_some_and(|reasons| {
            reasons.iter().any(|reason| {
                matches!(
                    reason.as_str(),
                    Some("orphaned_remote_display_process" | "old_temporary_profile_process")
                )
            })
        });
    kind == Some("remote_display") || has_display_reason
}

fn remote_view_privilege_status() -> serde_json::Value {
    let group_name = std::env::var("AGENT_BROWSER_PRIVILEGED_GROUP")
        .unwrap_or_else(|_| "agent-browser".to_string());
    let helper_path = std::env::var("AGENT_BROWSER_PRIVILEGED_HELPER").unwrap_or_else(|_| {
        "/usr/local/libexec/agent-browser/agent-browser-privileged-helper".to_string()
    });
    let sudoers_path = std::env::var("AGENT_BROWSER_PRIVILEGED_SUDOERS")
        .unwrap_or_else(|_| "/etc/sudoers.d/agent-browser".to_string());
    let current_user = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    let group_exists = command_success("getent", &["group", &group_name]);
    let user_groups = Command::new("id").arg("-nG").output().ok().map(|output| {
        String::from_utf8_lossy(&output.stdout)
            .split_whitespace()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    });
    let user_in_group = user_groups
        .as_ref()
        .map(|groups| groups.iter().any(|group| group == &group_name))
        .unwrap_or(false);
    let helper_exists = Path::new(&helper_path).exists();
    let sudoers_exists = Path::new(&sudoers_path).exists();
    let helper_check = command_output_summary("sudo", &["-n", &helper_path, "check"]);

    let mut issues = Vec::new();
    if !group_exists {
        issues.push(remote_view_privilege_issue(
            "remote_view_privileged_group_missing",
            "the agent-browser privileged group is missing",
            "run agent-browser install --with-remote-view-privileges from an interactive terminal",
            true,
        ));
    }
    if group_exists && !user_in_group {
        issues.push(remote_view_privilege_issue(
            "remote_view_privileged_group_membership_missing",
            "the current user is not in the agent-browser privileged group",
            "run agent-browser install --with-remote-view-privileges, then open a new shell or run newgrp agent-browser",
            true,
        ));
    }
    if !helper_exists {
        issues.push(remote_view_privilege_issue(
            "remote_view_privileged_helper_missing",
            "the root-owned remote-view privileged helper is missing",
            "run agent-browser install --with-remote-view-privileges from an interactive terminal",
            true,
        ));
    }
    if !sudoers_exists {
        issues.push(remote_view_privilege_issue(
            "remote_view_privileged_sudoers_missing",
            "the remote-view sudoers policy is missing",
            "run agent-browser install --with-remote-view-privileges from an interactive terminal",
            true,
        ));
    }
    if helper_exists
        && helper_check
            .get("success")
            .and_then(|value| value.as_bool())
            != Some(true)
    {
        issues.push(remote_view_privilege_issue(
            "remote_view_privileged_helper_not_usable",
            "the remote-view privileged helper cannot be run with sudo -n",
            "install the helper, confirm group membership in a new shell, and rerun agent-browser install doctor",
            false,
        ));
    }

    let ready = group_exists
        && user_in_group
        && helper_exists
        && sudoers_exists
        && helper_check
            .get("success")
            .and_then(|value| value.as_bool())
            == Some(true);

    json!({
        "ready": ready,
        "requiresInteractiveSudo": !ready,
        "currentUser": current_user,
        "groupName": group_name,
        "groupExists": group_exists,
        "userInGroup": user_in_group,
        "helperPath": helper_path,
        "helperExists": helper_exists,
        "sudoersPath": sudoers_path,
        "sudoersExists": sudoers_exists,
        "helperCheck": helper_check,
        "issues": issues,
    })
}

fn remote_view_privilege_issue(
    code: &str,
    message: &str,
    remediation: &str,
    first_install_sudo: bool,
) -> serde_json::Value {
    json!({
        "code": code,
        "message": message,
        "remediation": remediation,
        "requiresInteractiveSudo": first_install_sudo,
    })
}

fn command_success(command: &str, args: &[&str]) -> bool {
    Command::new(command)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn command_output_summary(command: &str, args: &[&str]) -> serde_json::Value {
    match Command::new(command).args(args).output() {
        Ok(output) => json!({
            "available": true,
            "success": output.status.success(),
            "exitCode": output.status.code(),
            "stdout": redact_doctor_text(String::from_utf8_lossy(&output.stdout).trim()),
            "stderr": redact_doctor_text(String::from_utf8_lossy(&output.stderr).trim()),
        }),
        Err(error) => json!({
            "available": false,
            "success": false,
            "exitCode": null,
            "stdout": "",
            "stderr": redact_doctor_text(error.to_string()),
        }),
    }
}

fn redact_doctor_text(text: impl AsRef<str>) -> String {
    text.as_ref()
        .lines()
        .map(|line| {
            let lower = line.to_ascii_lowercase();
            if lower.contains("password") || lower.contains("secret") || lower.contains("token") {
                if let Some((key, _)) = line.split_once('=') {
                    format!("{}=<redacted>", key.trim())
                } else {
                    "<redacted>".to_string()
                }
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn binary_fingerprint(path: Option<PathBuf>) -> serde_json::Value {
    let Some(path) = path else {
        return json!({
            "path": null,
            "exists": false,
            "sha256": null,
            "sizeBytes": null,
        });
    };
    let canonical_path = path.canonicalize().unwrap_or(path.clone());
    let metadata = fs::metadata(&path);
    let sha256 = file_sha256(&path).ok();
    json!({
        "path": path.display().to_string(),
        "canonicalPath": canonical_path.display().to_string(),
        "exists": metadata.is_ok(),
        "sha256": sha256,
        "sizeBytes": metadata.ok().map(|metadata| metadata.len()),
    })
}

fn file_sha256(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};

    let mut file =
        fs::File::open(path).map_err(|err| format!("failed to open {}: {err}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn command_name() -> &'static str {
    if cfg!(windows) {
        "agent-browser.exe"
    } else {
        "agent-browser"
    }
}

fn native_binary_name() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Some("agent-browser-linux-x64"),
        ("linux", "aarch64") => Some("agent-browser-linux-arm64"),
        ("macos", "x86_64") => Some("agent-browser-darwin-x64"),
        ("macos", "aarch64") => Some("agent-browser-darwin-arm64"),
        ("windows", "x86_64") => Some("agent-browser-win32-x64.exe"),
        ("windows", "aarch64") => Some("agent-browser-win32-arm64.exe"),
        _ => None,
    }
}

fn find_path_command(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            if !name.ends_with(".exe") {
                let candidate = dir.join(format!("{name}.exe"));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn find_pnpm_package_binary() -> Option<PathBuf> {
    let binary_name = native_binary_name()?;
    if let Ok(root) = std::env::var("AGENT_BROWSER_INSTALL_DOCTOR_PNPM_ROOT") {
        return Some(
            PathBuf::from(root)
                .join("agent-browser")
                .join("bin")
                .join(binary_name),
        );
    }
    let output = Command::new("pnpm").args(["root", "-g"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if root.is_empty() {
        return None;
    }
    Some(
        PathBuf::from(root)
            .join("agent-browser")
            .join("bin")
            .join(binary_name),
    )
}

fn find_workspace_binary() -> Option<PathBuf> {
    let binary_name = native_binary_name()?;
    let candidate = std::env::current_dir().ok()?.join("bin").join(binary_name);
    candidate.is_file().then_some(candidate)
}

fn report_install_status(status: io::Result<std::process::ExitStatus>) {
    match status {
        Ok(s) if s.success() => {
            println!(
                "{} System dependencies installed",
                color::success_indicator()
            )
        }
        Ok(_) => eprintln!(
            "{} Failed to install some dependencies. You may need to run manually with sudo.",
            color::warning_indicator()
        ),
        Err(e) => eprintln!(
            "{} Could not run install command: {}",
            color::warning_indicator(),
            e
        ),
    }
}

fn install_linux_deps() {
    println!("{}", color::cyan("Installing system dependencies..."));

    let (pkg_mgr, deps) = if which_exists("apt-get") {
        // On Ubuntu 24.04+, many libraries were renamed with a t64 suffix as
        // part of the 64-bit time_t transition. Using the old names can cause
        // apt to propose removing hundreds of system packages to resolve
        // conflicts. We check for the t64 variant first to avoid this.
        let apt_deps: Vec<&str> = vec![
            ("libxcb-shm0", None),
            ("libx11-xcb1", None),
            ("libx11-6", None),
            ("libxcb1", None),
            ("libxext6", None),
            ("libxrandr2", None),
            ("libxcomposite1", None),
            ("libxcursor1", None),
            ("libxdamage1", None),
            ("libxfixes3", None),
            ("libxi6", None),
            ("libgtk-3-0", Some("libgtk-3-0t64")),
            ("libpangocairo-1.0-0", Some("libpangocairo-1.0-0t64")),
            ("libpango-1.0-0", Some("libpango-1.0-0t64")),
            ("libatk1.0-0", Some("libatk1.0-0t64")),
            ("libcairo-gobject2", Some("libcairo-gobject2t64")),
            ("libcairo2", Some("libcairo2t64")),
            ("libgdk-pixbuf-2.0-0", Some("libgdk-pixbuf-2.0-0t64")),
            ("libxrender1", None),
            ("libasound2", Some("libasound2t64")),
            ("libfreetype6", None),
            ("libfontconfig1", None),
            ("libdbus-1-3", Some("libdbus-1-3t64")),
            ("libnss3", None),
            ("libnspr4", None),
            ("libatk-bridge2.0-0", Some("libatk-bridge2.0-0t64")),
            ("libdrm2", None),
            ("libxkbcommon0", None),
            ("libatspi2.0-0", Some("libatspi2.0-0t64")),
            ("libcups2", Some("libcups2t64")),
            ("libxshmfence1", None),
            ("libgbm1", None),
            // Fonts: without actual font files, pages render with missing glyphs
            // (tofu). This is especially visible for CJK and emoji characters.
            ("fonts-noto-color-emoji", None),
            ("fonts-noto-cjk", None),
            ("fonts-freefont-ttf", None),
        ]
        .into_iter()
        .map(|(base, t64_variant)| {
            if let Some(t64) = t64_variant {
                if package_exists_apt(t64) {
                    return t64;
                }
            }
            base
        })
        .collect();

        ("apt-get", apt_deps)
    } else if which_exists("dnf") {
        (
            "dnf",
            vec![
                "nss",
                "nspr",
                "atk",
                "at-spi2-atk",
                "cups-libs",
                "libdrm",
                "libXcomposite",
                "libXdamage",
                "libXrandr",
                "mesa-libgbm",
                "pango",
                "alsa-lib",
                "libxkbcommon",
                "libxcb",
                "libX11-xcb",
                "libX11",
                "libXext",
                "libXcursor",
                "libXfixes",
                "libXi",
                "gtk3",
                "cairo-gobject",
                // Fonts
                "google-noto-cjk-fonts",
                "google-noto-emoji-color-fonts",
                "liberation-fonts",
            ],
        )
    } else if which_exists("yum") {
        (
            "yum",
            vec![
                "nss",
                "nspr",
                "atk",
                "at-spi2-atk",
                "cups-libs",
                "libdrm",
                "libXcomposite",
                "libXdamage",
                "libXrandr",
                "mesa-libgbm",
                "pango",
                "alsa-lib",
                "libxkbcommon",
                // Fonts
                "google-noto-cjk-fonts",
                "liberation-fonts",
            ],
        )
    } else {
        eprintln!(
            "{} No supported package manager found (apt-get, dnf, or yum)",
            color::error_indicator()
        );
        exit(1);
    };

    if pkg_mgr == "apt-get" {
        // Run apt-get update first
        println!("Running: sudo apt-get update");
        let update_status = Command::new("sudo").args(["apt-get", "update"]).status();

        match update_status {
            Ok(s) if !s.success() => {
                eprintln!(
                    "{} apt-get update failed. Continuing with existing package lists.",
                    color::warning_indicator()
                );
            }
            Err(e) => {
                eprintln!(
                    "{} Could not run apt-get update: {}",
                    color::warning_indicator(),
                    e
                );
            }
            _ => {}
        }

        // Simulate the install first to detect if apt would remove any
        // packages. This prevents the catastrophic scenario where installing
        // these libraries triggers removal of hundreds of system packages
        // due to dependency conflicts (e.g. on Ubuntu 24.04 with the
        // t64 transition).
        println!("Checking for conflicts...");
        let sim_output = Command::new("sudo")
            .args(["apt-get", "install", "--simulate"])
            .args(&deps)
            .output();

        match sim_output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let combined = format!("{}\n{}", stdout, stderr);

                // Count packages that would be removed
                let removals: Vec<&str> = combined
                    .lines()
                    .filter(|line| line.starts_with("Remv "))
                    .collect();

                if !removals.is_empty() {
                    eprintln!(
                        "{} Aborting: apt would remove {} package(s) to install these dependencies.",
                        color::error_indicator(),
                        removals.len()
                    );
                    eprintln!(
                        "  This usually means some package names have changed on your system"
                    );
                    eprintln!("  (e.g. Ubuntu 24.04 renamed libraries with a t64 suffix).");
                    eprintln!();
                    eprintln!("  Packages that would be removed:");
                    for line in removals.iter().take(20) {
                        eprintln!("    {}", line);
                    }
                    if removals.len() > 20 {
                        eprintln!("    ... and {} more", removals.len() - 20);
                    }
                    eprintln!();
                    eprintln!("  To install dependencies manually, run:");
                    eprintln!("    sudo apt-get install {}", deps.join(" "));
                    eprintln!();
                    eprintln!("  Review the apt output carefully before confirming.");
                    exit(1);
                }
            }
            Err(e) => {
                eprintln!(
                    "{} Could not simulate install ({}). Proceeding with caution.",
                    color::warning_indicator(),
                    e
                );
            }
        }

        // Safe to proceed: no removals detected
        let install_cmd = format!("sudo apt-get install -y {}", deps.join(" "));
        println!("Running: {}", install_cmd);
        let status = Command::new("sudo")
            .args(["apt-get", "install", "-y"])
            .args(&deps)
            .status();

        report_install_status(status);
    } else {
        // dnf / yum path — these package managers do not remove packages
        // during install, so the simulate-first guard is not needed.
        let install_cmd = format!("sudo {} install -y {}", pkg_mgr, deps.join(" "));
        println!("Running: {}", install_cmd);
        let status = Command::new("sh").arg("-c").arg(&install_cmd).status();

        report_install_status(status);
    }
}

fn install_remote_view_privileges() {
    println!(
        "{}",
        color::cyan("Installing remote-view privilege helper...")
    );

    if !which_exists("bash") {
        eprintln!(
            "{} bash is required to install remote-view privileges.",
            color::error_indicator()
        );
        exit(1);
    }

    let temp_root =
        std::env::temp_dir().join(format!("agent-browser-privileges-{}", std::process::id()));
    if temp_root.exists() {
        let _ = fs::remove_dir_all(&temp_root);
    }

    let script_dir = temp_root.join("scripts");
    let helper_dir = script_dir.join("libexec");
    let installer_path = script_dir.join("install-agent-browser-privileges.sh");
    let helper_path = helper_dir.join("agent-browser-privileged-helper");

    if let Err(err) = fs::create_dir_all(&helper_dir)
        .and_then(|()| fs::write(&installer_path, REMOTE_VIEW_PRIVILEGE_INSTALLER))
        .and_then(|()| fs::write(&helper_path, REMOTE_VIEW_PRIVILEGED_HELPER))
    {
        eprintln!(
            "{} Failed to prepare remote-view privilege installer: {}",
            color::error_indicator(),
            err
        );
        let _ = fs::remove_dir_all(&temp_root);
        exit(1);
    }

    let status = Command::new("bash")
        .arg(&installer_path)
        .arg("--apply")
        .env("AGENT_BROWSER_PRIVILEGED_HELPER_SOURCE", &helper_path)
        .status();

    let _ = fs::remove_dir_all(&temp_root);

    match status {
        Ok(s) if s.success() => {
            println!(
                "{} Remote-view privilege helper installed",
                color::success_indicator()
            );
        }
        Ok(s) => {
            eprintln!(
                "{} Remote-view privilege helper install failed with status {}",
                color::error_indicator(),
                s
            );
            exit(s.code().unwrap_or(1));
        }
        Err(err) => {
            eprintln!(
                "{} Could not run remote-view privilege installer: {}",
                color::error_indicator(),
                err
            );
            exit(1);
        }
    }
}

fn which_exists(cmd: &str) -> bool {
    #[cfg(unix)]
    {
        Command::new("which")
            .arg(cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        Command::new("where")
            .arg(cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

fn package_exists_apt(pkg: &str) -> bool {
    Command::new("apt-cache")
        .arg("show")
        .arg(pkg)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::EnvGuard;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use zip::write::SimpleFileOptions;

    fn http_response(status: u16, reason: &str, body: &[u8]) -> Vec<u8> {
        let header = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            status,
            reason,
            body.len()
        );
        let mut resp = header.into_bytes();
        resp.extend_from_slice(body);
        resp
    }

    async fn accept_once(listener: &TcpListener, response: &[u8]) {
        let (mut s, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 4096];
        let _ = s.read(&mut buf).await;
        s.write_all(response).await.unwrap();
    }

    async fn accept_with_ua_check(listener: &TcpListener, response: &[u8]) -> String {
        let (mut s, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 4096];
        let n = s.read(&mut buf).await.unwrap();
        let request = String::from_utf8_lossy(&buf[..n]).to_string();
        s.write_all(response).await.unwrap();
        request
    }

    fn fingerprint(path: Option<&str>, sha256: Option<&str>) -> serde_json::Value {
        json!({
            "path": path,
            "exists": path.is_some(),
            "sha256": sha256,
            "sizeBytes": 1,
        })
    }

    fn issue_codes(issues: Vec<serde_json::Value>) -> Vec<String> {
        issues
            .into_iter()
            .filter_map(|issue| {
                issue
                    .get("code")
                    .and_then(|value| value.as_str())
                    .map(ToString::to_string)
            })
            .collect()
    }

    fn empty_runtime_inventory() -> serde_json::Value {
        json!({
            "schemaVersion": "agent-browser.runtime-inventory.v1",
            "status": "none",
            "runtimeCount": 0,
            "convergedCount": 0,
            "staleCount": 0,
            "runtimes": [],
        })
    }

    fn ready_live_dashboard_runtime() -> serde_json::Value {
        json!({
            "available": false,
            "ready": true,
            "state": "not_running"
        })
    }

    fn test_zip_with_chrome(chrome_bytes: &[u8], smoke: Option<&[u8]>) -> Vec<u8> {
        let mut cursor = io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut cursor);
            let options = SimpleFileOptions::default();
            writer
                .start_file("chrome-win64/chrome.exe", options)
                .unwrap();
            writer.write_all(chrome_bytes).unwrap();
            if let Some(smoke) = smoke {
                writer
                    .start_file("chrome-win64/smoke.json", options)
                    .unwrap();
                writer.write_all(smoke).unwrap();
            }
            writer.finish().unwrap();
        }
        cursor.into_inner()
    }

    #[test]
    fn install_doctor_dashboard_runtime_manifest_shape_stays_public_safe() {
        let runtime = runtime_manifest_json();

        assert_eq!(
            runtime["schemaVersion"],
            "agent-browser.runtime-manifest.v1"
        );
        assert_eq!(runtime["serviceContractVersion"], "service-ui-runtime.v1");
        assert!(runtime["dashboard"]["assetCount"].as_u64().unwrap_or(0) > 0);
        assert!(runtime["dashboard"]["sha256"]
            .as_str()
            .is_some_and(|value| value.len() == 64));
        assert!(runtime["supportedUiFeatures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|feature| feature == "workspace.detectedBrowsers"));
        assert!(runtime["supportedUiFeatures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|feature| feature == "workspace.noRetainedLiveRail"));
    }

    #[test]
    fn install_doctor_flags_path_and_pnpm_binary_mismatch() {
        let launch_config = json!({
            "stealthCdpChromiumRequired": false,
            "stealthCdpChromiumReady": true,
        });

        let current_executable = fingerprint(Some("/current/agent-browser"), Some("aaa"));
        let path_command = fingerprint(Some("/path/agent-browser"), Some("bbb"));
        let pnpm_package_binary = fingerprint(Some("/pnpm/agent-browser"), Some("ccc"));
        let workspace_binary = fingerprint(None, None);
        let service = json!({"ready": true});
        let service_resources = json!({"readinessImpactingCandidates": 0});
        let live_dashboard_runtime = ready_live_dashboard_runtime();
        let runtime_inventory = empty_runtime_inventory();
        let issues = install_doctor_issues(InstallDoctorIssueInputs {
            current_executable: &current_executable,
            path_command: &path_command,
            pnpm_package_binary: &pnpm_package_binary,
            workspace_binary: &workspace_binary,
            launch_config: &launch_config,
            service: &service,
            service_resources: &service_resources,
            live_dashboard_runtime: &live_dashboard_runtime,
            runtime_inventory: &runtime_inventory,
        });

        assert_eq!(
            issue_codes(issues),
            vec![
                "current_executable_path_command_mismatch",
                "path_command_pnpm_binary_mismatch"
            ]
        );
    }

    #[test]
    fn install_doctor_flags_launch_config_not_ready() {
        let launch_config = json!({
            "stealthCdpChromiumRequired": true,
            "stealthCdpChromiumReady": false,
        });

        let current_executable = fingerprint(Some("/same/agent-browser"), Some("aaa"));
        let path_command = fingerprint(Some("/same/agent-browser"), Some("aaa"));
        let pnpm_package_binary = fingerprint(Some("/pnpm/agent-browser"), Some("aaa"));
        let workspace_binary = fingerprint(Some("/workspace/agent-browser"), Some("aaa"));
        let service = json!({"ready": true});
        let service_resources = json!({"readinessImpactingCandidates": 0});
        let live_dashboard_runtime = ready_live_dashboard_runtime();
        let runtime_inventory = empty_runtime_inventory();
        let issues = install_doctor_issues(InstallDoctorIssueInputs {
            current_executable: &current_executable,
            path_command: &path_command,
            pnpm_package_binary: &pnpm_package_binary,
            workspace_binary: &workspace_binary,
            launch_config: &launch_config,
            service: &service,
            service_resources: &service_resources,
            live_dashboard_runtime: &live_dashboard_runtime,
            runtime_inventory: &runtime_inventory,
        });

        assert_eq!(issue_codes(issues), vec!["launch_config_not_ready"]);
    }

    #[test]
    fn install_doctor_flags_service_status_not_ready() {
        let launch_config = json!({
            "stealthCdpChromiumRequired": false,
            "stealthCdpChromiumReady": true,
        });

        let current_executable = fingerprint(Some("/same/agent-browser"), Some("aaa"));
        let path_command = fingerprint(Some("/same/agent-browser"), Some("aaa"));
        let pnpm_package_binary = fingerprint(Some("/pnpm/agent-browser"), Some("aaa"));
        let workspace_binary = fingerprint(Some("/workspace/agent-browser"), Some("aaa"));
        let service = json!({"ready": false});
        let service_resources = json!({"readinessImpactingCandidates": 0});
        let live_dashboard_runtime = ready_live_dashboard_runtime();
        let runtime_inventory = empty_runtime_inventory();
        let issues = install_doctor_issues(InstallDoctorIssueInputs {
            current_executable: &current_executable,
            path_command: &path_command,
            pnpm_package_binary: &pnpm_package_binary,
            workspace_binary: &workspace_binary,
            launch_config: &launch_config,
            service: &service,
            service_resources: &service_resources,
            live_dashboard_runtime: &live_dashboard_runtime,
            runtime_inventory: &runtime_inventory,
        });

        assert_eq!(issue_codes(issues), vec!["service_status_not_ready"]);
    }

    #[test]
    fn install_doctor_flags_readiness_impacting_resource_candidates() {
        let launch_config = json!({
            "stealthCdpChromiumRequired": false,
            "stealthCdpChromiumReady": true,
        });

        let current_executable = fingerprint(Some("/same/agent-browser"), Some("aaa"));
        let path_command = fingerprint(Some("/same/agent-browser"), Some("aaa"));
        let pnpm_package_binary = fingerprint(Some("/pnpm/agent-browser"), Some("aaa"));
        let workspace_binary = fingerprint(Some("/workspace/agent-browser"), Some("aaa"));
        let service = json!({"ready": true});
        let service_resources = json!({"readinessImpactingCandidates": 1});
        let live_dashboard_runtime = ready_live_dashboard_runtime();
        let runtime_inventory = empty_runtime_inventory();
        let issues = install_doctor_issues(InstallDoctorIssueInputs {
            current_executable: &current_executable,
            path_command: &path_command,
            pnpm_package_binary: &pnpm_package_binary,
            workspace_binary: &workspace_binary,
            launch_config: &launch_config,
            service: &service,
            service_resources: &service_resources,
            live_dashboard_runtime: &live_dashboard_runtime,
            runtime_inventory: &runtime_inventory,
        });

        assert_eq!(
            issue_codes(issues),
            vec!["service_resource_candidates_ready"]
        );
    }

    #[test]
    fn install_doctor_flags_duplicate_profile_pressure() {
        let launch_config = json!({
            "stealthCdpChromiumRequired": false,
            "stealthCdpChromiumReady": true,
        });

        let current_executable = fingerprint(Some("/same/agent-browser"), Some("aaa"));
        let path_command = fingerprint(Some("/same/agent-browser"), Some("aaa"));
        let pnpm_package_binary = fingerprint(Some("/pnpm/agent-browser"), Some("aaa"));
        let workspace_binary = fingerprint(Some("/workspace/agent-browser"), Some("aaa"));
        let service = json!({"ready": true});
        let service_resources = json!({
            "readinessImpactingCandidates": 0,
            "duplicateProfilePressureWarnings": 1
        });
        let live_dashboard_runtime = ready_live_dashboard_runtime();
        let runtime_inventory = empty_runtime_inventory();
        let issues = install_doctor_issues(InstallDoctorIssueInputs {
            current_executable: &current_executable,
            path_command: &path_command,
            pnpm_package_binary: &pnpm_package_binary,
            workspace_binary: &workspace_binary,
            launch_config: &launch_config,
            service: &service,
            service_resources: &service_resources,
            live_dashboard_runtime: &live_dashboard_runtime,
            runtime_inventory: &runtime_inventory,
        });

        assert_eq!(
            issue_codes(issues),
            vec!["service_duplicate_profile_pressure"]
        );
    }

    #[test]
    fn install_doctor_flags_stale_live_dashboard_runtime() {
        let launch_config = json!({
            "stealthCdpChromiumRequired": false,
            "stealthCdpChromiumReady": true,
        });

        let current_executable = fingerprint(Some("/same/agent-browser"), Some("aaa"));
        let path_command = fingerprint(Some("/same/agent-browser"), Some("aaa"));
        let pnpm_package_binary = fingerprint(Some("/pnpm/agent-browser"), Some("aaa"));
        let workspace_binary = fingerprint(Some("/workspace/agent-browser"), Some("aaa"));
        let service = json!({"ready": true});
        let service_resources = json!({"readinessImpactingCandidates": 0});
        let live_dashboard_runtime = json!({
            "available": true,
            "ready": false,
            "state": "stale_executable",
            "url": "http://127.0.0.1:4848/api/runtime/manifest",
            "executable": {
                "sha256": "bbb",
                "matchesExpected": false
            }
        });
        let runtime_inventory = empty_runtime_inventory();
        let issue = install_doctor_issues(InstallDoctorIssueInputs {
            current_executable: &current_executable,
            path_command: &path_command,
            pnpm_package_binary: &pnpm_package_binary,
            workspace_binary: &workspace_binary,
            launch_config: &launch_config,
            service: &service,
            service_resources: &service_resources,
            live_dashboard_runtime: &live_dashboard_runtime,
            runtime_inventory: &runtime_inventory,
        })
        .remove(0);

        assert_eq!(issue["code"], "dashboard_runtime_stale_or_unreadable");
        assert_eq!(issue["nextAction"], "converge_local_runtime");
        assert_eq!(
            issue["remedy"]["argv"],
            json!(["pnpm", "converge:local-runtime", "--", "--apply", "--json"])
        );
        assert_eq!(issue["remedy"]["requiresInteractiveSudo"], false);
    }

    #[test]
    fn runtime_convergence_summary_reports_expected_states() {
        let ready_dashboard = json!({
            "available": true,
            "ready": true,
            "state": "ready"
        });
        let stale_dashboard = json!({
            "available": true,
            "ready": false,
            "state": "stale_executable"
        });
        let unreadable_dashboard = json!({
            "available": true,
            "ready": false,
            "state": "unreadable_manifest"
        });
        let diagnostic_inventory = json!({
            "staleCount": 0,
            "runtimes": [{"state": "diagnostic"}]
        });
        let stale_inventory = json!({
            "staleCount": 1,
            "runtimes": [{"state": "stale"}]
        });
        let empty_inventory = empty_runtime_inventory();

        assert_eq!(
            runtime_convergence_summary(&ready_dashboard, &empty_inventory)["status"],
            "converged"
        );
        assert_eq!(
            runtime_convergence_summary(&ready_dashboard, &diagnostic_inventory)["status"],
            "partial"
        );
        assert_eq!(
            runtime_convergence_summary(&ready_dashboard, &stale_inventory)["status"],
            "stale"
        );
        assert_eq!(
            runtime_convergence_summary(&stale_dashboard, &empty_inventory)["status"],
            "stale"
        );
        assert_eq!(
            runtime_convergence_summary(&unreadable_dashboard, &empty_inventory)["status"],
            "manual_review_required"
        );
    }

    #[test]
    fn install_doctor_flags_stale_runtime_inventory() {
        let launch_config = json!({
            "stealthCdpChromiumRequired": false,
            "stealthCdpChromiumReady": true,
        });

        let current_executable = fingerprint(Some("/same/agent-browser"), Some("aaa"));
        let path_command = fingerprint(Some("/same/agent-browser"), Some("aaa"));
        let pnpm_package_binary = fingerprint(Some("/pnpm/agent-browser"), Some("aaa"));
        let workspace_binary = fingerprint(Some("/workspace/agent-browser"), Some("aaa"));
        let service = json!({"ready": true});
        let service_resources = json!({"readinessImpactingCandidates": 0});
        let live_dashboard_runtime = ready_live_dashboard_runtime();
        let runtime_inventory = json!({
            "schemaVersion": "agent-browser.runtime-inventory.v1",
            "status": "stale",
            "runtimes": [
                {
                    "kind": "daemon_session",
                    "session": "default",
                    "state": "stale"
                }
            ]
        });
        let issues = install_doctor_issues(InstallDoctorIssueInputs {
            current_executable: &current_executable,
            path_command: &path_command,
            pnpm_package_binary: &pnpm_package_binary,
            workspace_binary: &workspace_binary,
            launch_config: &launch_config,
            service: &service,
            service_resources: &service_resources,
            live_dashboard_runtime: &live_dashboard_runtime,
            runtime_inventory: &runtime_inventory,
        });

        assert_eq!(issue_codes(issues), vec!["active_runtime_stale_executable"]);
        let issue = install_doctor_issues(InstallDoctorIssueInputs {
            current_executable: &current_executable,
            path_command: &path_command,
            pnpm_package_binary: &pnpm_package_binary,
            workspace_binary: &workspace_binary,
            launch_config: &launch_config,
            service: &service,
            service_resources: &service_resources,
            live_dashboard_runtime: &live_dashboard_runtime,
            runtime_inventory: &runtime_inventory,
        })
        .remove(0);
        assert_eq!(issue["session"], "default");
        assert_eq!(issue["nextAction"], "restart_stale_daemon_session");
        assert_eq!(
            issue["remedy"]["argv"],
            json!(["agent-browser", "close", "--session", "default"])
        );
        assert_eq!(issue["remedy"]["requiresInteractiveSudo"], false);
    }

    #[test]
    fn active_runtime_inventory_classifies_unaddressable_pid_as_diagnostic() {
        let dir = std::env::temp_dir().join(format!(
            "agent-browser-runtime-inventory-diagnostic-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        fs::create_dir_all(&dir).unwrap();
        let guard = EnvGuard::new(&["AGENT_BROWSER_SOCKET_DIR"]);
        guard.set("AGENT_BROWSER_SOCKET_DIR", dir.to_str().unwrap());
        fs::write(dir.join("probe.pid"), std::process::id().to_string()).unwrap();
        fs::write(dir.join("probe.version"), env!("CARGO_PKG_VERSION")).unwrap();

        let inventory = active_runtime_inventory(Some("expected"));

        assert_eq!(inventory["staleCount"], 0);
        assert_eq!(inventory["runtimeCount"], 1);
        assert_eq!(inventory["runtimes"][0]["session"], "probe");
        assert_eq!(inventory["runtimes"][0]["state"], "diagnostic");
        assert_eq!(inventory["runtimes"][0]["metadata"]["addressable"], false);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn install_orders_remote_view_privileges_before_linux_deps() {
        let source = include_str!("install.rs");
        let run_install_start = source
            .find("pub fn run_install")
            .expect("run_install should exist");
        let run_install_source = &source[run_install_start..];
        let privileges_pos = run_install_source
            .find("install_remote_view_privileges();")
            .expect("run_install should call install_remote_view_privileges");
        let deps_pos = run_install_source
            .find("install_linux_deps();")
            .expect("run_install should call install_linux_deps");

        assert!(
            privileges_pos < deps_pos,
            "remote-view privileges must establish the sudo boundary before dependency installation"
        );
    }

    #[test]
    fn install_stealthcdp_chromium_archive_writes_current_manifest_and_preserves_smoke() {
        let dir = std::env::temp_dir().join(format!(
            "agent-browser-stealthcdp-install-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        let root = dir.join("install");
        let chrome = b"test chrome exe";
        let chrome_sha = sha256_bytes(chrome);
        let smoke = br#"{"success":true,"checks":{"navigatorWebdriver":"false"}}"#;
        let archive = test_zip_with_chrome(chrome, Some(smoke));
        let archive_sha = sha256_bytes(&archive);

        let current = install_stealthcdp_chromium_archive_bytes(
            archive,
            &root,
            false,
            &archive_sha,
            &chrome_sha,
        )
        .unwrap();

        assert!(current.join("chrome.exe").is_file());
        assert!(current.join("manifest.json").is_file());
        let manifest: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(current.join("manifest.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["schema"], "chromium-stealthcdp.artifact.v1");
        assert_eq!(manifest["executable"]["relativePath"], "chrome.exe");
        assert_eq!(fs::read(current.join("smoke.json")).unwrap(), smoke);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn install_stealthcdp_chromium_archive_rejects_sha_mismatch() {
        let dir = std::env::temp_dir().join(format!(
            "agent-browser-stealthcdp-install-sha-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        let root = dir.join("install");
        let archive = test_zip_with_chrome(b"test chrome exe", None);

        let err = install_stealthcdp_chromium_archive_bytes(
            archive,
            &root,
            false,
            "not-the-archive-sha",
            "not-the-exe-sha",
        )
        .unwrap_err();

        assert!(err.contains("downloaded archive sha256 mismatch"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn flatten_stealthcdp_artifact_root_moves_packaged_release_contents_up() {
        let dir = std::env::temp_dir().join(format!(
            "agent-browser-stealthcdp-flatten-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        let nested = dir.join("chromium-stealthcdp");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("chrome.exe"), "chrome").unwrap();
        fs::write(nested.join("smoke-win.json"), "{}").unwrap();

        flatten_stealthcdp_artifact_root(&dir).unwrap();

        assert!(dir.join("chrome.exe").is_file());
        assert!(dir.join("smoke-win.json").is_file());
        assert!(!nested.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn stealthcdp_chromium_install_root_honors_env_override() {
        let dir = std::env::temp_dir().join(format!(
            "agent-browser-stealthcdp-install-root-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        let guard = EnvGuard::new(&["AGENT_BROWSER_STEALTHCDP_CHROMIUM_INSTALL_ROOT"]);
        guard.set(
            "AGENT_BROWSER_STEALTHCDP_CHROMIUM_INSTALL_ROOT",
            dir.to_str().unwrap(),
        );

        assert_eq!(stealthcdp_chromium_install_root().unwrap(), dir);
    }

    #[tokio::test]
    async fn download_bytes_returns_body_on_200() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let body = b"fake-zip-content";
        let resp = http_response(200, "OK", body);

        let server = tokio::spawn(async move {
            accept_once(&listener, &resp).await;
        });

        let url = format!("http://127.0.0.1:{}/test.zip", port);
        let result = download_bytes_with_retry_backoff(&url, false).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), body);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn download_bytes_returns_error_on_404() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let resp = http_response(404, "Not Found", b"not found");

        let server = tokio::spawn(async move {
            accept_once(&listener, &resp).await;
        });

        let url = format!("http://127.0.0.1:{}/test.zip", port);
        let result = download_bytes_with_retry_backoff(&url, false).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("HTTP 404"),
            "expected HTTP 404 in error, got: {}",
            err
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn download_bytes_retries_on_500() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = tokio::spawn(async move {
            // First two attempts: 500
            let r500 = http_response(500, "Internal Server Error", b"error");
            accept_once(&listener, &r500).await;
            accept_once(&listener, &r500).await;
            // Third attempt: 200
            let r200 = http_response(200, "OK", b"ok-data");
            accept_once(&listener, &r200).await;
        });

        let url = format!("http://127.0.0.1:{}/test.zip", port);
        let result = download_bytes_with_retry_backoff(&url, false).await;
        assert!(
            result.is_ok(),
            "expected success after retries: {:?}",
            result
        );
        assert_eq!(result.unwrap(), b"ok-data");
        server.await.unwrap();
    }

    #[tokio::test]
    async fn download_bytes_gives_up_after_max_retries() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = tokio::spawn(async move {
            let r500 = http_response(500, "Internal Server Error", b"error");
            // All 3 attempts get 500
            accept_once(&listener, &r500).await;
            accept_once(&listener, &r500).await;
            accept_once(&listener, &r500).await;
        });

        let url = format!("http://127.0.0.1:{}/test.zip", port);
        let result = download_bytes_with_retry_backoff(&url, false).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("HTTP 500"),
            "expected HTTP 500 in error, got: {}",
            err
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn download_bytes_does_not_retry_on_403() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let resp = http_response(403, "Forbidden", b"forbidden");

        let server = tokio::spawn(async move {
            // Only one request should arrive (no retries for 4xx)
            accept_once(&listener, &resp).await;
        });

        let url = format!("http://127.0.0.1:{}/test.zip", port);
        let result = download_bytes_with_retry_backoff(&url, false).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("HTTP 403"));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn http_client_sends_user_agent() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let resp = http_response(200, "OK", b"ok");

        let server = tokio::spawn(async move {
            let req = accept_with_ua_check(&listener, &resp).await;
            req
        });

        let client = http_client().unwrap();
        let url = format!("http://127.0.0.1:{}/test", port);
        let _ = client.get(&url).send().await;
        let request_text = server.await.unwrap();
        let expected_ua = format!("agent-browser/{}", env!("CARGO_PKG_VERSION"));
        assert!(
            request_text.contains(&expected_ua),
            "expected User-Agent '{}' in request:\n{}",
            expected_ua,
            request_text
        );
    }

    #[test]
    fn download_bytes_connection_refused_includes_details() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let url = rt.block_on(async {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            drop(listener);
            format!("http://127.0.0.1:{}/test.zip", port)
        });
        let result = rt.block_on(download_bytes_with_retry_backoff(&url, false));
        assert!(result.is_err());
        let err = result.unwrap_err();
        // The new code should include a network-level connection failure cause
        // rather than just the vague "error sending request for url".
        assert!(
            err.contains("Connection refused")
                || err.contains("connection refused")
                || err.contains("actively refused it")
                || err.contains("timed out")
                || err.contains("deadline has elapsed")
                || err.contains("timed out while waiting"),
            "expected a connection failure in error, got: {}",
            err
        );
    }
}
