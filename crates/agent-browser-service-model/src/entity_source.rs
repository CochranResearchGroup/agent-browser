use std::collections::BTreeMap;

/// In-memory provenance for model entities after persisted state, configuration,
/// and shipped defaults are layered. This is intentionally not serialized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceEntitySource {
    PersistedState,
    Config,
    Builtin,
    RuntimeObserved,
}

impl ServiceEntitySource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PersistedState => "persisted_state",
            Self::Config => "config",
            Self::Builtin => "builtin",
            Self::RuntimeObserved => "runtime_observed",
        }
    }

    pub fn overrideable(self) -> bool {
        self == Self::Builtin
    }
}

/// In-memory provenance indexes for layered profile and site-policy entities.
///
/// These maps are intentionally separate from durable records: they describe
/// how the current model was assembled, not persisted authority.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServiceEntitySources {
    pub profiles: BTreeMap<String, ServiceEntitySource>,
    pub site_policies: BTreeMap<String, ServiceEntitySource>,
}
