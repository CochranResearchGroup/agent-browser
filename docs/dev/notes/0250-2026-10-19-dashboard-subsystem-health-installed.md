# P250: Installed dashboard subsystem health

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: P250 / #317
Related lanes: P249, P248, UX301

## Installed identity and integration

PR318 is merged. Concurrent main work had already allocated P249, so the health plan was renumbered P250 while preserving the stale-session-host repair from PR321. Canonical source `4c3f44032cca5ff338f439c3b01ac649257b41fd` built one sealed production-shaped release candidate:

- Candidate: `candidate-f1c3dbd604dfccf7-81eea74b6d6d29d2`.
- Binary SHA256: `81eea74b6d6d29d292791731b20dae1665624acf209a29ee5828f6bead61f39e`.
- Embedded dashboard SHA256: `9718c5501c0be19c334f1ea8f8cf914031edd323d484f2219913670da4e53151`.
- Production transaction: `upgrade-7fd0c69c-a3f5-48df-a329-7038c22067c5`, terminal `ready`.

The exact sealed bytes passed development publication before supported production installation. Both install doctors pass. Three disposable development open, URL-read, close and residue cycles pass with production unchanged. Development guidance was synchronized only into its pseudo-home during qualification; installed shared guidance was synchronized after production activation, with prior bytes retained privately and additional installed files preserved.

## Browser workflow acceptance

A fresh authenticated browser reaches the actual development and production dashboard, verifies the served binary digest, opens Subsystem health by keyboard, reads both real health endpoints, closes with Escape and verifies focus restoration. There are zero cold-open diagnostic reads, exactly one runtime-health and one Native-slot read per opening, no recurring diagnostic reads after closing, and no page errors. Production responses from both endpoints are HTTP 200. Browser evidence and drawer-only screenshots remain in private user-scoped receipts. Human-use acknowledgment was intercepted, so these checks do not consume pending-human retention or prove that endpoint.

The source projection, mounted real compact-workspace fixture, production stylesheet, mobile bounds and timeout-to-unverified checks remain qualified. Combined docs build, Rust format and strict workspace Clippy pass after reconciling main. The unchanged host repair retains PR321's provider-free evidence; the three installed browser cycles provide additional current-runtime proof. The older compact-workspace test's historical explicit-focus assertion still fails on both changed and untouched baseline; it is not counted as passing or silently repaired here.

Production and development validation preserve all 64 observed Native desktop process births, five observed owned-browser process births, active profile identities, 158 retained handoff identities, protected configuration hashes and Native generations. No desktop or browser restart was used. Fresh OS and modeled-state readbacks are separate receipts; no broad cleanup was performed. The primary dashboard checkout and pinned sealed candidates are retained.

## Exposed dependency failure and bounded recovery

The live indicator initially reported Native readiness unavailable. Pre-development and pre-production snapshots independently show the same `verification_slot_link` failure, so it was not introduced by dashboard activation. Ubuntu's Docker executable had disappeared again and its socket timed out, while the Windows Docker client and engine still responded.

The previously qualified private Windows CLI bridge was restored only to the Native verifier's PATH, followed by a verifier-only restart. It restored 16 verified Native desktops. Docker, desktops, browsers, authentication and ingress were not restarted or reconfigured. Subsequent live drawer checks move from unavailable to attention while keeping remaining unavailable-desktop and census findings visible. One sample also exposed a transient browser-inventory warning; current workspace reads and protected identities still pass. This is not an all-subsystems-healthy claim.

The Docker/WSL integration regression remains unresolved. The verifier workaround is a mitigation, with an exact private rollback receipt; remove it only after native Linux Docker and SQL verification pass. Full Native-provider health, receiving-account acceptance for P249, the historical compact test and broader dashboard work remain separate from this installed health-control outcome.
