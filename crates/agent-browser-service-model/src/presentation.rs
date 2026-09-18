//! Provider-free durable presentation records and deterministic record behavior.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::BrowserHost;

pub const SERVICE_VIEW_STREAM_PROVIDER_VALUES: [&str; 6] = [
    "cdp_screencast",
    "chrome_tab_webrtc",
    "virtual_display_webrtc",
    "novnc",
    "rdp_gateway",
    "external_url",
];
pub const SERVICE_CONTROL_INPUT_PROVIDER_VALUES: [&str; 4] = [
    "cdp_input",
    "webrtc_input",
    "vnc_input",
    "manual_attached_desktop",
];

/// Supported live-view transport families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewStreamProvider {
    CdpScreencast,
    ChromeTabWebrtc,
    VirtualDisplayWebrtc,
    Novnc,
    RdpGateway,
    ExternalUrl,
}

/// Supported remote-input transport families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlInputProvider {
    CdpInput,
    WebrtcInput,
    VncInput,
    ManualAttachedDesktop,
}

/// Durable intent and latest attachment for an operator remote-view URL.
///
/// Route, display, and browser target identifiers are retained only as the
/// latest resolution evidence. Resolving the opaque handoff ID must reacquire
/// replaceable infrastructure from `intent` instead of treating those IDs as
/// durable routing authority.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RemoteViewHandoff {
    pub id: String,
    pub state: String,
    pub intent: Value,
    pub handoff_url: Option<String>,
    pub desired_url: Option<String>,
    pub profile_id: Option<String>,
    pub browser_id: Option<String>,
    pub session_name: Option<String>,
    pub tab_id: Option<String>,
    pub target_id: Option<String>,
    pub view_stream_provider: Option<ViewStreamProvider>,
    pub control_input: Option<ControlInputProvider>,
    pub last_route_id: Option<String>,
    pub last_route_pool_entry_id: Option<String>,
    pub last_display_allocation_id: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub last_resolved_at: Option<String>,
    pub last_resolution: Option<Value>,
    /// Latest end-to-end presentation generation. A resolver may render only
    /// when this receipt matches the requested provider and retained target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presentation_receipt: Option<DurableHandoffPresentationReceipt>,
}

/// Browser-specific proof consumed by the authenticated durable handoff gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DurableHandoffPresentationReceipt {
    pub schema_version: String,
    pub generation: u64,
    pub dashboard_deployment_generation: String,
    pub logical_browser_id: String,
    pub daemon_owner_generation: Option<u64>,
    pub process_instance_digest: Option<String>,
    pub target_id: String,
    pub required_stream_provider: ViewStreamProvider,
    pub observed_stream_provider: ViewStreamProvider,
    pub route_id: String,
    pub display_allocation_id: String,
    pub observed_at: String,
    pub state: String,
}

/// Service-owned remote display allocation for a browser workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DisplayAllocation {
    pub id: String,
    /// Host boot that authenticated the display and its package-owned PIDs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boot_epoch: Option<String>,
    pub display_name: Option<String>,
    pub display_isolation: String,
    pub owner_browser_id: Option<String>,
    pub owner_session_id: Option<String>,
    pub profile_id: Option<String>,
    pub browser_build: Option<String>,
    pub host: Option<BrowserHost>,
    pub state: String,
    pub pid_hints: Option<Value>,
    pub route_ids: Vec<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub last_health_check_at: Option<String>,
    pub readiness: Option<Value>,
}

impl Default for DisplayAllocation {
    fn default() -> Self {
        Self {
            id: String::new(),
            boot_epoch: None,
            display_name: None,
            display_isolation: "private_virtual_display".to_string(),
            owner_browser_id: None,
            owner_session_id: None,
            profile_id: None,
            browser_build: None,
            host: Some(BrowserHost::RemoteHeaded),
            state: "allocating".to_string(),
            pid_hints: None,
            route_ids: Vec::new(),
            created_at: None,
            updated_at: None,
            last_health_check_at: None,
            readiness: None,
        }
    }
}

/// Service-owned provider route for a remote display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RemoteViewRoute {
    pub id: String,
    pub provider: ViewStreamProvider,
    pub display_allocation_id: Option<String>,
    pub browser_id: Option<String>,
    pub session_id: Option<String>,
    pub route_source: String,
    pub connection_id: Option<String>,
    pub connection_name: Option<String>,
    pub route_template: Option<String>,
    pub frame_url: Option<String>,
    pub external_url: Option<String>,
    pub route_descriptor: Option<Value>,
    pub read_only: bool,
    pub control_input: Option<ControlInputProvider>,
    pub provider_mode: String,
    pub state: String,
    pub viewer_lease_ids: Vec<String>,
    pub controller_lease_id: Option<String>,
    /// Monotonic fencing token for primary-controller authority on this route.
    pub controller_epoch: u64,
    pub last_provider_event: Option<String>,
    pub readiness: Option<Value>,
}

impl Default for RemoteViewRoute {
    fn default() -> Self {
        Self {
            id: String::new(),
            provider: ViewStreamProvider::RdpGateway,
            display_allocation_id: None,
            browser_id: None,
            session_id: None,
            route_source: "unknown".to_string(),
            connection_id: None,
            connection_name: None,
            route_template: None,
            frame_url: None,
            external_url: None,
            route_descriptor: None,
            read_only: false,
            control_input: Some(ControlInputProvider::ManualAttachedDesktop),
            provider_mode: "unknown".to_string(),
            state: "allocating".to_string(),
            viewer_lease_ids: Vec::new(),
            controller_lease_id: None,
            controller_epoch: 0,
            last_provider_event: None,
            readiness: None,
        }
    }
}

impl RemoteViewRoute {
    /// Advance primary-controller authority, including same-id re-grants that
    /// would otherwise permit an ABA reuse of an older authority receipt.
    pub fn advance_controller(&mut self, controller_lease_id: Option<String>) -> u64 {
        self.controller_epoch = self.controller_epoch.saturating_add(1);
        self.controller_lease_id = controller_lease_id;
        self.controller_epoch
    }
}

/// Configured provider route pool entry for remote-view allocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RoutePoolEntry {
    pub id: String,
    pub provider: ViewStreamProvider,
    pub route_id: String,
    pub connection_id: Option<String>,
    pub connection_name: Option<String>,
    pub frame_url: Option<String>,
    pub external_url: Option<String>,
    pub route_descriptor: Option<Value>,
    pub target: Value,
    pub provider_mode: String,
    pub state: String,
    pub current_route_allocation_id: Option<String>,
    pub readiness: Option<Value>,
}

impl Default for RoutePoolEntry {
    fn default() -> Self {
        Self {
            id: String::new(),
            provider: ViewStreamProvider::RdpGateway,
            route_id: String::new(),
            connection_id: None,
            connection_name: None,
            frame_url: None,
            external_url: None,
            route_descriptor: None,
            target: Value::Object(Default::default()),
            provider_mode: "unknown".to_string(),
            state: "unknown".to_string(),
            current_route_allocation_id: None,
            readiness: None,
        }
    }
}

/// Return one normalized string binding from a route-pool target.
pub fn route_pool_target_string(entry: &RoutePoolEntry, key: &str) -> Option<String> {
    entry
        .target
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

/// Return whether a route-pool entry's optional target bindings admit one
/// display allocation.
pub fn route_pool_entry_matches_display(
    entry: &RoutePoolEntry,
    display_allocation_id: &str,
    allocation: Option<&DisplayAllocation>,
) -> bool {
    if let Some(target_allocation_id) = route_pool_target_string(entry, "displayAllocationId") {
        if target_allocation_id != display_allocation_id {
            return false;
        }
    }
    if let Some(target_browser_id) = route_pool_target_string(entry, "browserId") {
        if allocation.and_then(|allocation| allocation.owner_browser_id.as_deref())
            != Some(target_browser_id.as_str())
        {
            return false;
        }
    }
    if let Some(target_session_id) = route_pool_target_string(entry, "sessionId") {
        if allocation.and_then(|allocation| allocation.owner_session_id.as_deref())
            != Some(target_session_id.as_str())
        {
            return false;
        }
    }
    if let Some(target_display_name) = route_pool_target_string(entry, "displayName") {
        if allocation.is_some_and(|allocation| {
            allocation.display_name.as_deref() != Some(target_display_name.as_str())
        }) {
            return false;
        }
    }
    true
}

#[derive(Debug, Clone)]
pub struct RetainedDisplayAllocationCandidate {
    pub id: String,
    pub class_name: &'static str,
    pub reason: &'static str,
    pub apply_safe: bool,
    pub linked_route_ids: Vec<String>,
    pub linked_browser_ids: Vec<String>,
    pub linked_session_ids: Vec<String>,
    pub linked_incident_ids: Vec<String>,
    pub linked_route_pool_entry_ids: Vec<String>,
}

impl RetainedDisplayAllocationCandidate {
    pub fn to_json(&self) -> Value {
        json!({
            "class": self.class_name,
            "reason": self.reason,
            "applySafe": self.apply_safe,
            "linkedRouteIds": self.linked_route_ids,
            "linkedBrowserIds": self.linked_browser_ids,
            "linkedSessionIds": self.linked_session_ids,
            "linkedIncidentIds": self.linked_incident_ids,
            "linkedRoutePoolEntryIds": self.linked_route_pool_entry_ids,
        })
    }
}

/// Pending or completed acquisition transaction for an operator-visible route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RemoteViewAcquisitionLease {
    pub id: String,
    /// Host boot that authenticated this in-flight acquisition observation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boot_epoch: Option<String>,
    pub browser_id: String,
    pub session_id: String,
    pub route_id: String,
    pub display_allocation_id: String,
    pub route_pool_entry_id: Option<String>,
    pub state: String,
    pub phase: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub completed_at: Option<String>,
    pub failed_at: Option<String>,
    pub failure_reason: Option<String>,
    pub cleanup: Option<Value>,
    pub previous_route_pool_entry: Option<RoutePoolEntry>,
    pub previous_display_allocation: Option<DisplayAllocation>,
    pub previous_remote_view_route: Option<RemoteViewRoute>,
    pub previous_browser_display_allocation_id: Option<String>,
}

impl Default for RemoteViewAcquisitionLease {
    fn default() -> Self {
        Self {
            id: String::new(),
            boot_epoch: None,
            browser_id: String::new(),
            session_id: String::new(),
            route_id: String::new(),
            display_allocation_id: String::new(),
            route_pool_entry_id: None,
            state: "pending".to_string(),
            phase: "planned".to_string(),
            created_at: None,
            updated_at: None,
            completed_at: None,
            failed_at: None,
            failure_reason: None,
            cleanup: None,
            previous_route_pool_entry: None,
            previous_display_allocation: None,
            previous_remote_view_route: None,
            previous_browser_display_allocation_id: None,
        }
    }
}

/// Observer or controller lease for a remote-view route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ViewerLease {
    pub id: String,
    /// Host boot that authenticated the package-owned viewer observation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boot_epoch: Option<String>,
    pub route_id: Option<String>,
    pub browser_id: Option<String>,
    pub viewer_id: Option<String>,
    pub viewer_name: Option<String>,
    pub viewer_role: String,
    pub open_mode: String,
    pub state: String,
    pub last_viewer_event: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub last_heartbeat_at: Option<String>,
    pub service_event_id: Option<String>,
}

impl Default for ViewerLease {
    fn default() -> Self {
        Self {
            id: String::new(),
            boot_epoch: None,
            route_id: None,
            browser_id: None,
            viewer_id: None,
            viewer_name: None,
            viewer_role: "observer".to_string(),
            open_mode: "embedded".to_string(),
            state: "requested".to_string(),
            last_viewer_event: None,
            expires_at: None,
            created_at: None,
            updated_at: None,
            last_heartbeat_at: None,
            service_event_id: None,
        }
    }
}

/// Dashboard viewing mechanism for a browser or tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ViewStream {
    pub id: String,
    pub provider: ViewStreamProvider,
    /// Input transport expected to control this view stream, when known.
    pub control_input: Option<ControlInputProvider>,
    /// Backward-compatible primary stream URL for older clients.
    pub url: Option<String>,
    /// Dashboard-embeddable stream URL chosen by the service.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_url: Option<String>,
    /// Direct stream URL for external windows or popout viewers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_url: Option<String>,
    /// Structured remote-view route URLs by audience.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_descriptor: Option<Value>,
    /// Service-owned route id for providers that multiplex viewer routes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_id: Option<String>,
    /// Service-owned display allocation id backing this route.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_allocation_id: Option<String>,
    /// Guacamole connection id or route token, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    /// Human-readable Guacamole connection name, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_name: Option<String>,
    /// Source that selected the route metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_source: Option<String>,
    /// Provider concurrency mode for observers and controllers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_mode: Option<String>,
    /// Viewer lease ids associated with this route.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub viewer_lease_ids: Vec<String>,
    /// Current controller lease id for this route.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub controller_lease_id: Option<String>,
    /// Route-owned controller fencing token copied into this stream projection.
    pub controller_epoch: u64,
    pub read_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readiness: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_readiness: Option<Value>,
    /// Browser-first remote-view attachability derived from browser, route,
    /// display-allocation, route-pool, stream, and viewer lease state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachability: Option<Value>,
}

impl Default for ViewStream {
    fn default() -> Self {
        Self {
            id: String::new(),
            provider: ViewStreamProvider::CdpScreencast,
            control_input: Some(ControlInputProvider::CdpInput),
            url: None,
            frame_url: None,
            external_url: None,
            route_descriptor: None,
            route_id: None,
            display_allocation_id: None,
            connection_id: None,
            connection_name: None,
            route_source: None,
            provider_mode: None,
            viewer_lease_ids: Vec::new(),
            controller_lease_id: None,
            controller_epoch: 0,
            read_only: true,
            readiness: None,
            remote_readiness: None,
            attachability: None,
        }
    }
}

impl ViewStream {
    /// Copy the authoritative controller identity and fencing epoch from a
    /// route. Streams never advance controller authority independently.
    pub fn project_controller(&mut self, route: &RemoteViewRoute) {
        self.controller_lease_id = route.controller_lease_id.clone();
        self.controller_epoch = route.controller_epoch;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_preserve_remote_view_contracts() {
        let display = DisplayAllocation::default();
        assert_eq!(display.display_isolation, "private_virtual_display");
        assert_eq!(display.host, Some(BrowserHost::RemoteHeaded));
        assert_eq!(display.state, "allocating");

        let route = RemoteViewRoute::default();
        assert_eq!(route.provider, ViewStreamProvider::RdpGateway);
        assert_eq!(
            route.control_input,
            Some(ControlInputProvider::ManualAttachedDesktop)
        );
        assert_eq!(route.state, "allocating");

        let pool = RoutePoolEntry::default();
        assert_eq!(pool.target, json!({}));
        assert_eq!(pool.state, "unknown");

        let lease = RemoteViewAcquisitionLease::default();
        assert_eq!(lease.state, "pending");
        assert_eq!(lease.phase, "planned");

        let viewer = ViewerLease::default();
        assert_eq!(viewer.viewer_role, "observer");
        assert_eq!(viewer.open_mode, "embedded");
        assert_eq!(viewer.state, "requested");

        let stream = ViewStream::default();
        assert_eq!(stream.provider, ViewStreamProvider::CdpScreencast);
        assert_eq!(stream.control_input, Some(ControlInputProvider::CdpInput));
        assert!(stream.read_only);
    }

    #[test]
    fn remote_view_wire_names_defaults_and_omissions_are_stable() {
        let handoff: RemoteViewHandoff =
            serde_json::from_value(json!({"id": "handoff-1"})).unwrap();
        assert_eq!(handoff.id, "handoff-1");
        assert_eq!(handoff.intent, Value::Null);
        assert!(handoff.presentation_receipt.is_none());

        let display = serde_json::to_value(DisplayAllocation::default()).unwrap();
        assert!(display.get("bootEpoch").is_none());
        assert_eq!(display["displayIsolation"], "private_virtual_display");

        let stream = serde_json::to_value(ViewStream::default()).unwrap();
        assert_eq!(stream["provider"], "cdp_screencast");
        assert_eq!(stream["controlInput"], "cdp_input");
        assert!(stream.get("frameUrl").is_none());
        assert!(stream.get("viewerLeaseIds").is_none());
        assert_eq!(stream["controllerEpoch"], 0);

        let receipt = json!({
            "schemaVersion": "agent-browser.remote-view-presentation.v1",
            "generation": 2,
            "dashboardDeploymentGeneration": "dash-1",
            "logicalBrowserId": "browser-1",
            "daemonOwnerGeneration": 3,
            "processInstanceDigest": "digest",
            "targetId": "target-1",
            "requiredStreamProvider": "rdp_gateway",
            "observedStreamProvider": "rdp_gateway",
            "routeId": "route-1",
            "displayAllocationId": "display-1",
            "observedAt": "2026-09-16T00:00:00Z",
            "state": "ready"
        });
        let decoded: DurableHandoffPresentationReceipt =
            serde_json::from_value(receipt.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), receipt);
    }

    #[test]
    fn controller_projection_advances_and_fences_same_id_regrants() {
        let mut route = RemoteViewRoute::default();
        let mut stream = ViewStream::default();

        assert_eq!(
            route.advance_controller(Some("controller-a".to_string())),
            1
        );
        stream.project_controller(&route);
        assert_eq!(stream.controller_lease_id.as_deref(), Some("controller-a"));
        assert_eq!(stream.controller_epoch, 1);

        assert_eq!(
            route.advance_controller(Some("controller-a".to_string())),
            2
        );
        stream.project_controller(&route);
        assert_eq!(stream.controller_epoch, 2);
    }

    #[test]
    fn route_pool_target_string_trims_and_rejects_empty_or_non_string_values() {
        let entry = RoutePoolEntry {
            target: json!({
                "displayAllocationId": "  display-a  ",
                "empty": "   ",
                "number": 10
            }),
            ..RoutePoolEntry::default()
        };

        assert_eq!(
            route_pool_target_string(&entry, "displayAllocationId").as_deref(),
            Some("display-a")
        );
        assert_eq!(route_pool_target_string(&entry, "empty"), None);
        assert_eq!(route_pool_target_string(&entry, "number"), None);
        assert_eq!(route_pool_target_string(&entry, "missing"), None);
    }

    #[test]
    fn route_pool_entry_matches_exact_display_target_bindings() {
        let entry = RoutePoolEntry {
            target: json!({
                "displayAllocationId": "display-a",
                "browserId": "browser-a",
                "sessionId": "session-a",
                "displayName": ":10"
            }),
            ..RoutePoolEntry::default()
        };
        let allocation = DisplayAllocation {
            id: "display-a".into(),
            display_name: Some(":10".into()),
            owner_browser_id: Some("browser-a".into()),
            owner_session_id: Some("session-a".into()),
            ..DisplayAllocation::default()
        };

        assert!(route_pool_entry_matches_display(
            &entry,
            "display-a",
            Some(&allocation)
        ));

        let mut wrong_display_name = allocation.clone();
        wrong_display_name.display_name = Some(":11".into());
        assert!(!route_pool_entry_matches_display(
            &entry,
            "display-a",
            Some(&wrong_display_name)
        ));
        assert!(!route_pool_entry_matches_display(
            &entry,
            "display-b",
            Some(&allocation)
        ));
        assert!(!route_pool_entry_matches_display(&entry, "display-a", None));
    }

    #[test]
    fn display_name_target_allows_absent_allocation_context() {
        let entry = RoutePoolEntry {
            target: json!({ "displayName": ":10" }),
            ..RoutePoolEntry::default()
        };

        assert!(route_pool_entry_matches_display(&entry, "display-a", None));
    }

    #[test]
    fn retained_candidate_json_uses_stable_camel_case_keys() {
        let candidate = RetainedDisplayAllocationCandidate {
            id: "display-1".to_string(),
            class_name: "orphaned",
            reason: "no_owner",
            apply_safe: true,
            linked_route_ids: vec!["route-1".to_string()],
            linked_browser_ids: vec!["browser-1".to_string()],
            linked_session_ids: vec!["session-1".to_string()],
            linked_incident_ids: vec!["incident-1".to_string()],
            linked_route_pool_entry_ids: vec!["pool-1".to_string()],
        };

        assert_eq!(
            candidate.to_json(),
            json!({
                "class": "orphaned",
                "reason": "no_owner",
                "applySafe": true,
                "linkedRouteIds": ["route-1"],
                "linkedBrowserIds": ["browser-1"],
                "linkedSessionIds": ["session-1"],
                "linkedIncidentIds": ["incident-1"],
                "linkedRoutePoolEntryIds": ["pool-1"],
            })
        );
    }

    #[test]
    fn provider_constants_match_wire_variants() {
        assert_eq!(SERVICE_VIEW_STREAM_PROVIDER_VALUES[4], "rdp_gateway");
        assert_eq!(
            SERVICE_CONTROL_INPUT_PROVIDER_VALUES[3],
            "manual_attached_desktop"
        );
        assert_eq!(
            serde_json::to_value(ViewStreamProvider::ChromeTabWebrtc).unwrap(),
            json!("chrome_tab_webrtc")
        );
        assert_eq!(
            serde_json::to_value(ControlInputProvider::VncInput).unwrap(),
            json!("vnc_input")
        );
    }
}
