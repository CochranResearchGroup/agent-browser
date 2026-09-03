# Plan 0158: Frozen-Candidate Historical Failure Stress Campaign

Date: 2026-09-02

State: OPEN

Execution state: `w6_blocking_repair_candidate_validation_active`

Lane: P157

Role: installed-acceptance successor to Plan 0157

Parent: `docs/dev/plans/0157-2026-09-02-profile-permissions-and-request-provenance-plan.md`

Branch: plan/profile-permissions-and-request-provenance

Target: main

Source baseline: `e26a6b05c315cfed06a833a5c4d7406803bcc0fb`

Integration: merge

Authority: PLAN, DIAGNOSTIC HARNESS IMPLEMENTATION, PROVIDER-FREE FIXTURES,
ISOLATED DEVELOPMENT-RUNTIME STRESS, EXTERNALLY INGRESSED STAGING
PRESENTATION, DISPOSABLE BROWSER AND PROFILE EFFECTS, AND REDACTED EVIDENCE
PUBLICATION ARE IN SCOPE. PRODUCTION CREDENTIAL ENTRY, TENANT DATA MUTATION,
PRODUCTION PROFILE OR ACL MUTATION, PRODUCTION CLIENT EVICTION, AND PRODUCTION
RUNTIME REPLACEMENT ARE OUT OF SCOPE.

Dependencies: [P46, P97, P101, P125, P134, P142, P147, P148, P150, P155,
P156, P157]

Overlaps: [P144]

## Incident And Acceptance Correction

Plan 0157's W11 closeout proved valuable focused behavior, but it did not
satisfy its own adversarial installed-acceptance contract. The completed checks
did not replay every historically observed identity failure through
`remote_view_open`, exercise cross-generation Xvfb ownership across systemd
`PrivateTmp` namespaces, run the full ten-client and same-label concurrency
matrix as installed effects, or prove complete logging for every historical
failure path. Production subsequently retained identity, Xvfb, route,
presentation-proof, and Service-resource timeout failures that focused green
tests did not reproduce as one frozen campaign.

The older P46 campaign covered many valuable remote-view and dashboard cases,
but it repaired harnesses and runtimes between failed scenarios. That was a
valid hardening workflow, not a valid frozen-candidate reliability
measurement. This successor therefore reopens installed acceptance without
discarding P157's source-complete result or P46's historical evidence.

The preliminary read-only production review that triggered this successor
found the following terminal failures in a retained 200-job window beginning
2026-09-02 at 18:00 UTC. W1 must preserve the redacted source records and
recompute these counts before they become campaign fixtures:

| Observed signature | Count | Actions represented |
| --- | ---: | --- |
| `existing_session_profile_identity_unproven` | 14 | launch, `remote_view_open`, `tab_switch`, `view_focus` |
| `existing_session_profile_identity_inconsistent` | 3 | launch, `view_focus` |
| Xvfb `:90` automatic-launch failure | 8 | `tab_new` |
| Service-resource timeout | 5 | resource/status reads |
| Route-pool failure | 4 | `remote_view_open` |
| Presentation-proof failure | 2 | `remote_view_open`, reattach |
| Other launch or reattach failure | 3 | launch, reattach |

The same review found failed legacy jobs whose durable `failure` and
`provenance` fields were null. Those rows are mandatory logging-oracle inputs,
not historical noise.

## Objective

Attempt to reproduce the complete known Agent Browser failure envelope under
high deterministic pressure, using both agent-only control and human-simulated
remote desktop control. Human-simulated remote browsing must enter through a
real external HTTPS ingress from a client outside the service host and its
network namespace. Durable handoff URLs, dashboard performance, left-rail
truth, supervisor and install coherence, profile sharing, Xvfb allocation, and
end-to-end logging are first-class test subjects.

The campaign is diagnostic. Defer repairs to a future plan whenever the
remaining test sequence can continue safely and still produce representative
evidence. This preserves a holistic view of the defect surface so related
failures can be diagnosed and repaired together efficiently. Some defects may
prevent completion of a test sequence. When that happens, pause the affected
campaign environment, seal the evidence already collected, diagnose the
blocking defect, repair and validate it, install and identify a new frozen
candidate epoch, and resume the blocked sequence with distinct attempt IDs.
Never rewrite the earlier failure or represent results from different candidate
epochs as one unchanged run. The final work unit seals all epochs,
reconstructs causal timelines, evaluates architecture and logging completeness,
and produces a prioritized remediation ledger. No new product or runtime repair
occurs inside that final review.

## Frozen-Candidate Contract

1. Select exactly one source commit, built binary digest, dashboard digest,
   installed generation, browser executable digest, runtime manifest revision,
   provider configuration revision, and external-ingress deployment revision.
2. Complete fixture creation, synthetic-site deployment, external runner
   provisioning, observability checks, and a single clean baseline before the
   freeze point.
3. During an active frozen-candidate epoch, prohibit source edits, rebuilds, reinstalls,
   configuration rewrites, service remedies, incident resolution, garbage
   collection, retained-state pruning, route repair, profile repair, and
   unscheduled process termination until that epoch is complete or formally
   paused and its evidence is sealed. Defer nonblocking repairs for a future
   plan. If a defect makes the remaining declared sequence impossible, pause
   the affected environment, diagnose and repair the blocker, validate and
   install a new candidate, and begin a new immutable epoch before resuming.
   This prohibition applies to E1 and E2 only. It never blocks a production
   install, repair, or safety intervention; each production change closes one
   observation epoch and starts another.
4. Permit only effects named in the case manifest. Controlled browser crashes,
   supervisor transitions, route exhaustion, network degradation, policy
   mutations, eviction, and full shutdown use disposable isolated targets and
   are test stimuli, not reactions to observed failures.
5. Never retry a failed attempt opportunistically. Predetermined repetitions
   have distinct attempt identifiers and execute from their declared starting
   state. A pass after an earlier failure never erases the first failure.
6. Continue independent cases after a failure. Mark only cases whose declared
   prerequisite is lost as `skipped_blocked`, retaining the exact blocking
   case and state observation. When a blocker prevents completion of the test
   sequence rather than only one dependent case, invoke the bounded campaign
   pause, diagnosis, repair, revalidation, and new-epoch resume path.
7. Do not clean between cases unless that cleanup is itself a scheduled,
   observed case. State contamination and recovery failure are outcomes to
   measure.
8. Run a scheduled disposable teardown and capture its result before the final
   analysis. Teardown failure is evidence; it does not authorize manual
   cleanup during the campaign.

The campaign controller has one monotonic state machine:

```text
prepared -> frozen -> executing -> execution_terminal -> evidence_sealed
    -> analyzed
```

There is no transition from `executing` back to `prepared`. `analyzed` is the
terminal plan state.

## Test Environments And Isolation

### E0: Provider-Free Contract Fixtures

Use deterministic in-memory and file-backed fixtures for exhaustive state,
fault-boundary, schema, and transport-projection cases. E0 may generate large
synthetic inventories, histories, and logs without Chrome or provider
services.

### E1: Isolated Installed Development Runtime

Use the development pseudo-home, one pinned candidate, disposable Profiles,
real Chrome, real service host and supervisor units, and isolated Service
State. Do not borrow production profiles, sockets, routes, displays, browser
processes, or credentials. Candidate publication and installation finish
before freeze.

### E2: External-Ingress Presentation Staging

Use real Guacamole or RDP presentation behind the configured external HTTPS
ingress. At least two external clients must run outside the service host and
outside its container or network namespace. One client acts as an ordinary
human-paced observer/controller; the other supplies concurrency, reconnect,
and slow-client pressure. Local dashboard, embed, health, and raw provider URLs
may be captured as diagnostic fields but can never satisfy an E2 pass.

### E3: Production Read-Only Observation Epochs

Continuously collect privacy-bounded, append-only operational and failure
records from real production use. Production traffic supplies the eight-hour
resource-stability and 24-hour handoff-longevity windows when available. The
campaign does not synthesize production actions or mutate production Profiles,
ACLs, credentials, tenant data, routes, or browsers. An install, repair,
restart, or deployment is an explicit epoch boundary, not a reason to suppress
the work or discard earlier evidence. E3 evidence collection is asynchronous
and may never delay installation, repair, or emergency intervention.

## External-Ingress And Handoff Oracle

Every E2 operator-visible case must prove all of the following from the
external client:

- the starting URL is the authenticated, opaque
  `/remote-view/<handoff-id>` URL returned as `handoffUrl` or the durable
  `externalUrl` for that same handoff;
- `operatorVisible.state=ready` was observed before the case claims visibility;
- DNS, TLS, redirects, cookies, WebSocket upgrades, iframe loads, and reconnect
  traffic succeed from the external network vantage;
- the external client never receives a navigable `localhost`, `127.0.0.0/8`,
  IPv6 loopback, RFC 1918, link-local, `.local`, raw Guacamole,
  `providerExternalUrl`, `routeBinding`, `localEmbedUrl`, `dashboardEmbedUrl`,
  or `healthUrl` as its operator handoff;
- every `Location`, iframe `src`, form action, WebSocket endpoint, reconnect
  target, and client-visible error action is scanned for internal URL leakage;
- reopening the same durable handoff after viewer expiry, controller transfer,
  route change, display change, service-client restart, and browser recovery
  resolves to the intended retained browser without creating another browser;
- screen pixels, selected browser, Profile, session, tab target, visible URL,
  and page marker agree; and
- dashboard or provider success cannot substitute for missing external-client
  evidence.

The external client records HAR, console, WebSocket, navigation, TLS, video,
screenshots, input timing, focus changes, viewport dimensions, and the final
visible-state marker. Credential characters, passkey assertions, cookies,
tokens, page bodies, and password-manager vault content must be redacted or
excluded at capture time.

## Result And Evidence Contract

Each declared case and predetermined repetition ends in exactly one state:

- `passed`
- `reproduced_historical_failure`
- `new_product_failure`
- `harness_failure`
- `inconclusive`
- `skipped_blocked`
- `safety_stopped`

A failure is a successful diagnostic result only when its evidence contract is
complete. It is never relabeled as a passing product result.

Raw campaign evidence lives outside the product repo under a dedicated
user-scoped runtime-state root such as
`$XDG_STATE_HOME/agent-browser/campaigns/p158/<run-id>/`. The repo receives
only schemas, synthetic fixtures, harness code, and a curated redacted final
report under `docs/dev/notes/`.

The append-only campaign manifest records:

- run, environment, case, attempt, seed, schedule, candidate, and fixture IDs;
- wall and monotonic timestamps plus clock-offset observations for every host;
- request, connection, subject, assurance, Profile, policy revision, lease,
  runtime lane, browser, process, session, tab, target, view, handoff, route,
  display, provider, controller, viewer, job, event, trace, and incident IDs;
- precondition observations, declared stimuli, immediate responses, terminal
  outcomes, effect state, retry disposition, and downstream projections;
- command stdout and stderr, HTTP and MCP responses, service snapshots,
  journal slices, process and listener census, X11 socket and lock census,
  dashboard API snapshots, external-client telemetry, screenshots, and video;
- content digests, byte counts, capture gaps, redaction actions, and parent
  artifact hashes; and
- first-failure signature plus every later matching or divergent signature.

Artifacts are written atomically where possible, never overwritten, and joined
by a final manifest with SHA-256 digests. Sensitive-value canaries are injected
only as synthetic test data. The final auditor must prove that no canary or
forbidden private field appears in any response, log, report, screenshot
metadata, or committed artifact.

## Logging Completeness Oracle

For every accepted request, rejected request, timeout, cancellation, crash,
worker stop, queue-full result, wait reschedule, route failure, presentation
failure, Xvfb failure, supervisor transition, and dashboard action, the auditor
expects one causal envelope that can be followed without parsing error prose.

At minimum it must reconcile:

```text
ingress request
  -> immediate response
  -> durable job
  -> terminal Service event
  -> trace outcome
  -> incident when incident policy applies
  -> dashboard projection when operator-visible
```

The exact structured failure and immutable provenance must agree across every
surface that represents the outcome. Pre-dispatch and scheduler rejection are
not exceptions. The auditor reports missing rows, duplicate terminal rows,
conflicting IDs, broken parent links, timestamp inversions, terminal jobs with
null failure or provenance, unredacted private values, and outcomes visible in
only one transport. Logging completeness is reported as exact expected,
observed, missing, duplicate, conflicting, and redaction-violation counts, not
as a qualitative claim.

The bounded Service event ring is not the forensic authority. Every failed
browser launch, Guacamole or remote-view load, unusable durable handoff,
connected-but-non-streaming CDP feed, and failed dashboard action must also
produce an append-only `agent-browser.service-failure-record.v1` occurrence.
Server-observed failures are written at their authoritative terminal boundary.
Failures visible only to an authenticated external client use the restricted
failure-observation contract. Raw handoff IDs, operator URLs, credentials,
headers, page content, query strings, and bearer material are forbidden.
Journal write failure is itself counted and emitted to the process log so it
cannot silently erase the primary error.

## Historical Failure Families

The case registry must map every case to at least one source plan, note,
incident signature, or production sample. The initial closed-world families
are:

1. profile identity unproven or inconsistent during launch,
   `remote_view_open`, `tab_switch`, `view_focus`, retained reuse, and crash
   recovery;
2. self-conflicting exclusive or shared profiles, identical caller labels,
   stale owner generations, missing principal bindings, and legacy ambiguous
   sessions;
3. missing runtime-lane or Profile provenance, null structured failures,
   scheduler early return, duplicate or missing terminal records, and effect
   uncertainty after persistence timeout;
4. stale targets, duplicate same-origin tabs, `about:blank` selection, target
   disappearance, wrong-tab focus, and dashboard URL recovery;
5. route-pool exhaustion, stale route checkout, route/display crossover,
   finalization incompleteness, presentation-proof failure, and browser-window
   visibility failure;
6. raw provider or loopback URL exposure, external redirect or WebSocket
   failure, durable-handoff expiry, handoff-to-wrong-browser resolution, and
   reconnect that launches a duplicate browser;
7. X11 authorization denial, cross-generation Xvfb orphan collision under
   `PrivateTmp`, stale lock or socket evidence, concurrent display allocation,
   display-range exhaustion, PID reuse, and terminal or wrong-window focus;
8. supervisor non-cooperation, stale owner takeover, split executable
   generations, dashboard/backend/host mismatch, failed preserve transition,
   failed full shutdown, and installation convergence warnings on the wrong
   health axis;
9. Service State lock timeout, service-resources timeout, command timeout,
   queue pressure, slow or disconnected clients, cancellation races, worker
   stop, crash epochs, and stale retained state;
10. dashboard left-rail omissions, duplicates, stale actionable rows, foreign
    CDP promotion, wrong selected workspace, status-axis conflation, stream
    loss, deep-link drift, performance collapse, memory growth, focus loss, and
    responsive-layout defects; and
11. password-manager chooser, passkey, consent, secure-desktop, and native
    prompt visibility or focus failures during human-paced remote control,
    without capturing or automating secret material.

New families discovered during execution are appended to the registry as
findings. They do not expand the frozen execution schedule.

## Scenario Arsenal

### A: Agent-Only Browser And Permission Stress

| ID | Scenario | Required pressure and oracle |
| --- | --- | --- |
| A01 | Frictionless ephemeral debugger | 100 sequential and 25 concurrent self-declared clients acquire disposable sessions and tabs without principal enrollment, then release only their resources |
| A02 | Shared authenticated Profile | Ten compatible clients share one retained browser; repeat 20 times with attributable tabs and no Profile-wide self-conflict |
| A03 | Same-label collision | Ten live clients use the same label but distinct connection instances; cross-client command and tab theft must remain impossible |
| A04 | Policy-mode matrix | Exhaust administrator, participant, observer, `shared-local`, `restricted`, and `exclusive` operations across allowed and denied subjects |
| A05 | Revision and drain races | Concurrent widening, narrowing, admission, own-tab release, revision conflict, and drain completion at deterministic barriers |
| A06 | Revocation and eviction | Revoke during atomic and queued commands; exercise graceful and exact forced eviction against disposable tabs only |
| A07 | Crash and stale handles | Kill disposable Chrome at every command boundary, then test truthful handle refresh, stale-handle recourse, and retained policy identity |
| A08 | Ambiguous retained session replay | Recreate unproven and inconsistent identity states through launch, `remote_view_open`, `tab_switch`, and `view_focus` using redacted production-shaped fixtures |
| A09 | Target pathology | Combine missing targets, dead targets, duplicate same-origin tabs, blank tabs, popup targets, and rapid close/switch/navigation |
| A10 | Foreign CDP adjacency | Mix owned and non-owned browsers; prove inventory visibility never grants service lifecycle or route authority |
| A11 | Scheduler terminalization | Inject queue full, wait reschedule, cancellation, worker stop, pre-dispatch denial, and terminal persistence failure at every supported boundary |
| A12 | State-lock and timeout uncertainty | Induce lock wait, command timeout before effect, timeout after effect, and client disconnect; require exact effect state and recourse |
| A13 | Retained-session generation change | Transfer or reconnect disposable retained browsers across 25 planned daemon or supervisor generation transitions without cold launch |
| A14 | Full shutdown boundaries | Reject unauthorized, stale-digest, mismatched-target, and Profile-mutating plans; apply exact authorized shutdown only to a sacrificial runtime |
| A15 | Browser-history and Service-record parity | Navigate unique synthetic markers through CLI, HTTP, MCP, dashboard, and remote control; reconcile Chrome history and CDP targets with request, job, event, trace, incident, session, and tab records |

### H: Human-Simulated External Remote-View Stress

Human-simulated actions use the external client's ordinary pointer, keyboard,
focus, scrolling, resize, reload, back/forward, clipboard-safe text, and
disconnect behavior at human-observable pacing. CDP may observe the target but
must not perform the action being credited as remote human control.

| ID | Scenario | Required pressure and oracle |
| --- | --- | --- |
| H01 | External durable-handoff happy path | Open from two external networks or runners, verify ready pixels and identity, interact, close the client, and reopen the same handoff |
| H02 | Internal URL leak hunt | Scan every redirect, iframe, form, WebSocket, error action, reconnect target, and copied link; inject external-host and scheme variations |
| H03 | Route and display rebinding | Reopen one durable handoff after planned viewer expiry, route switch, display replacement, and provider-session replacement without a new browser |
| H04 | Observer and controller concurrency | Eight observers and two controller contenders exercise lease protection, transfer, stale input fencing, and independent reconnect |
| H05 | Human takeover | Transfer input from agent to human and back at deterministic barriers; old controller input must be fenced without ACL mutation |
| H06 | Window, tab, and focus truth | Alternate multiple windows, tabs, popups, native prompts, minimized and obscured states; visible pixels and selected target must agree |
| H07 | Route exhaustion | Occupy every route with healthy non-parkable controllers, request excess presentation, then observe truthful queued or denied state without fallback |
| H08 | Presentation failure matrix | Inject route unavailability, proof timeout, finalization interruption, stale checkout, wrong display, and browser-window-not-visible states |
| H09 | Adverse external network | Apply predetermined latency, jitter, bandwidth, packet loss, WebSocket interruption, TLS reconnect, and long-idle expiry profiles from the external side |
| H10 | Browser and service disruption | Crash the disposable browser and perform scheduled service-client or supervisor transitions while the same handoff remains bookmarked |
| H11 | Secure-surface visibility | Exercise synthetic chooser and prompt fixtures, then a bounded operator-assisted non-production LastPass test vault and test passkey relying party without recording secret input or vault content |
| H12 | Long-lived handoff | Reconnect the same durable URL 500 times across at least 24 hours, including client restarts and expired viewer/controller leases |

### X: Display, Supervisor, And Install-Coherence Stress

| ID | Scenario | Required pressure and oracle |
| --- | --- | --- |
| X01 | Cross-generation `PrivateTmp` Xvfb orphan | Leave a sacrificial old-generation Xvfb process on `:90` whose old namespace lock is invisible, start the frozen host, and observe allocator and logging behavior |
| X02 | Multi-process display race | Start concurrent allocator requests from separate daemon processes at deterministic barriers; no two live browsers may receive one display |
| X03 | Lock, socket, and PID disagreement | Cover stale lock, live socket without visible lock, abstract socket, dead PID, PID reuse, and owner-generation mismatch combinations |
| X04 | Display-range exhaustion | Occupy the configured range with owned and foreign fixtures, then request one more display and require bounded typed failure |
| X05 | X11 authority matrix | Vary cookie, authority file, user, environment, and display ownership while retaining exact diagnostic provenance |
| X06 | Desktop locator pathology | Exercise terminal-topmost, no browser window, wrong browser, obscured window, transient popup, resize, and focus races |
| X07 | Supervisor takeover | Run 25 planned stale-owner, unresponsive-owner, delayed-exit, duplicate-listener, and generation-replacement transitions |
| X08 | Preserve and full-shutdown install paths | In sacrificial homes, test coherent transfer, refused transfer, sealed full shutdown, interrupted effect, resume, and rollback evidence without rebuilding |
| X09 | Generation mismatch taxonomy | Inject host, backend, dashboard, manifest, helper, and browser digest mismatches one axis at a time and in combinations |
| X10 | Restart and crash epochs | Exercise boot ID, process epoch, socket epoch, route, handoff, and retained-browser identity across scheduled service and host restarts |

### D: Dashboard Performance And Left-Rail Truth

For every D case, compare the rendered dashboard to an independently captured
authoritative Service snapshot at the same correlation barrier. DOM text alone
is not sufficient; use screenshots, accessibility snapshots, network and
console logs, browser performance traces, and selected-record API readback.

| ID | Scenario | Required pressure and oracle |
| --- | --- | --- |
| D01 | Left-rail bijection | For empty, sparse, normal, and dense states, every actionable rail row maps to exactly one current controllable resource and every such resource has exactly one row |
| D02 | Churn accuracy | Create, update, crash, recover, transfer, close, evict, and expire resources while asserting row identity, label, count, ordering, badges, and action eligibility at barriers |
| D03 | Same-label and cross-Profile ambiguity | Render duplicate labels, many tabs, two windows, foreign CDP, historical rows, and access ambiguity without selecting or acting on the wrong resource |
| D04 | Multi-operator selection | Ten external dashboard clients navigate, refresh, back/forward, deep-link, and swap selections without cross-client selection leakage |
| D05 | Stale URL recovery | Exercise missing browser, session, tab, target, view, and handoff deep links; recover only to a semantically valid current target and explain the change |
| D06 | Warning-axis truth | Access ambiguity remains on the access axis; acquisition denial is request-scoped; only typed convergence failure produces runtime-out-of-sync messaging and one executable action |
| D07 | Event loss and reordering | Delay, duplicate, drop, and reorder snapshot and stream responses; the rail must converge or explicitly show stale state, never fabricate readiness |
| D08 | External handoff hygiene | Every dashboard copy/open action returns only the durable external handoff and never exposes loopback or provider internals |
| D09 | Dense-state performance | Test 100 Profiles, 500 browsers or historical rows, 2,000 tabs, 10,000 jobs, 10,000 events, and active stream churn through synthetic fixtures |
| D10 | Interaction performance | Measure initial load, rail update, selection, filtering, inspector open, viewport ready, and action feedback at p50, p95, p99, and worst case |
| D11 | Browser resource stability | Run an eight-hour dashboard session with churn, route video, selection changes, and reconnect; record heap, DOM nodes, listeners, CPU, network, and long tasks |
| D12 | Responsive, keyboard, and focus behavior | Cover small, typical, and wide viewports, keyboard-only operation, visible focus, modal focus return, reduced motion, loading, error, and overflow states |

External D03, D04, and supported D05 observations run only through the manually
dispatched `p158-w8-dashboard-external.yml` workflow. The workflow binds one
frozen action manifest to an exact action-specific public HTTPS route digest,
runs Chromium on a GitHub-hosted runner, and uploads an immutable terminal
result even when capture fails. It has no automatic trigger or retry. D05 tab
and target recovery are executable because the dashboard itself replaces the
stale tab selection and renders the recovery explanation. Browser, session,
view, and handoff target classes remain `skipped_blocked` until an equivalent
semantic product recovery route exists. The service-host handshake starts and
selects only the frozen action root, writes an append-only digest-only
dispatch-ready checkpoint plus a separate workflow manifest, and resumes only
when every process, port, root, candidate, and ingress identity is unchanged.
It consumes exactly one downloaded workflow terminal receipt bound to the
action, commit, workflow run, and attempt before exact teardown. A missing
terminal receipt pauses without dispatching or retrying. Lost claimed identity
becomes `effect_uncertain` and is never restarted. The workflow itself still
does not start or stop the service-host runtime, and this implementation has
not performed a live dispatch.

The action-route selector is frozen to cooper-webservices commit
`e70368ddbb2e61ae26a25072975c2953754b7479` and selector source SHA-256
`53a7ab94b7d40dc620b39bdae90b4429b2043e08776ca195dab8e5306bdd6f3e`.
W6 preparation seals both the reviewed selector source and executable digests.
The host then submits only the exact `/p158/<run>/<action>` identity and
digest-only process, root, and port bindings. Selection is an explicit
apply-gated pre-dispatch operation. Resume uses the read-only observation
operation, which independently rerenders and reads back the deployed route.
Neither operation returns a raw public origin, provider URL, internal URL, or
loopback URL. No selector apply, deployment, or restart was performed while
adding this contract.

### C: Combined Deterministic Pressure

Run the combined phases only after their declared prerequisites have terminal
results. They consume the state left by earlier cases rather than repairing it.

| Phase | Fixed workload |
| --- | --- |
| C01 calibration | 20 minutes, 25 agent clients, two external viewers, one controller, 500 service commands, 50 dashboard actions, and ten handoff reconnects |
| C02 burst | 100 agent clients, ten dashboard clients, maximum route occupancy, 2,000 service commands, 500 dashboard actions, 100 reconnects, and 20 controlled browser crashes |
| C03 generation churn | 25 scheduled supervisor transitions interleaved with retained-browser commands, dashboard use, and durable-handoff reopen attempts |
| C04 eight-hour production observation | Observe at least eight continuous hours of real production telemetry, split into explicit runtime epochs when installs or repairs occur; never generate tenant effects or delay intervention |
| C05 24-hour production handoff observation | Observe durable-handoff health across at least 24 elapsed hours of real use and idle periods; repairs and deployments remain allowed and create analyzable epoch boundaries |

Counts are minimum execution bounds, not success metrics. A safety stop may end
a phase early, but every unexecuted case must become `safety_stopped` or
`skipped_blocked` with exact evidence rather than disappearing from totals.

## Performance Budgets

Freeze numeric budgets from a clean calibration run before campaign execution.
The final report must show both the frozen environment-relative thresholds and
these absolute ceilings:

- external handoff first usable pixels: p95 at most 10 seconds, with no sample
  above 30 seconds;
- dashboard initial interactive state for the normal inventory: p95 at most 3
  seconds;
- left-rail authoritative update after a committed Service change: p95 at most
  1 second and no silent miss;
- selected-record action feedback: p95 at most 500 milliseconds before visible
  pending or terminal feedback;
- ordinary agent-only command excluding navigation: p95 at most 1 second;
- no unbounded upward trend in dashboard heap, DOM nodes, browser processes,
  Xvfb processes, route allocations, Profile leases, retained sessions, or
  unresolved jobs during steady portions of the soak; and
- zero wrong-resource actions, internal handoff URL leaks, duplicate physical
  ownership, missing terminal outcomes, or sensitive-value leaks.

A threshold miss is recorded. It does not trigger tuning during the campaign.

## Safety Stops

The controller stops the affected environment, captures one final read-only
census, and marks remaining dependent work when any predefined guard fires:

- host memory or swap reserve falls below the repository's configured safe
  runtime floor for two consecutive samples;
- artifact storage exceeds its reserved quota or the filesystem reaches 90
  percent utilization;
- process, display, route, or connection counts exceed the manifest's hard
  ceiling;
- production identity, Profile, route, credential, or tenant state appears in
  the isolated environment;
- synthetic secret canaries appear in an external response or unredacted log;
- external traffic escapes the allowlisted ingress and synthetic target set;
- the campaign cannot distinguish its disposable target from foreign or
  production state; or
- continued execution risks corrupting evidence already collected.

The stop action itself may terminate the campaign load generator. It may not
silently repair the tested runtime inside the same frozen epoch. A blocking
repair begins only after the controller seals the partial epoch and records the
pause reason. Emergency host protection outside the controller is reported as
an external intervention and invalidates subsequent frozen-state comparisons
until a new epoch is established.

## Work Units And Dependencies

| Unit | Scope | Depends on | Exit condition |
| --- | --- | --- | --- |
| W1 | Freeze the historical failure registry, production-shaped redacted fixtures, candidate manifest, resource ceilings, and case dependency graph | none | Every known failure family maps to cases and evidence sources; no open-ended discovery remains in execution |
| W2 | Build the append-only controller, deterministic scheduler, artifact manifest, fault injectors, safety monitor, and result schema | W1 | Provider-free self-tests prove no overwrite, no opportunistic retry, correct blocked propagation, and reproducible seeds |
| W3 | Build the cross-surface logging auditor, durable failure journal, authenticated external-observation intake, and synthetic sensitive-value scanner | W1, W2 | Deliberately missing, duplicate, conflicting, reordered, null, leaking, launch, Guacamole, handoff, CDP-stall, and dashboard-action records are all detected |
| W4 | Build the external-ingress runner and durable-handoff oracle | W1, W2 | A synthetic good path passes and loopback, private, raw provider, wrong-browser, and duplicate-launch fixtures fail |
| W5 | Build dashboard truth and performance probes plus large synthetic fixtures | W1, W2, W3 | Left-rail bijection, warning-axis, deep-link, multi-client, accessibility, and performance probes detect seeded defects |
| W6 | Publish and install one isolated candidate, prepare E1 and E2, verify the failure journal, capture calibration, then freeze | W2, W3, W4, W5 | Candidate and environment digests are sealed; the journal survives malformed lines and captures all five named failure surfaces; no test case has started |
| W7 | Execute A and X scenarios without repair | W6 | Every scheduled A and X attempt is terminal and raw evidence is append-only |
| W8 | Execute H and D scenarios exclusively through external ingress where operator-visible | W6 | Every scheduled H and D attempt is terminal; every visibility pass has external proof |
| W9 | Execute C01 through C03, schedule asynchronous C04 and C05 production observation epochs, perform isolated teardown, and seal deterministic evidence | W7, W8 | Every deterministic case is terminal, production observers are independently durable, teardown is recorded, raw artifacts are hashed, and no further isolated execution is permitted |
| W10 | Deeply analyze deterministic findings and completed production observation epochs, then publish the redacted review | W9 | Causal clusters, logging gaps, performance distributions, historical reproduction rates, epoch-aware architecture implications, and a bounded remediation backlog are independently checked and source-backed; incomplete endurance windows remain explicit evidence gaps and never block installation or repair |

Critical path:
`W1 -> W2 -> (W3, W4, W5) -> W6 -> (W7, W8) -> W9 -> W10`.
W7 and W8 may execute concurrently only when their disposable Profile,
display, route, and external-client ownership is disjoint. The campaign
controller, evidence writer, safety monitor, and final analysis each have one
owner. Nonblocking defects have no repair edge in this campaign and enter the
W10 remediation ledger. A sequence-blocking defect adds one bounded edge from
the affected work unit to `pause -> seal partial epoch -> diagnose -> repair ->
validate -> install new candidate -> resume blocked sequence`. The campaign
controller owns that edge. Each traversal preserves the failed epoch and uses
new candidate, environment, case-attempt, and evidence identities. Blocking
repair traversals are not consumable approval tokens and have no arbitrary
numeric stop. Continue sequential diagnosis, red-green repair, validation,
installation, and new-epoch retry while the blocker has a deterministic,
safe, in-scope repair seam. Stop only when continued work would cross a safety,
scope, authority, or resource ceiling; evidence can no longer be preserved; or
the blocker cannot be made deterministic after the bounded diagnosis loop.
That stop proceeds to W10 with the missing cases explicit rather than hiding
the incomplete sequence or repairing indefinitely without a pass/fail signal.

## Final Deep Review Protocol

W10 is the last step and may not begin until the sealed manifest proves that
every scheduled case is terminal. It performs no new browser, dashboard,
runtime, provider, Profile, route, or repair effects.

The review must:

1. verify artifact hashes, candidate identity, clock alignment, case counts,
   exclusions, external vantage, and freeze-policy compliance;
2. independently recompute logging completeness and sensitive-value leakage
   from raw indexes rather than trusting harness summaries;
3. group failures by observable signature and causal chain, separating product,
   infrastructure, harness, safety-stop, and inconclusive outcomes;
4. build event timelines for every reproduced historical failure and every
   high-severity new failure, including the earliest divergence from expected
   state;
5. compare reproduction frequency across seeds, concurrency, transports,
   Profiles, runtime generations, route states, network profiles, and time;
6. reconcile dashboard pixels and left-rail state with authoritative Service
   snapshots, including every wrong, missing, duplicate, stale, or late row;
7. analyze p50, p95, p99, worst case, long tasks, resource slopes, timeout
   distributions, and correlations between pressure and failure;
8. assess whether findings support or contradict the architecture review's
   Profile acquisition owner, cohesive lease client, Rust convergence owner,
   contract-aware output renderer, and semantic contract oracle boundaries;
9. identify failures a focused green suite could not observe and assign each
   to the cheapest durable regression seam that can protect it;
10. classify each finding as `blocking`, `nonblocking_backlog`,
    `needs_evidence`, or `rejected`, with criterion, evidence, consequence,
    reproducer, confidence, and recommended owner;
11. produce a remediation dependency graph that keeps product defects,
    infrastructure defects, logging defects, and harness defects distinct;
    and
12. state exactly which P157 acceptance criteria are proven, disproven, or
    still untested. Test volume or a lack of crashes cannot substitute for this
    criterion-by-criterion judgment.

One fresh-context evidence review checks the primary analysis against the
sealed artifacts. Disagreements are recorded, not optimized away. Remediation
for nonblocking findings begins in successor work after W10 closes. A repair
performed earlier under the explicit sequence-blocking pause path remains part
of the campaign history and must be analyzed as a separate candidate epoch.

## Acceptance Criteria

1. The registry demonstrates closed-world coverage of every historical failure
   family named in this plan and links each family to executed or explicitly
   blocked cases.
2. Every active candidate epoch is immutable while it runs. Nonblocking defects
   are deferred. A sequence-blocking defect may pause and seal the current
   epoch, receive a diagnosed and validated repair, and resume only under a new
   frozen candidate identity without erasing or combining the prior evidence.
3. Agent-only, human-simulated external remote-view, display/supervisor,
   dashboard, and combined deterministic tiers reach terminal evidence states.
   Eight-hour and 24-hour production observations are epoch-aware and may
   remain explicit evidence gaps without blocking installation or repair.
4. Every human-visible success is proven through external ingress and the
   durable handoff URL. Zero internal, loopback, private, raw provider, embed,
   health, or route-binding URL is accepted as an operator handoff.
5. The same durable handoff is tested across route, display, viewer,
   controller, client, browser, and scheduled supervisor transitions without
   accidental cold launch.
6. Dashboard performance is quantified, and left-rail rows, selected state,
   actions, warnings, and visible browser pixels are reconciled against
   authoritative Service truth under churn and dense load.
7. Every scheduled failure path has complete or explicitly missing causal
   logging measurements across response, job, event, trace, incident, and
   dashboard projections.
8. First failures, flakes, harness failures, inconclusive cases, safety stops,
   blocked cases, and external interventions remain visible in totals and raw
   evidence.
9. No production mutation, credential capture, private page-body capture,
   bearer-material capture, or cross-environment resource use occurs.
10. W10 produces a source-backed final report, a machine-readable result set,
    and a prioritized remediation graph without performing repairs.
11. Every failed launch, Guacamole load, handoff use, CDP stream, and dashboard
    action is either joined to a durable failure-journal occurrence or reported
    as an exact logging gap. Journal retention never depends on the bounded
    Service event ring or successful `state.json` reconciliation.

## Initial Checkpoint

State transition: `unregistered -> planned`.

Acceptance state: P157 source behavior remains complete, but adversarial
installed acceptance is reopened under this frozen-candidate campaign.

Progress classification: `outcome_progress`.

Evidence: P157's original W11 exit contract required multi-client sharing,
live policy edits, eviction, crash recovery, logging completeness, warning
taxonomy, and disposable shutdown. Focused installed validation passed, while
historical production records still contain identity, Xvfb, route,
presentation, and resource-timeout failure signatures. P46 also documents
dashboard, target, route, display, and external-viewer defects that were
repaired between attempts rather than measured against one frozen candidate.

Material blocker: the existing scenario harnesses do not provide one
append-only, no-repair controller, external-ingress handoff oracle, or complete
causal-log auditor for this closed-world matrix.

Next action: execute W1 only. Freeze the case registry and evidence sources,
including an exact mapping from each historical report to one or more case IDs,
before implementing load generators or touching an installed runtime.

## W1 Checkpoint: Historical Registry Frozen

State transition: `planned -> registry_frozen`.

Acceptance state: W1 complete. Installed acceptance remains open and no
candidate or runtime has been touched.

Progress classification: `outcome_progress`.

Evidence:

- `docs/dev/contracts/p158-historical-failure-registry.v1.json` freezes 11
  source-backed historical families, all 54 scheduled case or phase IDs,
  deterministic execution bounds, case dependencies, evidence profiles,
  candidate identity fields, and numeric resource and performance ceilings;
- `docs/dev/fixtures/p158/historical-failure-seeds.v1.json` preserves seven
  redacted production signatures plus the null terminal-envelope defect using
  synthetic relationship-preserving values;
- the read-only 200-job production comparison recomputed 34 failed and five
  timed-out jobs, with all 39 lacking top-level structured failure and
  provenance; and
- `pnpm test:p158-historical-failure-registry` validates registry closure,
  source existence, bidirectional family mappings, the complete case arsenal,
  dependencies, no-repair rules, and fixture redaction.

Historical harness adjudication: P46 and P67 fixtures and evidence mechanics
may be adapted, but their reset, repair, retry, reconcile, cleanup, mutable
summary, and loopback-fallback behaviors may not cross the P158 freeze point.
P67 rail persistence is not rendered-dashboard proof, and local Chromium is
not an E2 external-network vantage.

Material blocker: no append-only campaign controller or deterministic
scheduler exists yet. W1 intentionally produced no runtime or provider effect.

Next action: execute W2 only. Build the monotonic controller, deterministic
scheduler, atomic artifact manifest, fault-injector interface, safety monitor,
and terminal result schema against provider-free fixtures.

## W2 Checkpoint: Append-Only Controller Complete

State transition: `registry_frozen -> controller_complete`.

Acceptance state: W2 complete. Installed acceptance remains open and no
candidate or runtime has been touched.

Progress classification: `outcome_progress`.

Evidence:

- `scripts/lib/p158-campaign-controller.js` provides the provider-free
  monotonic controller, deterministic scheduler, exclusive atomic store,
  append-only typed ledger, safety monitor, teardown gate, evidence seal, and
  integrity verifier;
- `docs/dev/contracts/p158-campaign-manifest.v1.schema.json` and
  `docs/dev/contracts/p158-campaign-result.v1.schema.json` define the frozen
  manifest and every persisted ledger record;
- `scripts/test-p158-campaign-controller.js` performs 11 adversarial behavior
  tests and strict Ajv validation of the actual manifest and every emitted
  ledger record; and
- `pnpm test:p158-campaign` passes the registry and controller batteries.

Integration review rejected the initial parallel drafts until actual
controller output conformed to the schemas. It also corrected an in-memory-only
terminal-count mutation, removed the incorrect assumption that every non-pass
loses downstream prerequisites, rejected unknown evidence artifact IDs, and
aligned the campaign-process safety metric.

Material blocker: the controller preserves evidence but W3's cross-surface
logging completeness and sensitive-value scanner do not exist yet.

Next action: execute W3 only. Build the logging auditor and synthetic leakage
scanner against intentionally missing, duplicate, conflicting, reordered,
null, and leaking fixtures.

## W3 Checkpoint: Logging Completeness Auditor Complete

State transition: `controller_complete -> logging_auditor_complete`.

Acceptance state: W3 complete. Installed acceptance remains open and no
candidate or runtime has been touched.

Progress classification: `outcome_progress`.

Evidence:

- `scripts/lib/p158-logging-auditor.js` reconstructs causal envelopes across
  requests, immediate responses, durable jobs, events, traces, incidents,
  dashboard projections, artifacts, and redaction receipts;
- the two P158 logging schemas define the synthetic input and exact audit
  report contracts;
- the 13-fixture corpus isolates all 11 required defect classes plus complete
  and reordered-clean controls; and
- `pnpm test:p158-logging-auditor` performs strict schema, deterministic,
  no-mutation, exact-count, per-envelope, surface-correlation, and clean-control
  checks.

Integration review required report-schema conformance and fixture correlation,
then expanded terminal inspection to immediate responses, durable jobs, events,
traces, and dashboard projections. The production-shaped null fixtures now
prove missing failure and provenance on terminal durable jobs specifically.

Material blocker: no external-ingress runner or durable-handoff URL oracle
exists yet.

Next action: execute W4 only. Build the provider-free external-ingress and
durable-handoff oracle, including hard rejection of loopback fallback.

## W4 Checkpoint: External Handoff Oracle Complete

State transition: `logging_auditor_complete -> external_oracle_complete`.

Acceptance state: W4 complete. Installed acceptance remains open and no
candidate or runtime has been touched.

Progress classification: `outcome_progress`.

Evidence:

- `scripts/lib/p158-external-handoff-oracle.js` classifies external vantage,
  public HTTPS and WSS, forbidden hosts and URL roles, ingress checks,
  ready-before-pixels, retained identity, durable continuity, and cold launch;
- the two P158 external-handoff schemas define synthetic inputs and exact
  reports;
- the 36-session corpus covers all 23 finding codes, 13 URL roles, nine host
  classes, six identity axes, and one clean full-ingress path; and
- `pnpm test:p158-external-handoff-oracle` performs strict schema,
  classification, no-fallback, ingress, visibility, identity, continuity,
  deterministic, no-mutation, and no-repair checks.

Integration review changed the ingress checks from optional-by-absence to the
full eight-check fail-closed default. Isolated unit fixtures can suppress
unrelated gates; later E2 execution cannot.

Material blocker: correlated rendered-dashboard truth and performance probes
do not exist yet.

Next action: execute W5 only. Build provider-free dashboard rail, selection,
warning-axis, external URL hygiene, accessibility, and performance probes over
dense immutable fixtures.

## W5 Checkpoint: Dashboard Truth And Performance Oracle Complete

State transition: `external_oracle_complete -> dashboard_oracle_complete`.

Acceptance state: W5 complete. Installed acceptance remains open and no
candidate, browser, provider, or runtime has been touched.

Progress classification: `outcome_progress`.

Evidence:

- `scripts/lib/p158-dashboard-oracle.js` materializes immutable dashboard
  fixtures and audits rail bijection, stable identity, selection, inspector and
  action targeting, multi-client isolation, warning axes, handoff URL hygiene,
  stream convergence, browser evidence, timing distributions, and resource
  slopes;
- the two P158 dashboard schemas require strict synthetic inputs, all 46 exact
  finding counters, deterministic reports, and explicit no-repair evidence;
- the 51-case corpus includes four clean inventory densities, one clean typed
  convergence control, and one isolated seed for every finding class; and
- `pnpm test:p158-dashboard-oracle` proves the exact dense inventory of 100
  Profiles, 500 browsers, 2,000 tabs, 10,000 jobs, and 10,000 events is
  materially generated as 22,600 resources and audits cleanly with 600 rail
  rows.

Integration review corrected false zero resource budgets, dense ordering,
duplicate-row cascade findings, absent same-label semantics, incomplete
viewport controls, and missing explicit multi-client leakage coverage. The
primary agent independently reran W1 through W5 provider-free batteries.

Material blocker: no candidate or isolated E1 and E2 environment has been
published, calibrated, or frozen. W5 intentionally performed no live effects.

Next action: execute W6 only. Publish and install one isolated development
candidate, prepare distinct E1 and E2 environments, prove genuinely external
ingress, capture calibration, and seal candidate and environment digests before
any campaign case starts.

## W6 Preflight Checkpoint: Preparation And External-Ingress Contract Complete

State transition: `dashboard_oracle_complete -> w6_preflight_complete`.

Acceptance state: W6 remains open. No new candidate has been installed and the
campaign is not frozen or executing.

Progress classification: `outcome_progress`.

The first W6 readiness audit found that the prior development provider called
a loopback HTTP address its public operator URL. It also found that the
campaign manifest could not seal E1 and E2 identities, calibration, artifact
receipts, or an actual zero-start freeze instant. Executing W6 against those
contracts would have produced ambiguous evidence.

The prerequisite repair now provides:

- a reviewed public HTTPS operator origin and external-ingress revision as one
  atomic, fail-closed development-provider binding;
- a v2 provider manifest that retains loopback only as a local diagnostic URL
  and carries the external binding into route authority;
- manifest-bound installation, runtime, provider, ingress, external-vantage,
  handoff, calibration, and fixture artifacts;
- exact E1 and E2 environment seals, including two distinct off-host client
  identities and all eight external ingress observation classes;
- a canonical candidate digest and aggregate fixture digest;
- an immutable `campaign-freeze.json` receipt with the actual wall and
  monotonic freeze time and zero started case and attempt counts; and
- a no-repair preparation orchestrator with 26 isolated fail-closed fixtures.

`pnpm test:p158-campaign-preparation` passes all exact classifications, strict
schemas, persisted bytes and hashes, nested W4 validation, freeze ordering,
calibration chronology, deterministic output, input immutability, and the
zero-start gate.

The provisional binary built before these source corrections was not installed
and is not a candidate. W6 still requires one fresh build and isolated install,
external runner evidence, the exact 20-minute C01 calibration, environment
sealing, and controller freeze.

Next action: rebuild from the clean checkpoint, install only into the
development pseudo-home, configure the reviewed public HTTPS binding, prove E1
and E2 including two off-host clients, calibrate, and freeze without starting a
campaign case.

## W6 Preflight Revision: Durable Failure Journal And Nonblocking Endurance

State transition: `w6_preflight_complete -> w6_failure_journal_candidate`.

Acceptance state: W6 remains open. No candidate was installed and no live
campaign, production mutation, or external dispatch occurred in this revision.

The operator rejected a campaign design in which an eight-hour or 24-hour wait
could delay installation or repair. C04 and C05 now consume passive,
privacy-bounded production observation epochs. Deterministic destructive and
concurrency stimuli remain confined to C01 through C03 in E1 and E2. A
production install or repair closes the current observation epoch and starts a
new one without invalidating prior records or blocking the intervention.

The source candidate now adds a forensic journal independent of the bounded
Service event ring and `state.json`. It records browser-launch and terminal
service-action failures at server boundaries. Authenticated dashboard clients
can submit only the four externally observable categories through a strict
allowlist contract. Dashboard fetch failures, unusable handoffs, Guacamole
iframe failures, and CDP streams that connect without frames are wired to that
intake. Handoff IDs are accepted only as SHA-256 identifiers, summaries are
redacted and bounded, details are capped, records carry boot and runtime
environment identity, concurrent appends are locked and synced, and malformed
journal lines do not prevent later readback.

Focused validation at this checkpoint:

- four Rust journal tests pass, covering append behavior, URL and secret
  redaction, malformed-line recovery, latest-record reads, oversized details,
  raw-handoff rejection, and client-category confinement; and
- the optimized dashboard production build passes with the fetch observer,
  Guacamole observations, durable-handoff observation, and CDP frame watchdog.

Material blocker: the journal changes have not yet passed the full Rust,
clippy, dashboard contract, documentation, installed-development, and live
five-surface fault-injection gates. Root filesystem utilization also remains a
W6 safety input and must be measured before candidate publication.

Next action: finish contract and documentation parity, add exact source and
live journal coverage to the P158 logging auditor, run the selected validation
matrix, then reassess W6 publication safety without waiting for C04 or C05.

## W6 Preflight Revision: Last30Days Route-Acquisition Regressions

State transition: `w6_failure_journal_candidate ->
w6_preparation_contract_complete_last30days_regressions_integrated_w6_publication_blocked`;
the campaign has not crossed the freeze point.

Acceptance state: the provider-free Last30Days regressions are integrated into
the P158 harness. Candidate publication, installed development validation, and
live campaign execution remain open.

The source observation is the committed Last30Days handoff note at commit
`cc0cd29c` on branch `docs/reddit-handoff-errors-20260902`. It describes five
bounded Reddit acquisition attempts without contributing credentials, cookies,
page bodies, raw handoff URLs, or provider data to this repository state. The
findings map to existing P158 cases and do not add attempts or expand the
frozen 54-case schedule:

| Finding | Provider-free regression | Existing live case obligations |
| --- | --- | --- |
| Route-bearing `tab_new` accepted but launched an unrelated private display | Reject route and display intent before job creation and build a privacy-bounded pre-job failure record | A11 must correlate the rejected ingress with the failure journal; X03 must prove no unrelated display is allocated |
| A released display-number key described another display and blocked a healthy route | Seed `remote-view-display:10` for released `:11`, request a route targeting `:10`, and require a distinct route-derived allocation identity | X03 must replay the contradiction in E1; H08 must prove planning and route-open failure truth without manual repair |
| Successful `remote_view_open` did not give the caller an automation capability | Require the valid nested handle to be projected as top-level `serviceTabHandle` | H01 must immediately run one handle-scoped read after external route open; A15 must preserve that handle identity across request, job, trace, target, and release evidence |
| Direct CLI and authenticated Service authority were confused | Documentation directs protected route acquisition through authenticated `service_request` | A08 and H08 must record explicit Service-required recourse when the direct CLI cannot satisfy protected-profile authority |
| Pre-job rejection had no durable Service job to inspect | HTTP and MCP normalization failures build an append-only failure-journal occurrence without retaining request bodies | A11 and the W3 logging auditor must count the record, preserve the exact error code, and report a missing journal append as a logging defect |

`scripts/test-p158-last30days-regressions.sh` is now a first-class E0 battery.
It runs the six exact Rust regressions for route-intent rejection, collision-
resistant allocation, route-bound dry-run planning, rollback identity, and
route-open handle projection, then runs the generated-client and remote-view
documentation contracts. `pnpm test:p158-last30days-regressions` runs that
battery directly, and `pnpm test:p158-harness` includes it before the logging
auditor and live-driver contract suites.

This regression battery does not claim the corresponding live cases are
executable. Current source readiness remains:

| Work unit | Concrete source-bound cases | Still missing or explicitly blocked before execution |
| --- | --- | --- |
| W6 | Provider-free preparation, logging, external-ingress, dashboard, evidence, and analysis contracts | Fresh optimized candidate build, isolated development install, five-surface live journal injection, two off-host external clients, 20-minute C01 calibration, exact environment seals, and zero-start freeze receipt |
| W7 | A01, A02, A03, A05, A08, and A13 have specialized concrete driver bundles | A04, A06, A07, A09, A10, A11, A12, A14, A15, X01 through X05, and X07 through X10 still require complete effect-time materializers or product seams; X06 is exercised with human-visible W8 work |
| W8 | H01, H03, D01, D03, and D04 have receipt-bound external driver paths | H02 and H04 through H12 plus D02 and D05 through D12 remain explicit-blocked until their external producers and exact oracles exist |
| W9 | C01 through C05 orchestration, conditional concrete drivers, endurance epoch contracts, teardown, and evidence sealing exist | No W9 phase is executable until W7 and W8 are terminal and its exact distributed, external-receipt, declared-transition, observation-window, and teardown inputs are sealed |
| W10 | Deterministic analyzer, descriptor, runner, schemas, and provider-free tests exist | It cannot run until W9 seals every scheduled result and the available C04 and C05 production epochs; incomplete endurance epochs remain explicit gaps rather than repair blockers |

The exact retained-allocation quarantine or rekey operation and a general
retained-target handle-adoption action remain product gaps. They stay visible
as blocked assertions under X03 and A15 instead of being treated as completed
by the collision-resistant allocator and route-open handle projection.

Validation at this checkpoint:

- the new E0 regression command passes all six exact Rust tests plus the
  client and documentation contracts;
- the complete 1,934-test parallel-safe Rust partition and every serial
  environment-mutating partition pass across the combined current worktree;
- clippy with warnings denied, generated Service client checks, the full
  Service client suite, docs production build, formatting, and diff checks
  pass; and
- the user-scoped failure journal remains absent after provider-free tests,
  proving the tests did not contaminate production evidence.

Resolved blocker: the operator raised the frozen W6 filesystem safety ceiling
from 85 percent to 90 percent. Root filesystem utilization was 87 percent at
the revision point, which is inside the revised ceiling. Publication still
requires a fresh read-only preflight; the prior storage reading no longer
blocks the isolated candidate.

Next action: implement the remaining A11 live scheduler-rejection and journal
correlation seam, because it directly exercises the new pre-job failure path.
Then finish the five-surface live journal calibration, satisfy the W6 storage
preflight, publish one candidate, and freeze E1 and E2.

## W6 Live Checkpoint: Candidate Installed, External Provider Ready, A11 Partial

State transition: `w6_preparation_contract_complete_last30days_regressions_integrated_w6_publication_blocked ->
w6_candidate_installed_provider_ready_a11_predispatch_live_passed_freeze_open`.

Acceptance state: W6 remains open. The candidate and external presentation
provider are installed and ready, but E1 and E2 are not sealed, the 20-minute
calibration has not run, and the campaign has not crossed the zero-start freeze.

The fresh filesystem preflight measured 73 percent utilization against the
operator-approved 90 percent ceiling. Development generation
`0.28.0-2f9d25956e27` is selected with executable SHA-256
`2f9d25956e274af52dd439c4c37e27b22a83a014504d5abc25ee398f737474e2`.
Three disposable browser-launch smoke iterations passed, and each development
installation reported production unchanged.

Live A11 pre-dispatch evidence now proves that one route-bearing `tab_new`
request is rejected before a correlated job exists and produces exactly one
failure-journal record with the request identity and expected normalization
classification. Its receipt SHA-256 is
`8524ec79173663e65e83aaa1add0681fac3540bfd839e51257383b47ed6523d6`.
This is a partial A11 probe only. Queue-full, wait-reschedule, cancellation,
worker-stop, and terminal-persistence-failure boundaries remain unexecuted.

The live probe also exposed a dashboard routing defect: the authenticated
frontend forwarded `/api/service/request` to a backend-only dashboard that
could not rediscover itself and returned HTTP 503 before normalization or
journaling. The candidate now honors the configured backend port and maps all
authenticated dashboard Service requests to the backend command route. The
same A11 probe then passed against the installed candidate.

Provider staging exposed a second pre-freeze defect. The reviewed v2 external
binding correctly rejected the installed v1 loopback authority as drift, but
offered no exact migration path. Staging and explicit apply now admit only the
exact additive v1-to-v2 authority upgrade: the old loopback public URL must
equal the new local diagnostic URL and every remaining provider identity must
match. Status remains drifted until explicit apply writes v2 authority.

The upgraded provider is ready and binds public operator origin
`https://agent-browser-dev.ecochran.dyndns.org` to reviewed Cooper revision
`e70368ddbb2e61ae26a25072975c2953754b7479` with binding SHA-256
`4f24eefcac1008871c90c8e41804029aff8747c9ee0bc13ad7ebe58ad0539c4d`.
The provider manifest and route inventory SHA-256 values are respectively
`177a90339506a8f6bbb44873968923e65e0cac33f3bcbf387a893e50e5d58dd8`
and `3dc7955d8b8fa7c03ff5eb77b88dc45810f1cc19c3f69467c0b7605220367448`.
The apply receipt SHA-256 is
`35971c562177ffef2b02c785afabe87af4f749bbed4db8fd78ac081dd3f560c4`,
and the apply reported production unchanged.

The Cooper inventory, raw dashboard, local Traefik route, bastion host-header
route, and public HTTPS root all validated. Public HTML and response headers
contained no loopback or raw development-port leakage. Unauthenticated public
`/guacamole/` returned a same-origin login redirect. The bastion publication
added the reviewed Guacamole path router and restarted only bastion Traefik;
the bastion operations log records the change. This host-local and bastion
evidence does not satisfy the two-client off-host W6 requirement.

Validation at this checkpoint includes the full P158 harness after the final
dashboard routing and provider migration changes, the exact A11
module tests, the exact Rust pre-dispatch and dashboard backend-port tests,
clippy with warnings denied, formatting, diff checks, development provider
fixtures, live provider staging and preflight, provider apply, provider-required
doctor, route readbacks, and the three-iteration development browser smoke.
The full 1,934-test Rust partition predates the final dashboard routing repair;
the exact affected Rust tests, full P158 harness, and clippy passed afterward.

A post-install route-confusion gate and independent CDP streaming smoke then
found a live host-capacity failure before their product assertions. Two Cargo
attempts, at eight and four build jobs, failed to spawn Rust or Tokio workers.
The CDP smoke independently exhausted all three built-in Chrome launch attempts
with `pthread_create` and zygote fork `EAGAIN`. Kernel evidence identifies the
Cargo aggregate cgroup's 1,024-task controller as the direct Cargo rejection:
six completed test or build scopes remained active with 945 tasks, including
five test scopes retaining Chrome process trees. The user session had about
6,200 threads at observation time despite ample available memory. The test
attempts did not kill, garbage collect, restart, or repair those residues.
This is a new recorded failure and a W6 freeze blocker. The Service action
failure returned `effect_uncertain` with `blind_retry` prohibited, but the
temporary smoke root was removed and no durable failure-journal occurrence was
available afterward. That cleanup behavior is itself a logging-evidence gap.

Next action: preserve this pre-freeze state and diagnose the retained Cargo
scope and temporary-root evidence-loss findings without broad cleanup. Then
commit and publish the current source checkpoint so the immutable external
workflow can execute it, run the two-client off-host readiness and frozen
20-minute calibration receipts, finish the five-surface live journal
calibration, and seal exact E1 and E2 identities. Only then may the controller
write the zero-start freeze receipt.

### W6 External Handoff Admission and Checkout Finding

The immutable source checkpoint is commit
`0070fb0d3c70c364166f3cc6f9a396ab45fed041`, published to
`origin/plan/profile-permissions-and-request-provenance`. A fresh development
Service status read found four warm idle presentation slots and no retained
development browser or durable handoff that could be reused for the external
fixture.

One explicit unknown durable profile reproduced
`existing_session_profile_identity_unproven` before browser launch as job
`r925418`. The failure journal retained the typed profile-lease axis, no-effect
classification, `inspect_before_retry` disposition, and hard stops against a
blind retry or duplicate profile lane. The broker-first access plan for the
same synthetic request then selected deterministic disposable profile
`managed-ephemeral-55469e27a903`, admitted the self-declared client under the
shared-local policy, and required no lease choreography. This demonstrates
that the new promiscuous default works when clients follow the access plan,
while an explicit unknown durable identity still enters the strict legacy
path.

Two route-bound launches using that managed-one-time profile successfully
started headed stealth Chromium, navigated to the loopback-only synthetic
fixture, and retained launch, tab, stderr-path, and polite-close events. Both
then failed during route checkout. Automatic Route 1 job `r729795` and explicit
Route 2 job `r156356` each changed the selected route to `pending` during
acquisition and then rejected that same route as
`route_pool_entry_unavailable`. In both cases the newly launched browser was
closed politely and the acquisition rollback reported the route and pool entry
restored. The repeated failure is therefore a deterministic
acquisition-to-checkout state-transition defect, not Chrome launch pressure or
a lack of provider capacity.

Code-level diagnosis confirms the transaction-ordering defect. The coordinator
calls `begin_route_bound_handoff_plan_acquisition()` before browser launch and
later calls `runtime.checkout_route()` with the original checkout command.
Checkout rebuilds the acquisition plan from the newly persisted state. At that
point the selected entry is `state=pending`, its readiness component is
`remote_view_open_acquisition`, and its route is not yet bound to the new
browser. `service_remote_view_acquisition_plan_from_state()` accepts pending
only when `checked_out_route_matches_owner()` already succeeds, while the
separate `ensure_route_pool_entry_ready_for_checkout()` helper explicitly
allows this acquisition-owned pending state. The planner therefore rejects the
coordinator's own reservation before checkout can create the owner binding.
The repair must make those two readiness gates agree without admitting foreign
or stale pending acquisitions.

The two checkout failures are durably present in Service jobs and correlated
terminal events, and the browser launch, synthetic URL navigation, stderr log
path, and cleanup are present in Service events. Their generic failure objects
degrade to `service_operation_failed`, `axis=unknown`, and
`effectState=effect_uncertain` even though the appended error text reports a
completed rollback and closed browser. That mismatch is an additional failure
normalization defect. No third route was attempted, no route repair or broad
cleanup was applied, and no durable public handoff was minted. The protected
GitHub environment for `.github/workflows/p158-external-vantage.yml` is also not
currently available through the authenticated repository API, so the off-host
readiness workflow remains undispatched rather than producing a predictable
missing-secret failure.

The normalization mismatch also has a direct code cause. The rollback payload
is appended to the legacy error text, but `attach_service_failure_recourse()`
has no typed branch for `route_pool_entry_unavailable` or a parsed completed
rollback. It falls through to the default `ServiceFailureRecourse`, whose
unknown axis and effect-uncertain defaults override the stronger compensation
evidence. The repair must derive no-effect or compensated-effect state from a
structured coordinator result, not by reparsing arbitrary diagnostic text.

Revised next action: keep the campaign unfrozen and preserve all current
records. Treat the route acquisition and checkout transition as a
sequence-blocking pre-freeze defect. Repair the planner and checkout readiness
disagreement plus the incorrect effect-uncertain normalization, add exact
regression coverage for jobs `r729795` and `r156356`, validate and install a
new candidate, and retry readiness under new evidence identities. Do not
dispatch the external workflow until one durable `/remote-view/<handoff-id>`
URL has `operatorVisible.state=ready` and the protected environment contains
the exact synthetic identity, marker region, attestation, and dashboard
credentials. The 20-minute calibration and E1/E2 seal remain downstream of
that readiness gate.

Direct journal readback confirms one `guacamole_load` occurrence for each of
jobs `r925418`, `r729795`, and `r156356`. Each record retains the exact job,
request, runtime lane, profile, and session correlation without a raw handoff
URL. The latter two also faithfully retain the current, incorrect
`service_operation_failed` and `effect_uncertain` normalization, so the
postmortem can compare the journal projection with the stronger rollback and
shutdown evidence in the Service trace.

### W6 Bounded Blocking-Repair Traversals

The first blocking repair is source commit `0cfd2a54`. A deterministic
regression reproduced the live Route 1 and Route 2 state shape: provider
observation had refreshed the physical route projection while the pool row and
current-boot acquisition lease still proved the coordinator's exact pending
browser, session, route, display, and pool ownership. The planner now accepts
only that fully matching lease and continues to reject a foreign lease. The
same repair carries the typed route-bound blocker and compensation state across
the legacy compatibility error seam. Installed development generation
`0.28.0-588ac9f49ffe` passed three disposable browser launches and the
provider-required development doctor.

Fresh epoch `Plan158Epoch2` then advanced beyond
`route_pool_entry_unavailable` and failed as job `r411067` at the next checkout
guard with `route_pool_contention`. The warm provider route was `ready` with no
browser or session owner, but the guard rendered the missing owner as
`unknown` and treated the physical provider display identity as a foreign
active checkout. Rollback closed the new browser and restored the acquisition.
The repaired journal correctly recorded `code=checkout_failed`,
`axis=presentation`, `phase=finalize`, and `effectState=no_effect`; it no longer
degraded completed compensation to `service_operation_failed` and
`effect_uncertain`.

The second blocking repair is source commit `1e7089d0`. A new red-green
regression proves that an ownerless warm provider route may be claimed only
when the requested allocation and a current-boot pending acquisition lease
both match the exact browser, session, route, and display. Foreign lease
contention remains fail-closed. Installed development generation
`0.28.0-762a1be44f18` passed three disposable browser launches and the complete
provider-required development doctor.

Fresh epoch `Plan158Epoch3` advanced beyond both prior checkout defects and
failed as job `r570050` with the third distinct sequence blocker,
`presentation_bound_slot_missing`. Its newly launched browser was closed and
the route, pool entry, and acquisition lease were restored. The Service job
and terminal outcome again retain `checkout_failed`, `presentation`,
`finalize`, and `no_effect` plus exact request, subject, connection, profile,
runtime-lane, and session provenance. No durable handoff URL was minted and no
external workflow was dispatched.

The third blocker localizes the remaining identity disagreement. Checkout asks
`PresentationCapacityAuthority::activate_bound_browser()` for route
`development-route-1` plus logical acquisition display
`remote-view-display:development-route-1`, while the warm presentation slot is
bound to the provider inventory display `development-display-1`. The current
Service route pool contains both legacy route-id rows such as
`development-route-1`, whose target carries no provider display-allocation
identity, and canonical slot rows such as `development-slot-1`, whose target is
bound to `development-display-1`. Deterministic B-tree selection chooses the
legacy available row first. The first planner repair correctly recognizes the
exact pending lease; the second guard repair exposes rather than resolves this
upstream duplicate-row selection and migration defect. W10 must therefore
review whether that second repair remains safe and useful after canonical
route-pool migration, rather than assuming it is independently sufficient.

A separate high-severity validation-boundary finding occurred before the first
red regression completed. Under the already observed host process pressure,
`sccache` failed to spawn a compiler and its error diagnostic rendered the
compiler command with inherited environment names and values. Sensitive values
were visible in the interactive command output. No values are copied into this
plan or any campaign artifact. Subsequent Cargo validation used the documented
cache opt-out and bounded two-job execution. W10 must classify this as a secret
exposure through build-failure logging, identify the exact upstream and wrapper
redaction owners, and require operator credential-rotation review. A passing
rebuild does not erase the exposure.

The former two-traversal repair counter was an artificial campaign stop, not a
safety or authority boundary, and is superseded by the operator's direction to
keep repairing until this sequence is unblocked. Job `r570050` remains sealed
as the failed third epoch. Repair `presentation_bound_slot_missing` through a
deterministic regression at the route-pool selection and provider-inventory
seam, validate and install a new isolated candidate, and resume under a fresh
epoch and attempt identity. Repeat that evidence-preserving loop for any later
sequence blocker while a safe deterministic in-scope repair remains. Do not
mark unexecuted cases `skipped_blocked` merely because a repair count was
reached. W10 still begins only after the scheduled deterministic inventory is
terminal or a genuine stop condition above is recorded. The final review must
include every checkout divergence, the successful failure-normalization
repair, the access-plan retained-service epoch friction, and the Cargo/sccache
process-pressure observations as separate causal findings.

### W6 Last30Days X Shared-Profile Admission Finding

Production tick `tick-98e14987fc5e9a7b7b63f8b8ea1abb95` added a distinct
read-only observation after the third checkout repair. Three consecutive X
`tab_new` requests from Service `last30days`, agent `x-scraper`, and task
`x-feed` failed before execution with
`existing_session_profile_identity_unproven`. The same authenticated Profile
and retained browser subsequently supported successful LinkedIn and Reddit
acquisition. This isolates the failure to Agent Browser request admission and
lifecycle routing rather than Profile health, authentication, or scraper
content. The exact source is
`docs/dev/notes/2026-09-03-last30days-x-shared-profile-identity-rejection-handoff.md`,
integrated as commit `4f0ac55f`.

Readback exposed two coupled product defects. First, request normalization
treated any complete client-supplied `browserId` and `sessionName` pair as
self-validating and bypassed the current access plan. Second, automatic shared-
Profile attachment rejected every command that already carried either route
hint, so even an exact access-plan route could not select the retained browser
and fell through to legacy identity proof. The production failures also
retained null provenance, failure, and terminal-outcome fields, confirming the
historical logging defect on a current real client. No production retry,
Profile mutation, browser replacement, or cleanup was performed.

Blocking repair commit `34c7d85a` makes complete `tab_new` route hints pass
through the access plan, rejects browser or session contradictions before
effects, and uses a matching pair to constrain attachment to the exact retained
browser and owning session. Route conflicts now report no effect, require a
fresh access plan, expose an executable `service_access_plan` next action, and
forbid blind retry and duplicate Profile launch. A production-shaped terminal
test preserves the Last30Days service, agent, task, Profile, browser, and
session provenance across the response, durable job, and terminal event.

The repair was developed red-green. Before the implementation, the
normalization test accepted a contradictory complete route and the runtime
test returned no retained target for an exact route. Afterward, both route
conflict axes, exact retained selection, and terminal logging pass. The full
44-test access-plan slice, 17-test failure and journal slice, clippy with
warnings denied, formatting, generated-client contracts, documentation build,
and all six selected workstation and Guacamole installation fixtures pass.

This finding maps to frozen family F01 and cases A01, A02, A03, A08, A13, and
X10; it does not expand the scheduled case count. A01 must exercise at least
two serial self-declared clients against the same retained shared Profile using
fresh access-plan output, assert that both requests reuse the exact physical
browser without identity choreography, and correlate every terminal envelope.
One deliberately stale browser pair and one deliberately stale session pair
must fail before browser effects with the new typed recourse. This provider-
free and E1 obligation is folded into the adversarial battery rather than
treated as evidence that the complete live sequence has passed.

Next action: build, install, and identify a fresh isolated development
candidate containing `34c7d85a`; run the development browser smoke,
provider-required doctor, and exact shared-Profile serial regression under a
new epoch. If those pass, resume the external-handoff readiness sequence and
the remaining W6 calibration gates. Production remains read-only.

### W6 Shared-Local Cold-Route Blocking Repair

Development generation `0.28.0-4025f74d88fc` passed installation, three
disposable browser launches, skill synchronization, and the complete
provider-required doctor. A disposable `shared-local` Profile was then created
only in E1. Its access plan allowed the self-declared subject and recommended
`launch_new_browser`, but both a manually shaped request and the exact copied
access-plan request failed before launch with
`existing_session_profile_identity_unproven`.

The failure is distinct from the retained-route defect. A cold shared-local
plan emitted no logical `sessionName`, so action execution inherited the
runtime host's ambient daemon lane. Historical identity retained on that lane
was then mistaken for the requested Profile's session identity. Strict
registered principals already avoided this collision by receiving a stable
principal-and-Profile-derived cold route; ordinary self-declared clients did
not.

Blocking repair commit `8f4cd76f` gives every allowed cold shared-local
subject a stable `shared-profile-*` daemon route derived from the admitted
subject and Profile ID. The access plan emits that route, request normalization
accepts only its exact deterministic value, and no cryptographic identity or
owner proof is introduced. The provider-free regression failed first because
the plan contained no session route, then passed after the repair and proved
deterministic replay plus absence of internal capability authorization. The
retained-route regression, clippy, generated-client checks, docs build,
formatting, and diff validation also pass.

The failed E1 attempts remain evidence; no retry was made against their old
candidate identity. Next action: build and install a new isolated candidate
containing `8f4cd76f`, then restart the serial shared-Profile scenario from a
fresh disposable Profile and attempt identity. Production remains read-only.

Development generation `0.28.0-4066392e7fa1` then passed installation,
production-unchanged comparison, three browser smokes, skill synchronization,
and the provider-required doctor. Under fresh E6 identity, self-declared client
A launched one browser on its deterministic `shared-profile-*` session. A
differently labeled self-declared client B received
`reuse_existing_browser`, reused that exact physical browser and session, and
created a distinct attributable tab. Both terminal responses recorded
`verified_effect` plus complete subject, connection, Profile, browser, session,
request, and job provenance.

The two required stale-route probes each ran once and failed before effects
with their exact browser-conflict and session-conflict codes. They exposed a
third sequence-blocking API defect: HTTP returned only the legacy error string,
and the generated client discarded even that body in favor of a generic HTTP
status exception. The classifier contained correct recourse, but neither the
immediate HTTP nor MCP client could consume it. Pre-job journal normalization
also collapsed the exact conflict to generic `route_hint_failure`.

Blocking repair commit `e631da26` projects structured recourse on HTTP and MCP
normalization failures, preserves exact route-conflict codes in the append-only
journal, and gives generated-client callers a typed `ServiceRequestHttpError`
carrying status, code, response, and failure. Route-conflict responses now
state no effect, require access-plan refresh, expose the safe exact-route next
step, and forbid blind retry or duplicate Profile launch. Focused HTTP, MCP,
journal, complete Service-client, generated-type, clippy, formatting, and diff
validation pass. The failed E6 stale-route responses remain preserved as
pre-repair evidence.

Next action: build and install a fresh isolated candidate containing
`e631da26`, then repeat only the stale browser and stale session probes under a
new attempt identity to verify immediate recourse and exact durable logging.
Do not repeat the already successful E6 client A and B browser effects.

Development generation `0.28.0-8e5747e7ad76` passed isolated installation,
the production-unchanged comparison, three disposable browser launches, skill
synchronization, and the complete provider-required doctor. A fresh E7
shared-local Profile then received one deliberately stale browser route. The
installed HTTP surface rejected it before dispatch with status 400 and exact
code `service_access_plan_route_browser_conflict`. Its structured failure says
`effectState=no_effect`, `phase=launch_admission`, and
`retryDisposition=refresh_access_plan`; it exposes `service_access_plan` as the
executable next action and forbids blind retry and a duplicate Profile lane.
No rejection job was created. The only job in the bounded time window was the
preceding Profile upsert.

Direct installed-runtime journal readback found the same E7 request as a
single `http_service_request` record at `ingress_validation`, with the exact
route-conflict code, request ID, runtime environment, boot epoch, action, and
runtime-lane reference. This confirms that the new generation no longer
collapses the denial to generic `route_hint_failure`. The earlier E6
browser-conflict and session-conflict records remain preserved with that old
generic code as pre-repair evidence. The installed session-conflict response
was not repeated after the generation switch because no compatible live E7
browser existed; its provider-free response and journal regressions pass, but
that is not claimed as installed live evidence.

The named `test:p158-last30days-regressions` battery now contains thirteen
exact Rust regressions rather than six. The added tests cover retained-route
reuse, deterministic cold shared-local routing without owner proof, exact
route-constrained attachment, route-conflict recourse, exact pre-job journal
codes, HTTP response recourse, and Last30-shaped terminal provenance. The
generated Service-client and handoff documentation checks remain in the same
script. The expanded battery passes in full with cache disabled and bounded
two-job Cargo admission.

Two additional observations are retained without interrupting the campaign.
First, after the E6 browser process became terminal across installation, an
access plan for its shared-local Profile required `profile_capability_required`
instead of treating the terminal owner as replaceable shared history. This may
reintroduce strict proof friction for terminal shared-local replacement and is
deferred to W10 because a fresh E7 Profile allowed the no-effect sequence to
continue. Second, an accidentally malformed harness request omitted its action
while probing an unavailable plan; its generic rejection is classified as a
harness failure, not product evidence.

Revised next action: resume external-handoff readiness and the remaining W6
calibration gates. Register the existing external-vantage workflow through the
narrow reviewed integration path required by GitHub, then dispatch it against
this feature commit only after rechecking the protected synthetic environment.
Production remains read-only. The terminal shared-local replacement friction,
the absence of an installed post-generation session-conflict probe, and the
historical generic E6 journal records remain explicit W10 review inputs.

Default-branch registration PR 12 merged with only
`.github/workflows/p158-external-vantage.yml`; provider-free external-runner and
handoff-oracle tests passed before registration. The protected
`p158-external-vantage` environment exists and exposes all six required secret
names. No secret values were read back or copied into campaign artifacts.

The mandatory pre-dispatch live check then found another blocking defect. The
development dashboard reported runtime multiplicity drift by combining the
production dashboard backend generation with the development runtime-host
generation. Both runtimes were individually coherent and their distinct
systemd units, executable paths, ports, and environment bindings were live.
The warning was manufactured by `runtime_multiplicity_report_from_doctor_inputs`:
it always queried `agent-browser-dashboard-backend.service` and always trusted
the production workstation-selected generation, even when executing with
`AGENT_BROWSER_RUNTIME_ENVIRONMENT=development`.

The bounded repair makes multiplicity projection environment-scoped. The
production path retains the production dashboard unit and workstation-selected
generation. The development path queries
`agent-browser-dev-dashboard-backend.service` and accepts a selected generation
only when the observed development runtime hosts agree on exactly one
generation. It does not kill, transfer, or reinterpret either environment's
processes. Five focused multiplicity tests, formatting, and workspace clippy
with warnings denied pass.

Revised next action: commit the environment-scoped projector, build and install
a new isolated candidate, and require development runtime multiplicity to read
`steady_current` with one dashboard, one runtime host, no legacy daemons, and
one selected generation. Only then recreate or recover the synthetic durable
handoff and dispatch the registered readiness workflow under a fresh evidence
identity.

Installed development generation `0.28.0-105b02603e15` satisfied that exact
multiplicity assertion and three disposable browser launches. The complete
provider doctor also passed with six routes and four unique warm displays when
the already reviewed public URL and ingress revision were supplied. An earlier
doctor invocation set only the provider-required flag and therefore rendered a
configuration mismatch against the correctly persisted ingress manifest. That
failure is an invocation omission, not evidence that installation erased the
binding.

The dashboard remained in overall convergence attention for a separate
development-boundary reason. Production installs have a recurring workstation
maintenance receipt, but isolated development installs intentionally do not
install the production interlock because it can reconcile route users, prune
state, and collect generations. The shared status projector nevertheless
required that absent production receipt even though it had just performed a
fresh development-scoped dashboard and runtime-host census. The repair maps a
current steady development census to
`runtimeMonitor.state=development_live_observation`, explicitly records
`maintenanceEffectsApplied=false`, and remains fail-closed as
`development_runtime_drift` when multiplicity is not steady. Production still
requires its fresh persisted maintenance receipt. The focused regression,
formatting, and workspace clippy pass.

Revised next action: build and install the second dashboard-warning candidate,
prove both the multiplicity and live-observation gates make the current
development runtime lifecycle ready, and only then rebuild the external
handoff. Preserve the absence of development unattended maintenance as an
explicit W10 architecture input rather than representing the live observation
as retention or cleanup work.

Installed development generation `0.28.0-fee4d526b8c8` passes the full
provider-required doctor, three disposable launches, and direct status
readback with `runtimeLifecycle.ready=true`, multiplicity `steady_current`,
and reconciliation `development_live_observation`. The dashboard warning is
therefore resolved on the current isolated runtime without stopping the
coherent production services or conflating their identities.

The read-only resolution probe for durable handoff `r743478` then exposed the
next sequence blocker. Its retained record and ready presentation receipt are
real, but its original owner lane has no legacy per-session HTTP port after the
single-host runtime transition. The dashboard gateway correctly prepared that
lane through the current host, then incorrectly required
`session_port_for_name()` and returned HTTP 503 before the durable resolver
could decide whether to reconnect or reopen. No failure-journal record was
written for that server-side unusable-handoff result.

The bounded repair sends only
`service_remote_view_handoff_resolve` through the authenticated daemon command
relay after owner-lane preparation; ordinary Service requests retain their
current HTTP routing. Preparation, relay, invalid-response, and timeout
failures now return typed gateway codes and append a redacted handoff-link
journal record with stage, action, runtime lane, owning session, and hashed
handoff ID. The raw ID and URL are not journaled. The provider-free regression
and all 51 dashboard gateway tests pass, as do formatting and workspace clippy.
That regression is the fourteenth exact Rust test in the named P158 Last30Days
battery.

Revised next action: install the daemon-relay repair under a new candidate
identity, resolve the same durable handoff without an HTTP lane, and inspect
the structured resolver outcome. Reopen only through the durable handoff's
explicit `allowReopenClosed` path if its no-effect readback requests it. Then
require `operatorVisible.state=ready` before external workflow dispatch.

Development generation `0.28.0-f7389af3c928` passed isolated installation,
the production-unchanged comparison, three disposable launches, and the
provider-required doctor. The same durable handoff then resolved through the
current runtime host without a legacy HTTP lane. Its read-only result correctly
reported that the retained tab was deliberately closed and required one
explicit reopen action.

That single authorized reopen failed before browser launch with
`existing_session_profile_identity_inconsistent`. Compensation restored the
display, provider route, and route-pool entry, and the terminal job reported
`effectState=no_effect`, `retryDisposition=inspect_before_retry`, and a hard
stop against blind retry. The append-only journal retained the exact request,
job, browser, session, runtime lane, hashed handoff identity, outer
`browser_launch_failed` code, and inner identity error in its summary. No
second live attempt was made.

The retained browser, session, Profile, runtime owner, and terminal lifecycle
records all agree. The actual defect is ordering inside remote-view reopen:
route acquisition reserves a matching replacement display before Profile
selection, while the terminal-owner relaunch guard previously required
`displayAllocationId` to be absent. It therefore misclassified the newly
reserved display as evidence that the old browser remained live, fell into the
live-owner consistency path, and rejected the intentionally empty active
session set.

The provider-free regression reproduces that exact state transition. The
bounded repair accepts a prepared display only for `remote_view_open` and only
when a current-boot pending acquisition lease matches the terminal browser,
session, route, display allocation, and Profile. A completed or otherwise
stale lease continues to produce the identity-inconsistent rejection. The
focused red test failed with the historical signature before the repair and
passes afterward. This regression is now the fifteenth exact Rust case in the
named P158 Last30Days battery.

Revised next action: run the related route-host and named Last30Days batteries,
formatting, and workspace clippy. If green, install one new isolated candidate
and perform exactly one fresh explicit reopen attempt through the same durable
handoff. Require a ready operator-visible result and inspect the new journal
window before dispatching the external-vantage workflow.

The first bounded repair passed all listed gates and was installed as
development generation `0.28.0-0fb09e6099a7`; production remained unchanged,
three launch smokes passed, and the provider-required doctor was fully green.
The one E9 reopen attempt nevertheless reproduced the same identity error and
again rolled back with no effect. Inspection found that the initial regression
modeled the outer `remote_view_open` request, while route-bound execution
normalizes the browser effect to `action=launch` before calling the daemon.
The first guard therefore never recognized the real effect command.

The regression now models that normalized launch boundary. It fails with
`existing_session_profile_identity_inconsistent` against the first repair and
passes when the prepared-display exception recognizes either the outer
remote-view action or its normalized launch effect. The exception remains
conditional on the exact current-boot pending acquisition lease and matching
browser, session, route, display, and Profile. The completed-lease negative
control remains rejected.

Two unauthenticated local API probes returned `Login required` while the E9
harness established a dashboard session. They created no browser effect and
are harness-authentication failures, not additional reopen attempts. Whether
authentication denials need a separate security journal surface is retained
for W10; it does not block the authenticated handoff sequence.

Revised next action: repeat the focused and campaign validation gates, install
one further isolated candidate, and make one E10 authenticated reopen attempt.
Do not retry E9. Inspect its distinct terminal job and journal record as
preserved pre-repair evidence.

The corrected normalized-launch repair passes the focused regression, all 89
route-host tests, the complete fifteen-case Last30Days battery, formatting,
and workspace clippy with warnings denied. Development generation
`0.28.0-d8482cdd4991` then passed installation with production unchanged,
three disposable launches, skill synchronization, and the complete
provider-required doctor.

The single authenticated E10 reopen succeeded on the original durable handoff,
browser, session, and Profile identities. Its terminal job is `succeeded` with
`effectState=verified_effect` and `retryDisposition=do_not_retry`; the resolver
reports `status=ready`, `resolved=true`, and `reopenedClosedTab=true`. A
subsequent read-only resolution reports `operatorVisible.state=ready`,
`target.state=ready`, and a ready presentation receipt at generation 5. The
durable handoff URL did not change, and no raw provider URL is promoted as an
operator handoff.

Revised next action: commit this W6 readiness checkpoint and dispatch the
registered external-vantage workflow in readiness mode against that exact
branch commit. Preserve its two independent off-host artifacts and aggregate
receipt for the remaining W6 calibration gate.

### W6 External Readiness Run And Blocking Presentation Diagnosis

The first two-client external readiness dispatch ran against exact commit
`0a905c7cdbd12c366fdf2313772c388784a8c052`. Both independent GitHub-hosted
clients reached the authenticated durable handoff and preserved screenshots,
pixel crops, video, and failure receipts. Neither client produced the prepared
synthetic marker, so the aggregate correctly remained unsuccessful and no
automatic retry or repair occurred.

The original receipt classified both failures as a pixel digest mismatch. The
preserved visual evidence proved that this was only the downstream symptom:

- one client displayed `Stream sign-in expired` inside the remote viewport;
- the other remained at `Connecting to CDP stream`;
- both marker crops contained dashboard chrome instead of remote browser
  pixels; and
- the dashboard simultaneously reported a workstation transaction convergence
  action even though live runtime lifecycle evidence showed one current host,
  one selected generation, no legacy daemon, and a current healthy monitor.

The presentation root cause is deterministic. When the service supplied a
loopback Guacamole frame plus a bare public dashboard origin, the dashboard
used the bare public origin as the iframe source. That recursively loaded the
dashboard or login surface instead of the Guacamole client. The repaired URL
selector rebases only a recognized `/guacamole/` path onto the public origin,
continues to prefer an explicit non-loopback dashboard embed, preserves local
embedding for local dashboards, and rejects arbitrary loopback projection.

The external runner now waits up to twenty seconds for the exact prepared
marker after authoritative server resolution. On timeout it preserves the
final screenshot and emits a typed failure distinguishing expired stream auth,
non-rendering CDP, invalid iframe routing, and a genuine identity-marker
mismatch. Receipts contain only bounded URL-free diagnostics such as iframe
path class and marker digest. The no-repair and zero-retry invariants remain
unchanged.

The convergence warning had a separate cause. Development runtime health used
the production workstation-upgrade store under the isolated pseudo-home, where
no production-style upgrade transaction or selected payload exists. That made
an absent transaction look unfinished despite current live multiplicity proving
the selected development generation. Development convergence now derives its
selected generation from the current single-host multiplicity observation and
treats the production upgrade transaction as not applicable. It still requires
dashboard ingress and operator-journey evidence, so the next truthful action is
`reprove_operator_journey`, not `resume_workstation_transaction`.

Provider-free regression coverage now includes hosted Guacamole path rebasing,
arbitrary loopback rejection, four external stream-failure classifications,
typed failure-receipt preservation, and development convergence without
production upgrade history.

Revised next action: complete focused and selected validation, publish one
isolated development candidate, verify live runtime health no longer reports a
workstation transaction defect, then repeat the external readiness dispatch
once. Do not start the twenty-minute calibration until both clients render the
prepared pixels and the remaining operator-journey state is coherent.

Candidate generation `0.28.0-eb821699e652` is now installed in the isolated
development runtime with production unchanged. Three disposable browser
launches passed. The provider-required doctor passes when invoked with the
reviewed external-ingress binding. One earlier doctor invocation omitted those
two required binding inputs and correctly reported the provider configuration
as absent; this was a harness invocation error with no runtime mutation or
browser effect.

Authenticated live health now reports one current runtime host, one executable
generation, no legacy daemon, a fresh development live observation, and no
workstation-transaction finding. The only remaining convergence finding is
`operator_journey_not_ready`, with the exact next action
`reprove_operator_journey`. That is the evidence the repeated external
readiness run is intended to establish.

The first replacement dispatch was canceled because its manually expanded
expected commit value did not match `git rev-parse HEAD`. The workflow's exact
commit gate prevented browser execution from becoming campaign evidence. The
corrected E12 dispatch then reached both authenticated clients, but both stopped
before presentation capture because the durable handoff reported
`status=closed` with `reopenRequired=true`. Video evidence shows the intended
operator gate, `This browser tab was closed`, rather than a stream failure.
No service failure was appended because the backend correctly returned a
non-error closed state.

This exposed another runner-observability gap: it waited thirty seconds for a
ready response and emitted a generic timeout instead of preserving the
terminal closed-state classification. The runner now projects
`reopenRequired`, stops immediately on a terminal non-ready resolution, and
records `handoff_target_closed_operator_action_required` with bounded status
fields. It does not click the reopen control or bypass the operator boundary.

The operator preparation step then explicitly reopened the exact retained
handoff target through the authenticated service action. The response reports
`status=ready`, `resolved=true`, `reopenedClosedTab=true`, ready operator and
presentation states, and the expected retained browser identity. This is test
preparation under the campaign's existing authority, not an automatic runner
repair.

Revised next action: validate and commit the closed-handoff failure
classification, then dispatch one readiness observation against the prepared
open target. If either independent client still fails, preserve the artifacts
and pause for the newly classified blocker. Do not begin calibration yet.

The E13 replacement observation reached both independent external clients
after the explicit reopen. Both resolved one `/guacamole/` iframe without an
expired-auth or CDP-connecting signature, but neither rendered the prepared
identity marker. The preserved human-paced video exposes the exact provider
message: the requested connection does not exist. The dashboard selected the
expected Plan 158 workspace, and its only convergence warning was the truthful
operator-journey gate. This confirms the hosted iframe-path and false runtime
warning repairs while isolating a new provider-authentication blocker.

Read-only provider and database inspection proves that Guacamole connection 1,
its managed route, and its display are ready. The frame token also names that
exact connection. The dashboard forward-auth response instead projected the
authenticated dashboard username `codex` as `Remote-User`. Guacamole had
auto-created that principal with zero connection permissions, while the stable
development operator principal held all six managed connection grants. The
route therefore appeared nonexistent only because dashboard authentication
was incorrectly coupled to Guacamole authorization.

The bounded repair separates those identities. Forward auth preserves the
real dashboard actor in `X-Agent-Browser-User`; only `/guacamole/` requests use
the stable route-authorized provider principal from
`AGENT_BROWSER_GUACAMOLE_HEADER_USER` as `Remote-User`. Development runtime
publication now derives and installs that value from the same operator user
used by provider provisioning. Invalid principal syntax fails closed. The
provider-free regressions cover the separated headers, non-Guacamole behavior,
invalid-principal rejection, and generated development systemd environment.

The newly delivered Last30Days handoff at commit `6f198a8b` is byte-identical
to the intake note already integrated at `4f0ac55f`; it introduces no new
request evidence or repair requirement. It does, however, sharpen the rollout
finding: the branch contains provider-free fixes for the exact X route-hint
and shared-local admission defects, while the production runtime that served
tick `tick-98e14987fc5e9a7b7b63f8b8ea1abb95` did not. A green development
candidate is not production acceptance.

Revised next action: complete the selected documentation, Rust, development
runtime, and Last30Days shared-profile regression gates; publish one new
isolated candidate; re-run the provider-required doctor; and explicitly
prepare the retained handoff if installation closes it. Dispatch exactly one
new external readiness observation only after local forward-auth readback
shows the route-authorized Guacamole principal while retaining the dashboard
actor. Continue to keep production read-only.

Commit `51c3f59b` passed 13 dashboard-auth tests, 89 route-host tests, 55
access-plan tests, five shared-local tests, workspace clippy with warnings
denied, development runtime and workstation fixtures, remote-view docs, and
the docs build. Development generation `0.28.0-80d2a0ca8448` installed with
production unchanged, passed three browser launches and the provider-required
doctor, and projected the route-authorized provider principal while preserving
the signed-in dashboard actor. The retained handoff was explicitly reopened
after installation and returned ready operator and presentation states.

E14 then proved the Guacamole principal repair externally: the human-paced
artifact visibly rendered the exact synthetic fixture through the hosted RDP
iframe. It nevertheless failed the configured pixel digest because the frozen
fixture's page-relative coordinates were incorrectly applied to the outer
dashboard viewport. Its 960 by 320 crop contained dashboard banners and browser
chrome, not the blue fixture marker. The second external job also displaced
the first on the provider's declared single-viewer connection and was itself
displaced by the human job, producing the expected Guacamole ownership-change
surface. Neither condition is a browser, route, or Guacamole grant failure.

These are campaign-harness blockers. The bounded repair interprets an explicit
`remote-view-iframe` marker coordinate space relative to the sole rendered
iframe while retaining the synthetic-only attestation and exact pixel digest.
The readiness workflow delays the slow client by 45 seconds so two independent
off-host observations do not race on a provider that advertises one active
viewer. This sequencing applies only to readiness; concurrent calibration
still requires an explicit takeover schedule or separate route and must not be
claimed ready from this change.

Revised next action: validate and commit the external harness repair, rotate
only the protected synthetic marker region and expected-identity digest to the
new iframe-relative crop, explicitly reprepare the retained handoff, and run
one E15 readiness observation. Do not start calibration or conceal E14's two
failed receipts.
