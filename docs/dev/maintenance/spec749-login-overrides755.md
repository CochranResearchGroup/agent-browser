# Continued login and scoped override capacity

Product lane: PL-PLATFORM
Work item: #755 under #749
Status: IMPLEMENTED, awaiting combined Rust and public adapter acceptance
Base: ca9ef0544

## Login continuation

Owner admission now checks observed supported control kinds rather than imposing
one username, credential entry, submit sequence for the lifetime of a task.
Each episode retains consecutively numbered acknowledged originals and exact
context. Any unresolved login input in any previous episode prevents new input,
including under a different episode ID. A new scoped episode is allowed after
prior input is settled; a changed current context requires a fresh episode and
observation rather than replay. Native account-match, filled-submit, frame,
process and ownership validation remain intact.

The existing task-operation capacity policy now accepts optional
maximumLoginSteps, default32. The limit is per episode and updates immediately
through task_operation params.operation=capacity with the other required policy
fields. Exhaustion names this management path and preserves acknowledged and
uncertain originals. Missing reliable clock progression is distinguished from
exhausting the input count.

The Native Saved Login client retains its three-control default plan but accepts
resolve.steps as an explicit runtime sequence of supported control kinds. An
acknowledged plan can be extended with allowAcknowledgedResume=true; retained
originals are verified and never dispatched again. At an unverified end of a
settled plan, the result includes a continuation method/arguments and instructions
to observe the next page and append supported kinds. The retained step plan must
match all acknowledged input prefixes. Uncertain prefixes cannot be promoted by
changing the plan or episode.

## Scoped override working set

browser_policy operation=override_capacity accepts taskId, expectedRevision,
actor, reason, scope=app or task, and capacity={activeLimit:256}. Default scope
is task; task-specific capacity takes precedence over app defaults. The configured
limit is an active working-set bound, not a permission ceiling. There are no
unused extension/ceiling fields: every archived grant still resolves under exactly
the same task/action/request/current-policy checks.

When that task's active grant set reaches its configured limit, old grants move
to the existing owner policy document's archive. This removes the hidden global
256-grant veto. The archive does not optimize serialized policy-document size,
and no such latency benefit is claimed. Capacity updates preserve existing
permissions and unresolved work. Effective permission-policy changes revoke
matching active and archived grants, including if an equal policy is restored
later. Immutable override IDs cannot be overwritten.

browser_policy status reports overrideCapacity policy/source, activeCount,
archivedCount, archivalPreservesAuthorization and requiresRestart=false.
operation=override_lookup with overrideId reads one original in the addressed
host/task scope and reports action, request, revision, actor, reason and its
active/archived/revoked state. Status and lookup require no mutation attribution.

## Evidence and remaining gates

Actual RED: node scripts/test-native-saved-login.js exits1 because an unverified
completed page has no executable continuation. GREEN: the same existing suite
passes all assertions and6node tests, including extending an acknowledged plan
from3to4inputs with no replay. node --check passes. Existing docs TypeScript
checkJs command for the client and declaration also exits0. Source rustfmt and
git diff --check pass.

The existing Rust Native-login journal test now covers configured cap3to4,
fourth-step continuation, settled episode renewal, original replay protection and
unresolved-input rejection even under another episode. The existing model policy
suite gains scoped archive/exact-permission/restart/revocation and live
capacity-only update checks. No Rust compiler or browser launch was started by
this worker. Coordinator acceptance must run these source cases and the public
policy adapter before closing #755. Shared help, README, skill, CONTEXT and docs
site changes remain coordinator-owned.

## Public override-capacity adapter acceptance completion

The existing `native_policy_override_public_mcp_preserves_exact_scope_and_refusal`
case now exercises `browser_policy` capacity/status/lookup through the real MCP
schema, workflow routing and authenticated owner socket. Its existing isolated
endpoint calls the production owner policy implementation and reopens the real
SQLite store for each policy request. No new harness is introduced.

Assertions cover app defaults and task precedence; unchanged effective
permissions during capacity-only updates; automatic archival; exact archived-ID
lookup after store reopening; authorized use of an archived grant for its
original request while a peer task is refused; and permanent revocation after a
real policy change, equal-policy restoration and subsequent capacity extension.
The preexisting Native effect-count and cross-adapter assertions remain intact.
Only one additional synthetic capture is admitted; denied or revoked grants add
no Native effect. This fixture substitutes only provider/process transport and
uses no browser or private account.

Source rustfmt and patch hygiene pass. This worker did not compile or execute
the extended Rust case; the coordinator will run it in the final admitted Native
validation. A source assertion is not an executed acceptance result.
Memory disposition: forbidden by delegated worker boundary.
