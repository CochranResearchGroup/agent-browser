use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Validate the stable name accepted for a managed runtime profile.
pub fn validate_runtime_profile_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Runtime profile name cannot be empty".to_string());
    }
    if !name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(format!(
            "Invalid runtime profile '{}'. Must match /^[a-zA-Z0-9_-]+$/",
            name
        ));
    }
    Ok(())
}

/// Resolve the canonical writable-profile identity without exposing its path.
///
/// Existing symlinks are resolved. A not-yet-created leaf is anchored to its
/// nearest existing canonical ancestor. Windows identities are case-folded
/// before the versioned digest is computed.
pub fn canonical_profile_identity_digest(path: &Path) -> Result<String, String> {
    if path.as_os_str().is_empty() {
        return Err("profile identity path is empty".to_string());
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("Could not resolve profile identity base: {error}"))?
            .join(path)
    };
    let mut existing = absolute.as_path();
    let mut missing = Vec::new();
    let canonical_base = loop {
        match fs::canonicalize(existing) {
            Ok(canonical) => break canonical,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let name = existing.file_name().ok_or_else(|| {
                    "profile identity has no canonical existing ancestor".to_string()
                })?;
                missing.push(name.to_os_string());
                existing = existing.parent().ok_or_else(|| {
                    "profile identity has no canonical existing ancestor".to_string()
                })?;
            }
            Err(error) => {
                return Err(format!("Could not canonicalize profile identity: {error}"));
            }
        }
    };
    let mut canonical = canonical_base;
    for component in missing.iter().rev() {
        canonical.push(component);
    }
    let identity = canonical.to_string_lossy().to_string();
    #[cfg(windows)]
    let identity = identity.to_lowercase();
    Ok(format!(
        "{:x}",
        Sha256::digest(format!("agent-browser.profile-identity.v1\n{identity}").as_bytes())
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_names_preserve_the_frozen_contract() {
        assert!(validate_runtime_profile_name("default").is_ok());
        assert!(validate_runtime_profile_name("work_2").is_ok());
        assert!(validate_runtime_profile_name("bad/name").is_err());
        assert!(validate_runtime_profile_name("").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn canonical_identity_resolves_symlinks_and_missing_leaves() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!(
            "agent-browser-profile-identity-test-{}",
            uuid::Uuid::new_v4()
        ));
        let canonical_root = root.join("canonical");
        let alias_root = root.join("alias");
        fs::create_dir_all(&canonical_root).unwrap();
        symlink(&canonical_root, &alias_root).unwrap();

        let canonical =
            canonical_profile_identity_digest(&canonical_root.join("user-data")).unwrap();
        let alias = canonical_profile_identity_digest(&alias_root.join("user-data")).unwrap();
        assert_eq!(canonical, alias);
        assert_eq!(canonical.len(), 64);

        fs::remove_dir_all(root).unwrap();
    }
}
