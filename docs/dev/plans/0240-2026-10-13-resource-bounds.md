# Browser resource budgets and durable terminal history

Product lane: PL-PLATFORM
Work item: #105
Branch: platform/resource-bounds-105
State: IN_PROGRESS
Version: 1
Lane: P240

## Agreed outcome

The operator accepted pressure handling that reclaims only already eligible idle
work and briefly waits for capacity while preserving active browsers. Completed
history stays durable while its active-memory projection is bounded. Valid
handoffs, live sessions/tabs, profiles and ongoing operations are not discarded.
There is no default hard memory cap that kills current work.

## Ownership

P240 owns browser launch admission, the SQLite terminal-history archive and the
host publication adapter. P237 owns dashboard/native selection and handoff/tab
lifetime policy. Shared host-file edits are limited to publication synchronization;
P240 is the primary writer of that adapter, and P237 must consume its canonical
publication result when integrating. No service-model or expiry-policy edits are
part of this work. The clean enrollment checkout is reused; prior candidate bytes
and branches remain retained. Foreign P237 edits are preserved.

## Delivery and seams

First test launch pressure through the existing admission observation/preflight
seam, then add configurable reserve and bounded waiting. Status remains read-only;
existing profile reuse is distinct from a new process launch. Reclamation uses
the existing owner reaper and its eligibility protections.

Next test terminal-history publication through SQLite persistence across restart:
active terminal session/tab vectors are bounded, archived rows remain durable,
and a stale publication cannot erase new live state or archive data. Keep archive
and active-state publication atomic, including launch-custody publication. Native
host memory must adopt the exact published projection to avoid stale-state
conflicts. Preserve all live fields and handoff targets.

Finally qualify public workflow continuation and scoped close in an isolated
runtime, review Standards and Spec independently, and run focused Rust, format,
strict workspace Clippy and all touched contract/documentation checks. Integrate
through an issue-linked PR. Wider #105 soak/concurrency claims require their own
current evidence and are not inferred from these source changes.

Production publication, restart, cleanup, provider effects, private account work
and reboot are excluded. #183 remains deferred until after the next reboot.
