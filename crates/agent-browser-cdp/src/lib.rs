//! Chrome DevTools Protocol transport and protocol types for Agent Browser.
//!
//! This crate owns the websocket command lifecycle and CDP wire model. Browser
//! process launch, profile ownership, and runtime orchestration remain in the
//! `agent-browser` binary crate.

pub mod client;
pub mod types;
