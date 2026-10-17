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
