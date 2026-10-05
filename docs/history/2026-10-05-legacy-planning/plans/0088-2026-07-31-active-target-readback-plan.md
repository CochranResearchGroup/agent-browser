# Plan 0088: Active-Target Remote-View Readback

State: CLOSED

Created: 2026-07-31

Lane: distinct media-heavy-page successor to Plan 0087

## Goal

Avoid a redundant CDP target activation when remote-view is already reading
the exact active target created for the handoff.

## Evidence

- The LinkedIn source target reached the exact feed URL and title.
- The browser then degraded on target discovery after the remote-view readback
  path timed out and rollback began.
- `remote_view_open_wait_for_target_url` calls `tab_switch_target_id` even when
  its target is already active; the passing lightweight gate completed before
  the feed's media load made that redundant command stall.

## Bounds

- Reuse retained page metadata only when active target ID exactly matches.
- Preserve actual target-switch behavior for every non-active target.
- Keep blank-target creation, no-wait navigation, proof, and rollback intact.
- One implementation/build/install/gate cycle; no push or release.

## Acceptance

1. Focused tests cover exact-active reuse and non-active rejection.
2. Formatting, clippy, route-confusion, and remote-view parser gates pass.
3. The installed media-heavy LinkedIn feed gate returns operator-visible ready.

## Result

All three acceptance gates passed. A subsequent cold-start service run exposed
a distinct CDP focus stall after exact-target readback; that successor is Plan
0089 and does not reopen this target-selection fix.
