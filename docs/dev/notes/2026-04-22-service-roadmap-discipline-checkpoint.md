# Service Roadmap Discipline Checkpoint

Date: 2026-04-22

## Decision

Pause further dashboard expansion until backend service state is authoritative.

The dashboard work completed on `service-model-phase-0` is useful scaffolding:
it validates the service entities, provides operator visibility, and exposes
the shape of future operations workflows. It should not become the main
implementation lane until the service itself owns live browser truth.

## Why

The roadmap says agent-browser should become a durable browser control plane.
That means the service must be the authority for:

- browser process lifecycle
- CDP connections and targets
- session leases
- tab lifecycle and ownership
- queued browser control requests
- health and crash state
- event history

Recent frontend work moved faster than the backend authority. The Service view
can inspect browsers, sessions, tabs, and events, but some of that data is
currently persisted contract state rather than live-reconciled service truth.
Continuing to add dashboard panels before strengthening reconciliation and
control-plane ownership risks building UI around assumptions.

## Near-Term Rule

Do not add new dashboard service surfaces unless the data is already backed by
service-owned state or the slice includes the backend work that makes it so.

Allowed dashboard work:

- fixing regressions in existing Service view scaffolding
- small UI adjustments needed to verify backend service state
- rendering newly authoritative backend fields added in the same slice

Avoid for now:

- new jobs, monitors, challenge, provider, auth-flow, or policy panels
- workflow controls that imply service authority before leases/jobs enforce it
- frontend-only representations of entities that are not live-reconciled

## Backend-First Next Slice

Return to Phase 0 and Phase 1 of the roadmap.

The next implementation slice should make service state more real by
reconciling live CDP target state into `ServiceState.tabs` and relationships:

- discover targets for known live browsers
- update tab records with target ID, URL, title, lifecycle, browser ID, and
  owning session where known
- remove or mark stale tab records when targets disappear
- update browser/session tab relationships where the service has enough
  information
- emit bounded service events for meaningful tab lifecycle changes
- expose the reconciled state through the existing CLI, HTTP API, and dashboard

## Validation Standard

Backend authority slices should include at least:

- unit tests for state reconciliation behavior
- targeted Rust tests for service commands or control-plane helpers
- one live smoke when Chrome or CDP behavior is involved
- dashboard smoke only when the slice changes visible fields

The dashboard should now be treated as a consumer of service truth, not the
driver of the service design.

## Progress

### 2026-04-22

The first backend-first correction is implemented on `service-model-phase-0`:

- reachable persisted browser CDP endpoints are queried through `/json/list`
- live page and webview targets are reconciled into `ServiceState.tabs`
- stale tab records for a reconciled browser are marked `closed`
- known active browser sessions are linked to discovered tab IDs
- reconciliation event details include `tabCount` and `changedTabs`
- normal auto-launching browser commands now persist service browser records,
  so a plain `open` followed by `service reconcile` can discover live tabs from
  the stored service state
- daemon-launched sessions now enable background service reconciliation every
  60000 ms by default, while `0` remains the explicit opt-out
- tab reconciliation now emits `tab_lifecycle_changed` events for discovered
  tabs, lifecycle changes, URL/title changes, and closed tabs
- control-plane requests now persist bounded service job records with action,
  priority, timestamps, final state, and error text
- `service jobs` and `GET /api/service/jobs` expose recent job records without
  requiring agents or operators to parse the full service status payload
- `service jobs --id <job-id>` and `GET /api/service/jobs/<job-id>` expose one
  retained job for dashboard and operator detail inspection
- `service cancel <job-id>` and `POST /api/service/jobs/<job-id>/cancel`
  conservatively cancel queued jobs before dispatch; running jobs are not
  interrupted until active-request cancellation is designed
- `service.jobTimeoutMs`, `--service-job-timeout`, and
  `AGENT_BROWSER_SERVICE_JOB_TIMEOUT_MS` add opt-in worker-bound timeouts that
  mark long-running dispatched jobs as `timed_out`

This deliberately uses the existing service status, reconcile, HTTP API, and
dashboard surfaces instead of adding more dashboard-only state.
