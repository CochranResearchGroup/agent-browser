# P264: missing browser selection

Status: CLOSED
Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#344
Branch: fix/missing-browser-selection
Owner: ecochran76

## Outcome and custody

An explicit browser absent from inventory must remain missing instead of falling
through to another workspace through profile/session/tab/job hints. Preserve live
explicit-browser and profile-only navigation. Reuse the clean dashboard primary
from canonical 59021fe0f. This writer owns only selected-workspace-context and its
fixture/documentation; P260/P261 are integrated. Preserve foreign root/resource
worktrees and sealed candidates. No runtime, native selection, auth or deployment.

## Acceptance

Reproduce the wrong peer with direct/browser-workspace IDs and valid weaker hints.
Require missing workspace and no peer page, resources or browser evidence. Run
context/chat/projection/inspector checks, dashboard type/build and documentation
gates. No Rust source changes or Cargo gates required.

## Accepted outcome

The wrong-peer regression is reproduced and repaired. Ten explicit-browser/hint
combinations pass along with existing live and profile-only navigation.
[Acceptance](../notes/0264-2026-10-21-missing-browser-selection.md) records gates
and source-only limits. #68 remains open; no runtime publication.
