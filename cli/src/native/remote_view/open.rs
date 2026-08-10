//! Route-bound browser acquisition and durable handoff resolution.

#![allow(unused_imports)]

mod deadline;
pub(crate) use deadline::*;
mod runtime;
pub(crate) use runtime::*;
mod coordinator;
pub(crate) use coordinator::*;
mod planner;
pub(crate) use planner::*;
mod compensation;
pub(crate) use compensation::*;
mod operator_route;
pub(crate) use operator_route::*;
mod proof;
pub(crate) use proof::*;
mod preflight;
pub(crate) use preflight::*;
mod route_lifecycle;
pub(crate) use route_lifecycle::*;
mod target;
pub(crate) use target::*;
mod route_pool;
pub(crate) use route_pool::*;
mod shared;

#[cfg(test)]
mod tests;
