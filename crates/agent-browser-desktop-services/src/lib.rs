//! Provider-neutral desktop transaction services.
//!
//! Platform capture, input, OCR, browser lifecycle, Service State, and durable
//! filesystem adapters remain in the CLI package.

mod coordinator;
mod transaction;

pub use coordinator::{
    DesktopControlCoordinator, DesktopControlEventGuard, DesktopControllerMutationGuard,
    DesktopInteractionClaim,
};
pub use transaction::*;
