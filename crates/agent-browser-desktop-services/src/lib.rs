//! Provider-neutral desktop transaction services.
//!
//! Platform capture, input, OCR, browser lifecycle, Service State, and durable
//! filesystem adapters remain in the CLI package.

mod candidate_intent;
mod coordinator;
mod transaction;

pub use candidate_intent::*;
pub use coordinator::{
    DesktopControlCoordinator, DesktopControlEventGuard, DesktopControllerMutationGuard,
    DesktopInteractionClaim,
};
pub use transaction::*;
