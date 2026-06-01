# Workspace Tab Field Map

Date: 2026-06-01
Plan: `docs/dev/plans/0019-2026-06-01-workspace-tab-dense-inspector-plan.md`

## Available Now

The Workspace tab can source these fields from the selected-workspace context
and `WorkspaceNode` model:

- identity: workspace ID, browser ID, service session ID, daemon session name,
  profile ID, tab ID, target ID, source, and label
- state: state, health, live, retained, viewable, controllable, missing reason,
  attention reason, diagnostics, incidents, and jobs
- process: PID, running flag, resident memory, CPU seconds, CDP port, and
  stream port
- page: title, URL, lifecycle, active flag, tab ID, and target ID
- stream: provider, route summary, stream URL, embeddability, control input,
  controllability, viewer leases, controller lease, connection ID, route ID,
  and display allocation ID
- ownership: service name, agent name, task name, profile allocation, and
  related job or incident IDs
- actions: advertised workspace actions with enabled state and unavailable
  reason

## Unavailable Or Partial

- True browser process uptime is not reported by the selected service status
  model. The dense Workspace tab must label it `not reported` until backend
  service state exposes a process start timestamp or uptime field.
- Last frame age is represented in the selected context shape but is not wired
  by `useSelectedWorkspaceContext` yet. The tab must label it `not reported`
  unless a stream-port timestamp is supplied.
- Repair, close, kill, launch, seed, resume, and add-tab are represented as
  workspace actions, but the compact Workspace pane should only run actions
  that are already wired through this component. Other advertised actions must
  show an unavailable reason rather than pretending they execute.

## Presentation Decision

The Workspace tab should use a compact header strip, small runtime indicators,
one action row, and a dense fact grid. Raw IDs, stream URLs, diagnostics, and
related records belong in a collapsed Evidence disclosure so the first screen
remains operator-focused.
