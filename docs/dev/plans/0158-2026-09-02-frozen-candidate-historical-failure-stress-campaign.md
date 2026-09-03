# Plan 0158: Frozen-Candidate Historical Failure Stress Campaign

Date: 2026-09-02

State: OPEN

Execution state: `w6_preparation_contract_complete_candidate_publication_ready`

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

The campaign is diagnostic. Once the candidate is frozen, it accumulates
failures without repairing them. Its final and last work unit seals the
evidence, reconstructs causal timelines, evaluates architecture and logging
completeness, and produces a prioritized remediation ledger. No product or
runtime repair occurs inside that review.

## Frozen-Candidate Contract

1. Select exactly one source commit, built binary digest, dashboard digest,
   installed generation, browser executable digest, runtime manifest revision,
   provider configuration revision, and external-ingress deployment revision.
2. Complete fixture creation, synthetic-site deployment, external runner
   provisioning, observability checks, and a single clean baseline before the
   freeze point.
3. After the freeze point, prohibit source edits, rebuilds, reinstalls,
   configuration rewrites, service remedies, incident resolution, garbage
   collection, retained-state pruning, route repair, profile repair, and
   unscheduled process termination until execution is complete and evidence is
   sealed.
4. Permit only effects named in the case manifest. Controlled browser crashes,
   supervisor transitions, route exhaustion, network degradation, policy
   mutations, eviction, and full shutdown use disposable isolated targets and
   are test stimuli, not reactions to observed failures.
5. Never retry a failed attempt opportunistically. Predetermined repetitions
   have distinct attempt identifiers and execute from their declared starting
   state. A pass after an earlier failure never erases the first failure.
6. Continue independent cases after a failure. Mark only cases whose declared
   prerequisite is lost as `skipped_blocked`, retaining the exact blocking
   case and state observation.
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

### E3: Production Read-Only Comparison

Freeze a redacted production incident and job sample before execution, then
compare campaign signatures with those historical records during final
analysis. E3 performs no launches, closes, reattachments, policy edits,
remedies, incident resolution, credential interaction, or provider
navigation.

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
semantic product recovery route exists. A separate service-host handshake is
still required to keep the exact isolated runtime alive while this manual
external action runs; the workflow does not start or stop that runtime.

### C: Combined Deterministic Pressure

Run the combined phases only after their declared prerequisites have terminal
results. They consume the state left by earlier cases rather than repairing it.

| Phase | Fixed workload |
| --- | --- |
| C01 calibration | 20 minutes, 25 agent clients, two external viewers, one controller, 500 service commands, 50 dashboard actions, and ten handoff reconnects |
| C02 burst | 100 agent clients, ten dashboard clients, maximum route occupancy, 2,000 service commands, 500 dashboard actions, 100 reconnects, and 20 controlled browser crashes |
| C03 generation churn | 25 scheduled supervisor transitions interleaved with retained-browser commands, dashboard use, and durable-handoff reopen attempts |
| C04 eight-hour soak | At least 10,000 agent commands, 2,000 dashboard actions, 200 handoff reconnects, 50 controlled browser crashes, and continuous resource and log capture |
| C05 24-hour handoff endurance | One unchanged durable handoff, 500 external reconnects, viewer and controller expiry, client restarts, and scheduled network profiles |

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
- artifact storage exceeds its reserved quota or the filesystem reaches 85
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
repair the tested runtime. Emergency host protection outside the controller is
reported as an external intervention and invalidates subsequent frozen-state
comparisons.

## Work Units And Dependencies

| Unit | Scope | Depends on | Exit condition |
| --- | --- | --- | --- |
| W1 | Freeze the historical failure registry, production-shaped redacted fixtures, candidate manifest, resource ceilings, and case dependency graph | none | Every known failure family maps to cases and evidence sources; no open-ended discovery remains in execution |
| W2 | Build the append-only controller, deterministic scheduler, artifact manifest, fault injectors, safety monitor, and result schema | W1 | Provider-free self-tests prove no overwrite, no opportunistic retry, correct blocked propagation, and reproducible seeds |
| W3 | Build the cross-surface logging auditor and synthetic sensitive-value scanner | W1, W2 | Deliberately missing, duplicate, conflicting, reordered, null, and leaking records are all detected |
| W4 | Build the external-ingress runner and durable-handoff oracle | W1, W2 | A synthetic good path passes and loopback, private, raw provider, wrong-browser, and duplicate-launch fixtures fail |
| W5 | Build dashboard truth and performance probes plus large synthetic fixtures | W1, W2, W3 | Left-rail bijection, warning-axis, deep-link, multi-client, accessibility, and performance probes detect seeded defects |
| W6 | Publish and install one isolated candidate, prepare E1 and E2, capture calibration, then freeze | W2, W3, W4, W5 | Candidate and environment digests are sealed; no test case has started |
| W7 | Execute A and X scenarios without repair | W6 | Every scheduled A and X attempt is terminal and raw evidence is append-only |
| W8 | Execute H and D scenarios exclusively through external ingress where operator-visible | W6 | Every scheduled H and D attempt is terminal; every visibility pass has external proof |
| W9 | Execute C phases, scheduled teardown, and evidence sealing | W7, W8 | Every manifest case is terminal, teardown is recorded, raw artifacts are hashed, and no further execution is permitted |
| W10 | Deeply analyze findings and publish the redacted review | W9 | Causal clusters, logging gaps, performance distributions, historical reproduction rates, architecture implications, and a bounded remediation backlog are independently checked and source-backed |

Critical path:
`W1 -> W2 -> (W3, W4, W5) -> W6 -> (W7, W8) -> W9 -> W10`.
W7 and W8 may execute concurrently only when their disposable Profile,
display, route, and external-client ownership is disjoint. The campaign
controller, evidence writer, safety monitor, and final analysis each have one
owner. No repair edge exists in the graph.

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
begins only in separately authorized successor work after W10 closes.

## Acceptance Criteria

1. The registry demonstrates closed-world coverage of every historical failure
   family named in this plan and links each family to executed or explicitly
   blocked cases.
2. One frozen installed candidate is tested without reactionary source,
   binary, configuration, harness, or runtime repair.
3. Agent-only, human-simulated external remote-view, display/supervisor,
   dashboard, combined burst, eight-hour soak, and 24-hour handoff endurance
   tiers all reach terminal evidence states.
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
