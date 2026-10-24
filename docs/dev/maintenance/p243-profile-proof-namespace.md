# P243 / P232 maintenance: exact profile proof across user namespaces

Lane: P243-profile-proof-namespace
Branch: fix/profile-proof-namespace
Target: main
Base: 71863e094db33ea86b85c72c99bcce9e5c21ce83
Parent roadmap homes: P243 ordinary browser availability and P232 profile reuse.

The bounded outcome is to resume an existing task browser when Linux permits
process observation but denies descriptor-link observation across user namespaces.
Keep the exact profile database, executable, PID and boot/start checks. The
existing bounded user-manager observer may return a typed proof for the requested
canonical profile; it must not export descriptor paths or command-line arguments.

The source repair owns only the profile reuse and process observer seam and
focused regressions. The canonical root's unrelated authentication journey edits,
platform matrix checkout, enrollment checkout, existing browsers and installed
runtime stay untouched. Production qualification and serialized adoption belong
to the parent operator. This checkout is retained until the parent integrates or
archives its exact committed result. No new browser or authentication workflow
is authorized by this source packet.

Validation: a denied local descriptor observation must recover only through an
exact delegated proof. Wrong PID, reused start identity, executable or profile
must reject. Validate with bounded cargo-safe tests and an immutable candidate;
installed public workflow acceptance remains separate.

## Recovery checkpoint

The original profile-reuse seam failed with
`browser_session_profile_process_identity_unavailable` when every local
file-descriptor target was unavailable. The extracted production proof seam has
a captured failing test before enabling fallback and passes afterward. The
profile observer now uses the existing bounded user-manager transport and returns
only the requested canonical-profile match with exact process identity. The
consumer checks process and canonical-profile continuity after the response.
Focused Cargo qualification (four tests), candidate build, synthetic profile proof
and three public process-namespace cases passed. Independent review found no
blocker. A broader selection hit an untouched test stack overflow of unqualified
cause; no full-suite pass is claimed. Installed public acceptance remains pending.

## Follow-up: accepted recovery continuity

Branch: fix/accepted-upgrade-recovery. Reuse this admitted parent-lane checkout.
Preserve the exact prior supervisor acceptance checkpoint when a later resume
replaces the stop reason. The supported recovery command must requalify the
selected sealed candidate and current live host/dashboard identity and preserve
owned-drain compare checks. Permit an entirely absent presentation receipt only
in this recovery mode, matching ordinary post-commit acceptance; reject partial,
stale or invalid receipts. The ordinary unrelated-history readiness contract
remains strict. Source tests and an optimized controller candidate are required;
only the parent operator may perform production recovery or installation.

## Follow-up: explicit debug-port reuse

Branch: fix/profile-explicit-debug-endpoint, based on canonical 5f2cc772d.
After exact profile/process proof, preserve a valid DevToolsActivePort endpoint.
Only genuinely absent files permit the same observed PID's explicit nonzero
u16 debug port through existing joined-argv parsing and loopback discovery.
Present malformed files and unavailable/invalid argv must reject. Keep the
existing verified attachment's CDP PID and before/after process checks unchanged.
Finite acceptance is focused lookup cases and the existing wrong-PID attachment
regression. This source packet authorizes no production build or runtime effects.
