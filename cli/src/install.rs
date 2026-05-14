use crate::color;
use crate::flags::{launch_config_status, Flags};
use serde_json::json;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{exit, Command, Stdio};

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

pub fn run_install(with_deps: bool) {
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
    let issues = install_doctor_issues(
        &current_executable,
        &path_command,
        &pnpm_package_binary,
        &workspace_binary,
        &launch_config,
    );

    json!({
        "success": issues.is_empty(),
        "data": {
            "version": env!("CARGO_PKG_VERSION"),
            "currentExecutable": current_executable,
            "pathCommand": path_command,
            "pnpmPackageBinary": pnpm_package_binary,
            "workspaceBinary": workspace_binary,
            "launchConfig": launch_config,
            "issues": issues,
        }
    })
}

fn install_doctor_issues(
    current_executable: &serde_json::Value,
    path_command: &serde_json::Value,
    pnpm_package_binary: &serde_json::Value,
    workspace_binary: &serde_json::Value,
    launch_config: &serde_json::Value,
) -> Vec<serde_json::Value> {
    let mut issues = Vec::new();

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

    issues
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
    fn install_doctor_flags_path_and_pnpm_binary_mismatch() {
        let launch_config = json!({
            "stealthCdpChromiumRequired": false,
            "stealthCdpChromiumReady": true,
        });

        let issues = install_doctor_issues(
            &fingerprint(Some("/current/agent-browser"), Some("aaa")),
            &fingerprint(Some("/path/agent-browser"), Some("bbb")),
            &fingerprint(Some("/pnpm/agent-browser"), Some("ccc")),
            &fingerprint(None, None),
            &launch_config,
        );

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

        let issues = install_doctor_issues(
            &fingerprint(Some("/same/agent-browser"), Some("aaa")),
            &fingerprint(Some("/same/agent-browser"), Some("aaa")),
            &fingerprint(Some("/pnpm/agent-browser"), Some("aaa")),
            &fingerprint(Some("/workspace/agent-browser"), Some("aaa")),
            &launch_config,
        );

        assert_eq!(issue_codes(issues), vec!["launch_config_not_ready"]);
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
