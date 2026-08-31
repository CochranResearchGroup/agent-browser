# Plan 0146 Service State Stale-Revision Repair Acceptance

Date: 2026-08-31

Status: ACCEPTED

Branch: `fix/service-state-stale-revision-recovery-20260831`

Source baseline: `9fd50ec9ebbf3c15200fc2815d9f8a362d60d4b7`

## Incident Finding

The stale revisions observed by SoyLei jobs `r591783` and `r813532` were real
compare-and-swap conflicts at revisions 13 to 14 and 20 to 21. The product
defect was that mutation preparation loaded authoritative Service State outside
the cross-process file lock. A selected runtime host and a retained fallback
host could therefore prepare from the same revision, after which only the first
writer could commit. The historical identity of the writer that produced
revisions 14 and 21 is not recoverable because the old revision record did not
retain writer provenance.

Removing the realistic fixture's caller-local retry reproduced the failure.
The repaired store now holds the exclusive cross-process lock from the
authoritative load through durable commit, while releasing the in-process mutex
before serialization. The deterministic competing-writer test proves that both
writers commit exactly once at successive revisions without lost state or a
hidden retry.

The AuraCall `service_state_lock_timeout: process mutation lock` report is a
separate contention symptom. It proves that a command exceeded its wait budget,
not that state was corrupt. AuraCall's first `remote_view_open` launched Chrome
before reporting the timeout, which is why reconciliation remains mandatory
before any retry. The source report is retained in
`docs/dev/notes/0149-2026-08-30-auracall-gemini-development-runtime-remote-view-handoff.md`.

## Operator Recourse

The global `--service-state-lock-timeout-ms <ms>` option accepts a bounded range
from 1 through 300000 milliseconds. It extends only the serialized mutation
lane wait. It does not overwrite state, delete a lock, retry an uncertain
effect, or weaken browser, profile, route, tenant, or provider authority.

The workstation installer also accepts `--force-browserless-upgrade`. This is
an audited availability override rather than a general interlock bypass. Two
adjacent census rounds must each prove that no cooperative live owner,
adoptable orphan, conflicting owner, or insufficiently identified owned browser
exists. Stable external observation and manual-preservation records remain
untouched. Churn confined to those preserved external records does not block
installation. The transaction records whether the override was requested and
applied, its reason, and the combined census digest.

## Installer Defects Repaired During Acceptance

Production-candidate qualification exposed three additional browserless
upgrade defects:

1. The override path skipped full post-commit workstation reconciliation and
   returned an installed candidate with `complete=false` and `ready=false`.
   Reconciliation now runs after every real-host commit; only the shadow
   candidate presentation proof is skipped.
2. Runtime finalization returned early when there were zero browser handoffs,
   leaving the exact old browserless runtime host alive on its fixed ports.
   Finalization now also requires convergence of the recorded old host.
3. Renderer churn from a protected external browser made the full census digest
   unstable indefinitely. The browserless collector now requires two
   independently safe rounds and ignores only churn confined to preserved
   external or manual-preservation records. Any owned or ambiguous runtime still
   blocks.

An intermediate install required one exact operator-authorized termination of
old host PID `50911` after its PID, start token, binary hash, generation, socket,
and owned port were verified. The definitive candidate did not require manual
termination. Transaction
`upgrade-c9a883aa-7685-4bc0-be01-197ec21ac0a9` automatically retired old host
PID `80870`, start token
`linux:0f3a695c-8d37-4ad7-8969-1db795247d03:46040097`, and generation
`0.28.0-7467ac547a08-661cc01fcd73`.

## Source And Fixture Validation

The final candidate binary SHA-256 is
`fa99bc026aa47db43141888876e38afa90cc8976c58286d301d38962be3d895d`.
Validation completed on the final source included:

- strict Rust formatting and Clippy;
- the complete 122-test workstation installer partition;
- 29 Service Store tests, including deterministic competing writers and the
  realistic 3.15 MiB mixed-reader and writer fixture;
- all 86 route-host tests;
- workstation fixture, host provision, fresh VM harness, Guacamole asset and
  durability, route-user sync, route-confusion, and remote-view documentation
  checks;
- the docs site production build; and
- live CDP tab-streaming acceptance using the built binary directly, outside
  the Cargo validation scope's process limit.

The large Service State fixture measured a 246 ms maximum and p95 exclusive
lock hold. A redundant final Cargo recheck was not admitted because an unrelated
mail-receipts indexing process consumed approximately 29 GiB and nearly all
swap. It was cancelled while still waiting for host-reserve admission; it did
not replace or invalidate the completed post-change checks above.

## Development Acceptance

The candidate was installed as development generation
`0.28.0-fa99bc026aa4`. The repository binary and installed generation binary
matched the final SHA-256. Development doctor passed after publication, and the
three-cycle disposable open, URL-read, close, and residue smoke passed.

The exact reported failure shape was then exercised with the installed
development candidate:

```text
agent-browser-dev --service-state-lock-timeout-ms 30000 set viewport 1440 1000
```

The command returned width 1440 and height 1000. An independent page evaluation
returned `{"width":1440,"height":1000,"dpr":1}`, and the disposable session
closed.

## Production Acceptance

The production dry run applied the browserless override only after a stable
census proved no owned live browser. Its digest was
`b536f9555cd403990507d88ca33825b85d1ff387bb4091cdd8828162f0e2794c`.
The protected external browser was classified for manual preservation and was
not signaled.

The exact development-approved binary installed as production generation
`0.28.0-fa99bc026aa4-a04fbee7185d`. The transaction reached `accepted` with
`complete=true`, `ready=true`, post-commit validation, workstation
reconciliation, and supervisor rebinding recorded. Current executable, PATH
command, dashboard runtime, and selected runtime host all match the final
SHA-256.

Production doctor passed with:

- runtime multiplicity `steady_current`;
- one runtime host and one dashboard process;
- one executable generation;
- converged dashboard ingress and operator journey;
- zero stale or diagnostic runtimes; and
- zero blocking issues.

A disposable production browser opened a data URL titled `P146 Production`.
The exact viewport command returned 1440 by 1000, and independent page
evaluation returned the same dimensions. The first close response reported
`closed=true`, but fresh OS evidence still showed the exact disposable Chrome
PID `78535`. After that reconciliation proved the effect absent, one exact
owner-aware close retry removed the same PID without a force signal. No process
using the disposable production profile remains.

The repository Agent Browser skill was published to the shared user-scoped
skill path and exact file equality passed.

## Result

Plan 0146 is accepted. Prepared Service State writes are serialized across
runtime hosts, operators have a bounded contention wait override, and a narrow
browserless installation override preserves work while refusing live-owned or
ambiguous runtime state. The exact accepted candidate is installed and
converged in production.
