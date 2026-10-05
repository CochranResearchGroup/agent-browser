# Plan 0087: Remote-View Blank-Target Acquisition

State: CLOSED

Created: 2026-07-31

Lane: distinct live successor to Plan 0086

## Goal

Keep slow destination loading out of `Target.createTarget` setup by creating and
attaching a blank route-bound tab first, then using the existing no-wait target
navigation and exact-target readback path.

## Evidence

- Plan 0086 proved command-level `jobTimeoutMs=90000` reached the live job.
- Reddit and LinkedIn collection successors still failed after about 15 seconds.
- Both runs recorded the intended tab URL before cleanup, showing the target
  existed while `handle_tab_new` still returned a CDP operation timeout.
- `BrowserManager::tab_new` currently creates the target at the destination URL
  before enabling domains and activating it.

## Bounds

- Change only `remote_view_open` new-target acquisition; ordinary `tab new`
  behavior remains unchanged.
- Create `about:blank`, then retain the existing `WaitUntil::None` navigation,
  target-bound readback, duplicate cleanup, and rollback behavior.
- One implementation pass, focused unit/route gates, one build/install, and one
  no-collection durable-profile gate.
- Preserve the operator-owned untracked `--full-page` file.
- No push, release, tag, or publication.

## Acceptance

1. A focused test proves remote-view new-target setup rewrites only the initial
   tab URL to `about:blank` while preserving caller and routing fields.
2. Rust formatting, clippy, remote-view parser tests, and route-confusion gates
   pass.
3. The installed durable-profile gate opens a previously slow destination with
   operator-visible readiness or returns a distinct post-acquisition result.

## Next Action

- Implement, validate, install, and return to fresh last30 source identities.

## Result

Remote-view now creates `about:blank` and navigates through exact-target
readback. The installed feed gate reached its destination; later active-target
and focus stalls were bounded as Plans 0088 and 0089.
