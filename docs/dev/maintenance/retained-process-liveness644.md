# Retained process liveness before Remote View operations

Product lane: PL-BUGFIX. Work item:644.
Branch: `fix/retained-liveness644`. State: OPEN. Owner: ecochran76.

Repair signed retained-return preflight using an exact observation-only baseline.
Ordinary browser_is_live and effectful operations retain their operation guard.
The dedicated initializer validates the exact persisted browser, initializes
only its local verified-attachment binding and preserves peer bindings. It does
not settle activities, reconcile retirements, advance pending mode, or call the
Remote View provider. The observation scope admits only exact liveness and the
independently signed fresh-URL read; launch, close, attachment and other effects
remain denied. Both preflight and execution use this scope.

Source custody: exclusively borrowed existing630 checkout, reanchored from
fa1de16 to canonical2830a7f; no new worktree. Root owns integration, candidate
build, installation and actual public return verification. No runtime effects,
credentials, browser input, mode actions or memory writes are authorized here.
This is separate from Contacts' designated review and findings TDD.

## Regression evidence

The concrete configured Remote wrapper reproduced the missing-baseline error
before repair. RED receipt: /tmp/agent-browser-retained644/job-20261111T213446Z-d0fc74710850
(one failure, no passing tests). A neutral temporary liveness hook initially
exposed the call chain and was removed: a process-only answer cannot restore
the verified manager needed by the later URL read. Generic operation setup was
rejected because it can settle persistent activity and retirement state.

The final regression retains the original general-liveness guard assertion,
checks exact observation liveness and effect refusal, and exercises the full
signed return chain through configured Remote effects with a pending mode
operation. Peer-binding tests reject changed process/profile/endpoint/target
identity without modifying peer caches. Existing signature, original-process,
target and post-observation checks remain authoritative.

CodeGraph was initialized as an ignored derived index under operator discovery
instructions. Canonical policy fallback was used when governance MCP was
unavailable. Hosted CI is excluded under the operator cost policy. Local final
qualification is recorded below when terminal; source tests do not establish
installed acceptance.

## Local qualification

Exact CLI test executable SHA256: `6cf9a332e070f7865349d2ee98551d4fe8659db8a2a4ad4155a58eed66bac422`.
Thirty-six distinct focused tests passed, with zero failures or ignored tests:
configured Remote liveness and effect guard; peer identity/cache preservation;
full signed Remote and local return during pending mode conversion; retained
app-attachment baseline; host-control unit/integration; public standalone MCP
return identity/epoch races; and fresh-URL recovery refusal. Overlapping Remote
filters are counted once. The source test executable is not an installed
candidate and no production browser was touched.

No-run compile receipt: job-20261111T215544Z-7842d7fffdf3, 149.544 seconds,
zero compiler diagnostics. Initial partial-assembly/fixture compilation errors
are retained separately and do not count as runtime regressions. Candidate
architecture, policy wiring, Remote View handoff documentation and links in four
changed documentation files passed. Strict workspace Clippy and formatting
results are recorded at final freeze. Hosted CI was not run under cost policy.

Memory disposition: forbidden; delegated source-only packet leaves durable
closeout and installed acceptance to the parent lane.

Final-source no-run job-20261111T220028Z-2936cfcb53be passed in236.189seconds
including Cargo lock wait. After mandatory help-only documentation completion,
seven key tests were repeated successfully: full configured Remote signed
return, concrete liveness/effect guard, peer-binding conflicts, and four public
standalone MCP return cases. Final executable SHA256:
`45a08545b6f946301603d246f99e666148b289db8014ed0aa457faded48c3ffd`.
The original36-test packet precedes only the help text change; the repeat
qualifies the final source. Root independently owns issue645's scalar-schema
integration harness extraction in separate files.

Exploratory strict workspace Clippy with all test targets failed on existing
unrelated test-style lints and the separately tracked645 integration harness
compile error. That broader exploratory command is not the repository-normal
required Clippy command; no broad unrelated lint cleanup is included.

Required final strict check `clippy --workspace -- -D warnings` passed:
job-20261111T220338Z-3b5b57825a61,121.358seconds including lock wait,
zero diagnostics. Final formatting check passed. Root skill remains9949bytes;
all six candidate contract literals are retained. Five changed documentation
files have zero link findings. Source is qualified; installed acceptance remains
open and belongs to the parent lane.
