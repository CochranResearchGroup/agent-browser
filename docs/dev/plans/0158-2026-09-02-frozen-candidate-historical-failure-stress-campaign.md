# Plan 0158: Frozen-Candidate Historical Failure Stress Campaign

Date: 2026-09-02

State: OPEN

Execution state: `w6_external_readiness_reconstruction_active`

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

The first E15 dispatch never entered the browser harness because its manually
transcribed expected commit suffix did not equal the checked-out branch head.
The workflow's exact-commit gate rejected both jobs before dependency
installation, secret use, or browser execution, and therefore produced no
campaign artifact. This is a dispatch-operator error, not evidence about the
candidate. The replacement must source the full value directly from
`git rev-parse HEAD`; no hand-copied commit string is admissible.

The E15b replacement also stopped before browser execution. Both independent
clients emitted `external_vantage_probe_failed` with `Invalid expected identity
JSON`; the aggregate correctly rejected their failure receipts as zero
observations. The protected expected-identity value produced during the marker
rotation was malformed. This is a configuration-preparation defect, not remote
presentation evidence, and no retry or automated repair occurred inside the
campaign.

The blocking input has been reconstructed directly from the authoritative
retained handoff and its exact retained tab. Before replacement, the eight-field
object was parsed locally, every required field was proven to be a non-empty
string, and the pixel digest was proven to be lowercase SHA-256. Only the
protected expected-identity secret was replaced; the handoff, browser, Profile,
fixture, and other protected inputs were not changed.

Revised next action: verify that the retained handoff remains ready without a
browser effect, commit this E15b diagnostic checkpoint, and dispatch exactly
one E15c readiness observation using `git rev-parse HEAD` as the commit input.
If the clients reach capture but the marker still differs, preserve the direct
iframe-relative crops and diagnose decoded pixel content before changing any
digest again.

E15c passed the commit and expected-identity gates but stopped at the next
configuration boundary: both clients reported `Invalid pixel marker region
JSON` before browser launch. The aggregate again rejected the two failure
receipts as zero observations. Together, E15b and E15c prove that the earlier
secret-rotation procedure malformed both changed JSON inputs; they do not
describe two independent runtime failures.

The pixel region has now been replaced from one fixed compact JSON value and
parsed locally before publication. Its coordinate space is
`remote-view-iframe`, all four coordinates are non-negative integers, its
dimensions are positive, and the region fits the frozen viewport. This changed
only the protected synthetic marker-region input. No handoff or browser effect
was requested.

Revised next action: commit this E15c diagnostic checkpoint and dispatch one
E15d readiness observation from the exact repository head. Treat another
pre-execution configuration rejection as a preparation-system defect requiring
repair, not as permission for repeated blind secret edits.

E15d was the first replacement to reach external dashboard capture. Its slow
client proved the new iframe-relative marker contract end to end: the preserved
400 by 100 crop is the intended solid synthetic blue and its SHA-256 exactly
equals the configured digest. That client then failed on its second page. The
human-paced client also failed before a marker crop. Both failures reported a
zero-iframe condition.

The screenshots preserve materially different dashboard states. The
human-paced page selected a detected non-owned copy of the fixture tab and
reported no embeddable stream. The slow client's first page selected the exact
service-owned browser and rendered Guacamole; its later page reported
`readiness failed` with no iframe. The dashboard also continued to display the
`explicit_profile_conflicts_with_current_owner` action result on the owned
workspace. These remain product and dashboard-accuracy findings; the exact
marker success means the external route, Guacamole principal, iframe-relative
geometry, and digest are no longer speculative blockers.

The runner contained a separate deterministic observability defect. Despite
its documented twenty-second marker convergence window, it counted iframes
once before that window and immediately threw a generic failure when the count
was zero. It therefore could neither tolerate a transient dashboard projection
nor emit the typed no-stream evidence required by this campaign. A provider-free
red test now proves that zero iframes remain eligible to converge while two
iframes fail closed. The repair re-evaluates the iframe and its bounds during
the marker loop, and a terminal zero-iframe state is recorded as
`external_stream_not_embeddable` with bounded diagnostics.

Revised next action: validate and commit the bounded runner repair, then run one
E16 readiness observation. If either page remains without an iframe for the
full convergence budget, pause on the typed dashboard/product defect and repair
that defect rather than altering the already-proven marker or route identity.

E16 passed the repaired convergence behavior and produced two new exact
findings. The human-paced client rendered the expected marker on both initial
and reconnect captures, but its otherwise public dashboard session received 23
provider-internal URL observations in durable-handoff resolve responses. Its
redacted HAR contains 212 requests and every request remained on the public
dashboard origin, proving the defect is response-data disclosure rather than
loopback navigation. The slow client rendered the exact marker on its initial
and concurrent pages, but its reconnect marker showed Chrome's New Tab page.

The slow-client root cause is in the synthetic interaction harness. It clicked
40 pixels inside the remote frame before capture, which is Chrome tab-strip
space rather than the attested synthetic page. The repaired interaction derives
its point from the center of the already-attested synthetic marker region. A
provider-free regression fixes that point at the page marker and prevents a
return to the tab-strip coordinate.

The URL disclosure is in the authenticated dashboard proxy. The durable
handoff resolver legitimately returns infrastructure diagnostics to its local
owner, but the public dashboard forwarded that response unchanged. The bounded
public-boundary repair recursively removes provider route, route-binding,
embed, and health URL fields only from authenticated durable-handoff resolve
responses. It preserves the opaque handoff URL, status, presentation receipt,
tab identity, and open intent required by the dashboard. Its focused red test
contains loopback provider values at both top-level and nested locations and
proves none survive the public response.

The repairs pass the external-runner provider-free suite, all 52 dashboard
stream module tests, workspace clippy with warnings denied, the durable-handoff
dashboard contract check, and the documentation build. The docs, CLI help,
README, shared skill, and inline contract now state the public response rule.

Revised next action: commit the two E16 blocker repairs, publish one isolated
development candidate, run the three-launch smoke and provider-required doctor,
and explicitly restore the retained handoff if installation closes it. Then
dispatch one external readiness observation from the exact installed commit.

E17 ran from installed source commit `051c66d5ad17a41bc6de2fad0ae57098ed7ec103`
as workflow run `33817245594`. Both external clients reached the public
dashboard, resolved the same durable handoff, and rendered the exact expected
synthetic pixel marker on initial and reconnect observations. The slow client
also rendered the exact marker from its concurrent page. All five marker crops
have SHA-256
`7f642adcc83d962dcf542faedfee0a7bd9027bd45aa1bcba2fe6842c1d6ac527`.
This clears the E16 tab-strip interaction failure and proves that the public
ingress, authenticated handoff, Guacamole data plane, reconnect path, retained
browser identity, and synthetic content were usable during the run.

Both clients nevertheless failed the URL oracle. The human client reported
eight flagged observations and the slow client reported eighteen. Their
redacted HARs contain 210 and 492 requests respectively, all on the reviewed
public dashboard origin. Direct `/guacamole/` resource paths are present in
those public-origin requests. This is distinct from E16: the repaired public
resolver response omits provider route fields, while the external data plane
still uses a Guacamole path for its iframe and transport. Plan 0158 requires
raw Guacamole to be rejected when supplied as an operator handoff, but it does
not require a public authenticated iframe or WebSocket transport to disguise
its implementation path. The current oracle applies the raw-provider finding
to every URL role and therefore makes the already accepted public Guacamole
data-plane topology impossible to pass.

E17 also exposed two evidence-quality defects. The failure receipts retain only
the number of URL findings, not their safe role and finding-code breakdown. The
aggregate scans only files named exactly `receipt.json`, so it reported zero
observations even though two complete `failure-receipt.json` artifacts were
downloaded. Finally, the ordinary screenshot is captured before the marker
convergence loop, so it can show the transient no-stream dashboard while the
subsequent marker crop proves that the iframe became usable.

Revised next action: repair the oracle so public `/guacamole/` iframe and WSS
data-plane observations remain subject to protocol, host, DNS, and TLS checks
but are not misclassified as operator handoffs. Preserve rejection for a raw
Guacamole starting URL, redirect, reconnect target, copied action, error
action, or other operator-navigable link. Add safe URL-role and finding-code
details to failure receipts, aggregate both successful and failed receipts,
and capture the ordinary screenshot after marker convergence. Develop each
change red-green, then dispatch one fresh readiness epoch without changing the
handoff or expected identity.

E18 ran from commit `115f91b41eb84b630a67b89189f5bedb548ce082` as
workflow run `33818219471`. Both external clients reached the public dashboard
and rendered the exact expected synthetic marker, but both terminated with an
expected-tab identity mismatch. The repaired aggregate correctly retained both
failure receipts and their client identities, and the post-convergence full
screenshot showed the live fixture rather than a transient no-stream state.

The mismatch was a dispatch-preparation defect, not a retained-browser or
presentation regression. The workflow binds both probe jobs to the
`p158-external-vantage` GitHub environment. Its environment-level
`P158_DEV_EXPECTED_IDENTITY_JSON` secret takes precedence over a repository-level
secret with the same name. Preparation refreshed only the repository-level
secret, so E18 compared the live browser against the stale environment value.
The secret value itself remains unlogged. This failure demonstrates that an
apparently successful secret update is not sufficient evidence unless it
targets the workflow's effective scope.

Revised next action: rebuild the expected identity from current authoritative
development state without printing it, update
`P158_DEV_EXPECTED_IDENTITY_JSON` in the exact `p158-external-vantage`
environment, and dispatch a fresh readiness epoch from the exact pushed
commit. Treat environment scope as a required pre-dispatch invariant for every
future external-vantage run.

E19 ran from commit `7aeb9dc321caaa4ac086397b4f89f2b544ed0990` as
workflow run `33818989216` after the environment-scoped identity was rebuilt
from a current ready resolution. The identity gate cleared. The human client
captured the exact marker before and after reconnect, and the slow client
captured the same marker on both its initial and concurrent pages. Every
captured marker has SHA-256
`7f642adcc83d962dcf542faedfee0a7bd9027bd45aa1bcba2fe6842c1d6ac527`.
The repaired aggregate retained both failure receipts.

The human client then correctly rejected eight loopback, non-TLS WebSocket
observations. A bounded external reproduction showed that the authenticated
public handoff page opened four `ws://localhost:<dynamic-port>/` sockets and
one `ws://localhost:9223/` socket alongside its valid public
`wss://<public-origin>/guacamole/websocket-tunnel`. The usable Guacamole image
therefore coexisted with unusable legacy CDP socket attempts in the external
operator browser. This is a product defect, not an oracle false positive: the
global dashboard stream hook always constructs `ws://localhost:<port>`, even
when the dashboard itself is loaded through external HTTPS ingress.

The slow client's scheduled reconnect exposed an independent contention
failure. The authoritative Service job started at
`2026-09-03T23:48:52.744198242Z` and failed with
`service_state_lock_timeout` after a 1001 ms file-lock wait during route
checkout. Its route and display rollback completed, and no replacement browser
was launched. The external failure receipt retained only the later generic
resolution timeout because the runner discards unsuccessful resolver response
envelopes. This is both a remote-view robustness defect and a post-mortem
logging gap.

The full screenshots also preserve two separate dashboard findings for later
analysis: the selected owned workspace displays
`explicit_profile_conflicts_with_current_owner`, and the top banner says the
dashboard generation lacks a current operator-journey receipt even while the
remote view is ready.

Revised next action: route legacy dashboard CDP sockets through the authenticated
same-origin `/api/stream/<port>` WebSocket proxy whenever the dashboard origin
is not local, give durable handoff resolution a bounded Service State lock wait
within its existing 90-second job budget, and retain typed unsuccessful
resolver response evidence in external failure receipts. Develop all three
repairs red-green. Do not weaken the public URL oracle or retry the campaign
until the provider-free regressions pass.

The first installed `6024da72` validation stopped before campaign dispatch.
The dashboard correctly supplied a 30-second lock budget, but the canonical
HTTP service-request normalizer rejected the top-level
`serviceStateLockTimeoutMs` field before relay. Placing the value inside
`params` passed envelope validation but would not activate the command-scoped
Service State lock override, so that shape was not accepted as a repair. The
daemon and direct CLI already support this bounded field. The corrective slice
adds it to the canonical service-request field authority, JSON schema,
generated client types and validators, trace projection, and focused Rust
regression. This is a pre-execution contract defect with no browser effect.

Revised next action: install the contract-complete candidate, prove a real
authenticated HTTP durable-resolution request accepts and retains the bounded
lock field, explicitly reopen the disposable handoff target if installation
closed it, refresh the environment-scoped identity if the target changed, and
only then dispatch the next external readiness epoch.

Candidate generation `0.28.0-e87200d2d908` installed from commit
`7a446e2228df841f981be5c74395f4ca8edd5791` with production unchanged. A live
authenticated HTTP request accepted the top-level 30-second Service State lock
budget and returned the deliberate closed-target contract without recreating a
tab. The separately authorized reopen then returned a ready presentation with
a matching receipt. Three independent development launch smokes completed
three open, read, close, and residue iterations each. The provider-required
doctor passed against the reviewed public ingress and all six isolated routes.
A bounded public-origin browser probe observed only authenticated same-origin
TLS WebSockets for the two CDP proxy ports and Guacamole tunnel. It observed no
loopback WebSocket.

E20 ran from that exact commit as workflow run `33822225799`. Both external
clients rendered the exact expected synthetic marker. The human client captured
matching initial and reconnect marker SHA-256 values of
`7f642adcc83d962dcf542faedfee0a7bd9027bd45aa1bcba2fe6842c1d6ac527`,
which clears the E19 loopback-stream failure, but its oracle reported
`iframe_failure`. Its full screenshots still show the ready embedded remote
view. A local public-origin DOM reproduction observed exactly one connected
`/guacamole/` iframe after presentation convergence. The runner sampled iframe
URLs once before its pixel-convergence loop, so it could record no iframe and
then prove iframe-relative pixels moments later. This is an evidence-ordering
race, not a missing stream.

The slow client completed its initial marker capture, then timed out waiting
for an authoritative ready resolution on its concurrent page. Service State
records one correlated resolver job beginning at
`2026-09-04T00:34:52.10803933Z` and succeeding at
`2026-09-04T00:34:52.651895547Z`; the earlier one-second lock failure did not
recur. Its failure receipt did not retain the observed non-ready resolution
shape, which prevents exact post-mortem classification. The dashboard currently
normalizes `resolved=true` receipt mismatches into `converging`, but leaves an
incoherent `status=ready, resolved=false` response outside both its ready path
and retry effect. That state can leave the runner waiting even though the
server-side operation succeeded.

The E20 aggregate correctly retained both failure receipts, both client IDs,
zero retries, and zero repairs. Its SHA-256 is
`158a72434878ee30f0ac9ee5043e6c10e2bf92a906ed94a808b599f6d21cb5fa`.

Revised next action: bind iframe and form evidence collection to the accepted
post-convergence pixel observation, normalize every potentially usable but
receipt-incoherent `ready` or `converging` response into the existing bounded
dashboard retry loop, and retain safe resolver observation counts, statuses,
readiness gaps, and oracle finding summaries in failure receipts. Validate the
provider-free runner, oracle, dashboard handoff, inspector, and dashboard build
before installing and dispatching one fresh external readiness epoch.

E21, workflow run `33823413339`, stopped at the exact-commit pre-execution
guard. It was dispatched with expected commit `914fdd05`, then the tracked
branch advanced to documentation-only commit `219769f0` before the runners
checked it out. Both client jobs and the aggregate rejected the mismatch before
dependency installation or browser execution. No test evidence was created,
and the mandatory artifact uploads correctly failed because their source
directories did not exist. This is a dispatch preparation error, not a product
or external-ingress result.

Revised next action: commit this pre-execution record, build and install the
resulting exact head using the wrapper's normal eight-job default, repeat live
readiness and effective environment-secret preparation, and dispatch only when
the branch head, expected commit, and installed source are identical.

E22 ran from exact clean remote head `10352020` as workflow run `33823571606`.
Both clients reached the external handoff and captured the exact synthetic
pixel marker, but each stopped during initial identity validation with
`Authoritative handoff resolution does not match expected tabId`. No reconnect
or concurrent-page assertion ran. The aggregate retained both failure receipts,
both client identities, zero retries, and zero repairs. Its SHA-256 is
`7fe68177d5cda3b50eaa53cb8b653421fe961072ea90b701581e63ece7cb7a90`.

Authoritative runtime events show one target, recorded only here by SHA-256
`520ade2f9071882e73c57bf29b924e6e47691278dd025dcdf5c1af653e7a648a`,
was created during the explicit preparation reopen at
`2026-09-04T00:52:43Z` and was reused by both external resolver jobs at
`00:54:24Z` and `00:55:04Z`. The browser process identity and logical browser
remained unchanged. The public handoff therefore did not replace the physical
browser or target. The failure is a tab-identity representation or
effective-secret comparison defect, but the current receipts retain neither
value in safe form.

Revised next action: add a typed `visible_identity_mismatch` receipt with hashed
expected and observed values, the exact identity field, and boolean tab-to-target
canonical-equivalence flags. Do not expose either opaque identifier. Run the
provider-free runner test, then use one fresh external epoch to distinguish
prefix normalization from a genuinely different identity before changing the
identity acceptance rule.

E23 ran from exact clean remote head `0c3bbc61` as workflow run `33824001712`.
Both external clients captured the exact synthetic pixel marker and then
reported the same typed `visible_identity_mismatch` for `tabId`. Their safe
details were identical: expected SHA-256
`04045e3fa7a5ccfee71f7d3a2fe37aa7e20920750d8fe8b6c502ca12c50583f6`,
observed SHA-256
`7ce87de04bdf21d5e846640ee91099584311928d64c2d9351cf5a31c694d28fa`,
`expectedTabMatchesTarget=false`, and `observedTabMatchesTarget=true`. The
aggregate retained both failure receipts, both client identities, zero
retries, and zero repairs. Its SHA-256 is
`4ed6f58169004b8847afa6f7bfff2fa34f78aa7156ebe210d1ba15a321b573f0`.

This proves that the externally observed identity was internally coherent and
that the freshly prepared ready response was not. The durable resolver built
that response from two generations: it copied `tabId` from the pre-reopen
handoff snapshot while taking `targetId` from the replacement presentation
receipt. The browser and external clients were not at fault.

Revised next action: build the response identity atomically from the fresh open
result, with the newly persisted handoff and presentation target as coherent
fallbacks. Add a provider-free regression for a closed target replaced during
explicit reopen, validate the focused resolver path, then install and dispatch
one fresh external readiness epoch without weakening identity validation.

The resolver repair landed at `391c8315`. Its focused 25-test route-bound-open
module and full workspace clippy gate passed with the normal eight Cargo jobs.
The development candidate installed as generation `0.28.0-95bbbbc9063e`; the
provider-required doctor passed. Installation exposed a retained handoff with
no exact PID, reported as `runtime_handoff_orphan_pid_missing`. The explicitly
authorized reopen path recovered it in one attempt. Preparation then proved
that the top-level tab and target were canonically coherent and exactly matched
the fresh service tab handle before the external identity secret was updated.

E24 ran from exact clean remote head `391c8315` as workflow run `33825298209`.
Both external clients captured the exact initial pixel marker and the exact
reconnect pixel marker without any identity mismatch. This accepts the resolver
repair against external ingress and proves that E22 and E23 were caused by the
mixed-generation preparation response.

Both clients then failed only in the final oracle with
`external_handoff_oracle_rejected: raw_guacamole_url_leak`. The human client
recorded two iframe observations and the slow client recorded three. The
aggregate retained both failure receipts, zero retries, and zero repairs. Its
SHA-256 is
`fee05d05ad4ac5fb0a3afe7301ea3609d07c1784afd96f1f35fb5b98e258785d`.

Diagnosis found a false positive in the oracle's second URL-classification
pass. The captured public Guacamole path was correctly classified as an
allowed `iframe_src` observation, but the matching successful iframe ingress
check was reclassified as a generic `location_header`, where raw Guacamole
paths are intentionally forbidden. A provider-free regression now reproduces
that exact mismatch. The repair maps every ingress-check kind to its actual URL
role and includes the affected safe URL roles in future oracle failure details.

Revised next action: validate and commit the oracle repair, then dispatch one
fresh external readiness epoch from that exact head. Do not rebuild or reinstall
the unchanged runtime binary for this runner-only correction.

The first E25 dispatch, workflow run `33825793040`, was canceled before browser
execution because the manually supplied expected full commit contained a
correct short prefix but an incorrect suffix. The replacement dispatch read
the full SHA directly from Git and used exact remote-head equality before
submission.

E25 ran from exact clean remote head `9a788a6a` as workflow run `33825809638`.
The slow external client passed the complete readiness sequence, including
initial, concurrent, and reconnect pixel observations, external URL policy,
the corrected oracle, and zero physical browser relaunches. The human-paced
client failed its initial pixel observation with
`external_stream_identity_marker_missing`. Its screenshot contained exactly
one Guacamole iframe with a blank stream surface; the sign-in-expired and CDP
connecting markers were both absent. The aggregate SHA-256 is
`0c7ff01a5ee743901134fe515dd2b6c7a8145685d60c8609c58b67f450a91715`.

Provider logs show no Guacamole authentication or tunnel connection for the
failed human observation before its `2026-09-04T01:29:09Z` failure. The three
subsequent Guacamole connections align with the slow client's three successful
visits. This excludes the synthetic page, retained identity, and browser
process as common causes, but the failed runner discarded its network capture
before writing the HAR, so the exact missing HTTP or WebSocket transition
cannot be recovered from E25.

The failure path now always writes the sanitized network HAR before closing the
browser and retains safe counts for all network entries, Guacamole entries and
HTTP statuses, WebSocket observations, console entries, and resolver
observations. Provider-free tests cover the new failure-detail allowlist.

Revised next action: commit the failure-instrumentation repair and repeat one
fresh readiness epoch. If the blank Guacamole load recurs, use the retained HAR
and counts to diagnose the exact ingress transition. If it does not recur,
retain E25 as an intermittent first-load failure and continue the broader
campaign without pretending it did not occur.

E26 ran from exact clean remote head `5ca8dee5` as workflow run `33826284459`.
The human-paced client passed its full initial and reconnect sequence. The slow
client passed its initial view, then its concurrent page remained on the
dashboard login screen in the busy `Checking` state until the resolver
observation timeout. Its failure bundle retained 182 completed network entries,
37 successful Guacamole responses, eight WebSocket observations, and 40 console
observations. The aggregate SHA-256 is
`126611302f2498f3612132ff650a46d3dbd22493ee9928c8bcb6076c6bd81168`.

Service State recorded the human initial and reconnect resolver jobs and the
slow initial resolver job, but no concurrent resolver job. The failed page's
video independently shows that it never left the authentication gate. The
completed-response HAR contains an earlier 503 and recovery for the slow
initial resolver, but no completed authentication-status response for the
concurrent page. This localizes the campaign blocker before handoff resolution:
the dashboard authentication gate issued one fetch with no timeout or retry,
so a transport-starved request could suspend the page indefinitely. The
underlying source of request starvation remains unproven.

The repair gives authentication status three bounded five-second attempts and
then releases the UI to its existing signed-out recourse instead of retaining
an infinite busy state. A provider-free behavioral check proves recovery after
an aborted first request and bounded failure after exhaustion. Failure capture
now records request starts, request failures, safe endpoint classes, and safe
service action names. It writes a separate redacted transport diagnostic with
requests that were still pending when the probe failed, so the next epoch can
distinguish authentication-status starvation from resolver starvation without
retaining request bodies, credentials, or opaque URLs.

Revised next action: validate the dashboard build and affected dashboard and
external-runner contract suites, install the exact candidate, and dispatch one
fresh external readiness epoch. If the underlying transport starvation recurs,
the bounded gate must either recover or produce a finite signed-out state, and
the new pending-request record must identify the stalled endpoint class.

E27 ran from exact clean remote head `4c25a6eb` as workflow run `33827335722`.
Both external clients passed initial, concurrent, and reconnect observations.
The aggregate recorded two distinct runner identities, zero retries, zero
repairs, zero internal URL disclosures, zero physical browser relaunches, and
the exact shared pixel SHA-256
`7f642adcc83d962dcf542faedfee0a7bd9027bd45aa1bcba2fe6842c1d6ac527`.
The aggregate SHA-256 is
`4d165fa2481a2e52dade33e098affa924ead5ba998316674b522fe9e51188a6f`.
This closes the E26 infinite authentication-gate blocker without erasing E25
or E26 from the defect record.

The first concurrent C01 calibration attempt, workflow run `33827681309`,
failed before the shared measurement window. The slow client received
`service_state_lock_timeout` after 1,001 milliseconds while resolving the
durable handoff. Service State proved that the job retained the requested
`serviceStateLockTimeoutMs` value of 30,000 and `jobTimeoutMs` value of 90,000.
The failure bundle contained 19 completed network entries with no failed or
pending browser request, no pending authentication-status request, no pending
service request, and no Guacamole or WebSocket attempt. This localizes the
failure inside the route-bound repository operation rather than external
ingress or dashboard transport.

Source diagnosis found that the route-bound supervisor replaced every caller
repository-lock budget with a private one-second cap. The run was canceled
once one of its two required clients had failed, because that individual epoch
could no longer produce a valid two-client calibration. Canceling the epoch did
not stop Plan 0158. The plan remains active through blocker repair and a fresh
calibration attempt.

The repair makes the route-bound command supervisor preserve the caller's
bounded repository-lock budget for both direct opens and durable handoff
resolution. The budget remains capped at five minutes and is further shortened
by the forward or compensation deadline. Provider-free regression tests cover
the exact 90-second job and 30-second repository-lock command values plus
deadline contraction. The complete route-bound open module passes serially.

Revised next action: publish and install the repaired development candidate,
revalidate the exact provider and handoff identity, then run a fresh concurrent
C01 calibration. Preserve the canceled calibration and its diagnostics as a
historical failure receipt.

The replacement C01 attempt ran from exact remote head `6d314705` as workflow
run `33830733709`. Both clients reached the shared window, proving that the
one-second repository-lock truncation no longer blocked handoff resolution.
The slow client passed initial, concurrent, and first reconnect pixel checks
with the exact expected hash, then failed its second reconnect at
`2026-09-04T02:59:24Z`. The companion job was canceled immediately because the
required two-client calibration could no longer succeed. Plan 0158 remained
active.

The failure receipt classified `external_stream_identity_marker_missing` and
retained 5,705 completed network entries, zero failed network requests, 41
pending requests, 132 successful Guacamole responses, 101 WebSocket
observations, and 787 console observations. The failed frame rendered the
dashboard's explicit message that viewer reconnect had targeted service session
`dashboard-service-backend`, which had no HTTP route. The failure journal also
recorded the Guacamole disconnect sequence and failed viewer-lease action.

Source diagnosis found that viewer and controller lease requests omitted the
selected browser's daemon session. Even when a caller supplied that session in
`params`, the HTTP gateway reserved parameter-based relay for focus and
takeover actions, so viewer-lease operations fell back to the dashboard backend
session. The repair adds the exact browser session to dashboard viewer lease
request, controller takeover, and release requests, and routes viewer lease
request, heartbeat, release, and controller takeover actions through the same
explicit-session relay rule. Provider-free tests cover all four gateway actions
and the dashboard request shape.

Revised next action: complete Rust and dashboard validation, publish and install
the repaired development candidate, revalidate the durable handoff identity,
and repeat C01. Preserve this failed attempt and its captured frame, transport
diagnostics, journal records, and failure receipt for final analysis.

E28 ran from exact remote head `7ebda2ff` as workflow run `33832442510` after
the viewer-lease routing repair was installed. Both external clients passed the
complete readiness sequence. The aggregate recorded two distinct off-host
clients, the same durable handoff and retained identity, all ingress checks,
zero retries, zero repairs, zero internal URL leaks, zero physical browser
relaunches, and aggregate SHA-256
`b023895b20e40dddae2939692752599859754cafe6c78bbb4d8eed842a62937b`.

The next C01 attempt, workflow run `33832786999`, failed before the shared
window when Guacamole returned HTTP 429 to the human client. The captured frame
explicitly reported that the authenticated Guacamole user had exhausted its
simultaneous connection limit. The viewer-lease request had succeeded, proving
that E28's relay repair remained effective. The failure receipt retained 196
completed network entries, zero failed browser requests, two pending dashboard
reads, 74 Guacamole entries including the single 429, eight WebSocket
observations, and 27 console observations. The companion client was canceled
immediately and Plan 0158 remained active.

The managed Guacamole route definition allowed only four total connections and
two connections per authenticated route user. That bound cannot reliably hold
the campaign's legitimate overlap of two external clients, the slow client's
second concurrent page, and brief old/new viewer overlap during reconnect.
The repair preserves a hard provider limit while raising each managed route to
eight total connections and eight per authenticated route user. The generated
SQL regression test rejects the former limits and asserts both new bounds.

Revised next action: apply the reviewed provider configuration through the
development provider plan, stage, preflight, and apply workflow; verify the
live database values and provider doctor; then repeat C01 from an exact clean
head. Preserve the 429 receipt and frame for final analysis.

The first capacity apply emitted `provider_ready_ingress_pending`, but exact
development-database readback still showed the former four-total and
two-per-user limits on all six routes. The reconciler observed only connection
identity and RDP username. Because it did not observe connection limits, it
classified the drifted database as ready, skipped `syncConnections`, and wrote
a misleading successful receipt. A production-database query was also
discarded as evidence because it addressed the wrong isolated container.

The corrected provider model now carries the desired connection limits,
includes them in the staged desired-provider descriptor, reads both live
Guacamole columns, and requires exact equality in readiness. The SQL renderer
accepts bounded values from that descriptor rather than relying on an
unobservable text-only default. Provider-free tests prove that a four/two live
observation fails doctor and forces reconciliation through `syncConnections`,
while the exact eight/eight state passes without quarantine.

Revised next action: commit the observable-capacity repair, restage and apply
it, and require exact eight/eight readback from the isolated development
PostgreSQL container before repeating any external browser epoch.

The first corrected reconciliation detected the capacity drift, but the
provider's full reconcile also tried to reopen four already-owned warm viewer
profiles and hit `explicit_profile_conflicts_with_current_owner`. The provider
quarantined itself rather than performing broad cleanup. The four exact
development warm-viewer sessions were closed individually, after which the
reviewed provider apply succeeded with ingress deferred. Exact readback from
the isolated development database then showed eight total and eight per-user
connections for all six managed routes. Production remained unchanged.

E29 ran from exact clean remote head `6aeb2dd0` as workflow run `33834110809`.
Both external clients passed the complete readiness sequence, including the
slow client's concurrent page. The aggregate passed with the same durable
handoff, retained service browser identity, external ingress, exact synthetic
pixel marker, zero retries, zero repairs, zero internal URL disclosures, and
zero physical browser relaunches. This accepts the capacity reconciliation as
sufficient for readiness while preserving the earlier 429 as a campaign
finding.

The next C01 attempt, workflow run `33834420571`, reached the shared
measurement window. Initial and first reconnect pixel checks passed. The human
client failed the second reconnect with
`external_stream_identity_marker_missing`, and the companion client was
canceled because the epoch could no longer produce a valid paired result. The
failure bundle retained 3,646 completed requests, 13 failed requests, 42
pending requests, 159 Guacamole entries with 144 HTTP 200 responses and no
429, 57 WebSocket observations, and 476 console observations.

The failed screenshot proves that the Guacamole stream, retained fixture, and
dashboard were all visible. The sampled 400 by 100 marker crop instead
contained the fixture's prompt-like-dialog control. Repeated simulated human
actions had scrolled the remote synthetic document away from its attested
origin: each action issued balanced wheel input, but asynchronous remote input
delivery did not guarantee an exactly reversible scroll position. This is a
false-negative harness defect, not a browser or ingress outage.

The repair makes every simulated human action finish with a remote
`Control+Home`, restoring the synthetic document to the attested origin before
the next identity check. A provider-free regression test drives the input
sequence through a fake page and requires that deterministic reset to be the
last remote input.

Revised next action: validate, commit, and push the harness repair, then repeat
C01 from the clean exact head. Continue to preserve the failed epoch and its
diagnostics for the final defect-surface analysis.

The immediate repeat from exact head `ef622060`, workflow run `33835467006`,
failed both clients before the shared window. Both rendered the retained
synthetic browser and Guacamole route without a 429, but their initial marker
crops showed the same already-scrolled fixture state inherited from the prior
epoch. The post-action reset could not run because initial identity validation
correctly preceded the first scheduled action.

The repair now focuses the remote-view iframe and sends the deterministic
`Control+Home` reset at the start of every visit before pixel validation, as
well as after every simulated human action. This lets a fresh epoch normalize
retained synthetic fixture state without restarting or replacing the retained
browser. The provider-free fake-page regression requires iframe focus to
immediately precede the reset key.

Revised next action: commit and push the pre-visit normalization, then repeat
C01. A successful repeat must prove that the same retained browser can recover
the inherited scroll state and pass all scheduled reconnect checks.

The repeat from exact head `bfaf1245`, workflow run `33835828862`, again
failed both clients during initial marker validation. The Guacamole iframe was
healthy and returned only successful HTTP responses, but the fixture remained
scrolled. This proves that focusing the outer iframe element does not itself
route Playwright keyboard input into Guacamole's remote input handler.

The reset now focuses the iframe and clicks a fixed blank point inside the
synthetic fixture's remote browser content before sending `Control+Home`. The
click is intentionally confined to the attested synthetic fixture and gives
Guacamole an actual pointer event with which to acquire remote keyboard focus.
The regression test requires iframe focus, remote click, and reset key in that
exact final sequence.

Revised next action: publish the click-to-focus repair and run one readiness
epoch first. If readiness proves inherited-state recovery, immediately proceed
to a fresh C01 calibration; otherwise retain its receipt and continue diagnosis
without restarting the retained browser.

E30 ran from exact head `34e57927` as workflow run `33836102804`. The human
client still rendered the inherited scrolled fixture and failed initial marker
validation; the slow companion was canceled. Guacamole recorded only
successful HTTP responses, so the pointer event acquired the remote surface
without a route or capacity failure, but `Control+Home` did not move the remote
document. The keyboard shortcut is therefore not a reliable reset primitive
through this Guacamole and RDP translation path.

The reset retains the shortcut as a secondary route and now follows it with
three bounded, large upward wheel events over the focused remote synthetic
surface. Earlier campaign evidence proves wheel delivery through this route,
including the scroll drift that exposed this harness defect. The regression
test requires the three bounded wheel events in the final reset sequence.

Revised next action: publish the wheel-normalization repair and repeat the
bounded readiness epoch against the unchanged retained browser. Do not dispatch
another full calibration until initial and reconnect marker continuity pass.

E31 ran from exact head `273fd3b4` as workflow run `33836416629`. The human
client still failed initial marker validation with a healthy observer stream;
the slow companion was canceled. Neither the shortcut nor large wheel events
changed the remote document. Dashboard source and the rendered notice agree on
the missing authority transition: durable handoff resolution reconnects an
observer lease, while remote input requires the separate explicit controller
takeover action.

The harness now assigns distinct roles. The human client uses the dashboard's
public Advanced, Take control action and requires a successful
`service_controller_lease_takeover` response before sending remote input or
normalizing the fixture. The slow concurrent client remains an observer and
performs only non-mutating dashboard pacing. Provider-free tests cover the
exact takeover request discriminator, public control sequence, and absence of
remote click or wheel input from the observer pacing sequence.

Revised next action: publish the role-correct controller acquisition repair and
repeat bounded readiness. Require both observer continuity and human controller
input before resuming C01.

E32 ran from exact head `b56f9519` as workflow run `33836791368`. The human
client's dashboard projection temporarily lost its embeddable service stream
before the new controller request could be emitted. The simultaneously-started
response waiter timed out after 15 seconds. Because the UI click promise and
response promise were not awaited as one operation, the abandoned response
waiter rejected as an unhandled promise and bypassed the structured failure
receipt. Only the video survived. This is a campaign logging defect.

Live readback then exposed the lifecycle boundary underneath the projection
failure. The old E4 browser record still projected PID 50340 as healthy and
route-attached while the runtime owner registry classified the same logical
browser as terminal with process absence proven and cleanup satisfied. Direct
session access returned `existing_session_profile_identity_unproven`. The
process subsequently exited, and bounded service reconciliation repaired three
orphaned display allocations without broad cleanup.

A fresh `Plan158Epoch5` access plan selected a one-time managed profile with
shared-local access, self-declared client identity, and no acquisition blocker.
The current runtime launched a new remote-headed browser and established a new
durable handoff on an exact ready route. A current authenticated resolution
proved ready presentation generation, exact tab identity, and the same
synthetic fixture. The environment-scoped handoff and expected identity secrets
were refreshed without printing their values.

The harness repair now waits for exactly one public Guacamole iframe before
controller acquisition and again after the accepted takeover before remote
input. Controller UI actions and their response waiter execute under one
`Promise.all`, so either failure remains handled by the outer structured
failure-receipt path. Provider-free tests cover iframe convergence and the
takeover request discriminator.

Revised next action: publish the logging and convergence repair, then run a
bounded readiness epoch against the new coherent E5 runtime. If it passes,
resume C01 immediately. Preserve E32 as evidence that contradictory owner and
browser projections can outlive the daemon route and that abandoned waiters
can defeat post-mortem logging.

E33 ran from exact head `103fbefc` as workflow run `33837558315`. The new E5
runtime repeatedly converged from temporary no-stream and reconnect notices to
one ready Guacamole iframe rendering the fixture at its attested origin. The
human client then timed out waiting for the controller-takeover response. Its
complete structured failure receipt retained 185 network entries, zero request
failures, two pending reads, 50 Guacamole entries, seven WebSocket
observations, and 11 console observations. No controller service request was
present, proving that the public Advanced, Take control dashboard item silently
completed without invoking its service action. The repaired promise handling
therefore closes E32's logging gap.

Explicit controller takeover is not part of the frozen C01 precondition. The
durable handoff is already opened in dashboard Control mode with
`manual_attached_desktop` input, and prior campaign evidence proves remote
input delivery. The speculative takeover prerequisite is removed from ordinary
capture while its tested helper and E33 receipt remain available for the later
dashboard-action defect repair. The human client still waits for a real
Guacamole iframe before input; the slow client remains non-mutating.

Revised next action: validate and publish the corrected role behavior, then run
readiness against the coherent E5 browser. Track the silent Take control action
as a dashboard defect in final analysis rather than allowing it to block the
external stream calibration it was not required to authorize.

E34 ran from exact head `d1d6de0c` as workflow run `33837874198`. Both clients
proved the initial external handoff path before failing during a later visit:
the human client retained one completed resolver observation and the slow
client retained two. Each timed out waiting for the next visit's resolver
observation while the external dashboard continued to carry authenticated
Guacamole traffic. The human receipt retained 253 completed requests, six
failed requests, five pending reads, 53 Guacamole entries, and seven WebSocket
observations. The slow receipt retained 436 completed requests, 16 failed
requests, 12 pending requests, 103 Guacamole entries, and 14 WebSocket
observations. Both structured failure receipts and the aggregate failure
receipt were sealed without retry or repair inside the epoch.

The runner had a shorter observation contract than the product it was testing.
It allowed only 30 seconds for the dashboard to emit and project a durable
handoff resolution even though that dashboard request explicitly permits a
90-second service job. Under the loaded external path, a new page could finish
navigation and stream setup without producing the next resolver projection
inside the runner's shorter window. The failure receipt then obscured the
distinction by replacing the visit-local observation count with the total
count accumulated across earlier successful visits.

The repaired runner gives handoff resolution 95 seconds, covering the public
90-second service job plus bounded response-projection grace. Failure evidence
now preserves `resolutionObservationCount` for the exact visit and records the
separate `totalResolutionObservationCount` accumulated by the client. The
provider-free runner check was observed red for the missing timeout contract
and for the dropped total-count field, then green after each focused repair.

Revised next action: validate and publish the E34 harness and logging repair,
then run one new readiness epoch against the unchanged coherent E5 browser. If
both external clients pass, dispatch C01 immediately with its declared shared
barrier and preserve E34 as a distinct failed epoch.

E35 ran from exact head `47a1c0ab` as workflow run `33858705179`. It proved the
E34 timeout and count repairs were active: each client received one current
authoritative resolver observation and each failure receipt separately
reported one total observation. Both clients then stopped without retry at
`Pixel marker region does not fit the rendered remote-view iframe`. The human
receipt retained 146 completed requests, three failed requests, five pending
reads, 53 Guacamole entries, and six WebSocket observations. The slow receipt
retained 94 completed requests, zero failed requests, 11 pending requests, 20
Guacamole entries, and four WebSocket observations. The aggregate sealed both
failures.

The retained videos disprove a changed marker or route. They show the intended
browser and synthetic fixture through the public Guacamole iframe, but the
global `Runtime healthy` notice expands into the complete set of migrated
legacy-profile access messages. It consumes most of the viewport, leaves the
otherwise usable remote workspace only a short clipped iframe, and labels an
access-attention state as healthy because the summary predicate ignored the
access axis. This is a dashboard layout and truth defect exposed by the marker
oracle.

The dashboard repair moves access summarization behind one bounded pure
module. Runtime readiness now requires the access axis to be `allowed`; an
attention, denied, or unknown axis cannot render `Runtime healthy`. The global
notice reports only access state, finding count, blocking count, and a pointer
to Service diagnostics. It never concatenates individual finding messages.
The full structured findings remain available on the diagnostic surface. The
marker loop also treats a temporarily undersized iframe like a temporarily
missing iframe and continues its bounded convergence window instead of
throwing an untyped immediate failure; final pixel and hash requirements are
unchanged.

Provider-free tests were observed red for both the unbounded health summary
module and the undersized-iframe path, then green after their focused repairs.
The complete dashboard production build also passed.

Revised next action: publish and install this dashboard repair only in the
isolated development runtime, re-establish a coherent ready E5 successor if
the install closes the disposable browser, and dispatch one new external
readiness epoch from the exact installed commit. Production remains read-only.

The isolated development services were stopped through their exact managed
units so the candidate build could preserve the host memory reserve. The same
queued build then admitted eight Cargo jobs, completed in the optimized CI
profile, and installed generation `0.28.0-c3d42279662b`. Three disposable
browser launch, URL read, close, and residue iterations passed. The install
receipt proved production identity unchanged. Development provider doctor
then identified only configuration drift. A reviewed reconcile restored the
six-route provider at connection limits 8 and 8, while local and public HTTPS
ingress checks remained successful.

During the capacity readback, an operator query selected an RDP route password
that should never have entered diagnostic output. The affected disposable
development route credential is treated as exposed. A provider-free red test
first proved that the route-user helper lacked safe targeted rotation. The
helper now accepts one or more exact route IDs for rotation, rejects unknown
IDs, preserves every other route password, and supports a quiet mode that
emits no resolved inventory. Its focused test passes. Route 1 was rotated by
exact ID and the provider was reconciled successfully; no production
credential or route was involved, and production remained unchanged.

Revised next action: commit and install the targeted rotation helper, run the
development doctor, then create a fresh managed browser and durable handoff
for a new external readiness epoch. If both external clients pass, dispatch
C01 from that exact installed commit with its declared shared barrier.

Exact source commit `de4eef79` produced installed development generation
`0.28.0-6227d67369c4`. The three-launch smoke passed, the reviewed provider
reconcile returned ready with no failed doctor checks, and a fresh
`Plan158Epoch6` access plan selected managed one-time profile
`managed-ephemeral-7f458f8238ec` under self-declared shared-local access. The
new browser opened the synthetic fixture and its durable handoff resolved
ready at presentation generation 1 without reopening the tab. Effective
environment-scoped workflow secrets were refreshed without printing their
values.

E36 ran from exact head `de4eef79` as workflow run `33860851759`. Both clients
failed deterministically at the final receipt safety check with `Receipt
retained secret P158_DEV_HANDOFF_URL`. Both artifact uploads found no files and
the aggregate could record only missing client evidence. No retry occurred.
The apparent receipt leak was misleading: the GitHub CLI invocation used
`--body -`, which stores the literal value `-` rather than reading standard
input. Both refreshed environment secrets therefore contained one dash. The
receipt assertion matched every ordinary hyphen as the supposed secret and
then prevented the failure receipt from being written. This is a dispatch-
preparation and defensive-redaction defect, not a browser launch, handoff
resolution, or ingress result.

The repair adds a final recursive receipt-boundary sanitizer before digesting
or writing successful, failed, and W8 action receipts. It replaces exact
handoff, dashboard username, and dashboard password occurrences even when a
new nested diagnostic field bypasses its field-specific sanitizer. The
existing assertion remains after that boundary, so persistence still fails
closed if the defense does not remove a secret. A provider-free red test first
proved the boundary was absent, then passed with nested direct and embedded
secret occurrences removed while unrelated evidence remained unchanged.

Revised next action: publish the receipt-boundary repair, preserve E36 as the
logging failure epoch, and dispatch a fresh two-client readiness observation
against the unchanged ready E6 browser. A passing readiness result may proceed
directly to C01 from the new exact source head.

E37 ran from exact head `d30eebd6` as workflow run `33861451832` while the
effective secrets still contained the literal dash. The slow client again
escaped without an artifact after the assertion matched a hyphen. The human
client reached the new boundary sanitizer, which replaced every hyphen in its
receipt, including those in its client ID and timestamps, before persisting an
`Invalid URL` failure. Its artifact therefore proved persistence but was not
semantically usable. The aggregate observed only that malformed client
receipt. E37 is sealed as a failed preparation epoch. Source edits began after
the slow client had failed but before the human job finished; the runners
remained pinned to the exact published commit, but this violated the local
no-edit discipline and independently disqualifies E37 as a frozen result.

The environment secrets were then refreshed correctly by piping each value to
`gh secret set` without a body argument. No secret value was printed. The
redaction boundary now ignores values shorter than eight characters, so an
invalid or placeholder value cannot corrupt ordinary punctuation throughout a
receipt. A provider-free regression first reproduced the dash corruption and
then passed with the client ID unchanged. Long handoff and credential values
remain scrubbed and asserted absent.

Revised next action: publish the short-value hardening, then dispatch E38 from
the corrected environment against the unchanged E6 browser. Make no local
source edit until both client and aggregate jobs finish and their artifacts
are sealed.

E38 ran from exact head `732cad35` as workflow run `33862331219`. The slow
client completed the entire readiness sequence with one reconnect, two exact
service-browser observations, no internal URL leak, no retry, and no physical
browser relaunch. The human client loaded and operated the same prepared
browser successfully on its initial visit, then its reconnect failed before
application execution. Four hashed dashboard JavaScript or font requests
returned HTTP 504 and two related requests ended without a response. The
failure receipt retained 181 network observations, six failed requests, two
pending requests, 72 Guacamole observations, seven WebSocket observations,
32 console observations, both videos, the HAR, and transport diagnostics. The
aggregate sealed the passing slow receipt and failed human receipt without a
retry or repair inside the epoch.

The external reverse proxy evidence localized the failed asset requests to
the development stable dashboard ingress. Five parallel static asset reads
waited approximately 27.6 seconds for `127.0.0.1:4948` before the proxy
returned 504, while other assets immediately before and a complete slow-client
load immediately afterward returned 200. Neither development dashboard unit
restarted. Code tracing then exposed an ingress scheduling defect: every
connection synchronously acquired the same exclusive filesystem lock to read
the atomically published backend registry, and that bounded polling loop ran
inside an asynchronous worker. A request burst could therefore occupy the
workers needed to accept, proxy, and time out the same burst.

A provider-free current-thread regression held the registry writer lock while
the ingress request read path ran. It reproduced the defect by blocking for
the complete two-second lock timeout. The repair keeps exclusive locking on
compare-and-swap writers, reads the last atomically committed registry without
the writer lock, and moves request-path filesystem parsing to Tokio's blocking
pool. The same regression then passed in approximately 20 milliseconds.

The first attempted red-test build also exposed a separate post-mortem logging
hazard. When sccache could not spawn a compiler under host process pressure,
its debug error rendered the compiler's complete inherited environment,
including unrelated credential-shaped variables. The Cargo wrapper now uses a
dedicated sccache boundary that removes credential-shaped variables only from
the compiler-cache process while leaving the Cargo and test environments
unchanged. Provider-free wrapper tests prove API keys and refresh credentials
do not reach the cache process and ordinary Cargo settings do.

Revised next action: validate and publish both blocking repairs, install the
exact candidate in the isolated development runtime, re-establish a coherent
managed browser and durable handoff if installation closes E6, and dispatch a
new two-client readiness epoch. If both clients pass, dispatch C01 immediately
from the exact installed commit with the declared shared barrier.

The ingress and cache-boundary repairs passed their focused regressions,
dashboard-ingress tests, Cargo wrapper tests, format, workspace clippy, and
patch-hygiene checks. Commit `11631ec9` was pushed and installed as development
generation `0.28.0-f7018bd8815a`; the three-launch development smoke passed and
production identity remained unchanged. The development presentation provider
was then staged, preflighted, applied, and republished through Cooper. Local and
public dashboard probes returned 200, provider doctor passed with six routes,
and the prior E6 browser was confirmed closed by installation.

Fresh E7 preparation exposed two operator mistakes and one product defect. The
first dashboard request carried an HTTP-only `id` field and was rejected before
effects. The corrected request initially found no
`dashboard-service-backend` route. A later direct `tab_new` request selected an
occupied display owned outside the development runtime and failed three times;
that action was also the wrong acquisition path for a route-bound RDP browser.
No foreign process was terminated. A subsequent `remote_view_open` without an
explicit relay session created a browser in the internal dashboard lane. That
development-only browser was closed exactly, and service reconciliation
repaired its orphaned display allocations.

The startup defect was a readiness-boundary error. Under runtime-host
admission, `daemon_ready(session)` proves only that the shared runtime-host
socket accepts connections. It does not prove that the named
`<session>.stream` route exists. Dashboard bootstrap and recovery used that
weaker check both before launch and immediately after a successful child exit,
so they could report readiness too early or report a false failure while route
publication was still converging.

A focused red test first established that a named lane is required and that an
explicit lane-refresh request cannot reuse the old route. The repair now uses
the same named-lane readiness predicate as ordinary daemon startup, and after
the recovery child exits it polls for up to five seconds for the route to
become visible. The regression, the existing cold runtime-host lane test,
format, workspace clippy with warnings denied, service API and MCP parity,
generated client contract and type checks, and the service collection
no-launch smoke all pass.

Revised next action: publish and install the named-lane readiness repair,
reconcile the development provider after the generation change, then prepare
E7 through an explicit service-owned remote-view lane. Seal its opaque handoff
and identity values into the external-vantage environment without printing
them, and dispatch a fresh two-client readiness epoch from the exact installed
commit.

Commit `5170374f` installed as development generation
`0.28.0-1aa5838cd691`. The three-launch smoke passed, production remained
unchanged, provider generation drift was reconciled, all six provider routes
passed doctor, and local and public dashboard probes returned 200. The live
post-install check then showed the first repair was necessary but incomplete:
the dashboard still exhausted its five-second named-lane wait during the
simultaneous systemd startup, and `dashboard-service-backend.stream` remained
absent. A later manual invocation created the lane immediately and returned
the expected already-enabled stream result.

Runtime-host and dashboard journals showed all three generation units starting
in the same second. The dashboard's one bootstrap attempt could therefore lose
the runtime-host replacement race, even though its child command exited zero.
No later dashboard path retried the missing internal lane. A second focused red
test models that exact sequence: the first convergence attempt fails and the
next succeeds. Dashboard bootstrap now performs at most three attempts with a
250 millisecond gap after a failed bounded recovery, records every failed
attempt and its ordinal in stderr, and returns immediately on success. The
earlier five-second named-lane publication wait remains in each recovery
attempt. The retry regression, named-lane regression, format, and workspace
clippy with warnings denied pass.

Revised next action: publish and install the bounded dashboard-bootstrap retry,
repeat the provider reconciliation, and require a fresh unattended startup to
materialize `dashboard-service-backend.stream` before preparing E7.

The bounded-retry candidate installed as development generation
`0.28.0-58dc65a4a42c`, but the unattended proof still failed all three attempts.
The new ordinal logging made the failure deterministic: every recovery child
exited zero and no named lane appeared. Comparing only non-secret process-mode
environment names exposed the final cause. Stable dashboard ingress sets
`AGENT_BROWSER_DASHBOARD_INGRESS`; the recovery child removed the ordinary
dashboard selector but inherited the ingress selector. It therefore entered
stable-ingress mode before parsing `stream enable`, failed to bind the already
occupied dashboard port, and returned without creating the lane. The same
command from an ordinary shell had no ingress selector and created the lane.

A third focused red test now freezes the child-environment contract. Recovery
must scrub ordinary dashboard, stable dashboard ingress, and backend-only
process selectors. The implementation centralizes all recovery-child removals,
adds the two missing dashboard modes and the backend port, and preserves the
runtime-host ingress state required to reach the selected host. The new
process-mode regression, format, and workspace clippy with warnings denied
pass.

Revised next action: publish and install the process-mode scrub repair, then
repeat the unattended named-lane proof. Do not proceed to E7 unless the lane is
created without a manual recovery command and the startup journal contains no
terminal dashboard-backend initialization failure.

The process-mode scrub candidate installed as development generation
`0.28.0-25fb1e283f6d`. Its first dashboard-backend attempt lost the simultaneous
runtime-host startup race and recorded child exit 1; the second attempt created
`dashboard-service-backend.stream` without manual intervention. No terminal
initialization failure was recorded. Provider reconciliation, all six route
checks, the three-launch smoke, and production-unchanged proof then passed.

Fresh E7 access planning reproduced the Last30Days identity defect against the
accidentally retained terminal profile. The revisioned shared-local ACL allowed
the stable self-declared subject and required no missing permission, but the
separate terminal-replacement branch unconditionally returned
`profile_capability_required`. This was an internal policy contradiction, not
a client omission: the exact same plan simultaneously said `allowed: true` and
withheld its executable request.

A focused red regression extends the existing terminal-replacement fixture
across both policy modes. A restricted profile still fails closed for an
unproven subject, while changing that same profile to shared-local makes the
terminal replacement request available without a cryptographic capability.
The repair gates the legacy terminal-replacement capability requirement on
strict identity mode. The terminal lifecycle regression and all five
shared-local focused tests pass, as do format and workspace clippy with
warnings denied.

Revised next action: publish and install the shared-local terminal-replacement
repair, refresh the E7 access plan, and execute its service-owned remote-view
request through the deterministic replacement lane. Preserve restricted and
exclusive capability enforcement unchanged.

The shared-local repair landed at `64460847` and installed as development
generation `0.28.0-f9cb4543a693`. Unattended startup created the dashboard
backend lane on its second bounded attempt. Provider state and Cooper ingress
were reconciled without changing production. The refreshed access plan was
internally consistent: shared-local access was allowed, no permission was
missing, and the terminal replacement request was executable. A first
preparation request mistakenly placed the target URL outside `params` and
opened `about:blank`; the corrected request reused the same browser and opened
the loopback synthetic fixture. Its operator-visible proof was ready, its
service tab handle was valid, and its opaque durable handoff was published to
the external environment without printing it.

E39, workflow run `33868209317`, was a preparation failure. The external pixel
region was copied from the fixture's 1440 by 1000 document attestation instead
of the previously proven 400 by 100 iframe crop. The human client rejected the
oversized crop, while the slow client could not match its pixels. Both failure
receipts and the aggregate were sealed without repair inside the epoch. The
environment secret was corrected to the exact provider-free tested iframe
region before a new epoch.

E40, workflow run `33868662141`, reached the public dashboard from both hosted
runners. The human client successfully resolved the prepared handoff and
loaded Guacamole, but its simulated input changed the shared tab from
`/fixture` to `/error-action`. The delayed client then correctly failed its
post-checkout proof because the retained target no longer matched the durable
handoff's expected URL. The human pixel check also correctly rejected the
resulting error page. The aggregate retained both typed failures, both client
identities, zero retries, and zero repairs.

The job record and captured video identify a runner defect rather than an
identity, resolver, browser, or ingress defect. Human-paced input scrolled the
remote document before clicking a coordinate defined in unscrolled document
space. The shifted point landed on the fixture's intentional error link and
invalidated the shared target before the second client resolved it. A
provider-free regression now freezes the safe ordering: keyboard focus checks
and the harmless marker click occur before any scrolling input, while the
existing arrow-key exercise and deterministic origin reset remain afterward.

Revised next action: publish the runner-only correction, restore the existing
E7 target to the attested fixture without replacing its browser or handoff,
refresh the external expected identity only if live readback changes, and
dispatch one fresh readiness epoch from the exact clean commit. If both clients
and the aggregate pass, dispatch C01 immediately from that same commit and
freeze source and runtime state through the shared calibration barrier.

E41 ran the first input-order correction from exact head `bbf5fb78` as workflow
run `33869354975`. The E7 target was restored to `/fixture` before dispatch and
the first external resolver again succeeded. The human client nevertheless
navigated the shared tab to `/error-action` immediately after the first good
fixture frame. The delayed client then reported the same truthful wrong-tab
proof failure. Both receipts and the aggregate sealed without retry or repair.

Frame-by-frame review shows the remaining unsafe operation is the blind remote
coordinate click itself, including the click used to focus the Guacamole
surface before reset. A visible pixel location is not yet a proven input
location across the Guacamole coordinate transform. The generic human-paced
probe must therefore use iframe focus, keyboard traversal, arrow-key scrolling,
and deterministic reset without any coordinate click. Target-located W8 action
cases retain click coverage after their own locator proof. The provider-free
runner test now rejects any mouse click in generic human pacing and proves the
focused keyboard and reset sequence.

Revised next action: publish this stricter runner correction, restore the same
E7 tab once more, and dispatch a fresh exact-commit readiness epoch. A further
self-induced navigation is a blocking test-harness defect and must be diagnosed
before C01; a clean two-client aggregate permits immediate C01 dispatch.

E42 ran from exact head `3e2adf17` as workflow run `33869785636`. Eliminating
blind clicks fixed the destructive symptom: both clients resolved the durable
handoff, both reached Guacamole, both observed the same non-marker pixel hash,
and live CDP readback after the epoch proved the shared target remained on
`/fixture`. The human crop showed fixture controls where the solid marker
should have been, while the slow crop showed the same displaced viewport state
later in the sequence. This proves the focused arrow and reset keystrokes still
mutated shared scroll state and that iframe focus did not provide a reliable
remote `Control+Home` reset boundary.

Generic readiness observation does not need to mutate the remote document.
Human pacing is now limited to dashboard keyboard traversal and a mouse move
over a blank part of the remote view. It performs no click, wheel, focused
remote key, or other shared-browser mutation. Explicit locator-proven action
cases remain responsible for state-changing remote input and its postcondition
proof. The provider-free regression freezes this passive event sequence and
rejects coordinate clicks.

Revised next action: publish the passive-readiness correction and dispatch a
fresh exact-commit readiness epoch against the still-correct E7 target. If its
two clients and aggregate pass, begin C01 immediately from that same commit.

E43 ran from exact head `c348d2d498da7484474e64d50b46b3a669744715`
as workflow run `33870248318`. The human client rendered the expected synthetic
marker. The slow client initially rendered it alone, but its concurrent view
became black when the human view connected. Existing provider live tests had
classified simultaneous viewing from frame presence alone and therefore
missed the content loss.

The route used one direct Guacamole connection for each dashboard iframe.
Each iframe consequently opened an independent RDP login, and xrdp logged off
the earlier desktop when the later client arrived. The provider repair now
creates one stable managed Guacamole sharing profile for every route. The
dashboard authenticates to Guacamole, finds the current active connection and
its exact route sharing profile, requests a transient share credential, and
uses that credential only inside the iframe. It does not expose the credential
through the Agent Browser service API or durable operator handoff. Provider
schema drift is now visible to readiness probes. Focused sharing, provider,
workstation, dashboard, fixture, release-asset, format, clippy, build, install,
doctor, launch-smoke, and external-ingress checks passed. Commit `1d061647`
was pushed and installed as development generation
`0.28.0-bd846cd12ea0`; production remained unchanged.

E44 ran from exact head `1d061647d18ec61cc10796c12aefabaa8b95bcd3`
as workflow run `33873479843`. Both clients received
`handoff_target_closed_operator_action_required` before Guacamole access. The
development installation had closed the E7 browser, but preparation had
incorrectly assumed the retained identity and handoff remained live. Both
failure receipts and the aggregate were sealed before repair. This was a
campaign preparation defect, not evidence about connection sharing.

The profile was then opened through a fresh service-owned remote-view lane.
The request returned a verified terminal effect, a valid service tab, and
`operatorVisible.state=ready`. The new opaque durable handoff and exact browser,
profile, session, tab, and target identity were sealed in the protected GitHub
environment without logging their values.

E45 ran from the same exact head as workflow run `33874034647`. Both clients
resolved the new handoff and successfully reached the Guacamole authentication
and active-connection APIs, but neither retained an iframe long enough to
render the marker. The dashboard remained at “Checking stream.” Network
evidence showed repeated successful token requests interleaved with aborted
tunnels. The sharing effect depended on the whole projected stream object.
Routine projection refreshes recreated that object, aborted the current
effect, cleared the iframe, generated another one-time share credential, and
started the cycle again. The failure journal recorded the aborted Guacamole
loads, while the external receipts supplied the missing higher-level context.

A focused red regression now requires the component to memoize the five
semantic Guacamole route fields and forbids whole-projection object churn from
invalidating the sharing effect. The repair passes that regression and the
dashboard production build.

Revised next action: publish and install the stable-sharing-effect repair,
reconcile the development provider and external ingress if the generation
changes, reopen and reseal a synthetic handoff if installation closes it, then
dispatch a fresh two-client readiness epoch. Do not begin C01 until both
external clients retain the exact marker through concurrent viewing and the
aggregate seals successfully.

E46 ran from exact head `97c054eb0ce01f6bf9269dd150c128ae00af0e9a` as
workflow run `33874989172`. The human client passed initial load, concurrent
marker observation, reconnect, exact browser identity, and exact marker pixels.
The delayed client failed before handoff resolution because three required
hashed dashboard JavaScript chunks returned HTTP 504 after approximately 27.2
seconds. The aggregate sealed the passing human receipt and the failed delayed
receipt without retry or repair inside the epoch. This result proves the stable
Guacamole sharing mount works through concurrent use, while disproving that the
earlier dashboard-ingress repair eliminated the complete external-load failure
class.

A local external-ingress reproducer then kept one authenticated durable handoff
viewer connected while opening fresh clients through the public origin. Twelve
fresh clients produced ten failures. The scenario remained red after reduction
to the historical shape of one established client plus two fresh clients: the
fourth bounded repetition produced one client that never acquired an iframe.
That client observed repeated service-resource 503 responses, a service-request
502, a service-status 504, a failed Guacamole sharing-credential request, and
multiple API reads pending for 5 to 17 seconds. A separate 30-way comparison
kept the generation backend below 6 milliseconds, stable ingress below 8
milliseconds, and settled public static reads predominantly below 750
milliseconds. The defect therefore depends on fresh dashboard convergence and
is not ordinary static-file throughput saturation.

The durable failure journal exposed the causal amplification. Every newly
opened control viewport automatically submitted `view_focus`. The runtime host
had committed its lane configuration before the retained managed browser was
attached. It consequently injected the lane's obsolete `lease-fail-open`
runtime-profile default into each later profile-omitting focus command. The
active browser correctly rejected that invented profile mismatch. Repeated
clients accumulated failed and timed-out focus jobs, then contended on the
Service State lock needed by viewer-lease acquisition. The journal recorded the
dashboard and Guacamole failures visible after application startup, but a 503
emitted by stable ingress itself had no durable record. That omission explains
why the E46 bundle-load failure depended on external HAR evidence for its first
post-mortem signal.

Two focused red-to-green regressions now freeze the repaired invariants.
`view_focus` is an exact operation on the already-selected runtime lane and no
longer accepts an implicit lane-initialization profile default. An explicitly
supplied profile remains in the command and therefore still reaches the
existing mismatch guard. Separately, every stable-ingress unavailable response
constructs and appends a privacy-bounded `dashboard_action` failure record with
source `dashboard_ingress`, stage `request_proxy`, the exact typed ingress code,
and action `dashboard_load`. The record deliberately excludes raw request URLs,
headers, backend messages, and credentials. Nine runtime-host tests, 28
dashboard-ingress tests, Rust format, and workspace clippy with warnings denied
pass. The first focused build encountered a transient sccache daemon fork
failure under host process pressure; the preserved cache-sanitized diagnostics
contained no credential values, and the same tests passed with only the cache
disabled while retaining the wrapper's admitted eight Cargo jobs.

Revised next action: commit and publish this bounded runtime-host and ingress
logging repair, install its exact candidate only into the development runtime,
and prepare a fresh synthetic handoff because installation may close the current
browser. Verify that one direct focus request against the installed retained
browser does not inherit an obsolete lane profile, verify an induced stable-
ingress unavailable response appears in the development failure journal without
private request material, and rerun the reduced public two-client load loop.
Only after that loop and a fresh externally hosted two-client E47 aggregate pass
may the campaign dispatch C01 from the same frozen commit.

The first installed-candidate focus proof exposed one remaining layer of the
same blocker before E47 was dispatched. A fresh service-owned remote-view open
returned a valid shared tab on the selected managed profile, a visible browser
window, and `operatorVisible.state=ready`. The direct `view_focus` request then
failed with `explicit_profile_conflicts_with_current_owner`. Inspection before
retry proved that the browser, session, profile, CDP endpoint, and active
fixture tab were live and mutually consistent, that the tab was already
focused, and that the failure journal contained no new `lease-fail-open` or
`existing_session_profile_identity` signature. The failed job retained an
`effect_uncertain` terminal outcome, while readback showed no changed tab
effect.

The runtime-host repair had stopped writing the obsolete lane profile into the
command, but daemon launch options still inherited the process environment's
runtime-profile default before retained-browser recovery. Existing-owner
selection could not distinguish that inherited default from a caller-authored
profile and rejected it. A focused regression reproduced the exact
profile-omitting `view_focus` case against a proven current owner and failed
before the correction. The bounded correction lets only a profile-omitting
`view_focus` use the current retained owner's profile. Caller-authored
`runtimeProfile`, `profileId`, or `profile` values continue through the strict
conflict checks. The focused existing-session group now passes all five tests,
including the explicit-conflict and inconsistent-owner guards.

Revised next action: complete Rust format, clippy, and affected focused tests;
publish and install the exact corrected development candidate; reopen and
reseal the synthetic handoff; then repeat the direct focus, durable ingress-
failure logging, reduced public-load loop, and external E47 gates. E47 remains
undispatched, so no epoch freeze was violated.

The next installed proof showed why command-level provenance was also
required. The daemon accepted current-owner selection, but the final active-
browser guard still received `runtimeProfile=lease-fail-open-...` and rejected
it against the correct managed profile. The failure occurred through both
stable ingress and the generation backend, ruling out proxy mutation. Exact
job and tab readback again showed an effect-uncertain failure with the fixture
tab still active. The service request body omitted all profile fields; the
stale value came from cached lane routing between normalization and final
dispatch.

The completed repair records one internal, caller-non-forgeable boolean during
service-request normalization stating whether `runtimeProfile`, `profileId`,
or `profile` was actually caller-authored. At the shared runtime-host boundary,
a normalized `view_focus` marked profile-omitting discards any cached profile
fields before dispatch. Explicitly profiled service requests and direct daemon
commands retain their fields and the existing fail-closed mismatch guard. The
runtime host removes the internal marker before daemon action dispatch. A new
red-to-green regression covers inherited, explicit, and direct-command cases;
the seven focused view-focus, HTTP-routing, dashboard-routing, and service-
normalization tests pass.

Revised next action: validate, publish, and install this provenance-backed
repair, then repeat the same installed direct-focus gate before any load or
external epoch. Preserve both failed installed proofs as pre-repair evidence.

The provenance-backed candidate was installed as development generation
`0.28.0-8cf35c42e39b`; required-provider doctor, local ingress, and public
ingress all passed while production remained unchanged. A fresh synthetic
remote-view acquisition returned a verified effect, a valid fixture tab, and
`operatorVisible.state=ready`. The exact profile-omitting direct `view_focus`
proof then passed with `verified_effect`, brought the retained browser forward,
and requested maximize. A deliberately incomplete ingress request added
exactly one `dashboard_ingress` record with stage `request_proxy`, code
`invalid_ingress_request`, and action `dashboard_load`; the record contained no
raw URL, headers, message, credential, or token fields.

The first reduced-load harness attempt then found two more seams before E47.
It also incorrectly accumulated a new anchor and two fresh clients on every
round instead of retaining one anchor and closing each round's two fresh
clients. That excess occupancy is preserved as an out-of-schedule finding, but
it is not the acceptance shape for this gate. Even the first intended
three-client slice proved two product defects. The dashboard's special
owner-session focus proxy bypassed normal service-request normalization, so it
did not attach the internal profile-omission marker and four automatic focus
requests failed or timed out with the stale-profile mismatch. In parallel,
each viewport polled `/api/service/resources` every seven seconds. Status reads
were coalesced and cached, but resource reads were not. Duplicate process-table
scans exceeded the ordinary two-second proxy budget, left queued and running
resource jobs, caused one viewer-lease Service State lock timeout, and made the
stable ingress emit and journal `selected_backend_unavailable`. The failure
journal also captured repeated Guacamole disconnect observations, three focus
HTTP 502s, and the viewer-lease application failure.

Focused red regressions now require the special focus proxy to attach the
non-forgeable profile-omission marker it constructs from an allowlisted request,
and require authenticated status and resource reads to share the existing
path-keyed single-flight cache and ten-second read budget. The repair keeps
mutating requests outside that cache. All 53 dashboard gateway tests, Rust
format, workspace clippy with warnings denied, and the focused single-flight
test pass. An initial eight-job compile could not spawn a rustc work thread
because old Cargo scopes contained hundreds of retained Chrome descendants and
were near their aggregate task limit; a one-job compile passed without
discarding those ownership-sensitive processes. This is retained as a separate
resource-admission and test-residue finding for later campaign analysis.

Revised next action: commit and publish the focus-proxy and resource-read
coalescing repair, install its exact development candidate, recreate the
synthetic handoff, and rerun one retained authenticated anchor plus two fresh
clients for five bounded rounds, closing the fresh clients after each round.
Require focus success, usable Guacamole pixels, no resource backlog, no ingress
503 or 504, and no new stale-profile mismatch before dispatching E47.

The installed focus-proxy proof then isolated a later mutation boundary. The
dashboard proxy and shared runtime host both removed the inherited profile as
designed, but the control-plane scheduler subsequently applied the legacy
profile-acquisition gate to `view_focus`. Under the configured
`fail_open_ephemeral` escape hatch, a lease conflict caused that gate to write a
new `lease-fail-open-*` runtime profile into the already routed focus command.
The final active-browser guard correctly rejected that rewritten profile
against the retained managed browser. A uniquely tagged diagnostic response
proved that `runtimeProfile`, `profileId`, and `profile` were all absent after
runtime-host reconciliation; all temporary diagnostic response fields were
then removed.

A focused scheduler regression reproduced the historical control-path failure
and failed with `existing_session_profile_identity_unproven` before the repair.
The bounded correction exempts only commands already classified as retained
browser focus or bounded desktop control from profile-acquisition admission.
Those commands do not acquire or launch a profile. Explicit profile fields are
still preserved for the final active-browser mismatch guard, and ordinary
browser acquisition commands retain the enforced, fail-open, and unsafe-claim
lease behavior. The new regression, both neighboring lease-admission tests,
and the inherited-profile runtime-host regression pass.

The same installed investigation found a separate development display residue:
an Xvfb process on display `:90` had survived since September 2 without any
client carrying `DISPLAY=:90`. It caused three logged remote-headed launch
attempts to fail before the display became ready. The exact process was closed,
the display socket disappeared, and the same access-plan launch then succeeded
without a profile, identity, or Xvfb failure. This residue and the discarded
runtime-host stderr channel remain campaign findings for the final holistic
analysis; the browser-launch failure itself was present in the durable failure
journal.

Revised next action: complete format, clippy, and focused validation; commit and
publish the scheduler repair; install its exact development candidate; recreate
one coherent managed browser and prove the dashboard-routed `view_focus` no
longer receives a fail-open profile. Then rerun the corrected five-round public
load gate and dispatch E47 only if its failure-journal delta remains clean.

The exact scheduler repair was committed, published, installed as development
generation `0.28.0-31868282c605`, and re-proved through the dashboard route. A
fresh remote-view acquisition reached `operatorVisible.state=ready`, and the
same retained handoff browser accepted dashboard-routed `view_focus` without a
profile mismatch or identity rejection. Production remained unchanged.

The corrected external-ingress load gate then stopped on its anchor because one
`GET /api/service/resources` returned HTTP 503 after roughly two seconds. The
durable failure journal recorded two nearby `selected_backend_unavailable`
events while direct readback showed the service worker ready with queue depth
zero. The defect was a split timeout contract: the inner dashboard proxy had
already been widened to ten seconds for both status and resources, but the
outer stable ingress granted ten seconds only to status and still terminated
all resource reads after two seconds. The backend subsequently returned HTTP
200, explaining the observed 503 followed by 200 sequence. This was a campaign
sequence blocker, so the campaign paused for diagnosis and repair while the
Plan 0158 goal remained active.

A red regression proved that `/api/service/resources` received only the
ordinary two-second ingress allowance. The bounded repair gives status and
resources the same ten-second outer-ingress allowance, matching their shared
single-flight inner proxy contract. Focused red-to-green validation passes.
Ingress failures now also journal a redacted request route, request method,
route-specific action, retry safety, selected generation, fallback attempt,
failure phase, and first-response timeout. Handoff identifiers, Guacamole
session paths, query strings, cookies, and tokens remain excluded. Focused
tests cover both the richer resource-failure record and path redaction.

Revised next action: finish workspace format and clippy validation, publish and
install the exact timeout-alignment candidate in development, then recreate the
handoff and rerun the corrected external five-round load gate from its anchor.
Only dispatch E47 if that gate has no 502, 503, or 504 response, no unusable
handoff or stream, no resource backlog, and no new identity rejection.

The timeout-alignment candidate was committed and published as `85a8be6c`,
installed as development generation `0.28.0-eccfb7c03455`, and passed the
three-launch smoke plus the required six-route provider doctor. Local and
bastion ingress were republished from the reviewed Cooper inventory while its
pre-existing unrelated worktree changes remained untouched. A new access plan
selected the intended managed profile, and a fresh remote-view request reached
`operatorVisible.state=ready` with a verified terminal effect.

The next corrected five-round public run kept one anchor and created ten fresh
clients, closing each round's two fresh contexts. The anchor and two clients
passed, while eight clients failed. Crucially, no resource endpoint returned
503, confirming the first timeout repair. The next pressure surface was broader:
browser-capability-registry reads and same-session tab reads remained
uncoalesced and retained two-second response budgets. The public clients saw
503 responses for those reads while the new structured ingress records proved
`first_response_timeout`, `retrySafe=true`, and `firstResponseTimeoutMs=2000`.
Several `view_focus` calls returned client-visible HTTP 502, but authoritative
job readback showed every completed focus job succeeded with
`verified_effect`; the special focus proxy had abandoned those results after
two seconds. One focus job remained queued at readback and is retained for the
campaign analysis.

Three red regressions reproduced the cache-membership, focus-budget, and outer
ingress-budget gaps. The bounded repair adds service contracts,
browser-capability registry, and per-session `/api/tabs` reads to the existing
five-second path-keyed single-flight cache; the tab key remains isolated by
browser port. These pressure-sensitive reads receive matching ten-second inner
and outer budgets. The special dashboard focus route now receives the existing
fifteen-second remote-view action budget, so a successful queued effect is not
reported as a client failure. The ingress journal now recognizes the new exact
read routes while continuing to redact queries and dynamic handoff or
Guacamole paths. All three regressions pass green.

Revised next action: run the complete dashboard and ingress module tests,
format, clippy, and patch checks; commit and publish the bounded read and focus
repair; install its exact development candidate; then repeat the same external
anchor plus ten-fresh-client gate without changing the harness.

The bounded read and focus repair was committed and published as `dcb1eb29`,
installed as development generation `0.28.0-a26c9bd8af0c`, and passed the
three-launch smoke plus all 59 required provider checks. The launch smoke
recorded one transient one-second Service State lock wait but completed all
three iterations. A new managed browser and durable handoff again reached
`operatorVisible.state=ready` with a verified effect.

The unchanged public gate improved from two to nine passing fresh clients out
of ten, while its retained anchor also passed. There were no client-visible
focus 502s and no 503s from resources or browser-capability registry. One
first-round client failed after a 504 from `/api/session-tabs`; its Guacamole
iframe never appeared. Isolated external probes then showed all 15 current
session-tab reads returning HTTP 200 in at most 0.717 seconds, so the endpoint
was not intrinsically slow. The pressure source was the dashboard's polling
amplification. Its current service-status payload is 4.36 MB, dominated by
historical remote-view acquisition leases, runtime-owner history, jobs,
profiles, and events. One client took about 2.6 seconds to fetch it, while an
eleven-client coalesced wave took about ten seconds. Three independent
seven-second UI pollers and an overlap-prone five-second session poller could
start new cycles while earlier cycles remained unresolved, repeatedly sending
that projection and every live session's tabs to each client.

The next bounded repair prevents overlapping cycles independently in session
synchronization, the workspace navigator, selected-workspace context, and the
service panel. It preserves polling and left-rail coverage but refuses to start
a second cycle while the same component's first cycle is in flight. Because
backend stalls affect static and lightweight reads as well as the explicitly
heavy routes, stable ingress now gives every idempotent GET, HEAD, or OPTIONS
request the same ten-second first-response budget. Mutation requests retain
their separate delivery-aware budgets and no-replay rules. Expanded structured
logging recognizes sessions, models, runtime health, dashboard auth, and chat
status routes without storing query strings or dynamic route identifiers. A
red regression proved generic GET and HEAD requests previously retained only
two seconds; it now passes. The dashboard production build, non-overlap source
contract, selected-workspace, navigator, inspector-action tests, all 29 ingress
tests, Rust format, and workspace clippy pass.

Revised next action: commit and publish the non-overlap and generalized
idempotent-read repair, install its exact development candidate, and rerun the
same public gate. If it passes, inspect its full anchor and failure-journal
delta before dispatching E47; the current harness validates the anchor only at
startup and therefore needs an end-of-run anchor health assertion before the
campaign can treat a green client count as acceptance.

The selected live CDP streaming check initially failed because Chrome could
not create threads or fork its utility process. Direct capacity readback found
the shared Cargo validation slice at 945 of its 1,024-task ceiling. Five Rust
test scopes had remained active since September 2 or 3, and one command scope
was stranded by the failed smoke. This was not a browser, dashboard, memory, or
per-service limit failure. Stopping those six exact stale validation scopes
reduced the slice to nine tasks without disturbing either installed runtime.
The unchanged live CDP streaming check then passed. Retain this as campaign
evidence that abandoned validation scopes can starve browser launches and that
the current admission surface does not reclaim or clearly report stale task
claims.

The exact `5433b4e5` candidate installed as development generation
`0.28.0-44a4e6ae02e5` with production unchanged. Its immediate launch smoke
then failed on a one-second Service State file-lock wait while the freshly
restarted development services were reconciling. The services remained ready
and no crashed holder existed. The smoke had omitted the repository's existing
per-command lock-timeout escape hatch. It now supplies a bounded ten-second
budget to each open, read, and close operation, retaining a finite failure for
a wedged holder. The unchanged three-iteration launch and residue sequence
passes with that contract.

After provider reconciliation, reviewed Cooper ingress publication, and all
59 required provider checks, the fresh external handoff reached
`operatorVisible.state=ready` with a verified effect. The corrected load gate
again passed nine of ten fresh clients, but its new final anchor assertion
found two session-tabs 504s, one Service Status 504, and one runtime-health
504 accumulated after startup. The retained iframe still pointed to the
Guacamole route and showed no disconnect text. One fresh client also failed to
render its iframe after a session-tabs 504 and a Guacamole token failure. No
matching application-ingress 5xx record existed, locating the 504 boundary
outside the Agent Browser journal.

The first non-overlap repair was necessary but insufficient. The retained
client still recorded 51 successful Service Status reads and 188 successful
session-tabs reads during the five-round run. Three independent dashboard
components each downloaded the same 4.36 MB status projection, and the
five-second backend cache expired before their seven-second poll cycles. The
next bounded repair provides one module-level ten-second Service Status flight
and ready response shared by all dashboard components. The backend cache now
uses the same ten-second freshness bound for status, resources, contracts,
browser-capability registry, and per-session tab reads. This keeps the left
rail bounded to ten-second freshness while coalescing both component-local and
cross-client load. The dashboard build, source contracts, all 53 dashboard
backend tests, format, and workspace clippy pass.

The `9c7ff9cb` candidate installed as development generation
`0.28.0-a9259486b184` with production unchanged. The immediate launch smoke
again exposed the one-second pre-command file-lock boundary. A read-only
preflight was added so the smoke never retries an `open` effect. Its first
implementation then misclassified a 1 MB `spawnSync` buffer overflow as live
lock contention because the partial 4.36 MB status payload contained a
historical lock-timeout string. The corrected preflight uses a bounded 16 MB
buffer and classifies only the top-level structured error. It and all three
launch cycles pass.

The next external gate loaded all ten fresh clients and reduced retained-anchor
Service Status reads from 51 to 10. No status, runtime-health, resource,
registry, focus, handoff, Guacamole, or fresh-client failure occurred. The
final anchor check alone retained two session-tabs 504s. A follow-up external
per-port probe issued 110 requests across all ten retained session ports with
zero failures and a maximum latency below 0.8 seconds, proving no session route
was intrinsically slow. The remaining load exists only where the five-second
all-session tab walk overlaps full dashboard startup. Session and tab polling
now uses the same ten-second freshness bound as Service Status, cutting that
background volume approximately in half without sacrificing the left rail's
bounded accuracy.

The polling-cadence candidate was committed and published as `e3ab4677`,
installed as development generation `0.28.0-aa2dccdc3a41`, and left production
unchanged. The stricter local public-ingress harness now requires the exact
synthetic fixture marker from every client and rechecks the retained anchor at
the end of the run. That harness exposed periodic whole-environment stalls:
realtime and monotonic observations moved backwards by approximately 2.4 to
2.7 seconds every 27 to 30 seconds while the WSL journal independently recorded
backwards time jumps. Stopping timesyncd, testing each available Hyper-V clock
source, disabling implicit Hyper-V time synchronization, unbinding the exact
Hyper-V time device, and relieving memory fragmentation did not remove the
jumps. Each diagnostic change was restored. No userspace clock adjustment was
observed. This is retained as a host-runtime defect and does not stop the
campaign or authorize a production restart.

External run `33903748862` exercised both clients and uploaded the restricted
artifacts, but both rendered `about:blank`. The preparation request itself had
asked for a blank page. Top-level URL normalization was not defective: the
service normalizer supports that form, and the generated client deliberately
moves it into request parameters. The campaign corrected the fixture request
to the canonical synthetic target instead of changing valid product behavior.

External run `33905363192` then proved the human-simulated client, the initial
slow client view, the browser target, the RDP route, and the first shared view.
The second concurrent slow-client join received successful active-connection,
sharing-profile, credential, and token responses but rendered the Guacamole
connection-list home instead of the shared browser. Guacamole may report the
primary connection and one or more shared child connections with the same
connection identifier. The dashboard selected the first matching row without
distinguishing its role, so ordering could make it create a share from an
already shared child.

The bounded repair selects only a matching active connection whose sharing
profile identifier is absent, which identifies the primary owner. If no rows
match, the existing direct connection fallback remains available. If matching
shared children exist but the primary is unavailable, the resolver now returns
an explicit diagnostic error instead of initiating a direct connection that
could displace the existing viewer. A regression supplies the shared child
first and proves that credentials are created from the later primary row. A
second regression proves the shared-child-only state fails diagnostically.
The focused connection-sharing test, dashboard inspector-action contract, and
production dashboard build pass.

Revised next action: commit and publish this exact primary-owner selection
repair, install it only in the development runtime, recreate the canonical
synthetic handoff, and rerun the exact two-client external oracle. A green
result advances directly to the frozen campaign sequence; another failure is
diagnosed and recorded under the same still-active Plan 0158 goal.

The primary-owner selection repair was committed and published as `c59405b3`,
installed as development generation `0.28.0-9f7c26412225`, and passed the
three-launch smoke. The six-route provider was reconciled to that exact
generation, all required provider checks passed, public HTTPS returned 200,
and production remained unchanged.

External run `33906793894` did not exercise the sharing repair because the
durable handoff's browser had been deliberately closed during candidate
replacement. Both clients returned
`handoff_target_closed_operator_action_required`; each recorded 18 successful
HTTP requests with no failed or pending transport. The campaign then used the
explicit operator-confirmed reopen path. Route preparation rolled back cleanly
when launch failed with `existing_session_profile_identity_inconsistent`.

The retained owner, lifecycle, profile, session, browser, and tab evidence
identified an overbroad terminal-relaunch guard. The exact owner lifecycle was
terminal with satisfied cleanup, process absence and profile-lock release were
proven, the session lease was released, and its two tab handles were closed and
inert. However, the tab projection also classified every historical closed tab
that happened to reference the same profile as related to the current browser.
Those older tabs correctly named different terminal browser and session IDs,
so they could never satisfy the current browser's inert-handle predicate. The
result was a false profile-identity inconsistency despite the access plan
correctly reporting `replacementEligible=true` and
`requiredAction=supersede_terminal_owner`.

The bounded repair defines terminal-relaunch tab ownership by the exact browser
or session identity only. A closed tab from another terminal owner no longer
blocks the current exact replacement merely because both used the same shared
profile. The existing profile-wide browser occupancy guard remains intact, so
a live or retained browser projection for the same profile still prevents an
unsafe relaunch. The focused regression recreates the historical closed-tab
shape and passes.

Revised next action: complete Rust format and clippy validation, publish and
install the exact terminal-relaunch repair in development, explicitly reopen
the same durable handoff, then rerun the two-client external oracle so the
primary Guacamole share-owner repair is finally exercised.

The terminal-relaunch repair was committed and published as `9047dd8d`, then
installed as development generation `0.28.0-9682c30f056f`. The three-launch
smoke passed and production remained unchanged. Explicit reopen still found 34
inert browser projections and 42 closed tabs from older attempts on the shared
profile. The first-class guarded prune escape hatch removed exactly those 34
browsers and 42 tabs plus 27 stale session browser references; it removed no
profiles, displays, or sessions. The same durable handoff then reopened with a
verified terminal effect and `operatorVisible.state=ready`. This unblocked the
sequence while retaining the broader accumulation defect for final campaign
analysis rather than hiding it with ad hoc cleanup.

External run `33908153792` then exercised the intended sharing path. The
human-paced client passed. The slow client initially rendered the prepared
marker, but its concurrent second context rendered Guacamole's connection-list
home instead. Its preserved network evidence shows successful active
connection, sharing-profile, sharing-credential, and token responses before
the wrong page appeared. The earlier primary-owner selection repair therefore
worked as designed but was not sufficient.

The remaining cause is identity aggregation at the ingress boundary. The
transient sharing key authenticates the second viewer as intended, while the
normal `/guacamole/` route also injects the stable full-operator `Remote-User`
header. Guacamole accepts both identities and resolves the full operator home
instead of entering only the shared connection. This is not a timing wait or
an epoch problem.

The bounded repair gives transient shared-view traffic a separate
`/guacamole-share/` capability path. Cooper ingress rewrites that prefix to the
existing Guacamole servlet path but deliberately omits the dashboard
forward-auth middleware on that one route. The one-time sharing key remains
required. The ordinary `/guacamole/` path retains stable operator
authentication. Renderer validation and a regression prove the rewrite is
attached to both local and external Cooper routers. Local and external no-key
probes return only the Guacamole shell and expose no connection names, while
the ordinary route continues to require authentication. The focused dashboard
sharing test and production dashboard build pass.

Revised next action: commit the isolated Cooper route and dashboard URL repair,
install the exact development candidate, preserve the current handoff identity,
and rerun the unchanged two-client external oracle. Treat a no-key route that
can enumerate or open any connection as a security failure. A green concurrent
marker and reconnect sequence advances to the frozen campaign; any new failure
is preserved, diagnosed, and repaired under this still-active goal.

The path-isolation attempt was committed as Agent Browser `81a43e70` and Cooper
`46a70b0`, installed as development generation `0.28.0-013bcece3d17`, and
passed three browser launches, all provider checks, local ingress, public
ingress, and no-key connection-name checks. The original durable handoff was
explicitly reopened without rotating its identifier or URL. E52 ran as external
workflow `33910218160`; the human client passed, while the slow client's
concurrent view again rendered Guacamole's full connection home.

E52's preserved HAR proves the new path carried 36 successful Guacamole
requests, so this was not stale code or route selection. It also recorded 79
requests on the ordinary Guacamole path and a header-authenticated user resource
inside the nominal share path. The path removed ingress header injection but
did not isolate Guacamole's origin-scoped browser authentication state. The
dashboard's primary-owner API calls and prior iframe share cookies and storage
remain available to another path on the same origin. The visible full operator
home is therefore the same symptom with a narrower, now-proven cause.

Revised repair: move the capability-only Guacamole client to a distinct sibling
origin with no stable operator header. Keep primary-owner discovery and
credential creation on the authenticated dashboard origin, then navigate the
iframe to the sibling origin with only the one-time sharing key. This separates
both ingress authentication and browser storage. Remove the now-insufficient
same-origin capability route. The next external oracle must prove the sibling
origin was used, the concurrent marker remained correct, and a no-key client
cannot enumerate or open connections.

The sibling-origin repair was committed as Agent Browser `781967a4` and Cooper
`dbbb423`, installed as development generation `0.28.0-f9b8f97789db`, and
passed the three-launch smoke, provider doctor, trusted public TLS, and a fresh
no-key browser probe that exposed only the login surface. The original durable
handoff reopened directly through its explicit repair action and retained its
identifier and URL.

E53 ran as external workflow `33911361222`. The human client passed. The slow
client reached the sibling origin but rendered a blank iframe. Its HAR recorded
one successful sibling-origin document response and no sibling assets; direct
header readback found `X-Frame-Options: DENY`. The generic external
`security-headers` middleware was therefore blocking the deliberate cross-origin
embed before Guacamole could consume the sharing key. This differs from E51 and
E52: the full operator connection home did not reappear.

Revised next action: remove only that generic middleware from the capability
origin, leaving the authenticated dashboard origin unchanged. Republish and
prove the sibling document is embeddable, while a fresh no-key browser still
shows only login and no operator identity or connection list. Then rerun the
same two-client external oracle against the already-installed exact candidate;
no Agent Browser rebuild is required for this ingress-only correction.

Cooper commit `ffbe065` removed the frame-denial middleware only from the
capability origin. A fresh browser embedded that public origin successfully,
while its no-key view still exposed only login and no operator identity or
connection list. E54 ran as external workflow `33911906623`. Both distinct
off-host clients passed initial marker capture, concurrent viewing, exact
identity comparison, and durable-handoff reconnect. The aggregate sealed both
receipts with `success=true`, zero retries, and no repair inside the run. This
clears the E51 through E53 simultaneous-view blocker.

The first C01 calibration dispatch, workflow `33912440452`, then exposed a
harness-only timing defect before browser access. The shared start was three
minutes after dispatch, but the slow runner entered the probe with 104 seconds
remaining and failed a redundant requirement that two full minutes must still
remain after runner setup. The companion job was canceled immediately because
a valid two-client aggregate had become impossible; no 20-minute failed run was
allowed to continue.

The dispatch contract already requires the shared start to be at least two
minutes in the future. The calibration loop separately records readiness,
waits at the shared start, and rejects arrivals more than 30 seconds late. The
bounded repair removes only the duplicate post-setup two-minute check and keeps
the actual shared-barrier late-arrival rejection. A red source regression
reproduced the false rejection and now passes.

Revised next action: commit the timing-harness repair and redispatch C01 against
the unchanged ready handoff and installed candidate. Preserve both 20-minute
client action streams and their aggregate without retry or repair inside the
run; if either client reaches the shared barrier more than 30 seconds late, stop
and diagnose that distinct failure.

The corrected C01 dispatch, workflow `33913357430`, passed checkout, dependency
setup, external ingress, dashboard authentication, durable-handoff resolution,
and the first visible pixel capture on both clients. The slow client then opened
its second simultaneous viewer and received Guacamole's disconnected overlay
instead of the prepared marker. The human client's preserved initial screenshot
remained correct. Guacamole and guacd logs show that all four viewers were
admitted to the same underlying RDP connection, ruling out the configured
eight-viewer route limit. They then record a background viewer as not responding
and close its WebSocket session. This is a distinct long-lived concurrency
failure that E54's short readiness probe did not expose.

The slow runner models independent clients with two pages in one headless
Chromium process. Chromium can throttle timers and rendering in the page that
becomes backgrounded, which prevents that page from acknowledging Guacamole
protocol updates. The bounded harness repair disables background timer,
occluded-window, and renderer throttling for this external runner. It preserves
the product failure oracle and the true simultaneous viewer load; it does not
add retries or weaken marker comparison. The provider-free external-runner
regression now fixes the exact launch contract.

Revised next action: commit and push the external-client scheduling repair,
then redispatch C01 with the same durable handoff and unchanged installed
candidate. A further disconnect remains a campaign blocker and must be
diagnosed from the preserved screenshots, HAR, transport receipt, and provider
logs before another attempt.

Workflow `33914568752` proved that the background-throttling repair worked:
initial and concurrent marker captures passed, three viewers remained attached,
and the earlier not-responding disconnect did not recur. The slow client failed
later at the first synchronized reconnect. Both external clients closed their
current dashboard pages at the same scheduled instant. The primary Guacamole
connection disappeared between active-connection discovery and sharing-key
creation, which returned HTTP 404. The dashboard treated that expected
stale-primary race as terminal and removed the iframe, producing the preserved
`external_stream_not_embeddable` receipt and visible Stream unavailable state.

The product repair now treats two precise states as primary re-election points:
only shared children remain after discovery, or sharing-credential creation
returns 404 because the discovered primary vanished. In either case the
dashboard uses the already authorized direct frame URL to establish a new
primary. Other credential failures remain terminal. Red tests reproduced both
states before the repair; the focused dashboard sharing suite and production
dashboard build now pass without weakening the external marker oracle.

Development generation `0.28.0-faf7a6687342` contains the repair. The isolated
provider was restaged and applied after exact ingress preflight, the reviewed
dashboard and capability origins were republished, provider doctor passed, and
the three-launch smoke passed with production unchanged. The original durable
handoff was explicitly resolved after installation, returned to ready, and its
protected expected-identity binding was refreshed without disclosing operator
or provider URLs.

Revised next action: commit and push the stale-primary recovery, then redispatch
C01. Preserve the exact simultaneous reconnect schedule. If it fails again,
classify the next failure from the retained client and provider records rather
than adding a retry inside the run.

Workflow `33916185995` did not exercise the product repair. Both clients stopped
before page access with `handoff_target_closed_operator_action_required`. The
post-install recovery had selected the oldest handoff sharing the expected
browser and profile, but historical execution left several such handoffs. The
protected external URL names a different exact handoff. This was an operator
selection error in campaign preparation, not a product or ingress regression.

The exact handoff was recovered without exposing its identifier or URL by
matching the SHA-256 of each retained handoff URL against the hash sealed in
E54's green external receipt. Explicit reopen then returned `status=ready`,
`resolved=true`, `reopenedClosedTab=true`, and
`operatorVisible.state=ready`. The protected expected-identity secret was
refreshed from that exact retained record. Future campaign recovery must select
the sealed handoff hash, never infer identity from a browser/profile pair that
can legitimately own multiple historical handoffs.

Revised next action: redispatch C01 at the current exact commit and unchanged
installed candidate. The external URL hash remains the E54-sealed identity.

Workflow `33916458742` reached the exact sealed handoff but exposed a product
race before the shared barrier. With no active Guacamole primary, both external
clients independently selected the direct route. The second direct RDP
connection displaced the first, whose preserved screenshot showed the correct
fixture beneath Guacamole's disconnected overlay and the dashboard's viewer
ownership warning. Provider logs independently recorded the first RDP client
disconnect, a new direct client, the prior session's manual logoff, and a later
sharing join. The run was canceled because a valid paired calibration was no
longer possible.

Stale-primary recovery alone cannot arbitrate simultaneous primary creation.
The bounded repair adds an authenticated dashboard-local, ten-second primary
claim keyed by the provider route and connection. Claim mutation is atomic in
the dashboard server. When no primary exists, exactly one client receives the
direct route; contenders poll for its active Guacamole connection and then
request sharing credentials. An abandoned claim expires so a later contender
can recover. A disappearing primary during credential creation returns to the
same election loop instead of allowing two direct reconnects. Focused tests
cover claim exclusion and expiry, plus a simultaneous two-client resolution
that must yield exactly one direct and one shared result.

Revised next action: finish the Rust and dashboard validation gates, publish
and install the repaired development candidate, reconcile provider ingress if
the generation changes, resolve the exact sealed handoff, and redispatch C01
without staggering or weakening its shared-start schedule.

The primary-election repair was committed and published as `4e9e287d` after
the focused Rust exclusion/expiry test, dashboard sharing test, production
dashboard build, workspace clippy, Rust formatting, and the complete Plan 0158
provider-free aggregate harness passed. Development generation
`0.28.0-9c8935eb95b0` was installed, the exact reviewed provider binding was
restaged and applied, both Cooper ingress routes were republished, provider
doctor passed, and three disposable browser launches passed with production
unchanged. The exact E54 handoff was recovered by its sealed URL hash and
returned ready at a new presentation generation.

Workflow `33918919898` and the first follow-up readiness workflow
`33919334254` stopped before useful product execution because campaign setup
updated the repository-level `P158_DEV_EXPECTED_IDENTITY_JSON`, while the
workflow consumes the same-named secret from the `p158-external-vantage`
environment. The environment binding correctly took precedence and both
clients rejected the stale expected tab ID. This was a confirmed pre-effect
campaign-configuration error. The environment-scoped secret was then refreshed
from the current exact handoff resolution without exposing its contents.

Readiness workflow `33919582358` proved the corrected identity binding: the
human-paced external client passed. Its deliberately delayed companion then
rendered the correct retained browser through Guacamole but lost the shared
tunnel after the first client's primary connection closed. The dashboard
classified this simultaneous-view disconnect as a single-viewer takeover,
retained a visually stale iframe briefly, and then removed the iframe instead
of invoking primary election. The preserved failure is
`external_stream_not_embeddable`; its redacted HAR contains one Guacamole 404
and no leaked operator URL.

The bounded follow-up repair gives simultaneous-view disconnects a separate
three-attempt recovery budget and re-runs the existing server-arbitrated
connection resolver. A surviving primary is joined with a fresh share key; a
departed primary causes one contender to claim and recreate it. Single-viewer
takeover behavior is unchanged, and the recovery budget does not reset on each
iframe load.

Revised next action: validate, commit, rebuild, and install the bounded
disconnect recovery. Re-run the two-client readiness probe before redispatching
C01 so a departing first viewer and delayed second viewer are both proven.

The disconnect recovery was committed as `928b38bb`, built into development
generation `0.28.0-cb0e442bd453`, and installed without changing production.
The reviewed provider binding was restaged and applied, both external ingress
routes were republished, provider doctor passed, and the three-launch browser
smoke passed. Readiness workflow `33920592707` then passed both clients and its
aggregate: the first client could leave, the delayed shared viewer recovered,
and the same durable handoff remained usable.

C01 workflow `33920919559` completed the full synchronized 20-minute external
component of the calibration window at the exact frozen commit `928b38bb`.
Both distinct
off-host clients passed all 30 actions, including five durable-handoff
reconnects each. The aggregate receipt is `success=true`; both clients used
zero retries and attempted no repair inside the run. The evidence records zero
internal URL leaks, zero duplicate server browser launches, correct retained
identity, successful DNS, TLS, cookie, WebSocket, iframe, and reconnect checks,
and a passing visual oracle with no finding codes. This is retained as
successful external many-to-many and identity-continuity evidence, not a
complete C01 result: the simultaneous local half of 500 one-shot Service reads
across 25 agent identities did not run and cannot be reconstructed after the
shared window.

The passing oracle did not suppress diagnostic noise. The human-paced client
recorded 2,286 network entries, 159 console entries, four HTTP 404 responses,
one HTTP 403 response, and 21 HTTP 504 responses. The slow client recorded
4,269 network entries, 230 console entries, five HTTP 404 responses, two HTTP
403 responses, four status-zero requests, and 36 HTTP 504 responses. All 60
scripted actions still passed. Preserve these redacted records for W10 causal
classification; do not infer that the response codes are harmless merely
because C01 passed, and do not repair them during the frozen campaign unless a
later sequence cannot complete.

Artifact correlation classifies the four status-zero requests as canceled
page and reconnect work and all nine HTTP 404 responses as stale Guacamole
active-connection observations around setup, reconnect, or teardown. Three
Guacamole token HTTP 403 responses recovered without losing identity. The 57
HTTP 504 responses are different: they span session-tab, runtime-health,
legacy-session, Service status and resource, Service contract, and browser
capability-registry reads throughout the 20-minute window. All but one URL
identity later returned success, but this remains actionable dashboard
reliability degradation.

The receipt also contains 316 console errors. Four otherwise unexplained
message hashes account for 247 of them, but the redacted evidence cannot
classify their causes. Captured console and network arrays were not passed into
the external handoff oracle, and the captured console shape differs from the
dashboard oracle's input contract. The oracle therefore returned clean despite
the error traffic. Classify this workflow `complete_degraded` and repair the
oracle and evidence shape before repeating the synchronized full C01.

Revised next action: add the missing source-owned five-surface live journal
calibrator, W6 evidence projection and live-adapter assembler, and authenticated
E2 calibration preparation seam. Repair the external oracle's console and
network evidence wiring. After provider-free validation and development-only
installation, run one fresh synchronized C01 with both the external clients and
the 500-command local half, finalize its calibration artifacts, seal exact E1
and E2 identities, and write the zero-start campaign freeze. Only after that
checkpoint may W7 execution begin.

### W6 Calibration Evidence And Harness Repair

State transition: `w6_external_component_complete_degraded ->
w6_repaired_candidate_ready_for_publication`.

Acceptance state: W6 remains open. No replacement development candidate has
yet been installed from this source state, no fresh synchronized C01 has run,
and no zero-start campaign freeze exists.

The external runner now normalizes its console and network captures into the
dashboard oracle contract and writes a redacted oracle artifact. Unexplained
console errors, HTTP failures, and transport failures fail the oracle. Narrow
Guacamole setup, reconnect, and teardown noise remains recorded under explicit
classifications. Replaying workflow `33920919559` now rejects both client
receipts and preserves the exact 316 console errors and 57 HTTP 504 responses
that the earlier oracle omitted.

The new five-surface journal calibrator no longer treats a request-parser
rejection as a browser launch failure. Its live inducer invokes the exact
installed development generation with a deliberately unsupported engine. This
reaches `BrowserManager::launch` and fails before any browser process spawn.
The calibrator then requires two stable readbacks of exactly one matching
`browser_manager` failure followed by exactly one authenticated observation
for each of the other four named surfaces. The malformed-line check remains
isolated from the live journal and is candidate-bound.

The W6 evidence assembler now supplies exactly 54 source-bound case adapters
and 24 hook bindings. Cases without a separately frozen phase-specific live
bundle remain honest `explicit_blocked` zero-effect adapters. It also projects
two downloaded external receipts and their complete oracle reports into the
W6 external-vantage contract. The distributed C01 driver can authenticate E2
from ephemeral environment material or a private nonsymlink file without
serializing credentials or its session cookie.

The W9 provider-free harness delay was an implementation defect, not an epoch
wait. The 835-attempt run cloned its growing controller graph 842 times and
rescanned 15,955 logging expectations for every attempt. Indexed lookups and
four bounded snapshots reduce the focused run from about 200 seconds to about
20 seconds while preserving interruption, safety-stop, blocker propagation,
receipt, harvest, and evidence-sealing behavior.

Validation at this checkpoint is green:

- the full `pnpm test:p158-harness` suite passes under a directly observed
  process;
- the W9 focused test passes with exactly four controller snapshots;
- release fixture, Service API and MCP parity, generated client contract, and
  client type gates pass; and
- `git diff --check` passes.

Next action: commit this repaired source state, build and install a fresh
development candidate, run development doctor and launch smoke, and execute the
candidate-bound five-surface live journal calibration. Then schedule a fresh
synchronized C01 far enough ahead to start the 500-command local half and both
external viewers inside the same 20-minute window. Finalize those artifacts,
seal E1 and E2, and write the zero-start campaign freeze before W7.

### W6 Live Journal Calibration Unblocked

State transition: `w6_repaired_candidate_ready_for_publication ->
w6_live_journal_calibration_passed`.

The repaired source was committed as `4ee8534c`. Because the Rust executable
was unchanged, development publication retained generation
`0.28.0-cb0e442bd453` while installing the new dashboard assets. Exact-bound
development doctor and all three disposable browser-launch smoke iterations
passed with production unchanged.

The first live five-surface attempt exposed two harness defects. The induced
invalid-engine command correctly produced one exact `browser_launch` record,
but the Service control plane also wrote a legitimate companion
`service_action` failure. The harness incorrectly required the entire live
journal delta to contain only the target record. It now requires exactly one
stable engine-bound BrowserManager record, rejects missing or duplicate target
records, and retains other concurrent records as redacted background evidence.

The next attempt exposed a shared-profile dependency in the live inducer. A
client using the development pseudo-home inherited its default profile and was
rejected with `existing_session_profile_identity_unproven` before reaching
BrowserManager. The inducer now gives each one-shot client a disposable home,
removes inherited profile selectors, preserves the exact development socket
and ingress binding, and removes that home after the command. This reaches the
unsupported-engine failure without opening Chrome or disturbing a retained
profile. Provider-free tests prove both isolation and cleanup.

The live calibration then passed against the exact E2 development candidate.
It observed one correlated record for each of `browser_launch`,
`guacamole_load`, `handoff_link`, `cdp_stream`, and `dashboard_action`, with two
stable BrowserManager readbacks. It also retained the companion
`service_action` record as background evidence. The isolated malformed-line
seam used the same installed candidate for writing and readback while proving
that neither the production nor live development journal changed.

Revised next action: finish aggregate validation and commit this calibration
repair. Then schedule a fresh synchronized C01 far enough ahead to start the
500-command local half and both external viewers inside the same 20-minute
window. Finalize those artifacts, seal E1 and E2, and write the zero-start
campaign freeze before W7.

### C01 Predispatch Integration Defects

State transition: `w6_live_journal_calibration_passed ->
c01_predispatch_repair_active`.

The first synchronized replacement dispatch, workflow `33927882151`, stopped
before the shared start and before any campaign browsing action. Both external
clients retained `handoff_target_closed_operator_action_required`, zero
retries, and no in-workflow repair. The exact hash-selected durable handoff
still exists, but its browser target requires the already authorized explicit
operator reopen before the next dispatch.

Local distributed preparation independently stopped before writing its
preparation envelope. It first rejected an authenticated dashboard port used
as the unauthenticated E1 Service target. The correct E1 agent endpoint is the
development runtime lane, while E2 remains the authenticated public ingress.

After that configuration correction, preparation exposed a source-level
integration defect. The external workflow schedules 25 actions strictly
inside the 20-minute window by dividing it into 26 intervals. The distributed
validator independently reconstructed offsets with 25 intervals, placing its
last expected action at the window boundary. Both implementations passed
isolated tests while rejecting each other in the live seam. The validator now
uses the same 26-interval contract, and the external-runner test directly
passes a real workflow descriptor through distributed preparation so this
drift cannot recur unnoticed.

Revised next action: validate and commit the schedule-contract repair, then
explicitly reopen the one handoff selected by its sealed URL digest. Dispatch a
new exact-commit workflow with a fresh shared start, prepare E1 on the runtime
lane and E2 on external ingress, and start all 500 local reads at the same
barrier as both external viewers.

### C01 External Readiness Stream Defect

State transition: `c01_predispatch_repair_active ->
c01_external_readiness_repair_validated`.

The next readiness workflow reached the exact retained browser through both
external clients and rendered the expected browser pixels. Its strict oracle
correctly rejected the run because the dashboard also made repeated CDP stream
WebSocket requests that returned HTTP 200 and 502 instead of upgrading. The
visible RDP route itself was healthy. This was a dashboard defect, not
Guacamole unavailability and not harmless oracle noise.

The root cause was the global legacy stream synchronizer. It connected the
active daemon session's CDP stream on every dashboard page even when a durable
workspace route had selected and rendered an RDP or snapshot stream. The hook
now accepts an explicit enablement decision, tears down when disabled, and is
disabled whenever the durable workspace route owns the viewport. Native
fallback pages retain the existing CDP behavior. A source-level dashboard
regression binds the handoff route to this exclusion.

The same external receipts contained status-zero Guacamole asset requests
cancelled during page replacement and then successfully fetched milliseconds
later. The oracle now classifies such a request as expected lifecycle noise
only when the request is in the Guacamole transport class and a later 2xx or
3xx response for the exact URL digest and method proves recovery. Console
capture also records a safe message class and location digest. A matching
resource-load console error inherits lifecycle classification only from that
exact recovered network record. Unrecovered Guacamole transport failures and
all CDP WebSocket handshake failures remain actionable.

Focused durable-handoff, external-vantage, dashboard production-build, and
whitespace checks pass. Revised next action: commit and publish these dashboard
assets into the development candidate, rerun exact candidate doctor and the
external readiness workflow, then proceed directly into synchronized C01 only
if both external oracles are clean.
