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

## Completion Record

Status: implemented on 2026-05-23.

Completed scope:

- Added the shared inspector primitives in
  `packages/dashboard/src/components/service-panel.tsx`: `InspectorHero`,
  `InspectorActionBar`, `InspectorSection`, `InspectorFactRows`, and
  `InspectorEvidenceDisclosure`.
- Converted the selected browser, profile, tab, session, job, and incident
  right-pane detail views to a selected-record structure with a hero summary,
  primary action row, semantic sections, related-record jumps, and Evidence
  disclosure for raw records, long IDs, endpoints, and JSON payloads.
- Kept dashboard actions wired through `ServiceInspectorActions` instead of
  embedding mutable action state into selected-record data.
- Added related-record navigation handlers for browser, profile, session, tab,
  and job IDs already present in service-owned records.
- Preserved selected-browser and selected-tab remote-control behavior through
  the queued `view_focus` path before opening the embedded stream.
- Moved job cancellation, incident acknowledge/resolve, and incident trace
  actions into the shared selected-record action surface.
- Updated dashboard CSS so the inspector reads as a dense operations sidebar
  rather than one panel per field.
- Updated README, docs site pages, the repo skill, and the installed
  `agent-browser` skill copy to describe the selected-record inspector model.

Rendered QA notes:

- The Browser plugin was not available in this session, and local Playwright was
  not installed. Rendered QA used the local `dev-browser` tool against
  `http://127.0.0.1:3100`.
- Initial rendered QA found a React maximum-update-depth overlay when switching
  into the Service view. The loop was caused by the right-pane action publisher
  depending on an unmemoized `recentJobs` array. `recentJobs` is now memoized.
- Desktop Service view QA proved the page loads without framework overlay after
  the fix and that selecting a profile allocation opens the right-pane
  inspector with one hero, four semantic sections, and one Evidence disclosure.
- Evidence disclosure QA opened the selected profile Evidence section and
  verified the raw allocation JSON is available behind the disclosure.
- Mobile QA at 390 by 844 verified the Service view loads without framework
  overlay and the profile allocation detail dialog renders the same selected
  record structure with a hero and Evidence disclosure.
- Follow-up QA after visual review used the repo-owned `agent-browser` CLI
  against the local dashboard, selected a real profile allocation, and captured
  desktop and mobile screenshots. That pass confirmed the original styling was
  too close to the old detail list, so the selected-record hero, status chip,
  section groups, and Evidence disclosure were restyled to read as a distinct
  inspector surface.
- The rendered environment still logged the existing disconnected stream
  warning for `ws://localhost:9223/` and one 404 resource request during desktop
  smoke. They did not produce a framework overlay and were not introduced by the
  inspector action loop.

Validation evidence:

- `graphiti-runtime doctor` passed before implementation.
- `pnpm validation:select -- --base HEAD` selected the dashboard, docs, and
  skill-sync gates for this slice.
- `pnpm test:dashboard-view-streams` passed.
- `pnpm test:dashboard-browser-row-actions-render` passed.
- `pnpm test:dashboard-browser-table` passed.
- `pnpm test:dashboard-profile-allocation` passed.
- `pnpm test:dashboard-inspector-actions` passed.
- `pnpm test:dashboard-incident-summary` passed.
- `pnpm build:dashboard` passed.
- `pnpm --dir docs build` passed.
- `agent-browser` desktop screenshot:
  `/tmp/agent-browser-service-inspector-qa/profile-inspector-after.png`.
- `agent-browser` mobile screenshot:
  `/tmp/agent-browser-service-inspector-qa/profile-inspector-mobile-dialog-after.png`.
- `git diff --check` passed.
- `diff -q skills/agent-browser/SKILL.md
  /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md` passed.
- `node scripts/dev/select-validation.js --base HEAD --json` passed and
  returned the same recommended gate list.

## Route and Density Follow-up

Status: implemented on 2026-05-23.

Follow-up scope:

- Added direct dashboard routes for `/service`, `/browsers`, and `/activity`.
- Preserved the active Service workspace through `/service?workspace=<name>`.
- Defaulted the Service workspace to `browsers` so browser records appear first.
- Removed the Service panel hero and helper copy that consumed first-viewport
  space before the managed browser records.
- Compacted the status strip, retained-state alert, workspace tabs, workspace
  header, profile routing strip, and record content padding.

Rendered QA:

- Used the repo-owned `agent-browser` CLI by attaching to the existing default
  runtime profile with `--runtime-profile default`.
- Desktop `/service` rendered with the Browsers workspace selected, 50 browser
  rows in the DOM, and the first table row visible in the first viewport.
- `/service?workspace=jobs` retained the Jobs workspace after reload.
- `/browsers` retained the Browsers section after reload.
- Mobile `/service` rendered the managed browser row card inside the first
  viewport.
- Screenshots:
  `/tmp/agent-browser-dashboard-route-qa/service-desktop-after-compact.png` and
  `/tmp/agent-browser-dashboard-route-qa/service-mobile-after-compact.png`.
