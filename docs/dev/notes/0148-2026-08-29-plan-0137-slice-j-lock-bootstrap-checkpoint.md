# Plan 0137 Slice J Lock Bootstrap Checkpoint

Date: 2026-08-29

Status: SOURCE AND ISOLATED VALIDATION ACCEPTED, PRODUCTION DRY-RUN NEXT

## Scope

Validate the bounded Service State compatibility repair, create one
provider-free candidate presentation, and stop on any uncertain mutation
outcome. No provider navigation, credentials, tenant profile, duplicate
browser lane, broad cleanup, or production candidate installation was in
scope.

## Candidate and installed identity

- source baseline before this checkpoint: `c27d05c45d854f5bd578b666666ccd23fedb6e45`;
- candidate binary SHA-256:
  `067e8e8d573a89ae0ee5b408b743fdf4e566f37e1dd789786f0286c43fb509bc`;
- selected production generation:
  `0.28.0-2851117fd877-04e7cf4c8b54`; and
- production remained on that generation throughout this checkpoint.

The candidate contains the closed Plan 0142 client recourse and lock
diagnostic behavior. The selected production generation does not expose
`serviceStateLockDiagnostics` and still returns the legacy lock-timeout string.

## Service State compatibility result

The compatibility repair treats a historical `controllerEpoch` as fencing
history rather than current controller authority when the route is orphaned,
its controller lease is absent, its viewer lease list is empty, and its
retained process identity is confirmed absent.

Focused tests prove both sides of the boundary:

- an orphaned route with historical controller epoch materializes an inert
  browser and released session placeholder; and
- the same route with a current controller lease remains a migration blocker.

The release candidate then completed `install workstation --dry-run --json`
against current production state. The migration plan reported `not_required`,
`mutation=false`, zero protected-record removals, and no
`service_state_display_browser_missing` error.

## Provider-free presentation result

Automatic route selection was rejected because Route A still had a live BILL
browser visible on its display. Route 3 was selected explicitly only after a
fresh census showed its prior owner terminal, its route orphaned, its
controller lease absent, its viewer lease list empty, and no live browser on
display `:12`.

One disposable browser used:

- session `plan0137-slice-j-presentation`;
- profile `managed-one-time-plan0137-slice-j-presentation`;
- URL `about:blank`;
- Route 3 and display `:12`; and
- opaque handoff `r474915`.

The open returned `operatorVisible.state=ready`. It did not contact AuraCall,
Gemini, Google Ads, Amazon, SoyLei, Odollo, or another provider.

## Bootstrap blocker and reconciliation

The candidate installer still reported zero eligible handoffs. The fresh
handoff was classified as `presentation_receipt_unready` because the old
selected production generation did not persist a generation-bound
`presentationReceipt` for the direct CLI open.

Exactly one request attempted to resolve the same opaque handoff through the
service resolution path. It failed with:

```text
service_state_lock_timeout: process mutation lock
```

The retained job is:

```text
mcp-service-request-service_remote_view_handoff_resolve-3759678a-5e4e-4855-8887-46c904b17350
```

Fresh job and trace reconciliation found:

- job state `failed`;
- action `service_remote_view_handoff_resolve`;
- zero related events;
- zero related incidents;
- no browser or session identifier on the job;
- no recorded display allocation; and
- no handoff presentation receipt.

No second resolve, second handoff, duplicate profile lane, route switch,
cleanup, or candidate installation was attempted.

## Assessment

Closed Plan 0142 mitigates this incident class in source and the isolated
development runtime. Its final acceptance proved normal concurrent readers
and writers without lock timeouts and verified structured failures carrying
`effect_uncertain`, `inspect_before_retry`, exact job and trace recourse, and
bounded lock-holder diagnostics.

That mitigation is not yet available to production consumers such as AuraCall
because production remains on the older selected generation. The current
upgrade bootstrap requires a presentation receipt that the old generation
cannot produce through this contended resolution path.

## Transactional bootstrap repair | 2026-08-30

The bounded source repair separates two proof phases:

- bootstrap admits one opaque handoff only when its retained browser process,
  target, and unique current runtime owner are exact and healthy; and
- candidate commit still requires a new authenticated presentation receipt
  matching the staged generation, current owner and process identity, route,
  display, target, and provider.

Bootstrap deliberately does not require the old selected generation to persist
the candidate's receipt or keep replaceable route and display infrastructure
ready. The candidate stages first, adopts the retained browser lane, reacquires
presentation, and commits only after its own receipt passes the strict gate. If
proof times out or fails, rollback removes the staged dashboard candidate and
preserves the selected installed generation.

Provider-free tests cover persisted Service State admission without an old
receipt or ready route, rejection without current ownership, strict candidate
receipt validation, and rollback without generation-selector change. Resume
from `candidate_ready` already waits on the same candidate proof and does not
reapply bootstrap.

## Isolated acceptance | 2026-08-30

The repository's formatting, strict Clippy, source-free workstation fixture,
documentation build, remote-view guidance check, 118 serial workstation tests,
and partitioned Rust harness pass. Development generation
`0.28.0-d9577e0ed57a` has candidate SHA-256
`d9577e0ed57a240302995c002ebf4a2a08dd94705b1c06d8ad194db95a40d368`.
The development runtime and presentation provider report ready, the repository
skill is synchronized into the development pseudo-home, and three disposable
browser launch, URL-read, close, and residue iterations pass. Publication
verified that the production selected generation, process identities, and
Service State hashes were unchanged.

## Remaining gate

Run a fresh production dry-run with this exact candidate. Production apply may
enter its transactional staging path only when bootstrap identifies an
adoptable current handoff and every migration, census, rollback, and host gate
is exact. Provider acceptance remains a separate authority boundary, and no
uncertain failed service request may be retried as part of the upgrade.
