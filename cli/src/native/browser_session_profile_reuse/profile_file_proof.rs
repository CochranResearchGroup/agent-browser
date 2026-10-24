use std::path::Path;

pub(crate) fn qualify_profile_files(
    expected: &Path,
    files: Vec<std::path::PathBuf>,
    delegated: impl FnOnce() -> Result<bool, String>,
) -> Result<bool, String> {
    if files.is_empty() {
        return delegated()
            .map_err(|_| "browser_session_profile_process_identity_unavailable".into());
    }
    let profile_files = [
        "Default/History",
        "Default/Web Data",
        "Default/Network/Cookies",
    ];
    Ok(files.iter().any(|file| {
        profile_files
            .iter()
            .any(|relative| file == &expected.join(relative))
    }))
}

#[cfg(test)]
mod namespace_profile_tests {
    use super::*;

    #[test]
    fn unavailable_or_negative_delegated_proof_cannot_authorize_profile_reuse() {
        let expected = Path::new("/private/profile");
        assert_eq!(
            qualify_profile_files(expected, Vec::new(), || Ok(false)),
            Ok(false)
        );
        assert!(qualify_profile_files(expected, Vec::new(), || Err("unavailable".into())).is_err());
        assert_eq!(
            qualify_profile_files(
                expected,
                vec!["/private/profile/uploads/History".into()],
                || panic!("Known unrelated descriptors must not acquire custody")
            ),
            Ok(false)
        );
    }

    #[test]
    fn exact_delegated_profile_proof_recovers_permission_denied_descriptor_observation() {
        // EACCES readlinks produce an empty local descriptor-target collection.
        let result = qualify_profile_files(Path::new("/private/profile"), Vec::new(), || Ok(true));
        assert_eq!(result, Ok(true));
    }
}
