/// Target-identity effect used by daemon-owned dependent batches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetEffect {
    Stable,
    Rebind,
}

/// Classifies commands that may intentionally change the active target.
/// Unknown commands are conservatively treated as stable so an unexpected
/// target change stops the batch instead of silently rebinding.
pub fn target_effect(action: &str) -> TargetEffect {
    match action {
        "navigate"
        | "back"
        | "forward"
        | "reload"
        | "tab_new"
        | "tab_close"
        | "tab_switch"
        | "close"
        | "launch"
        | "cdp_attach"
        | "cdp_detach"
        | "external_byop_adopt"
        | "state_load" => TargetEffect::Rebind,
        _ => TargetEffect::Stable,
    }
}

pub fn nested_batch_allowed(action: &str) -> bool {
    !matches!(
        action,
        "dependent_batch"
            | "batch"
            | "close"
            | "launch"
            | "cdp_attach"
            | "cdp_detach"
            | "external_byop_adopt"
    )
}

#[cfg(test)]
mod tests {
    use super::{nested_batch_allowed, target_effect, TargetEffect};

    #[test]
    fn target_stable_interactions_keep_the_binding() {
        for action in ["click", "snapshot", "fill", "gettext", "evaluate"] {
            assert_eq!(target_effect(action), TargetEffect::Stable, "{action}");
        }
    }

    #[test]
    fn target_changing_commands_require_rebinding() {
        for action in ["navigate", "tab_new", "tab_switch", "tab_close", "close"] {
            assert_eq!(target_effect(action), TargetEffect::Rebind, "{action}");
        }
    }

    #[test]
    fn nested_dependent_batches_are_rejected() {
        assert!(!nested_batch_allowed("dependent_batch"));
        assert!(!nested_batch_allowed("batch"));
        assert!(!nested_batch_allowed("close"));
        assert!(nested_batch_allowed("click"));
    }
}
