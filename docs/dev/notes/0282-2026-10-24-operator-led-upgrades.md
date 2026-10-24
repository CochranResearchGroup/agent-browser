# Operator-led upgrades, P282 / issue399

Product lane: PL-PLATFORM
Disposition: accepted-evidence
Owning plan or work item: P282 / CochranResearchGroup/agent-browser#399
Related lanes: P281, P280

Clients could remain locked out after an upgrade because persistent drain
metadata was treated as admission authority. PR397 removed the client gate.
This follow-through removes the installer and supervisor producers, marker
ownership checks and automatic marker-driven recovery. Old marker bytes,
including malformed files and directories, have no control-plane meaning.
Historical transaction receipts remain evidence; a new takeover archives the
previous record before publishing its own operation.

The operator controls valid requests on their workstation. Warnings may explain
consequences and remedies without vetoing the request. Optional
`install workstation live --json` reports current beliefs and uncertainty without
launching or mutating a browser. Full shutdown remains an explicit operator
choice; prior inspection and digest pinning are optional. Digests bind effect
targets rather than unrelated observation churn. Exact process checks and
competing-writer coordination remain execution integrity mechanics.

## Qualification in progress

Eleven takeover tests pass, including live and uncertain work, historical record
preservation and selected listener targeting among peers. Policy wiring and the
documentation build pass. Runtime adoption, replacement, workstation fixtures,
strict Clippy and exact installed qualification remain in progress. CI stays
disabled. No production shutdown, browser interruption or binary publication is
claimed by this source checkpoint.

The explicit operator rule is saved in user-scoped memory and queued to the
project memory group. Queue acceptance is not persistence or retrieval proof.
