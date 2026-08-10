use super::shared::*;

/// Normalized compatibility command facts accepted by a route-bound effect.
/// Unknown edge fields are retained for compatibility, while known fields are
/// deserialized into concrete types and therefore cannot silently change
/// shape inside the coordinator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouteBoundCommandRecord {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) target_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) browser_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) session_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) runtime_profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) route_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) handoff_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) dry_run: Option<bool>,
    #[serde(default, flatten)]
    pub(crate) extensions: Map<String, Value>,
}

impl RouteBoundCommandRecord {
    pub(crate) fn from_compatibility(value: Value, effect: &'static str) -> Result<Self, String> {
        if !value.is_object() {
            return Err(format!("{effect} requires a JSON object command"));
        }
        serde_json::from_value(value)
            .map_err(|error| format!("{effect} has invalid typed command fields: {error}"))
    }

    pub(crate) fn into_value(self) -> Value {
        serde_json::to_value(self).expect("route-bound command record must serialize")
    }
}

/// Concrete facts returned by the permanent daemon/browser effect adapter.
/// The compatibility extension map remains at the serialization edge; route
/// identity, state, target, and mutation facts are typed and validated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouteBoundEffectRecord {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) target_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) browser_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) session_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) route_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) pid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) focused: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) closed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) launched: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) reused: Option<bool>,
    #[serde(default, flatten)]
    pub(crate) extensions: Map<String, Value>,
}

impl RouteBoundEffectRecord {
    fn from_compatibility(value: Value, effect: &'static str) -> Result<Self, String> {
        if !value.is_object() {
            return Err(format!("{effect} returned a non-object result"));
        }
        serde_json::from_value(value)
            .map_err(|error| format!("{effect} returned invalid typed fields: {error}"))
    }

    pub(crate) fn into_value(self) -> Value {
        serde_json::to_value(self).expect("route-bound effect record must serialize")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RouteBoundBrowserIdentity {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) browser_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) session_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) pid: Option<u32>,
    #[serde(default, flatten)]
    pub(crate) extensions: Map<String, Value>,
}

impl RouteBoundBrowserIdentity {
    pub(crate) fn from_compatibility(value: Value) -> Result<Self, String> {
        if !value.is_object() {
            return Err("close_created_browser requires an object identity".to_string());
        }
        serde_json::from_value(value)
            .map_err(|error| format!("close_created_browser has invalid identity: {error}"))
    }
}

macro_rules! command_record {
    ($name:ident, $effect:literal) => {
        #[derive(Debug, Clone, PartialEq)]
        pub(crate) struct $name {
            record: RouteBoundCommandRecord,
        }

        impl $name {
            pub(crate) fn from_compatibility(value: Value) -> Result<Self, String> {
                Ok(Self {
                    record: RouteBoundCommandRecord::from_compatibility(value, $effect)?,
                })
            }

            pub(crate) fn into_value(self) -> Value {
                self.record.into_value()
            }
        }
    };
}

macro_rules! effect_record {
    ($name:ident, $effect:literal) => {
        #[derive(Debug, Clone, PartialEq)]
        pub(crate) struct $name {
            record: RouteBoundEffectRecord,
        }

        impl $name {
            pub(crate) fn from_compatibility(value: Value) -> Result<Self, String> {
                Ok(Self {
                    record: RouteBoundEffectRecord::from_compatibility(value, $effect)?,
                })
            }

            pub(crate) fn into_value(self) -> Value {
                self.record.into_value()
            }
        }
    };
}

command_record!(LaunchBrowserCommand, "launch_browser");
command_record!(OpenTargetCommand, "open_target");
command_record!(FocusTargetCommand, "focus_target");
command_record!(CheckoutRouteCommand, "checkout_route");

effect_record!(LaunchBrowserResult, "launch_browser");
effect_record!(SwitchTargetResult, "switch_target");
effect_record!(NavigateTargetResult, "navigate_target");
effect_record!(OpenTargetResult, "open_target");
effect_record!(FocusTargetResult, "focus_target");
effect_record!(CloseCreatedTargetResult, "close_created_target");
effect_record!(CloseCreatedBrowserResult, "close_created_browser");
effect_record!(CheckoutRouteResult, "checkout_route");
effect_record!(DisplayAccessResult, "ensure_display_access");
effect_record!(VisibleWindowResult, "observe_visible_window");
effect_record!(OperatorAccessResult, "observe_operator_access");
