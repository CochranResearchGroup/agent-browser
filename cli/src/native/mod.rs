#[allow(dead_code)]
pub mod actions;
#[allow(dead_code)]
pub mod auth;
#[allow(dead_code)]
pub mod browser;
#[allow(dead_code)]
pub mod browser_session_authority;
#[allow(dead_code)]
pub mod cancellation;
#[allow(dead_code)]
pub mod cdp;
#[allow(dead_code)]
pub mod clipboard;
#[allow(dead_code)]
pub mod control_plane;
#[allow(dead_code)]
pub mod cookies;
#[allow(dead_code)]
pub mod daemon;
#[allow(dead_code)]
pub mod dependent_batch;
#[allow(dead_code)]
pub mod diff;
#[allow(dead_code)]
pub mod element;
#[allow(dead_code)]
pub mod inspect_server;
#[allow(dead_code)]
pub mod interaction;
#[allow(dead_code)]
pub mod network;
#[allow(dead_code)]
pub mod policy;
#[allow(dead_code)]
pub mod providers;
#[allow(dead_code)]
pub mod recording;
#[allow(dead_code)]
pub mod remote_view;
#[allow(dead_code)]
pub mod remote_view_attachability;
#[allow(dead_code)]
pub mod remote_view_finalization;
#[allow(dead_code)]
pub mod remote_view_handoff;
#[allow(dead_code)]
pub mod remote_view_lease;
#[allow(dead_code)]
pub mod remote_view_proof;
#[allow(dead_code)]
pub mod screenshot;
#[allow(dead_code)]
pub mod service_access;
#[allow(dead_code)]
pub mod service_activity;
#[allow(dead_code)]
pub mod service_config;
#[allow(dead_code)]
pub mod service_contracts;
#[allow(dead_code)]
pub mod service_health;
#[allow(dead_code)]
pub mod service_incidents;
#[allow(dead_code)]
pub mod service_jobs;
#[allow(dead_code)]
pub mod service_lifecycle;
#[allow(dead_code)]
pub mod service_model;
#[allow(dead_code)]
pub mod service_monitors;
#[allow(dead_code)]
pub mod service_resources;
#[allow(dead_code)]
pub mod service_store;
#[allow(dead_code)]
pub mod service_trace;
#[allow(dead_code)]
pub mod snapshot;
#[allow(dead_code)]
pub mod state;
#[allow(dead_code)]
pub mod storage;
#[allow(dead_code)]
pub mod stream;
#[allow(dead_code)]
pub mod tracing;
#[allow(dead_code)]
pub mod webdriver;

#[cfg(test)]
mod e2e_tests;
#[cfg(test)]
mod parity_tests;
