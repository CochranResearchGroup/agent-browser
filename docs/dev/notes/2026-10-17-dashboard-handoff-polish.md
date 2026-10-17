# Dashboard handoff list and desktop availability

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: [issue 301](https://github.com/CochranResearchGroup/agent-browser/issues/301)
Related lanes: P237, P240

## Outcome

The handoff drawer uses up to 1240 pixels on wide displays while staying inside
the available space beside the rail. Its three data columns sort by task name,
creation time and link expiry, with small SVG direction controls and accessible
sort state. Inactive links are hidden by default behind Show inactive, marked
amber when revealed, and offer inspection rather than an Open action. An expired
task with an active link remains visible because target and link lifetime differ.
The toolbar refresh, sign-out and rail collapse controls have hover help.

Managed ownership does not establish native presentation availability. The viewer
selector prefers an assigned native desktop, allows an already-qualified stream
for the same selected browser, and otherwise explains that no native desktop is
available. A missing tab remains a separate state. It never borrows a peer's
session, launches a replacement browser or changes a retained task.

## Validation and limits

Actual Chromium component fixtures cover both sort directions, date ordering,
perpetual expiry ordering, inactive filtering and color, active links for expired
targets, wide/narrow layout, hover labels, retained task availability messaging,
no extra handoff requests, and owned process cleanup. Managed selection tests
cover assigned desktops, missing tabs, qualified stream fallback and mismatched
session ownership. Dashboard build and related projection, selected context,
inspector, durable handoff, policy-refresh and handoff guidance checks qualify the
source change. Rust format and strict workspace Clippy cover the CLI guidance
update; the docs site build qualifies mirrored guidance.

Initial fixture and TypeScript failures are retained in the private validation
root rather than overwritten: modal-hidden controls required explicit background
queries, the new unavailable-desktop test needed to dismiss its modal, tabId needed
null normalization, and a durable-handoff source assertion needed the new viewer
selection variable. These were test/wiring failures, not production mutations.

Production installation and actual retained private-task recovery are not claimed.
The reported production task was inspected read-only; raw runtime facts and private
identifiers remain outside the repository. Existing provider assignment ambiguity
from the previous production smoke is separate and unchanged by this UI slice.

## User-directed live rail reconciliation

The same issue and primary checkout now own a bounded correction to the existing
read-only observed workspace status. Remote View supplies current desktop/window
presence; Agent Browser joins the exact browser process and CDP page target.
Neither a retained task, an allocated desktop nor a browser-looking window title
proves a live owned tab. No new endpoint or viewer is introduced. Provider reads
remain bounded; failed or incomplete observation is unknown, not deletion proof.
The rail excludes unverified/absent managed tabs and empty groups, preserves the
requested identity and handoff records, and updates on later successful polls.
P240 TTL and cleanup semantics remain untouched. UX301 is primary writer for the
observation adapter, generated display contract and rail consumer; foreign source
and production state remain preserved.


### Live rail qualification

Seven focused Rust tests pass, including the exact process/CDP read path, native
window-to-process-family matching, empty and peer desktops, stale generations,
missing tab targets, unknown reads, retained-state preservation and later recovery.
The native adapter fixture asserts exactly inventory, observe, windows, observe;
any effect operation fails the fixture. Released historical assignments are ignored
for presence; multiple active assignments remain unknown. This does not repair the
separate native handoff issuance lookup.

The Chromium fixture composes the real rail and task viewport. Absence removes
Alice's group and frame while retaining the requested URL; unknown observation
shows distinct guidance. Fresh evidence restores Alice and prepares only that exact
existing target. An empty desktop never appears as a browser. Its fresh owned
process census is empty after cleanup. The metadata collector itself issues no
focus, grant, launch, cleanup or durable-state write. Reopening the selected native
presentation after verified recovery remains the normal addressed-tab workflow.

The twelve-check contract/client/projection/policy batch passes. Generated client
parity, dashboard and docs builds, format and strict workspace Clippy pass. Presence
is additive in the existing workspace schema. Missing or older-owner evidence and
observations older than 30 seconds are unverified rather than live. An optional
independent Python schema probe could not run because jsonschema is not installed;
no result from that probe is claimed. Existing contract and generated-type gates
are the retained schema qualification.

Private check logs and synthetic screenshots are under the task's local evidence
packet. Prior UI failure logs remain retained. No production binary, native
provider configuration, TTL cleanup policy or private task was changed.


## Rail speed and accuracy audit

The user requested a rail audit after the presence repair. UX301 remains primary
writer. Audit scope covers refresh/cache delay, stalled reads, observation freshness,
collector fairness, exact selection, and grouping/render work. No new endpoint,
provider mutation, task relaunch or production installation is authorized by this
audit. Repairs and measurements stay on PR302; P240 TTL/cleanup remains separate.

The [rail audit](2026-10-17-rail-audit.md) records the subsequent speed/accuracy
repairs, measured grouping/rendering results, and retained validation failures.
