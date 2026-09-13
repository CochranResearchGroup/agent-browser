# Runbook History | 2026-09-10 Through Turn 309

Archived from the active runbook during Turn 310 reconciliation. These entries
preserve superseded execution evidence and are not current authority.

## Turn 309 | 2026-09-12

Plan0175 PR57 reran after the MCP smoke correction. Its fast gates and
Workstation Fixtures passed, while the comprehensive Rust lane exposed two
unrelated nondeterministic process-identity fixtures: shell exec changed the
captured executable in one case, and immediate post-spawn capture returned no
identity in another. The fixtures now use stable executable identity and a
bounded publication wait. Both focused tests, 20 repeated executions, format,
and strict Clippy pass at source checkpoint `a7c1b637`. Merge and install remain
withheld pending a fresh green PR57 head.

## Turn 308 | 2026-09-12

Plan0175 correction PR57 passed every fast gate except the final Rust no-launch
smoke phase. That phase returned the intended ordinary Google posture
`unknown`, `attachable_ok`, and `not_required`, while the JavaScript smoke still
asserted the former detached-login values. The aligned no-launch smoke passes
locally against the qualified candidate. Merge and production installation
remain withheld until a fresh PR57 head passes every required check.

## Turn 307 | 2026-09-12

Plan0175 merged through PR55 as `97aa399a`, but its post-merge comprehensive
Rust gate exposed three stale MCP readiness fixtures. Installation is withheld.
The bounded correction makes ordinary built-in Google resource expectations
`unknown` plus `attachable_ok` and preserves detached handoff coverage through
an explicit `requiresCdpFree` fixture. Four focused MCP tests, formatting and
strict Clippy pass. Production remained on binary SHA-256 `cae894cf25f2` with a
ready singular supervisor. Doctor separately reported one live retained browser
identity missing a lifecycle cleanup record, which transactional preflight had
to reconcile without blind closure or relaunch. The correction was committed at
`0572b065` and ready for protected CI.

## Turn 306 | 2026-09-12

[Plan0175](docs/dev/plans/0175-2026-09-12-stealth-routing-and-login-posture-repair.md)
was source-qualified at `27ec5aae` on `fix/plan-0175-stealth-routing-auth`, with
integration-ready custody. The bounded batch preserved explicit nested
browser-build selections, routed built-in Google and Gmail sign-in to headed
stealth Chromium, and retained detached CDP-free login only for
`requiresCdpFree`. It authorized no rename, install, launch, sign-in, profile,
or provider effect. Focused tests, formatting, strict Clippy, selected contracts
and fixtures, the docs build, and the exact branch-binary workstation fixture
passed. Production installation and live account acceptance remained separate.

## Turn 305 | 2026-09-12

[Plan0174](docs/dev/plans/0174-2026-09-12-ci-production-scale-and-process-identity-reliability.md)
is CLOSED. PR54 merged the CI repair as `3f680a13`; first-attempt run
`34729883713` passed every required check, including comprehensive Rust.
Reconciled P171 cleanup PR51 merged as `00796d5c`; exact-head run `34731270343`
also passed on its first attempt. GitHub reported no `main` branch protection or
repository ruleset, so its auto-merge request merged PR51 before checks
completed. The completed green run is post-merge evidence, and missing
enforcement remains a repository-settings follow-up. No stale-process cleanup,
install, or runtime effect occurred.

## Turn 304 | 2026-09-12

[Plan0173](docs/dev/plans/0173-2026-09-12-expired-session-retained-browser-reuse-repair.md)
is CLOSED. PR52 merged as `2156aaad`; exact binary `cae894cf25f2` installed as
generation `0.28.0-cae894cf25f2-a4332e7facd9` through accepted transaction
`upgrade-e371f1fa-80a6-485a-a3dd-2e2c6e785454` revision 13. Installed
acceptance proved stale same-principal rejoin, exact retained-browser reuse,
canonical valid handle, page probe, verified exact-tab release, 16 surviving
peer tabs, and Chrome PID 49619 continuity. BILL itself redirected to login; no
provider or accounting mutation occurred. The closeout retained a separate
shared-tab cleanup-policy projection-label mismatch for bounded follow-up.

## Turn 303 | 2026-09-12

Plan0171 is CLOSED through PR43 at `40da27f7`; final CI `34707619741` passed
after Cargo-target isolation repaired a parallel runner race and the no-launch
MCP inventory was synchronized. Exact SHA-256
`2c185ec7ccd691deaf0ee59412d3d31485ab0d8ad464cc02775785fb81be621d` was
installed as `0.28.0-2c185ec7ccd6-f318ad66074f` through transaction
`upgrade-1fcb7d57-671f-4cf9-afc6-1bcb45293e1a`. No browser, profile, provider,
route, cleanup, or unrelated-worktree effect occurred.

## Turn 297 | 2026-09-11

[Plan0168](docs/dev/plans/0168-2026-09-11-worktree-branch-review-merge-and-closure-campaign.md)
is CLOSED. It closed integrated and remote-only custody, integrated two P0240
residuals through PR30 at `de614fbe`, found no P157 residual beyond `ad673377`,
and merged its record through PR31 and PR32. The canonical Rust edits remained
attributable Odollo custody. No runtime or provider effect occurred.

## Turn 296 | 2026-09-11

Plan0167 is CLOSED through PR29 at `14fb3db0`. Its 9,642,672-byte
two-writer/two-reader regression completed in 709 to 739 ms across six samples
with zero commit wait and 358 to 380 ms exclusive holds. The one-second
deadline was unchanged. Required gates passed. Production installation, shared
skill publication, installed doctor, and X evaluation remained separate.

## Turn 294 | 2026-09-11

Plan0166 is CLOSED via PR27 at `6030c71b`. Git custody was reconciled; client
EOF releases exact connection custody while accepted work finishes
independently. Provider-free gates passed; P0240 and P157 remained preserved,
and Plan0162 was unblocked.

## Turn 291 | 2026-09-10

The Last30Days degraded tick added A1 and AX cases. LinkedIn's lock timeout
stopped before effects and retained `no_effect` plus holder context. Reddit
created tabs, then redundantly enabled CDP domains. Five historical job lookups
were lost, so AX required durable read-only lookup. `last30days-facebook`
retained its owner and profile but no locks or process; Plan0161 W2 owned
preserving repair. Recovery r348638 was uncertain; diagnosis r513089 proved no
launch. Only Last30Days could authorize a later tick. A3, BILL/QBO, and PID45449
remained untouched.

Production had three Guacamole routes but two admitted capacity slots;
selection exposed `presentation_bound_slot_missing`. Plan0162 adopted
Plan0124's arbitrary-N model: three warm desktops, logical-browser allocation,
and just-in-time multi-viewer streaming. Plan0161 repair remained the critical
path; Plan0162 then supplied Plan0160 A2 and A3.
