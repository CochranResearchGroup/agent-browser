# Task recovery source checkpoint for issue468

Product lane: PL-PLATFORM
Parent implementation: P296 / spec466
Worker branch: platform/task-recovery-468
Base: 42875e6291fb394c40d47278f7ba9520cf45fbd4
Status: worker implementation and exact-source synthetic browser acceptance qualified; ready for parent integration.

## Public seams and evidence

MCP browser_command task_operation and CLI mcp task-operation read one exact
retained original request without creating a transport or exposing its private
fingerprint. The first public JSON-RPC test failed because the action was
unsupported, then passed with the typed existing-owner aggregate and explicit
projection. Exact command: scripts/ci/rust-tests.sh --focused
public_original_task_operation_readback. Red5m29, green4m18, one case each.

The software helper getTaskOperation uses the authenticated read route and
rejects mismatched task/request identity without retries. Node protocol red was
the explicit unavailable helper; node scripts/test-task-operation-client.js
then passed encoded identity and no-retry cases. The coordinator applied central
MCP/catalog/HTTP wiring in this worker checkout.

## Durable recovery and physical proof

The existing BrowserSessionState owner aggregate now retains typed original task
operations before transport startup. A private 0600 HMAC key compares requests;
raw values, scripts, page bodies and results are not journaled. Missing keys do
not regenerate when original history exists. Public projections expose original
identity/status and proven screenshot references, never fingerprints or private
process identity. Auth and multi-step owner journals remain separate.

The original physical HTTPS red dropped one successful owner's fill reply. The
application independently counted two nonempty fills instead of one. Raw receipt
is user-scoped outside Git: recovery468-606d4eea-1c88-401c-8353-81e47acff5c8.json.
Setup-only TLS and profile failures were not treated as behavioral reds. Every
fixture run reported zero owned process residue.

Existing-profile adoption revealed a required local preparation: the public
owner already qualified the exact profile, kernel process and CDP endpoint but
discarded the returned immutable process identity. The new public model test
failed with missing provenance, then passed after atomic adoption retention.
The map binds profile/endpoint/process and never transfers lifecycle ownership;
missing/conflicting/stale provenance remains unverified. Original operation
snapshots retain their prior process identity through later replacement.

scripts/smoke-task-operation-recovery.mjs then passed against source candidate
SHA256 3d405be3ee6bbe3fe5333f31b9c02d8441a471de8d433bdd71aaf81f59c2804a
and reviewed Chrome SHA256
0ba9d6c54dbc1a46dfaa5f95d5975bc9b963030c8647880f5b6a537f8af7d2e8.
It used its own prelaunched existing-profile Chrome, disposable home/sockets and
synthetic loopback HTTPS origin, with a certificate exception confined to those
owned Chrome processes. No trust-store or production launcher change was used.

The own Unix socket proxy dropped successful owner replies for fill, action and
submission. Independent application counts were exactly one for each. Repeating
the original addressed request returned before transport; changed fill arguments
with the same ID conflicted without transport. The replacement client and one
exact owned runtime-host SIGKILL/restart retained the same logical task, physical
target, both unsaved drafts, original uncertain operations and actual captured
PNG/reference. A separate owned physical browser loss returned explicit
intervention and unavailable transient state without navigation replay. Fresh OS
census before and after showed no owned process residue. Raw acceptance receipt:
recovery468-f8672d04-b903-4780-bb95-0627ed06f79e.json, stored user-scoped outside Git.
Candidate bytes remained unchanged during that acceptance run.

## Final source and actual browser qualification

All final source gates passed: public original-operation and Native-policy failed
cases individually; full MCP151/151; focused host14/14; exact adoption-provenance
regression1/1; model manager49/49; both client protocol fixtures; JavaScript
typechecking through the reviewed docs TypeScript installation; cargo-safe fmt
check; strict workspace clippy with warnings denied; and diff hygiene.

The initial MCP149/151 run found a sealed unit archive cleanup requirement and
incorrect CDP-gate application to Native input. The owned unit cleanup now
unseals only its exact fixture directories. Native input is excluded from the
CDP journal, preserving its original owner/request context and Native workflows.
A provider-free case also checks that unscoped requests retain canonical task-ID
validation without acquiring the new receipt-ID rule. Missing original MAC keys
refuse replay and never regenerate; readback remains available. The dispatch
metadata says dispatchAttempted, without implying a completed physical effect.

A final exact-source candidate was rebuilt after those changes. Its SHA256 is
7e0f6e99398915328b642058e7cd8e1ba29e2df67e1c4c2c9db9a54148df9165.
The final extended HTTPS run passed, receipt
recovery468-52de9c00-4fbb-4570-b092-081220304ac3.json in the same user-scoped
receipt directory. Four successful owner replies were dropped: screenshot,
fill, action and submission. The lost screenshot reply left a produced PNG and
an uncertain original operation; repetition caused no transport and left file
hash/mtime unchanged. No successful artifact metadata was invented from that
lost acknowledgement. Fill/action/submission independently counted one effect
each. Original same-ID submission readback after the isolated host restart did
not create a replacement host. Explicit continuation then preserved task,
browser, target, both drafts, the acknowledged artifact and original uncertain
capture/submission references. Separate physical process loss required
intervention without navigation replay. Fresh OS census before and after was
empty; the private fixture home was removed and candidate bytes stayed fixed.

Final commands used the existing cargo-safe wrapper with an 8GiB admitted claim,
MemoryHigh7G and MemoryMax8G, preserving the configured host reserve. No Cargo
command bypassed admission. No installed runtime or shared browser was changed.

## Scope limits

Receipts apply to addressed MCP page effects only. CLI and authenticated HTTP
operation lookup are read-only; unrelated legacy/unscoped effects are not made
idempotent. Native input remains outside the new CDP journal and preserves its existing
Native owner/cooperative journal and original request context. It has no opaque
provider receipt in its current contract;
none is invented. An owner acknowledgement does not prove site completion.
Screenshot references attest original file production, not continuing availability.
Healthy continuation reads the current retained target; lost state is never
silently reconstructed by replay.

This is source plus actual owned synthetic browser evidence, separate from an
installed production candidate, private-provider authentication, Native viewer,
physical media, speech services and final478 acceptance. No broad code review or
postreview remediation has run in this worker. Memory disposition: forbidden,
because the delegated task explicitly excludes Graphiti writes. The parent owns
final program closeout.
