# Operator-led upgrades, P284 / issue399

Product lane: PL-PLATFORM
Disposition: accepted-evidence
Owning plan or work item: P284 / CochranResearchGroup/agent-browser#399
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

## Source and exact-candidate qualification

The operator-rule checkpoint passes 220 workstation cases, 20 selected doctor
cases, 15 supervisor cases, 51 census cases, eight replacement cases, 11 takeover
cases and eight failure-recourse cases. Filters overlap. After incorporating
PR407's selected-dashboard proof, the combined 222-case workstation lane passes.
Strict workspace Clippy and formatting pass on the combined source. Policy
wiring, planning, changed links and the documentation build pass. Unchanged
focused results are reused within their exact source scope under policy0042.

Clean source4909ec1c9 builds production-shaped candidate
`candidate-69f0a2fc4bdd06f1-417d5886c68ec363`, binary SHA256
`417d5886c68ec36303499c4eed87b8be2505f6b87705ee7d1a83b01bea96ca19`.
Its release profile uses full optimization and fat LTO. All562 recorded executable
inputs remain verified. Both the deprecated error literal and marker filename
are absent from the binary.

The exact binary passes the complete source-free installer fixture with absent,
malformed, valid and directory-shaped markers; existing marker bytes remain
unchanged. A source-free apply accepts its payload without claiming live host
services. The supervisor takeover fixture passes with retained malformed marker
bytes and no browser launch. The real Chrome fixture passes two sequential
graceful owner-exit cycles, preserving its browser PID, CDP endpoint, occupied
profile and continued work. This isolated fixture does not claim workstation-wide
shutdown or a change between binary generations. Fresh independent process
readback finds zero fixture-owned residue; inaccessible foreign metadata is not
claimed as inspected.

Initial fixture attempts exceeded the Unix socket path budget and are retained
as failed evidence. The corrected short namespaces pass. Intermediate failed or
interrupted tests and cancelled pre-freeze builds are not acceptance evidence.

## Installed boundary

The candidate is qualified for the requested preserve-mode repair. PR406 source
integration and installed qualification remain pending. An owned disposable
canary contains a verified unsaved edit for the installed continuity check.
Its task identity and runtime receipts remain private and outside Git. No
Native Remote View or cross-platform acceptance is claimed by these fixtures.

The explicit operator rule is saved in user-scoped memory. Memory disposition:
queued; terminal processing and grouped visibility are verified. Private memory
provenance stays outside the repository.
