//! Deterministic, effect-free raw-event planning for admitted candidates.
//!
//! The planner accounts every move, button-down, and button-up against the
//! permit budget. It does not call an input provider, claim a route, capture a
//! surface, persist state, or emit an event.

use crate::{
    validate_desktop_candidate_effect_permit, DesktopCandidateEffectPermit, InputEvent, PixelPoint,
};
use sha2::{Digest, Sha256};

/// An inert, exact-budget sequence for a later separately governed executor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopCandidateEventPlan {
    pub source_permit_digest: String,
    pub pointer_start: PixelPoint,
    pub events: Vec<InputEvent>,
    pub started_at_ms: u64,
    pub duration_ms: u64,
    pub expires_at_ms: u64,
    pub plan_digest: String,
}

impl DesktopCandidateEventPlan {
    /// Reports the planner's no-effect construction invariant.
    pub fn emitted_effects(&self) -> bool {
        false
    }
}

/// Typed fail-closed outcomes from candidate event planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopCandidateEventPlanError {
    InvalidPermit,
    InvalidStart,
    UnsupportedKeyEvents,
    InsufficientPointerBudget,
    InvalidSchedule,
    Stale,
}

/// Plans exactly the pointer events authorized by one candidate permit.
pub fn plan_desktop_candidate_events(
    permit: &DesktopCandidateEffectPermit,
    pointer_start: PixelPoint,
    started_at_ms: u64,
    duration_ms: u64,
) -> Result<DesktopCandidateEventPlan, DesktopCandidateEventPlanError> {
    validate_desktop_candidate_effect_permit(permit)
        .map_err(|_| DesktopCandidateEventPlanError::InvalidPermit)?;
    if !point_on_surface(pointer_start, permit.binding.width, permit.binding.height) {
        return Err(DesktopCandidateEventPlanError::InvalidStart);
    }
    if permit.planned_key_events != 0 {
        return Err(DesktopCandidateEventPlanError::UnsupportedKeyEvents);
    }
    let target_count = u8::try_from(permit.selected_candidates.len())
        .map_err(|_| DesktopCandidateEventPlanError::InvalidPermit)?;
    let minimum_pointer_events = target_count
        .checked_mul(3)
        .ok_or(DesktopCandidateEventPlanError::InsufficientPointerBudget)?;
    if permit.planned_pointer_events < minimum_pointer_events {
        return Err(DesktopCandidateEventPlanError::InsufficientPointerBudget);
    }
    if duration_ms < u64::from(permit.planned_pointer_events) {
        return Err(DesktopCandidateEventPlanError::InvalidSchedule);
    }
    let ends_at_ms = started_at_ms
        .checked_add(duration_ms)
        .ok_or(DesktopCandidateEventPlanError::InvalidSchedule)?;
    if started_at_ms >= permit.expires_at_ms || ends_at_ms > permit.expires_at_ms {
        return Err(DesktopCandidateEventPlanError::Stale);
    }

    let button_events = target_count
        .checked_mul(2)
        .ok_or(DesktopCandidateEventPlanError::InvalidPermit)?;
    let move_events = permit
        .planned_pointer_events
        .checked_sub(button_events)
        .ok_or(DesktopCandidateEventPlanError::InsufficientPointerBudget)?;
    let base_moves = move_events / target_count;
    let extra_moves = move_events % target_count;
    let mut events = Vec::with_capacity(usize::from(permit.planned_pointer_events));
    let mut current = pointer_start;
    for (target_index, candidate) in permit.selected_candidates.iter().enumerate() {
        let target_index = u8::try_from(target_index)
            .map_err(|_| DesktopCandidateEventPlanError::InvalidPermit)?;
        let moves_for_target = base_moves + u8::from(target_index < extra_moves);
        for step in 1..=moves_for_target {
            let point = interpolate(current, candidate.center, step, moves_for_target)?;
            events.push(InputEvent::PointerMove {
                point,
                at_ms: scheduled_at(
                    started_at_ms,
                    duration_ms,
                    events.len() + 1,
                    usize::from(permit.planned_pointer_events),
                )?,
            });
        }
        current = candidate.center;
        events.push(InputEvent::LeftDown {
            at_ms: scheduled_at(
                started_at_ms,
                duration_ms,
                events.len() + 1,
                usize::from(permit.planned_pointer_events),
            )?,
        });
        events.push(InputEvent::LeftUp {
            at_ms: scheduled_at(
                started_at_ms,
                duration_ms,
                events.len() + 1,
                usize::from(permit.planned_pointer_events),
            )?,
            emergency: false,
        });
    }
    if events.len() != usize::from(permit.planned_pointer_events) {
        return Err(DesktopCandidateEventPlanError::InvalidPermit);
    }

    let mut plan = DesktopCandidateEventPlan {
        source_permit_digest: permit.permit_digest.clone(),
        pointer_start,
        events,
        started_at_ms,
        duration_ms,
        expires_at_ms: permit.expires_at_ms,
        plan_digest: String::new(),
    };
    plan.plan_digest = desktop_candidate_event_plan_digest(&plan);
    Ok(plan)
}

/// Digests every event-plan field except the digest field itself.
pub fn desktop_candidate_event_plan_digest(plan: &DesktopCandidateEventPlan) -> String {
    let mut parts = vec![
        plan.source_permit_digest.clone(),
        plan.pointer_start.x.to_string(),
        plan.pointer_start.y.to_string(),
        plan.started_at_ms.to_string(),
        plan.duration_ms.to_string(),
        plan.expires_at_ms.to_string(),
    ];
    for event in &plan.events {
        match event {
            InputEvent::PointerMove { point, at_ms } => {
                parts.push("pointer_move".to_string());
                parts.push(point.x.to_string());
                parts.push(point.y.to_string());
                parts.push(at_ms.to_string());
            }
            InputEvent::LeftDown { at_ms } => {
                parts.push("left_down".to_string());
                parts.push(at_ms.to_string());
            }
            InputEvent::LeftUp { at_ms, emergency } => {
                parts.push("left_up".to_string());
                parts.push(at_ms.to_string());
                parts.push(if *emergency { "1" } else { "0" }.to_string());
            }
            InputEvent::KeyDown { key, at_ms } => {
                parts.push("key_down".to_string());
                parts.push(key.to_string());
                parts.push(at_ms.to_string());
            }
            InputEvent::KeyUp {
                key,
                at_ms,
                emergency,
            } => {
                parts.push("key_up".to_string());
                parts.push(key.to_string());
                parts.push(at_ms.to_string());
                parts.push(if *emergency { "1" } else { "0" }.to_string());
            }
        }
    }
    digest_parts(&parts)
}

fn point_on_surface(point: PixelPoint, width: u32, height: u32) -> bool {
    point.x >= 0 && point.y >= 0 && point.x < i64::from(width) && point.y < i64::from(height)
}

fn interpolate(
    start: PixelPoint,
    end: PixelPoint,
    step: u8,
    steps: u8,
) -> Result<PixelPoint, DesktopCandidateEventPlanError> {
    if steps == 0 || step == 0 || step > steps {
        return Err(DesktopCandidateEventPlanError::InvalidSchedule);
    }
    let interpolate_axis = |start: i64, end: i64| {
        let start = i128::from(start);
        let delta = i128::from(end) - start;
        let value = start + delta * i128::from(step) / i128::from(steps);
        i64::try_from(value).map_err(|_| DesktopCandidateEventPlanError::InvalidSchedule)
    };
    Ok(PixelPoint {
        x: interpolate_axis(start.x, end.x)?,
        y: interpolate_axis(start.y, end.y)?,
    })
}

fn scheduled_at(
    started_at_ms: u64,
    duration_ms: u64,
    event_index: usize,
    event_count: usize,
) -> Result<u64, DesktopCandidateEventPlanError> {
    if event_count == 0 || event_index == 0 || event_index > event_count {
        return Err(DesktopCandidateEventPlanError::InvalidSchedule);
    }
    let offset = u128::from(duration_ms)
        .checked_mul(event_index as u128)
        .ok_or(DesktopCandidateEventPlanError::InvalidSchedule)?
        / event_count as u128;
    let offset =
        u64::try_from(offset).map_err(|_| DesktopCandidateEventPlanError::InvalidSchedule)?;
    started_at_ms
        .checked_add(offset)
        .ok_or(DesktopCandidateEventPlanError::InvalidSchedule)
}

fn digest_parts(parts: &[String]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
