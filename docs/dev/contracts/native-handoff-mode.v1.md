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
