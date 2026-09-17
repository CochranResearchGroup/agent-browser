use serde::{Deserialize, Serialize};

/// Available recourse for preserving an authenticated principal's continuity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalContinuityRecourse {
    ContinueWithActiveClaim,
    ContinueWithSelfDeclaredAccess,
    RejoinOwnedBrowser,
    ReplaceStaleSamePrincipalSession,
    WaitForForeignPrincipal,
    ReconcilePrincipalIdentity,
}

impl PrincipalContinuityRecourse {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ContinueWithActiveClaim => "continue_with_active_claim",
            Self::ContinueWithSelfDeclaredAccess => "continue_with_self_declared_access",
            Self::RejoinOwnedBrowser => "rejoin_owned_browser",
            Self::ReplaceStaleSamePrincipalSession => "replace_stale_same_principal_session",
            Self::WaitForForeignPrincipal => "wait_for_foreign_principal",
            Self::ReconcilePrincipalIdentity => "reconcile_principal_identity",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PrincipalContinuityRecourse;

    const CASES: &[(PrincipalContinuityRecourse, &str)] = &[
        (
            PrincipalContinuityRecourse::ContinueWithActiveClaim,
            "continue_with_active_claim",
        ),
        (
            PrincipalContinuityRecourse::ContinueWithSelfDeclaredAccess,
            "continue_with_self_declared_access",
        ),
        (
            PrincipalContinuityRecourse::RejoinOwnedBrowser,
            "rejoin_owned_browser",
        ),
        (
            PrincipalContinuityRecourse::ReplaceStaleSamePrincipalSession,
            "replace_stale_same_principal_session",
        ),
        (
            PrincipalContinuityRecourse::WaitForForeignPrincipal,
            "wait_for_foreign_principal",
        ),
        (
            PrincipalContinuityRecourse::ReconcilePrincipalIdentity,
            "reconcile_principal_identity",
        ),
    ];

    #[test]
    fn as_str_matches_each_wire_name() {
        for (recourse, wire_name) in CASES {
            assert_eq!(recourse.as_str(), *wire_name);
        }
    }

    #[test]
    fn serde_uses_exact_snake_case_wire_names() {
        for (recourse, wire_name) in CASES {
            assert_eq!(
                serde_json::to_string(recourse).unwrap(),
                format!("\"{wire_name}\"")
            );
            assert_eq!(
                serde_json::from_str::<PrincipalContinuityRecourse>(&format!("\"{wire_name}\""))
                    .unwrap(),
                *recourse
            );
        }
    }
}
