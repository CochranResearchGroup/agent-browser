# CDP-free private password paste

Product lane: PL-AUTH
Spec: CochranResearchGroup/agent-browser#786
Decision: ADR 0004
State: IN_PROGRESS
Integration branch: auth/spec786-integration (reused merged semantic-browser-learning checkout)
Comparison base: 551e99fdd4e07b3d05556cb441c435d935d12154
Integration tip: 551e99fdd4e07b3d05556cb441c435d935d12154

## Execution ledger

| Ticket | Blocks | State | Owner | Writes and validation |
| --- | --- | --- | --- | --- |
| #787 Synthetic private paste | #788 | validated | Primary | Remote View owns private delivery and clipboard isolation. Auth Receipts owns approval and custody. Agent Browser owns the task seam. Validate on the existing synthetic native browser path. |
| #788 Paste recovery | #789 | running | Worker after #787 validates | Same-request outcome and cleanup recovery through the existing workflow. Validate proven no-paste and uncertain delivery. |
| #789 Synchrony acceptance | none | pending | Worker after #788 validates | Verify stock Chrome CDP-free, one authorized attempt and the intended account. Retain a provider failure as a distinct outcome. |

Tracker parent and native edges verified. Review and TDD remediation passes
remain at zero. No source checkpoint, runtime change, or tracker closure.

## Admission and provider seam

Baseline readback pinned Remote View at 8b2fc487 and Auth Receipts at 06215c4.
Remote View already implements an X11 text paste using a temporary selection.
That path accepts text from ordinary input and answers the first supported
selection request. A credential path needs a private Auth-to-owner operation,
requestor binding and clipboard sharing isolation. Reuse the native control
and selection mechanics after adding those protections.

Reconciliation reduced the registration count from 24 to 17 by backing up
and pruning metadata for seven absent paths. Their branches remain available.
The clean semantic-browser-learning checkout was reused after its branch tip
was verified merged. The dirty Contacts and canonical checkouts were preserved.

Remote View provider checkout is on feat/private-native-paste-786 from
8b2fc487d611c6d686e691dcc04a970b82c1ee9c. The primary owns the provider
change. The first in-progress diff adds an X11 server-verified recipient
restriction to the selection transfer path. The existing ordinary paste
still uses its previous mode. The private transport and Auth composition
remain pending. This is an uncommitted checkpoint.

The Remote View provider now exposes an internal Rust owner operation. It
holds Native input admission for the expected epoch and desktop generation,
checks exact kernel process identity, and invokes only the X11 credential
path. The latter checks focus before delivery and uses the X server's process
identity for the clipboard recipient. It verifies selection ownership was
released after the attempt. Cargo check offline passed. No transport exposes
this operation yet.

Remote View now has a loopback-only HTTP paste route with a dedicated service
token. A durable metadata-only attempt record is written before dispatch;
the same attempt returns its recorded disposition on readback. The Auth
transport validates the exact receipt binding and never returns password
bytes. Owner composition, viewer clipboard isolation, and synthetic journey
acceptance remain pending. Auth source checkout reuses the merged recovery
worktree on fix/cdp-free-private-paste-786 from 06215c4.

Auth now has a managed native-paste receiver composition. The receiver keeps
the original request binding, invokes the existing authorization callback
before the private paste, and records acknowledged input separately from
submission. CDP remains the existing route for tasks identified as CDP.
The production proof resolver and owner configuration are still required.

Agent Browser now resolves a metadata-only native paste target through the
exact retained task, process/window join, and Remote View current agent epoch.
The Auth production resolver uses this action when the private paste endpoint
and token file are configured; qualified CDP tasks keep the existing route.
Agent Browser workspace check and Auth source check are running. Runtime
composition and the synthetic journey still need validation.

Remote View now holds the X server exclusion interval across each private
focus-to-shortcut and recipient-to-property operation. Non-browser selection
requests receive no private clipboard contents. Remote View offline check
passed after the change. Auth source check passed, and an isolated loopback
transport probe sent one synthetic paste and read back only its disposition.
Neither check proves stock Chrome input or the full clipboard-sharing
lifecycle. Remote View candidate build and Agent Browser workspace check are
running.

The Remote View offline candidate binary built successfully. Its actual
loopback HTTP route rejected ordinary operator authority and an invalid
token, accepted the configured private credential-service token for
request validation, and returned no password in errors. The probe used a
disposable provider-free owner. No private paste was dispatched.

Agent Browser workspace check passed with cache disabled. Remote View
stock Chrome X11 test executed once and was RED: the owner returned
success but the page did not observe the synthetic input. The revised
test explicitly clicks and positively confirms password field focus.
The credential path now waits for the recipient's X11 property
acknowledgement before releasing the selection. Revised isolated test is
running. The ordinary text paste keeps its previous receipt behavior.

The isolated Xvfb/Openbox test now passes. Earlier runs showed that a single
X11 property acknowledgement did not prove page input. The revised private
selection serves bounded reads from the same verified browser process for
one shortcut, then releases the selection and erases any outstanding
transfer property on failure. The real stock Chrome password field
reported the exact synthetic value. The current Agent Browser debug
candidate also built locally. Full Auth-to-browser synthetic sign-in
remains pending.

One focused Auth public-seam test passed. A cooperative request used the
normal parent decision, returned the approved username, made no secret
read before authorization, delivered one private synthetic password,
reported input acknowledged and submission not attempted, and reconciled
the same attempt without a second read or dispatch. The isolated full
native workspace harness is available; it currently launches CDP tasks
and must use a CDP-free synthetic task for the cross-service run.

The full synthetic composed runtime is executing in the existing private
Remote View workspace harness. Preflight found an occupied Docker subnet,
then a too-long Unix socket path under the first results root. Both runs
were cleaned up. A short-root run reached real Agent Browser startup and
reported a mode conflict because the helper opened an automation session
before requesting CDP-free mode. The current run opens directly in CDP-free
mode on an unused subnet and exact provider candidate.

The isolated composed run reached a live CDP-free native window and resolved
its target on the owner socket. The Auth-side MCP call timed out before the
credential store opened or a private attempt was recorded. Fixture cleanup
confirmed no owned process, Docker container or network residue. A fresh run
is observing the metadata call with a larger isolated harness timeout. Both
owners passed strict Clippy. Existing Auth and Remote View full suites are
running in parallel with the composed check.

The composed Auth call's original MCP error is now captured: the read-only
native paste action was absent from the MCP parameter catalog. Its empty
schema was added, and the exact Agent Browser candidate is rebuilding.
The isolated prior run left no managed processes or Docker resources.

Exact candidate rebuilt with the MCP catalog fix. The isolated composed run
now uses existing Native input for username, password focus, and submit,
with a page-observed focus marker before Auth invocation. Remote View's
full suite reported 387 passes, two viewer tunnel timeouts, and 34 opt-in
skips. Each timed-out case passed individually on the same test binary.

The isolated composed stock Chrome journey passed after the fixture proxy
advertised its existing GET desktop input policy route. Agent Browser
performed native username entry, confirmed password focus and submitted
only after Auth's receipt. The normal Auth parent decision produced one
secret read and one acknowledged private paste with submission not
attempted. The synthetic page verified the intended account after the
separate native submission. Cleanup found no owned processes or Docker
containers/networks. This run did not attach a viewer or use live credentials.

Stock Chrome delivered the exact synthetic password while a separate X11
client requested the private selection and received a refusal. The selection
was released afterward. This tests clipboard isolation at the display
boundary. Both earlier viewer tunnel timeout cases passed independently.
Provider checkpoint: f1e793d91fb9e4df3838e615d565477d956b8545.
Auth checkpoint: 6251a49bfa3981636f5bf05638b721695d885cc4.
The synthetic journey is validated locally. Tracker closure remains separate
from publication and integration on the component main branches.
