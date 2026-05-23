# Service Inspector Pane Plan

Date: 2026-05-23

## Trigger

The Service dashboard is now more usable than the earlier stacked-card version,
but the right inspection pane still reads as a pile of generic panels. It does
not yet answer the operator's core questions quickly:

- What selected record am I looking at?
- Is it healthy, live, retained, stale, or blocked?
- Who or what owns it?
- What can I safely do from here?
- What evidence supports those conclusions?

This note records the intended design before further implementation. The goal
is to make the inspection pane a selected-record command surface while keeping
the dashboard a consumer of service-owned state.

## Source Context

Relevant repo and memory context:

- `docs/dev/notes/2026-04-22-service-roadmap-discipline-checkpoint.md`
  requires dashboard surfaces to consume authoritative service state rather
  than inventing frontend-only service semantics.
- `docs/dev/notes/2026-05-18-service-view-redesign-plan.md` defines the
  Service view as an operations console with live state first and retained
  history second.
- `docs/dev/notes/2026-05-19-service-dashboard-ux-audit-design-plan.md`
  identifies the right inspector as the correct list-detail pattern, but notes
  that selection, row relationship, and empty-state copy need to be clearer.
- Graphiti discovery for `agent_browser_main` on 2026-05-23 was healthy and
  returned the same backend-first authority rule for service state.

Current code evidence:

- `packages/dashboard/src/components/service-panel.tsx` routes browser,
  profile, incident, session, tab, and job selections through
  `ServiceDetailInspector`.
- Detail renderers still rely heavily on `service-event-detail-grid` and
  `EventDetailItem`, which makes browser, session, tab, profile, and job
  records look like loosely grouped facts instead of operational objects.
- Incident detail is closer to the desired shape because it already highlights
  message, priority, recommended action, and timeline before raw fields.

## Product Principle

The inspector should answer one selected-record question at a time:

> What is this record, what state is it in, who owns it, what can I do, and
> what evidence should I trust?

It should not be a mini dashboard. It should not repeat every table column as a
panel. It should not make raw identifiers visually dominant unless the operator
opens evidence detail.

## Shared Inspector Layout

Every record type should use the same high-level structure.

### 1. Hero Summary

The top of the inspector should provide a compact operational summary:

- record kind, such as Browser, Profile, Tab, Session, Job, or Incident
- human label or best available identifier
- health, lifecycle, or state chip
- one sentence describing why the record matters
- ownership line with service, agent, task, profile, browser, or session when
  available

The hero is the place for human comprehension. Raw UUID-style values belong in
evidence unless no better label exists.

### 2. Primary Action Bar

Immediately under the hero, show the actions an operator is most likely to take.
Actions should be visible only when the service contract supports them and the
record has enough information to make the action meaningful.

Candidate actions:

- view or control browser
- focus tab
- terminate or close when supported
- cancel queued or running job
- acknowledge or resolve incident
- jump to related profile, browser, tab, job, session, incident, or trace
- copy URL, ID, or handoff command when useful

Disabled actions should explain why they are unavailable. The dashboard should
not pretend it can mutate retained history that no longer maps to a live service
object.

### 3. Operational Sections

Replace the field-card grid with two to four semantic sections. Sections should
be dense rows or compact cards, not one panel per field.

Recommended base sections:

- Runtime: executable, browser build, platform, host, process, profile path, or
  storage posture
- Control: health, control mode, CDP endpoint, view stream readiness, display,
  queue state, cancellation eligibility, or remote-control capability
- Ownership: service, agent, task, profile, browser, session, account identity,
  lease holder, or requester
- Activity: last observed timestamp, last state transition, last error,
  incident timeline, job timing, or related events

If a section has no useful data for a selected record, omit it rather than
showing blank rows.

### 4. Evidence Disclosure

Put raw records, long JSON, full endpoint URLs, and dense ID lists behind an
Evidence disclosure. Evidence remains available for agents and debugging, but
it should not dominate the default human view.

Evidence should include:

- raw service record JSON when available
- full identifiers
- full CDP and view-stream endpoint values
- request and response JSON for jobs
- allocation JSON for profiles
- related event lists or trace filters

## Record-Specific Design

### Browser Inspector

Primary question: Can this browser process or retained browser record be viewed,
controlled, repaired, or closed?

Default view:

- Hero: browser label, health chip, executable badge, platform badge, live or
  retained posture, profile summary
- Actions: View/control when a view stream exists, close or terminate when the
  service exposes that action, show related tabs, show related sessions, show
  incidents, open trace
- Runtime: executable, browser build, host, platform, PID, process state,
  runtime profile
- Control: health, CDP endpoint presence, control mode, display isolation,
  display, primary view stream, primary input stream
- Ownership: active sessions, service or task labels when known, profile
  allocation
- Evidence: full active-session IDs, view stream records, raw retained browser
  record

Implementation priority: convert this first. It is the highest-traffic
inspector path and will prove the shared primitives.

### Profile Inspector

Primary question: Which site or account identity does this profile serve, what
browser family should host it, and is it ready for the requested target?

Default view:

- Hero: profile label, readiness chip, browser preference, identity summary
- Actions: show related browsers, show leases, show target readiness, open
  readiness trace, edit profile once runtime profile CRUD is available
- Identity and routing: site identity, account identity, preferred browser,
  compatible browser families, primary host
- Storage and security: runtime profile path, external path posture, keyring
  policy, browser family, user-data isolation
- Readiness: seeding state, authenticated targets, last verified service,
  pending verification or monitor attention
- Leases and conflicts: current holder, waiting requesters, conflict reason,
  stale lease candidates
- Evidence: allocation JSON, profile-source provenance, raw profile record

### Tab Inspector

Primary question: What page is this tab showing, who owns it, and can I focus or
view it?

Default view:

- Hero: title or URL, lifecycle chip, browser and session ownership
- Actions: focus tab, view/control containing browser, copy URL, show related
  browser, show related job or trace
- Page: title, URL, origin, target service hint, login identity hint
- Control: tab lifecycle, control mode, focus availability, view-stream
  relationship, last observed
- Ownership: browser, session, service, agent, task
- Evidence: tab ID, target ID, raw tab record, related events

### Session Inspector

Primary question: Who holds this service session and what browser, profile, and
tabs does it bind together?

Default view:

- Hero: session label, active or retained state, owner summary
- Actions: show browser, show profile, show tabs, show trace
- Lease: session state, holder, service, agent, task, acquisition time,
  release or expiry time
- Bindings: browser IDs, profile ID, tab IDs, target identity hints
- Health: active lease status, abandoned candidate status, related incidents
- Evidence: raw session record and related job or event IDs

### Job Inspector

Primary question: Is this job still actionable, what requested it, and what did
it do?

Default view:

- Hero: action name, queued/running/done state chip, requester summary
- Actions: cancel when queued or running and supported, show target record,
  show trace, copy handoff
- Request: action, service, agent, task, display allocation, target browser,
  profile, session, tab, or URL
- Timing: queued, started, finished, elapsed, timeout or cancellation posture
- Outcome: success, error, result summary, related incident
- Evidence: request JSON, response JSON, raw job record

### Incident Inspector

Primary question: What needs operator attention and what remedy should be
performed?

Default view:

- Hero: incident kind, severity, handling state, escalation
- Actions: acknowledge, resolve, show remedy group, show related browser or
  monitor, show trace
- Recommendation: recommended action in plain language, affected records,
  expected remedy ladder
- Timeline: first seen, latest seen, state changes, related events and jobs
- Ownership: service, agent, task, browser, profile, session, monitor
- Evidence: raw incident record, related event and job records

Incident detail is already closest to this model. It should be refactored last
unless shared primitives require small alignment.

## Visual Direction

The inspector should feel like a dense operations sidebar:

- calm hero summary at the top
- one action row with explicit affordances
- compact semantic sections with row labels and values
- subtle dividers instead of a card around every field
- monospace only for IDs, paths, endpoints, and commands
- long values truncated with copy affordances or evidence expansion
- no hover lift on non-actions
- no native browser dialogs
- keyboard-visible focus for every action and disclosure

The design should preserve the existing dashboard visual language, but remove
the "pile of fat panels" motif from detail inspection.

## Implementation Slices

### Slice 1: Shared Primitives And Browser Conversion

Add shared inspector primitives in the dashboard component layer:

- `InspectorHero`
- `InspectorActionBar`
- `InspectorSection`
- `InspectorFactRows`
- `InspectorEvidenceDisclosure`

Convert the Browser inspector to these primitives. Keep behavior equivalent
except for layout and copy. Preserve the existing remote-control action and
view-stream readiness behavior.

Validation:

- `pnpm test:dashboard-inspector-actions`
- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-view-streams`
- `pnpm build:dashboard`
- `git diff --check`

### Slice 2: Profile And Tab Conversion

Convert profile and tab inspectors after the browser path proves the structure.
This slice should make profile readiness and tab focus/view relationships clear
without adding new backend authority.

Validation:

- `pnpm test:dashboard-profile-allocation`
- `pnpm test:dashboard-inspector-actions`
- `pnpm build:dashboard`
- `git diff --check`

### Slice 3: Session And Job Conversion

Convert session and job inspectors. Keep cancellation guarded by existing
service contracts. Do not add frontend-only assumptions about whether retained
jobs or sessions are mutable.

Validation:

- `pnpm test:dashboard-inspector-actions`
- `pnpm test:dashboard-browser-table`
- `pnpm build:dashboard`
- `git diff --check`

### Slice 4: Incident Alignment And Related-Record Navigation

Align incident detail with the shared primitives and add related-record jump
actions where the selected data already provides stable IDs.

Validation:

- `pnpm test:dashboard-inspector-actions`
- focused incident dashboard test if a fixture exists or is added
- `pnpm build:dashboard`
- `git diff --check`

## Non-Goals

- Do not add new service semantics in the dashboard.
- Do not make profile CRUD part of the inspector refactor unless the service API
  already exposes the operation.
- Do not add destructive browser, session, profile, or job actions without
  service-contract support and confirmation where appropriate.
- Do not hide raw evidence entirely. Move it out of the default scan path.
- Do not block backend roadmap work on perfect inspector polish.

## Recommended Next Step

Implement Slice 1 only: shared inspector primitives plus Browser inspector
conversion. That creates the reusable shape, addresses the most visible
operator pain, and limits risk before touching profile, tab, session, job, and
incident-specific behavior.
