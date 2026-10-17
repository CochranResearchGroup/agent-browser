# Rail speed and accuracy audit

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: [issue 301](https://github.com/CochranResearchGroup/agent-browser/issues/301)
Related lanes: P237, P240

Scope: the pending [PR302](https://github.com/CochranResearchGroup/agent-browser/pull/302)
rail, observed-workspace collector and existing dashboard read/cache path. This
extends the [cooperative presence receipt](2026-10-17-dashboard-handoff-polish.md).
Production installation, private-task recovery and TTL policy changes are excluded.

## Findings and repairs

### Workspace hook and read coordinator

- HIGH: Diagnostic resources blocked publication of live inventory. Status now
  publishes independently; resource generations reject late older responses.
- HIGH: A hung fetch could stop subsequent polling indefinitely. Shared reads now
  have an eight-second abort deadline; the original failed read remains observable,
  and a later read can start without inheriting the stalled flight.
- HIGH: Freshness depended on another render/poll and the client's wall clock.
  Owner observation tokens now have a local monotonic lifetime and an independent
  expiry timer. Repeated cached tokens do not extend it. The same rule drives rail
  membership and selected native-view preparation.
- MEDIUM: Seven-second polling and ten-second document/server caches could hide
  changes across multiple polls. The live summary now has two-second cache bounds
  at both layers and polls three seconds after completion. Diagnostic caches retain
  ten-second reuse. Hidden-document pause and explicit-refresh semantics remain.

### Browser collector and identity join

- HIGH: Fixed CDP ordering could starve later browsers when early probes stalled.
  Collection rotates its starting point and advances past its completed batch,
  preserving the 800 ms overall budget and four-client concurrency bound.
- HIGH: The display join lacked profile and active-session membership checks.
  Mismatched or inactive owner bindings now remain unverified, even when their
  process/tab observation fields claim live presence.

### Rail grouping and search

- MEDIUM: Each browser rescanned all tabs; managed sessions also repeatedly scanned
  the tab collection. Both paths now index once. Memoized grouping preserves row
  order, counts, closed-tab suppression and browser/tab search semantics.
- MEDIUM: Search copied embedded document payloads into searchable text. It now
  searches the displayed document label, task label and ordinary URL metadata,
  excluding data/blob/script payloads. A one-megabyte document regression covers
  this boundary.

## Measured results

| Measurement | Before | After |
| --- | --- | --- |
| Grouping 500 browsers and 10,000 tabs, 15-sample median | 49.42 ms | 1.76 ms |
| Tab identity reads in the indexed grouping test | browser-by-tab rescans | 37,370, bounded by four reads per tab |
| Live inventory while diagnostic resources are held | blocked until diagnostics completed | 277 ms from navigation in the synthetic HTTP browser fixture |
| Actual Chromium mount of 200 tab rows | no retained baseline | 277 ms |
| Actual Chromium search with 200 rows | no retained baseline | 31–46 ms across five search round trips |

The approximately 28-fold grouping improvement measures computation, not total
application latency. Browser timings include automation transport and use
synthetic inventory. Actual DOM rendering was measured at 200 rows; the 10,000-tab
case qualifies grouping only. Very large visible DOM lists remain an unqualified
scale boundary rather than an inferred performance claim.

## Validation

The seventeen-check batch passes: grouping/complexity and document-search tests,
real-hook HTTP refresh and deadline/freshness behavior, coordinator timeout/recovery,
actual Chromium rail and viewport workflows, handoff refresh and contracts,
workspace projection/context, inspector actions, generated client parity/types,
policy/guidance checks, and dashboard/docs builds. All owned browser process
identities are absent after the browser fixtures.

The HTTP fixture holds diagnostics while checking initial publication, stalls the
next status response while advancing elapsed time, and restores fresh evidence.
It sets the browser wall clock to 2040 and asserts that all requests remain GETs.
The collector's focused Rust lane covers stalled early browsers and later-group
progress. A separate Rust cache test proves expired summary eviction while keeping
warm diagnostic entries. Rust format and strict workspace Clippy qualify the
changed native surfaces.

The first batch remains a failure receipt: adding named fixture outputs exposed
an old first-output selector, and the new refresh test initially asserted a server
counter before request arrival. Named selectors and an explicit server-arrival
handshake repair those harness defects. The passing batch does not erase them.

No production binary, provider configuration, private task, handoff authority or
TTL cleanup policy was changed. The earlier native handoff issuance assignment
history defect remains a separate repair.


## Owner accounting follow-up

The user subsequently authorized diagnosis of internal TTL/reaper accounting.
This follow-up remains PL-BUGFIX under issue 301 and PR302. It owns only
handoff-observation error isolation in the session-host adapter and its actual
host/SQLite regression; P240 resource, quota and archive policies remain unchanged.
The effort ceiling is 45 minutes with two repair/review cycles, including validation.

An expired heartbeat is not sufficient evidence of incorrect retained accounting:
a valid handoff deliberately protects logical session identity. Physical presence,
logical retention, tab inactivity, pending-human protection and the rail's
30-second observation confidence window remain distinct. The original private
task's exact retention/cleanup cause has not been established by this fixture.

A separate defect is reproduced: the host refreshed one native handoff before
calling its TTL manager, and a provider error returned before any eligible cleanup.
The minimized regression uses two ordinary synthetic tasks, a pending-human hold
on one, explicit 30 ms tab inactivity on the peer, a lost native registration reply
and an exact SQLite read after reap at 130 ms. On unchanged source, the expired
peer remains in durable working state. With the repair, it is removed, the hold
remains and the original registration error is still returned. Existing manager
activity/readback and publication fences remain in force; a cleanup failure still
prevents a successful cleanup claim. No TTL values or retention eligibility change.

The initial fixture had an unsupported session name, corrected before product
attribution. A broader assignment-read failure also prevented safe cleanup under
the existing readback guard; that failure was narrowed to registration only,
rather than weakening the guard. These failed attempts remain in private logs.
The isolated baseline and candidate logs are `/tmp/rail-reaper-red-isolated.log`
and `/tmp/rail-reaper-green-isolated.log`. The isolated candidate passes.

Source quality checks and neighboring retention/restart checks are recorded in
the follow-up validation disposition below. No production install, account login,
private-task mutation, viewer grant or native provider mutation was performed.

### Follow-up validation disposition

Focused validation passes 45 Rust tests: the new actual-host/SQLite regression,
native registration/restart/generation/publication reconciliation, finite
pending-human expiry, retained-link recreation, two startup reaper tests and
39 help/output tests. Final Cargo format and strict workspace Clippy pass.
Remote View guidance checks, policy wiring and docs-site build pass. Patch hygiene
passes. The previous seventeen-check rail/browser/client batch remains applicable:
this follow-up does not change its collector, dashboard, schema or client code.
No new live/provider acceptance or production latency evidence is claimed.

Primary review found no additional Standards or Spec defect in the bounded
adapter change. It preserves observation error reporting, current cleanup guards,
CAS checks and pending-human retention. Observation errors can now be returned
after committed cleanup, which is explicitly documented in all user-facing guidance.
The broad installer fixture recommendations caused by touching shared CLI help
are excluded: no installer, provisioning, payload, asset or route code changed.
Shared installed guidance remains unchanged with the installed binary; source
skill parity is included in the pending candidate rather than published separately.
