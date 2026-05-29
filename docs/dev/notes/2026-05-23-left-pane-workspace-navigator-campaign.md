# Left Pane Workspace Navigator Campaign

Date: 2026-05-23

## Trigger

The current dashboard left pane is still a raw session and tab tree. It exposes
implementation details before it helps a human operator decide what browser
work is running, what needs attention, which profile or service owns it, and
what action is safe.

The campaign target is larger than a visual left-pane cleanup. The end state is
that a human can open the dashboard, choose a supported browser and profile
combination, launch or focus the matching service-owned browser, and interact
with it through the embedded dashboard viewport when a controllable remote
view stream is available.

This is a campaign-level plan. It should not be treated as the next immediate
slice unless a maintainer explicitly selects it.

## Source Context

Relevant roadmap and checkpoint sources:

- `docs/dev/notes/2026-04-22-agent-browser-service-roadmap.md` says
  agent-browser should own browser lifecycle, profiles, CDP connections,
  sessions, tabs, control queues, live viewing, and operator intervention.
- `docs/dev/notes/2026-05-19-service-dashboard-ux-audit-design-plan.md`
  defines the dashboard as an operations console and says Guacamole, noVNC,
  and other providers should stay behind dashboard-owned view chrome.
- `docs/dev/notes/2026-05-20-remote-view-control-posture-checkpoint.md`
  records that access-plan owns `viewStreamProvider` and
  `controlInputProvider`, and that dashboard control should queue
  `view_focus` before opening an embedded stream.
- `docs/dev/notes/2026-05-23-service-inspector-pane-plan.md` records the
  selected-record inspector and direct route follow-up as completed, including
  the route and density fixes that made `/service` usable as a stable entry.
- Graphiti discovery for `agent_browser_main` on 2026-05-23 was healthy and
  returned the backend-first authority rule: dashboard expansion should be
  grounded in service-owned state.

## Campaign Objective

Turn the left pane into the dashboard's workspace navigator.

A workspace is the operator-facing unit that joins service-owned facts about:

- browser process or retained browser record
- active or retained service session
- current tabs and tab lifecycle
- selected profile and profile allocation
- service, agent, and task ownership
- health and incident state
- queued jobs and current control posture
- view-stream and control-input readiness

The navigator should answer:

- What browser work exists right now?
- What is live, blocked, stale, or retained?
- Which profile, account, service, agent, or task owns it?
- Which browser/profile combinations can I launch safely?
- What should I click to view or control the selected browser?
- What action is blocked, and what service-sourced remedy is recommended?

## End Of Campaign Deliverables

- Replace `SessionTree` with a workspace navigator that has active,
  attention-needed, and retained groupings.
- Build a derived `WorkspaceNode` model from service-owned browser, session,
  tab, profile, allocation, readiness, job, incident, and view-stream state.
- Preserve existing session, tab, close, kill, and create behavior during the
  migration, but make raw session IDs secondary to human labels.
- Add URL-persisted workspace selection for workspace, browser, session, tab,
  profile, and top-level route.
- Add a guided launcher that lets an operator choose browser host or build,
  runtime profile, target site or identity hints, and launch posture.
- Drive launcher eligibility through access-plan, profile readiness, browser
  capability, and service-request contracts.
- Show incompatible or blocked browser/profile combinations with disabled
  actions and service-owned explanations.
- Launch supported combinations through the existing service request queue
  rather than direct dashboard-side process control.
- After launch, focus the resulting browser or tab and open the embedded
  remote viewport when `viewStreams` reports an embeddable and controllable
  stream.
- Treat `rdp_gateway` and Guacamole-backed streams as providers behind the
  dashboard viewport, with fullscreen and external-open fallback.
- Model human control as an operator takeover path with visible queue, lease,
  and resume semantics.
- Update README, docs site, `cli/src/output.rs`, `skills/agent-browser/SKILL.md`,
  and relevant inline comments when user-facing launcher or viewport behavior
  lands.

## Campaign Non-Goals

- Do not make the dashboard choose browser/profile compatibility by itself.
- Do not bypass service request queueing for launch, focus, close, repair, or
  takeover actions.
- Do not make Guacamole or any raw stream provider the main dashboard route.
- Do not silently launch a profile that service readiness says needs manual
  seeding, has incompatible browser-family evidence, or is under an exclusive
  lease conflict.
- Do not hide retained history completely. It should be collapsed and
  explainable, not deleted from the operator model.
- Do not add destructive actions without backend contract support, eligibility
  checks, and confirmation where appropriate.

## Design Contract

Freeze these product contracts before Slice 2 starts. Later slices may extend
them, but they should not redesign the navigator without a new planning note.

### Workspace Row Anatomy

Each workspace row should have a stable, compact anatomy:

- primary label from service, agent, task, target site, or profile, in that
  order of usefulness
- secondary label for browser family, runtime profile, account or login hint,
  active tab title, and retained state
- state badge for live, busy, blocked, needs seeding, retained, view-only, or
  controllable
- one primary action, usually Focus, View, Launch, Resume, or Seed
- overflow actions for less frequent close, kill, detach, copy link, and
  external-open behavior

Raw session, tab, browser, and profile IDs may be available for inspection, but
they must not be the first thing a human has to parse.

### Workspace State Taxonomy

The navigator should use a small state set shared across rows, filters, badges,
empty states, and tests:

- `active`: service-owned browser or session is live and selectable
- `busy`: service-owned job or queue state is changing the workspace
- `needs-attention`: operator action is required before automation can proceed
- `blocked`: service-owned authority says the requested action is currently
  unsafe or unsupported
- `retained`: service state exists, but no live controllable browser is present
- `view-only`: an embeddable stream exists without input control
- `controllable`: an embeddable stream exists with supported input control

Do not add one-off display states when one of these states plus a service-owned
reason can explain the row.

### URL Selection Schema

URL state should be stable enough for refresh, direct links, and back/forward
navigation:

- `view`: top-level dashboard route or mode when route alone is not enough
- `workspace`: derived workspace node ID
- `browser`: service browser ID when the selected workspace has one
- `session`: service or daemon session ID when the selected workspace has one
- `tab`: selected tab ID or retained tab key
- `profile`: runtime or service profile ID

The selected route and query must restore the same visible navigator row,
center content, and inspector target after a refresh. Internal query parameters
may exist during implementation, but the handoff for Slice 3 must name any that
are not stable.

### Launcher Contract

The launch surface must make the safe path obvious without hiding blocked
paths:

- browser host or build, runtime profile, target site, identity hints, display
  isolation, view-stream preference, and control-input preference are separate
  choices
- disabled choices remain visible with service-sourced reasons
- the UI never marks a combination launchable without access-plan, profile
  readiness, browser capability, and service-request evidence
- submission goes through the service request queue and returns to the
  selected workspace context

### Viewport Contract

The embedded viewport is dashboard chrome around a service-owned stream:

- provider names such as Guacamole, noVNC, or `rdp_gateway` stay secondary to
  the workspace and selected browser
- opening a controllable viewport queues the service-owned focus behavior
  before embedding when that contract is available
- view-only, controllable, unavailable, and external-open states are visually
  distinct
- service records and job posture remain visible enough that the viewport is
  not mistaken for the source of truth

### Density And Ergonomics Contract

The dashboard is an operations console. It should prioritize scan density,
stable layout, and repeated action over decorative composition:

- the first viewport must show useful workspace rows or service records without
  requiring the operator to scroll past oversized summary chrome
- left-pane controls should fit in a compact toolbar or segmented control row
  instead of stacking large descriptive blocks
- rows should keep stable height across hover, loading, badge, and disabled
  states
- button labels must not wrap awkwardly or cause row height shifts
- mobile layouts may collapse controls, but they must preserve route state,
  selection, and the primary action

### Visual QA Contract

Every UI-affecting slice must include rendered `agent-browser` inspection while
the work is in progress, not only at closeout. At minimum, inspect desktop and
mobile-width views when the changed surface can render locally.

Each rendered review must check:

- first-viewport usefulness and row density
- no incoherent overlap between text, controls, tables, or viewport chrome
- selected routes survive refresh
- top chrome does not push the first meaningful rows below the fold
- disabled and blocked states are understandable from service-sourced copy
- screenshots or recorded artifact paths are listed in the slice handoff

Screenshots should live outside the repo, usually under
`/tmp/agent-browser-dashboard-<slice>/`. A slice that cannot run rendered
inspection must record the blocker and the smallest follow-up needed to make it
inspectable.

## Policy Contract For Every Slice

Each slice in this campaign must follow the adopted repo policy:

- Start by checking `git status --short` and treating pre-existing dirty state
  as a constraint.
- Keep one bounded branch or worktree scope for the slice.
- Keep commits coherent and truthful. Do not mix unrelated refactors or
  release work into a navigator slice.
- Re-read planning policy before changing this campaign plan or roadmap
  authority.
- Re-read validation and closeout policy before claiming a slice complete.
- Use Graphiti discovery at the start of non-trivial planning, architecture,
  debugging, or handoff work, then verify claims against repo files or tests.
- Preserve service-owned authority. Dashboard code may derive presentation
  models, but it must not invent mutable service truth.
- Run `pnpm validation:select -- --base <ref>` for the touched slice and either
  run the recommended checks or record why a recommendation is not applicable.
- For UI-affecting changes, run rendered `agent-browser` inspection throughout
  the slice and preserve screenshot paths in the handoff.
- Explicitly check density, aesthetics, route persistence, and ergonomic
  interaction before calling dashboard UX complete.
- For user-facing behavior, update the required docs surfaces listed in
  `AGENTS.md`: CLI help, README, installed skill copy, docs site, and relevant
  inline comments.
- Close every slice with concrete validation evidence and remaining risk.

## Planning Slices

### Slice 1: Workspace Node Model

Goal: create the data model that lets the left pane show workspaces instead of
raw sessions.

Scope:

- Add a pure derived `WorkspaceNode` model in the dashboard layer.
- Merge `/api/sessions`, tab cache, service browsers, service sessions,
  service tabs, profile allocations, jobs, incidents, and view-stream metadata.
- Produce stable IDs, labels, health state, attention reason, retained/live
  posture, selected profile, ownership, primary tab, and available actions.
- Add fixture-driven tests for common states: live browser, retained browser,
  disconnected browser, profile conflict, auth-ready profile, manual-seeding
  required profile, and controllable remote-headed browser.

Validation:

- `pnpm test:dashboard-browser-table`
- new focused workspace-node test
- `pnpm build:dashboard`
- rendered `agent-browser` inspection only if this slice adds visual fixtures
  or placeholder navigator rendering
- `git diff --check`
- `pnpm validation:select -- --base <ref>`

Handoff:

- Record what state is still inferred for display only.
- Record which missing service fields would make the model cleaner.
- If rendered inspection was not applicable, say why.

### Slice 2: Workspace Navigator Refactor

Goal: replace the raw session tree with a navigator that is useful before
launching anything new.

Scope:

- Replace `SessionTree` with `WorkspaceNavigator`.
- Group rows into Active, Needs attention, and Retained.
- Keep existing tab switch, add tab, close session, kill session, and close-all
  actions working through existing atoms or service contracts.
- Make raw session IDs secondary to service, agent, task, profile, URL, and tab
  labels.
- Add search and scope controls for active, attention, and all workspaces.
- Make collapsed left-pane state a usable icon rail with health badges and a
  new-workspace action.

Validation:

- `pnpm test:dashboard-inspector-actions`
- `pnpm test:dashboard-browser-table`
- new focused navigator render test
- `pnpm build:dashboard`
- rendered `agent-browser` desktop and mobile smoke at `/service`, including
  first-viewport row density and collapsed rail checks
- `git diff --check`

Handoff:

- Include screenshots for desktop and mobile.
- Note whether retained rows remain dense enough with large service state.

### Slice 3: Selection And URL Persistence

Goal: make refresh, direct links, and row clicks preserve operator context.

Scope:

- Persist selected workspace, browser, session, tab, and profile in URL query
  state where appropriate.
- Keep top-level routes stable: `/service`, `/browsers`, `/activity`, and `/`.
- Synchronize left-pane selection with center view and right inspector.
- Make browser, tab, profile, session, and job related-record jumps update the
  navigator selection when the target has a workspace node.

Validation:

- `pnpm test:dashboard-inspector-actions`
- new route-selection test
- `pnpm build:dashboard`
- rendered `agent-browser` smoke proving reload and browser back/forward return
  to the selected workspace without resetting to the home view
- `git diff --check`

Handoff:

- Record any query parameters treated as internal and subject to change.
- Record browser back/forward behavior that was manually verified.

### Slice 4: Launch Eligibility And Access Plan Preview

Goal: show which browser/profile combinations can be launched before the user
clicks Launch.

Scope:

- Add a launcher data source that reads service profiles, profile allocations,
  readiness rows, browser capability registry, access-plan responses, and
  service request contract metadata.
- Model browser host/build options separately from runtime profiles.
- Show eligible, blocked, and needs-operator-action combinations.
- Use service-owned reasons for disabled choices: incompatible browser family,
  missing capability validation, exclusive lease conflict, stale readiness,
  manual seeding required, or unsupported service request action.
- Add no-launch tests that prove the dashboard does not mark an incompatible
  combination as launchable without service evidence.

Validation:

- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- new launcher eligibility test
- `pnpm build:dashboard`
- rendered `agent-browser` inspection of eligible, disabled, blocked, and
  needs-operator-action launcher states
- `git diff --check`

Handoff:

- Record any backend contract gaps found while building eligibility.
- Do not implement the mutating launch button in this slice unless explicitly
  approved as part of the same bounded change.

### Slice 5: Guided Browser/Profile Launch

Goal: let a human launch a supported browser/profile combination through the
service queue.

Scope:

- Add a guided launch dialog or pane from the workspace navigator.
- Inputs: browser host or build, runtime profile or service profile, target
  site or URL, login/account identity hints, display isolation, view-stream
  preference, and control-input preference when applicable.
- Submit through `POST /api/service/request` or the generated service client,
  not through dashboard-local process control.
- Use `profileLeasePolicy` and wait behavior according to access-plan advice.
- Show submitted job state in the navigator and center Service view.
- Preserve manual-seeding handoff paths for profiles that cannot be automated
  yet.

Validation:

- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- dashboard launcher action test
- `pnpm build:dashboard`
- one no-launch request-shape test
- rendered `agent-browser` inspection of the launch flow at desktop and mobile
  widths
- one bounded live launch smoke only if launch selection behavior changed
- `git diff --check`

Handoff:

- Include the exact service request shape used by the UI.
- Record whether the live smoke used an isolated service home or the operator
  service, and avoid mutating default runtime profiles in automated tests.
- Include screenshot paths for launch dialog, pending job state, and focused
  workspace state.

### Slice 6: Guac Viewport Control Integration

Goal: after launch or selection, let the operator interact with the selected
browser through the dashboard viewport when a controllable remote stream
exists.

Scope:

- Reuse existing view-stream metadata and `view_focus` behavior.
- Promote the embedded remote viewport from row-level dialog behavior into the
  selected workspace workflow when the selected browser has an embeddable,
  controllable stream.
- Support `rdp_gateway` and Guacamole-backed URLs as provider details behind
  dashboard-owned chrome.
- Before opening the viewport, queue `view_focus` with tab index and maximize
  request when the retained tab mapping is stable.
- Show view-only, controllable, unavailable, and fallback states clearly.
- Provide fullscreen and external-open controls.
- Keep Service records and job state visible enough that the viewport is not
  mistaken for service truth.

Validation:

- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-inspector-actions`
- `pnpm test:service-remote-view-control-live`
- `pnpm test:service-dashboard-remote-control-ui-live`
- `pnpm test:rdp-gateway-readiness-live` on operator workstations that rely on
  `rdp_gateway`
- rendered `agent-browser` smoke proving launch or selection opens the
  viewport through dashboard chrome, with desktop and mobile-width screenshots
  of view-only, controllable, unavailable, and fallback states when available
- `git diff --check`

Handoff:

- Include the iframe URL provenance, provider, and control-input evidence from
  retained service state.
- Record whether the smoke proved true interaction or only readiness and iframe
  embedding.
- Include screenshot paths and note any density or overlap issues around the
  viewport chrome.

### Slice 7: Human Takeover Lease And Resume Flow

Goal: make manual control tracked and reversible.

Scope:

- Add explicit operator takeover state if backend contracts already expose it,
  or write the backend contract plan first if they do not.
- Pause, cooperative-mode, or resume behavior must be service-owned.
- Show takeover owner, started time, selected browser/tab, queue impact, and
  resume action in the navigator and inspector.
- Prevent agents and humans from unknowingly fighting over focus or input.

Validation:

- backend contract tests for any new service-owned takeover state
- dashboard action tests for takeover and resume wiring
- `pnpm build:dashboard`
- rendered `agent-browser` inspection of takeover, paused, cooperative, and
  resume states when UI lands
- live smoke only after backend takeover behavior exists
- `git diff --check`

Handoff:

- State clearly whether takeover is implemented or only planned.
- Record queue semantics for pending agent work during manual control.
- Include screenshot paths for every takeover state that has a rendered UI.

### Slice 8: Campaign Documentation And Release Readiness

Goal: make the completed campaign discoverable and supportable.

Scope:

- Update README feature summary, service mode docs, dashboard docs, CLI help,
  inline comments, and `skills/agent-browser/SKILL.md`.
- Sync the installed skill copy.
- Add a campaign completion note with screenshots, validation evidence, known
  residual risks, and install doctor evidence if a release candidate binary is
  installed locally.
- Include a final rendered UX review set proving route persistence, row
  density, launch eligibility, launch submission, focused workspace state, and
  embedded viewport ergonomics.
- Add or update validation selector recommendations for new navigator,
  launcher, and viewport tests.

Validation:

- `pnpm --dir docs build`
- `pnpm build:dashboard`
- `pnpm validation:select -- --base <ref>`
- selector-recommended dashboard and service checks
- final rendered `agent-browser` desktop and mobile inspection set
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `agent-browser install doctor` after replacing an installed candidate binary
- `git diff --check`

Handoff:

- List which end-of-campaign deliverables are complete.
- List any browser/profile combinations still blocked by backend capability,
  missing readiness evidence, or gateway deployment.
- List the final screenshot paths and any remaining visual risks.

## Suggested Campaign Branching

Use one short-lived branch for each implementation slice when possible. If the
campaign needs parallel frontend and backend work, use worktrees with explicit
slice names and reconcile through an integration branch after the backend
authority is stable.

Do not merge a dashboard-only launcher that works by bypassing service request
contracts. The backend authority slice wins over frontend convenience whenever
the two conflict.

## First Recommended Slice When This Campaign Starts

Start with Slice 1 only: the derived `WorkspaceNode` model and fixture-driven
tests. It is the lowest-risk step because it can make the left pane understand
the service model without changing launch behavior, remote control, or browser
state.
