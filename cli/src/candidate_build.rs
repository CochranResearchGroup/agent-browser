//! Thin CLI bridge to the repository candidate build orchestrator.
//!
//! The repository script owns build claims, Cargo execution, and immutable
//! artifact sealing. This module forwards exact operator arguments and does
//! not interact with either installed runtime.

use std::path::{Path, PathBuf};
use std::process::Command;

fn candidate_build_invocation(
    args: &[String],
    json_output: bool,
) -> Result<(PathBuf, Vec<String>), String> {
    let mut repo_root = None;
    let mut index = 2;
    while index < args.len() {
        if args[index] == "--repo-root" {
            repo_root = args.get(index + 1).map(PathBuf::from);
            break;
        }
        index += 1;
    }
    let repo_root = repo_root
        .ok_or_else(|| "candidate build requires --repo-root <source-checkout>".to_string())?;
    let script = repo_root.join("scripts/candidate-build.js");
    if !script.is_file() {
        return Err(format!(
            "candidate build script is missing: {}",
            script.display()
        ));
    }
    let mut forwarded = args.iter().skip(2).cloned().collect::<Vec<_>>();
    if json_output && !forwarded.iter().any(|argument| argument == "--json") {
        forwarded.push("--json".to_string());
    }
    Ok((script, forwarded))
}

pub(crate) fn run_candidate_build(args: &[String], json_output: bool) -> ! {
    let (script, forwarded) =
        candidate_build_invocation(args, json_output).unwrap_or_else(|error| {
            eprintln!("Candidate build failed: {error}");
            std::process::exit(1);
        });
    let status = Command::new("node")
        .arg(&script)
        .args(&forwarded)
        .current_dir(
            script
                .parent()
                .and_then(Path::parent)
                .unwrap_or(Path::new(".")),
        )
        .status()
        .unwrap_or_else(|error| {
            eprintln!("Candidate build failed to start: {error}");
            std::process::exit(1);
        });
    std::process::exit(status.code().unwrap_or(1));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn build_bridge_requires_repo_and_preserves_exact_arguments() {
        let root = std::env::temp_dir().join(format!(
            "agent-browser-candidate-build-bridge-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("scripts")).unwrap();
        fs::write(root.join("scripts/candidate-build.js"), "fixture").unwrap();
        let args = vec![
            "candidate".to_string(),
            "build".to_string(),
            "--repo-root".to_string(),
            root.display().to_string(),
            "--artifact-class".to_string(),
            "fast_iteration".to_string(),
            "--dry-run".to_string(),
        ];
        let (script, forwarded) = candidate_build_invocation(&args, true).unwrap();
        assert_eq!(script, root.join("scripts/candidate-build.js"));
        assert_eq!(forwarded.last().map(String::as_str), Some("--json"));
        assert!(forwarded
            .windows(2)
            .any(|values| values == ["--artifact-class", "fast_iteration"]));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn build_bridge_rejects_missing_repository_root() {
        let error = candidate_build_invocation(
            &[
                "candidate".to_string(),
                "build".to_string(),
                "--dry-run".to_string(),
            ],
            false,
        )
        .unwrap_err();
        assert!(error.contains("--repo-root"));
    }
}
