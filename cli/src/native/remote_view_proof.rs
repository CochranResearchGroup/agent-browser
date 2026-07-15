pub fn remote_view_target_component_state(
    tab_present: bool,
    target_id: Option<&str>,
    url_readiness: &str,
) -> &'static str {
    if !tab_present {
        "not_checked"
    } else if target_id.is_none() {
        "cdp_target_unavailable"
    } else if matches!(url_readiness, "wrong_tab" | "target_url_missing") {
        if url_readiness == "wrong_tab" {
            "wrong_tab"
        } else {
            "target_url_missing"
        }
    } else {
        "ready"
    }
}

pub fn remote_view_operator_visible_state(
    route_state: &str,
    display_state: &str,
    target_state: &str,
    guacamole_state: &str,
    operator_access_state: &str,
) -> String {
    if route_state == "ready"
        && display_state == "ready"
        && target_state == "ready"
        && guacamole_state == "ready"
        && operator_access_state == "ready"
    {
        "ready".to_string()
    } else if route_state != "ready" {
        route_state.to_string()
    } else if display_state == "ready" && target_state == "ready" && guacamole_state == "ready" {
        operator_access_state.to_string()
    } else if display_state == "ready" && target_state == "ready" {
        guacamole_state.to_string()
    } else if display_state == "ready" {
        target_state.to_string()
    } else {
        display_state.to_string()
    }
}

pub fn remote_view_operator_visible_ready(
    route_state: &str,
    display_state: &str,
    target_state: &str,
    guacamole_state: &str,
    operator_access_state: &str,
) -> bool {
    remote_view_operator_visible_state(
        route_state,
        display_state,
        target_state,
        guacamole_state,
        operator_access_state,
    ) == "ready"
}

#[cfg(test)]
mod tests {
    use super::{
        remote_view_operator_visible_ready, remote_view_operator_visible_state,
        remote_view_target_component_state,
    };

    #[test]
    fn operator_visible_proof_reports_ready_only_when_all_components_are_ready() {
        assert!(remote_view_operator_visible_ready(
            "ready", "ready", "ready", "ready", "ready"
        ));
        assert_eq!(
            remote_view_operator_visible_state("ready", "ready", "ready", "ready", "ready"),
            "ready"
        );
    }

    #[test]
    fn operator_visible_proof_preserves_blocker_priority() {
        assert_eq!(
            remote_view_operator_visible_state(
                "stale_route_record",
                "ready",
                "ready",
                "ready",
                "ready"
            ),
            "stale_route_record"
        );
        assert_eq!(
            remote_view_operator_visible_state("ready", "terminal_only", "ready", "ready", "ready"),
            "terminal_only"
        );
        assert_eq!(
            remote_view_operator_visible_state("ready", "ready", "wrong_tab", "ready", "ready"),
            "wrong_tab"
        );
        assert_eq!(
            remote_view_operator_visible_state(
                "ready",
                "ready",
                "ready",
                "guacamole_route_unavailable",
                "ready",
            ),
            "guacamole_route_unavailable"
        );
        assert_eq!(
            remote_view_operator_visible_state(
                "ready",
                "ready",
                "ready",
                "ready",
                "public_operator_unavailable",
            ),
            "public_operator_unavailable"
        );
    }

    #[test]
    fn target_component_proof_distinguishes_missing_target_from_wrong_tab() {
        assert_eq!(
            remote_view_target_component_state(false, None, "not_checked"),
            "not_checked"
        );
        assert_eq!(
            remote_view_target_component_state(true, None, "ready"),
            "cdp_target_unavailable"
        );
        assert_eq!(
            remote_view_target_component_state(true, Some("target-1"), "wrong_tab"),
            "wrong_tab"
        );
        assert_eq!(
            remote_view_target_component_state(true, Some("target-1"), "target_url_missing"),
            "target_url_missing"
        );
        assert_eq!(
            remote_view_target_component_state(true, Some("target-1"), "ready"),
            "ready"
        );
    }
}
