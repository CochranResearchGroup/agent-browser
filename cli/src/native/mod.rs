#[allow(dead_code)]
pub(crate) mod action_runtime;
pub mod actions;
#[allow(dead_code)]
pub mod auth;
#[allow(dead_code)]
pub mod auth_workflow;
#[allow(dead_code)]
pub(crate) mod authentication_run;
#[allow(dead_code)]
pub mod browser;
#[allow(dead_code)]
pub mod browser_context;
#[allow(dead_code)]
pub mod browser_download;
#[allow(dead_code)]
pub mod browser_emulation;
#[allow(dead_code)]
pub mod browser_frame;
#[allow(dead_code)]
pub mod browser_input;
#[allow(dead_code)]
pub mod browser_inspection;
#[allow(dead_code)]
pub mod browser_lifecycle;
#[allow(dead_code)]
pub mod browser_locator;
#[allow(dead_code)]
pub mod browser_navigation;
#[allow(dead_code)]
pub mod browser_session_authority;
#[allow(dead_code)]
pub mod browser_tabs;
#[allow(dead_code)]
pub mod browser_wait;
#[allow(dead_code)]
pub mod cancellation;
#[allow(dead_code)]
pub mod cdp;
#[allow(dead_code)]
pub mod clipboard;
#[allow(dead_code)]
pub mod control_plane;
pub(crate) mod controlled_x11_provider;
#[allow(dead_code)]
pub mod cookies;
#[allow(dead_code)]
pub mod daemon;
#[allow(dead_code)]
pub mod dependent_batch;
#[allow(dead_code)]
pub mod desktop_capture;
#[allow(dead_code)]
pub(crate) mod desktop_control_coordinator;
#[allow(dead_code)]
pub(crate) mod desktop_evidence;
pub(crate) mod desktop_evidence_action;
#[allow(dead_code)]
pub(crate) mod desktop_evidence_cdp;
// The configured episode is landing adapter by adapter so each live boundary
// can remain fail-closed until the product caller is complete.
#[allow(dead_code)]
pub(crate) mod desktop_evidence_configured;
#[allow(dead_code)]
pub(crate) mod desktop_input_provider;
pub(crate) mod desktop_input_provider_admission;
#[allow(dead_code)]
pub(crate) mod desktop_interaction;
#[allow(dead_code)]
pub mod desktop_locator;
#[allow(dead_code)]
pub(crate) mod desktop_prompt_perception;
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
pub mod network_archive;
#[allow(dead_code)]
pub mod network_requests;
#[allow(dead_code)]
pub mod page_capture;
#[allow(dead_code)]
pub mod page_injection;
#[allow(dead_code)]
pub mod policy;
#[allow(dead_code)]
pub(crate) mod presentation_capacity;
pub(crate) mod presentation_inventory;
#[allow(dead_code)]
pub(crate) mod presentation_lifecycle;
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
pub(crate) mod runtime_lifecycle;
#[allow(dead_code)]
pub(crate) mod runtime_reconciliation;
#[allow(dead_code)]
pub mod screenshot;
#[allow(dead_code)]
pub mod service_access;
#[allow(dead_code)]
pub mod service_activity;
mod service_boot_epoch;
pub(crate) mod service_browser_retirement;
#[allow(dead_code)]
pub mod service_config;
#[allow(dead_code)]
pub mod service_contracts;
#[allow(dead_code)] // Slice I wires effects after Slice H publishes the contract.
mod service_crash_regeneration;
#[allow(dead_code)]
pub mod service_diagnostics;
#[allow(dead_code)]
pub mod service_failure;
#[allow(dead_code)]
pub mod service_file_transfer;
#[allow(dead_code)]
pub mod service_health;
#[allow(dead_code)]
pub mod service_incidents;
#[allow(dead_code)]
pub mod service_inventory;
#[allow(dead_code)]
pub mod service_jobs;
#[allow(dead_code)]
pub(crate) mod service_lease_authority;
#[allow(dead_code)]
pub(crate) mod service_lease_mode;
#[allow(dead_code)]
pub mod service_lifecycle;
#[allow(dead_code)]
pub mod service_model;
#[allow(dead_code)]
pub mod service_monitors;
#[allow(dead_code)]
pub mod service_network_capture;
#[allow(dead_code)]
pub(crate) mod service_principal;
#[allow(dead_code)]
pub mod service_probe;
pub(crate) mod service_profile_access_policy;
#[allow(dead_code)]
pub(crate) mod service_profile_acquisition;
#[allow(dead_code)]
pub(crate) mod service_profile_lease;
pub(crate) mod service_profile_lifecycle;
#[allow(dead_code)]
pub mod service_renderer_crash;
#[allow(dead_code)]
pub mod service_request;
#[allow(dead_code)]
pub(crate) mod service_request_provenance;
#[allow(dead_code)]
pub mod service_resources;
#[allow(dead_code)]
pub mod service_retained_state;
pub(crate) mod service_state_migration;
pub(crate) mod service_state_validation;
#[allow(dead_code)]
pub mod service_status_projection;
#[allow(dead_code)]
pub mod service_store;
pub(crate) mod service_terminal_outcome;
#[allow(dead_code)]
pub mod service_trace;
#[allow(dead_code)]
pub mod service_ui_action;
#[allow(dead_code)]
pub mod snapshot;
#[allow(dead_code)]
pub mod state;
#[allow(dead_code)]
pub mod storage;
#[allow(dead_code)]
pub mod stream;
#[allow(dead_code)]
pub mod stream_runtime;
#[allow(dead_code)]
pub mod tracing;
#[allow(dead_code)]
pub mod webdriver;
pub(crate) mod x11_scene;

#[cfg(test)]
mod e2e_tests;
#[cfg(test)]
mod parity_tests;
