# Native handoff browser mode adapter

Agent Browser#538. Source qualification is separate from installed acceptance.

Native Remote View serves the CDP switch from its existing fullscreen toolbar.
Authenticated native continuation adds the original `browser-handoff` locator
query to the selected desktop. This is a locator and grants no authority. Direct
slots and consumer grants do not gain browser ownership from this parameter.

The viewer posts `/api/browser-mode/<handoff-id>` on its own origin. The Native
gateway rechecks its authenticated operator, same-origin policy, a remembered
current page, slot and generation before making one private loopback owner call.
It forwards neither cookies nor browser-selected task, profile or browser IDs.
The existing configured owner token remains private. No POST is retried.

The owner callback is `POST /api/native-remote-view/<handoff-id>/mode` using the
existing private Native callback bearer and operator header. Cookies, transfer
encoding, duplicate lengths and bodies over 4096 bytes are rejected. The body
accepts only `slot`, `generation`, `choice` and, for an operation, `operationId`,
`mode` and `ownerConfirmed`. Unknown fields and malformed identities are refused.
`inspect` supplies no operation fields. Choices are the existing durable mode
operation choices, with confirmation requirements owned by that operation.

The owner reads the original logical session/tab, link retention and current
owned desktop route, then rejoins stable target identity after observation.
It derives task/profile/browser IDs and daemon lane from that retained target.
There is no daemon acquisition, fallback owner, new handoff or site replay.
Inspection returns `agent-browser.handoff-mode.v1`, verified physical `mode`
(`cdp`, `cdp_free` or `unverified`) and retained active public mode operations.
Mode mutations return the existing public operation or typed busy summary.

The browser never infers CDP-free from an absent endpoint. A review shows the
whole-browser impact; only explicit offered actions run. Lost replies retain the
original operation for reconciliation. Completion navigates the original Native
handoff. This contract does not introduce browser input authority, viewer grants,
profile resets, credential transport or new public Service request actions.


## Native human-control target readback

Inspection may additionally return `data.nativeControlTarget` for Remote View's
existing authenticated Native takeover and return flow. It is null when exact
current process or provider binding evidence is unavailable. Its schema is
`agent-browser.native-control-target.v1`, with `targetDigest`,
`processInstanceDigest` and `target`. The target uses the existing
`RemoteViewApplicationTarget` shape: assignment and registration IDs, lifecycle
desktop/generation and viewing desktop/generation. Digests are opaque SHA256
identities; no account data, process path, credentials or viewer URL is exported.

The requested viewing generation must match the owner-qualified provider target.
The original process identity must be available, freshly qualified as CDP or
CDP-free, and unchanged across the owner read. The logical target and provider
binding are rejoined after observation. Missing or changed evidence grants no
control. This response is private evidence, not an input permission: Remote View
must independently bind its authenticated operator, current exact page and
assignment, then use its existing Native input epoch and held-input arbitration.
Browser-selected handoff locators never become control authority. Unknown fields
remain rejected and existing mode-operation request semantics are unchanged.

The unbound managed viewer remains locked. The provider-side repair and gateway
input qualification are tracked by `CochranResearchGroup/remote-view#798`; this
owner response alone does not establish usable human takeover or installed proof.
