# Plan 0131 | Controlled X11 Desktop Provider Foundation Acceptance

Date: 2026-08-25

State: PRODUCTION CANDIDATE REQUALIFICATION IN PROGRESS

Execution state: `production_candidate_requalification_in_progress`

Lane: P131

Parent: `docs/dev/plans/0110-2026-08-12-desktop-perception-interaction-foundation-plan.md`

Source baseline: `e8695f82969e684cc4f9a7929b723777f548c3a3e`

Current authority: SLICE E SOURCE RECONCILIATION AND PRODUCTION AUTHORITY
GRANTED. THE SWAP-FREE CARGO
ADMISSION THRESHOLD MAY BE SET TO ZERO WHILE ALL OTHER WRAPPER AND CGROUP
GUARDS REMAIN ENFORCED. PLAN 0133 IS DEVELOPMENT ACCEPTED BUT RETAINS ITS
PRODUCTION-READ-ONLY BOUNDARY. LIVE INSTALLATION STILL REQUIRES EXPLICIT
CLEARANCE OF THAT BOUNDARY AND AN APPLY-SAFE FIXTURE ROUTE WITHOUT EXPANSION,
TAKEOVER, OR WORKLOAD DISPLACEMENT.

Current implementation branch:
`feature/plan0131-production-candidate-reconciled`

Execution start baseline: `a54b0f976fb20e801d8e09e844708753c80ac79d`

Owned worktree scope:
`feature/plan0131-production-candidate-reconciled` in the dedicated local Plan
0131 worktree.

Depends on:

- all five source-accepted Plan 0110 proofs;
- `docs/dev/notes/0110-f1-2026-08-23-passkey-and-two-factor-authentication-fieldwork.md`;
- completed Plan 0124 scalable desktop evidence and presentation capacity;
- accepted Plan 0125 development runtime isolation and build capacity;
- accepted Plan 0127 development presentation-provider isolation;
- closed Plan 0130 access-plan owner-reuse coherence;
- the current service-owned controller lease, desktop interaction operation
  ledger, route and display binding, capture-ready proof, and durable handoff
  contracts.

## Objective

Close Plan 0110's live Foundation Acceptance boundary with one production-grade
but initially development-only X11 input provider. Prove one controlled,
repository-owned RDP or Guacamole recipe through the installed development
binary while preserving exact browser, route, display, geometry, controller,
human-handoff, replay, and cleanup authority.

This plan does not implement a real credential, LastPass, passkey, one-time
code, identity-provider, consent, biometric, PIN, or master-password workflow.
It establishes the reusable provider boundary those later use cases require.

## Verified Starting Point

The existing `DesktopInteractionProvider` deep module already owns observe,
probe, effect, and after-observation semantics. The interaction engine already
requires an existing controller lease, derives principal-scoped effect keys,
persists terminal and uncertain operation records, performs bounded emergency
release, rechecks authority and geometry around events, and rejects configured
production dispatch with `desktop_input_provider_unavailable`.

The current `DesktopControlCoordinator` serializes interaction events and
controller mutation by route inside one process. That is sufficient for the
source fixtures but is not an operating-system fence. A second agent-browser
process can own another coordinator instance, and an XTEST effect has an
acknowledgement crash gap that a process-local map cannot close.

The isolated development presentation provider has four warm slots, a hard
maximum of six, distinct route users and displays, separate Guacamole
containers and ingress, and production read-only guards. Current display-access
authorization grants the unprivileged operator access to an exact route-owned
X server. The privileged helper must remain limited to that bounded access
grant and other existing workstation maintenance operations.

Graphiti discovery returned older RDP validation and current development
provider-isolation context. Current source, plans, and installed-runtime
receipts remain authoritative.

## Frozen Architecture

### 1. One Canonical Interaction Engine

The implementation extends the existing `desktop_interact` path and
`DesktopInteractionProvider` boundary. It does not add another public action,
an ingress-specific execution path, or caller-composed shell input.

The provider implementation belongs in a focused native module outside the
command dispatcher. The dispatcher continues to pass intent. The provider
owns X11 connection, surface probing, bounded event conversion, effect
acknowledgement, and typed provider failures.

### 2. Native Or Tightly Controlled XTEST Boundary

Input is emitted through XTEST using a native library or a small typed helper
whose arguments are a closed event schema. Product code must not construct
`xdotool` shell commands. No arbitrary executable, display, key sequence,
coordinate, path, environment variable, or provider URL crosses the public
service contract.

The initial registered recipe remains repository-owned and uses only bounded
pointer movement, one left click, fixed benign text, and emergency key or
button release. The provider does not expose a general desktop shell.

### 3. Exact Service-Owned Binding

Provider construction resolves all effect authority from the selected
service-owned browser and current records:

- logical browser and process generation;
- session and profile identity;
- route and display allocation;
- view stream and current controller lease;
- route and stream controller epochs;
- capture-ready geometry and coordinate mapping;
- provider capability and installed executable generation.

Callers may not provide a display name, Xauthority path, route user, socket,
lock path, provider executable, raw Guacamole URL, or coordinates outside the
registered recipe.

### 4. Cross-Process Route Effect Fence

Every ordinary input event, emergency release, and controller mutation uses
one OS-visible fence derived from environment identity plus stable route and
display-allocation identity. The canonical lock location is service-owned,
private, and derived internally beneath the runtime state root. Raw display
names and caller strings never become lock paths.

The existing process-local coordinator remains the fast in-process layer. The
external fence is the authority shared by independent agent-browser processes:

1. acquire the exact route fence with a bounded deadline;
2. re-read controller lease, epochs, route, display, process, focus, geometry,
   and provider identity;
3. persist the provider effect-key state before emission;
4. emit at most one closed-schema XTEST event;
5. persist acknowledgement or typed uncertainty;
6. release the fence;
7. re-observe before any following event.

Controller takeover, release, reconciliation, and other controller mutations
must acquire the same external fence before committing a new epoch. A mutation
waits for the current event to leave the fence, then its persisted epoch causes
the old transaction's next boundary check to fail.

### 5. Crash-Gap And Replay Semantics

The provider keeps a private atomic effect journal keyed by the existing
principal, operation, recipe, and event effect key. States are
`prepared`, `acknowledged`, or `uncertain`.

- An acknowledged key returns the original redacted acknowledgement without
  emitting again.
- A prepared key found after process loss is uncertain and never emits
  automatically.
- A write or fsync failure before emission fails with zero input.
- A failure after emission but before durable acknowledgement records or
  preserves uncertainty, attempts only the registered bounded release when
  required, and returns the existing opaque operator handoff.
- Reconciliation may classify evidence but cannot convert uncertainty into
  success or retry authority.

This is an at-most-once effect boundary with explicit uncertainty, not an
exactly-once claim.

### 6. Privilege Boundary

The root-owned privileged helper does not gain pointer, keyboard, XTEST,
arbitrary command, or arbitrary file capabilities. It may continue granting
the unprivileged operator access to an exact route user's X server through its
existing validated contract.

If display access is absent, stale, ambiguous, or requires an unsupported
grant, the provider returns a typed unavailable result before acquiring an
effect key or emitting input.

### 7. Development-First Configuration

Source implementation remains fail-closed until a generation manifest enables
the provider for the development runtime. Production continues to report
`desktop_input_provider_unavailable` throughout source and development
acceptance.

The first installed proof uses the isolated `agent-browser-dev` executable,
state root, dashboard, Guacamole provider, ingress, route users, and display
pool. It must prove production generation, processes, browser ownership,
dashboard ingress, and provider state unchanged across the transaction.

### 8. Evidence And Privacy

Durable records may contain opaque identities, hashes, epochs, provider
version, event kind, attempt and acknowledgement counts, typed state,
verification result, cleanup result, and opaque handoff identity.

They must not contain pixels, OCR text, typed plaintext, clipboard data,
credentials, account labels, raw event trajectories, display names, Xauthority
paths, filesystem paths, provider stderr, process command lines, or raw
provider and Guacamole URLs.

## Controlled Fixture

The live development fixture is a repository-owned X11 application rendered on
one exact development RDP route. It contains:

- one deterministic target and one visually similar decoy;
- a visible focus and geometry epoch indicator;
- a benign fixed-text field;
- an after-state marker independent from input acknowledgement;
- controls for ambiguity, focus loss, geometry drift, route replacement,
  controller takeover, partial effect, and verification failure;
- no network dependency, external account, extension, secret, credential, or
  private user data.

The accepted recipe must capture the desktop, locate exactly one target, move,
click, type fixed benign text, verify the after-state, and retain only redacted
receipts. The fixture must also prove that a CDP screenshot cannot substitute
for the desktop frame when the target is outside page content.

## Implementation Slices

### Slice A | Contract And Red Fixtures

Authority: SOURCE-ONLY after explicit execution start.

- Freeze provider capability, external-fence, effect-journal, error, receipt,
  and configuration schemas.
- Add the controlled fixture manifest and provider-free fake X11 sink.
- Demonstrate red coverage for current configured-provider unavailability,
  cross-process route contention, abandoned prepared effect keys, controller
  mutation during an event, and forbidden caller routing.
- Reconcile existing coverage before adding cases; retain one cheapest test per
  named risk where practical.
- Update the parent plan and source acceptance note without claiming installed
  or live capability.

### Slice B | Native Provider And External Fence

Authority: SOURCE-ONLY after Slice A acceptance.

- Implement the deep XTEST provider module, private effect journal, and
  OS-visible route fence.
- Make controller mutation and reconciliation participate in the same fence.
- Preserve the existing interaction engine, operation ledger, process-local
  coordinator, provider effect keys, cleanup, and handoff contracts.
- Add exact installed-generation and development-environment admission.
- Keep production provider selection unavailable.

### Slice C | Development Installation And Controlled Live Proof

Authority: DEVELOPMENT RUNTIME EFFECTS only after a separate live preflight
and explicit slice start.

- Build through bounded Cargo admission and install transactionally into
  `agent-browser-dev` only.
- Capture source, artifact, installed-generation, service, provider, process,
  browser, route, display, controller, resource, and production snapshots.
- Run the single controlled success recipe and bounded failure matrix.
- Prove same-route cross-process exclusion and unrelated-route independence.
- Prove controller takeover waits for the current event, advances authority,
  and prevents the old transaction's next event.
- Prove restart replay returns acknowledged evidence without input and a
  prepared crash-gap record returns uncertainty without input.
- Close the fixture browser and reconcile to the warm minimum with zero new GC
  candidates or ambiguous cleanup obligations.
- Roll back the development selector if any acceptance axis fails.

### Slice D | Stress, Audit, And Development Acceptance

- Run one bounded fresh architecture and safety audit against the frozen
  criteria.
- Adjudicate one candidate set and permit at most one remediation packet.
- Run closed-world verification only for accepted blocking findings and
  critical regressions introduced by the remediation.
- Record exact focused, presubmit, and live validation tiers, elapsed time,
  resource caps, retries, exclusions, and retained risks.
- Mark the provider `development_live_accepted` only if every criterion passes.

### Slice E | Transactional Production Candidate

Authority: GRANTED BY THE OPERATOR AFTER DEVELOPMENT ACCEPTANCE. The operator
also authorized setting only the Cargo wrapper's swap-free admission threshold
to zero. All other wrapper, job, cgroup, memory, and aggregate-slice controls
remain mandatory.

The earlier source commit `9ec6a2b4` and candidate SHA-256
`2202890a31370f6693f8f50db06448b5a4b2b1b36d930538afe34d910b6fc245`
were qualified before Plan 0133 landed and are no longer eligible for a
production transaction. Their historical validation receipts are recorded in
`docs/dev/notes/0131-4-2026-08-25-controlled-x11-provider-production-candidate-preflight.md`.

Current-main reconciliation split the unattended service-GC clock repair from
the provider feature and added exact provider-generation revalidation inside
the route fence before every event. A new candidate identity and receipt are
required before source integration is merge-ready.

The workstation hot-upgrade transaction and controlled repository fixture
remain unapplied. Plan 0133 is development accepted, but its
production-read-only boundary has not been cleared for another workstation
upgrade. The latest runtime census also had no apply-safe fixture route or
cleanup candidate. Continuing live execution would require fresh proof that
neither route expansion, takeover, nor workload displacement is necessary. It
does not use a real authentication challenge.

Production input remains unavailable unless the candidate transaction and the
controlled fixture both pass. Failure rolls back the selected generation and
leaves Plan 110 live acceptance closed.

### Slice F | Plan 0110 Closure And Authentication Entry

After accepted production proof only:

- update Plan 0110 and the roadmap with live Foundation Acceptance evidence;
- retain release acceptance as a separate maintainer-controlled boundary;
- open a new plan for read-only authentication observation and synthetic
  LastPass and second-factor orchestration;
- do not authorize a real credential workflow through Plan 0131.

## Acceptance Criteria

1. Public input remains `desktop_interact`; no parallel ingress or raw desktop
   command surface exists.
2. Only exact service-owned route, display, browser, process, geometry, and
   controller authority can construct the provider.
3. One OS-visible fence serializes effects and controller mutations across
   independent processes for the same route while unrelated routes progress.
4. Every effect has durable prepared, acknowledged, or uncertain state;
   acknowledged replay emits zero input and abandoned prepared replay fails
   closed.
5. Controller takeover cannot overlap an event and prevents all following
   events from the old authority epoch.
6. Focus, geometry, route, display, browser process, provider generation, or
   lease drift stops before the next effect.
7. Partial effects receive one bounded cleanup attempt and a durable uncertain
   receipt plus existing opaque handoff; they are never retried automatically.
8. The privileged helper gains no input capability and the provider never runs
   as root.
9. Durable and user-facing projections satisfy the privacy boundary.
10. The controlled development fixture proves observation, unique location,
    bounded input, after-state verification, replay, contention, takeover,
    failure, cleanup, and rollback.
11. Production remains unchanged throughout Slices A through D.
12. Plan 110 is not marked live accepted before a separately authorized and
    accepted Slice E.

## Test Tiers And Budgets

### Focused Development

Target: five minutes, one bounded Cargo invocation at a time, four Cargo jobs
maximum on WSL.

Protect only changed provider, fence, journal, controller, interaction, and
fixture invariants. Tests are hermetic and provider-free. A new regression case
must fail before its implementation when practical.

### Blocking Presubmit

Budget: twenty minutes wall time and the repository Cargo cgroup limits.

Includes formatting, strict Clippy, focused Rust modules, service contract and
client parity, dashboard receipt projection, actions architecture, WSL Cargo
safety, documentation build, validation selection, and diff hygiene. Unknown
impact widens to the repository's selected safe fallback.

### Comprehensive Regression

Cadence: manual before Slice D acceptance and before any Slice E candidate.

Includes the broader Plan 0110 capture, locator, prompt, interaction, handoff,
route, controller, runtime lifecycle, and development-provider suites. It is
reported as comprehensive, not silently folded into every focused edit.

### Live Or Provider

Opt-in only. The development live lane has one controlled browser, one primary
route, at most one independent competing provider process, a fifteen-minute
budget per scenario group, and no automatic retry after any possible effect.
The first failure is retained. Before a rerun, exact effect journal, controller,
browser, route, display, fixture, and cleanup state must be reconciled.

## Validation Matrix

Source validation must include the commands selected by
`pnpm validation:select` plus, at minimum:

```bash
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_interaction -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_control_coordinator -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml desktop_capture -- --test-threads=1
pnpm test:service-client
pnpm test:service-api-mcp-parity
pnpm test:service-contracts-no-launch
pnpm test:dashboard-inspector-actions
pnpm test:actions-architecture
pnpm test:wsl-cargo-safety
pnpm --dir docs build
git diff --check
```

Live validation commands and fixture scripts are not frozen until Slice A
defines their exact inputs, cleanup, resource budget, and receipt location.

## Rollback And Recovery

- Source slices roll back by reverting their coherent commits before any
  installed effect.
- Development installation retains the prior selected generation and uses the
  existing transactional selector rollback.
- A live test never retries an uncertain operation ID. It reconciles the
  effect journal and uses a new operation only after the old result is
  terminally classified.
- An acknowledged button-down or key-down without verified release enters
  cleanup-required state and blocks route reuse until bounded release or
  operator recovery is receipted.
- Route, display, controller, provider, or process ambiguity quarantines the
  fixture lane. It does not trigger browser termination, profile replacement,
  route replacement, or opportunistic GC.
- Production rollback is designed and rehearsed during development but is not
  executed without Slice E authority.

## Hard Stops

- Stop if implementation requires arbitrary `xdotool` command construction,
  root input injection, or a new privileged-helper input capability.
- Stop if a caller can choose a display, route user, Xauthority path, provider
  executable, lock path, raw coordinate, or raw provider URL.
- Stop if process-local coordination is presented as cross-process fencing.
- Stop if the effect journal cannot distinguish acknowledged replay from an
  abandoned prepared effect.
- Stop if controller mutation does not share the same external route fence.
- Stop if acknowledgement is treated as after-state verification.
- Stop if a partial or uncertain effect can retry automatically.
- Stop if private pixels, OCR text, plaintext input, paths, command lines, or
  provider stderr enter durable state.
- Stop if a live fixture needs a real credential, extension account, external
  identity provider, mailbox, network challenge, or private browser profile.
- Stop if development work changes production state.
- Stop if source, development-installed, production-installed, live,
  Foundation Acceptance, or formal release boundaries are conflated.

## Completion States

- `planned`: this document is accepted; no source implementation started.
- `source_accepted`: Slices A and B pass source gates; no installed claim.
- `development_live_accepted`: Slices C and D pass; production input remains
  unavailable.
- `production_controlled_fixture_accepted`: separately authorized Slice E
  passes and Plan 110 may close its live Foundation Acceptance boundary.
- `blocked`: one frozen safety invariant cannot be met without a new plan or
  explicit authority expansion.

## Current Next Action

Development acceptance is recorded in
`docs/dev/notes/0131-3-2026-08-25-controlled-x11-provider-development-live-acceptance.md`.
Production input remains unavailable, Plan 0110 remains open at its separate
live Foundation Acceptance boundary, and no further runtime effect is
authorized. The next executable slice is Slice E only after fresh explicit
authority and a new source, installed-generation, process, browser, handoff,
provider, resource, production, and rollback preflight.
