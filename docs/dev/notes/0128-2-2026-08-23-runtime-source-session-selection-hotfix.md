# Plan 0128 Runtime Source Session Selection Hotfix

Date: 2026-08-23

Branch: `hotfix/runtime-source-session-selection`

State: `SOURCE_ACCEPTED_INSTALL_PENDING`

## Failed Install Evidence

The exact merged-main candidate had SHA-256
`7d19b21c7801bbed90ca398967662b4e3fbf121c851bcbaef27e93d376cc583d`.
Transactional install failed before candidate activation in transaction
`upgrade-56f5c32a-d939-4610-a299-fee113b5571e` with:

`Cannot prepare runtime handoff for session 'last30days-facebook': browser PID is unavailable`

The old generation remained selected. Supported workstation reconciliation
removed the failed candidate generation and did not terminate a live browser
or mutate provider content.

## Diagnosis

The installer correctly selected the current browser-bearing owner route
`last30days-facebook--last30days-facebook`. It then probed a retained
historical alias bound to the same logical browser. That alias shared the live
runtime host but had no browser PID. The installed compatibility response was
an error instead of the newer structured no-browser result, so alias fallback
aborted after the exact primary had already prepared successfully.

## Repair

The installer now recognizes the exact legacy `browser PID is unavailable`
diagnostic only for `handoff prepare`. It may retire that result only when all
of the following are true:

- a valid browser-bearing primary has already been selected;
- the diagnostic came from a different alternate session;
- the normal supported idle-daemon retirement surface accepts the alias.

The diagnostic remains blocking for a primary route, for handoff resume, and
when no browser-bearing route has been selected. Conflicting browser-bearing
aliases also remain blocking.

## Validation

- all seven runtime handoff preparation regressions passed;
- the failure-classification regression passed;
- all 90 workstation installer unit tests passed;
- canonical Rust formatting, strict Clippy, and diff hygiene passed;
- workstation install and host-provision fixtures passed;
- fresh-workstation VM harness passed;
- Guacamole asset and PostgreSQL durability contracts passed;
- route-specific RDP and Guacamole user synchronization passed.

## Runtime Boundary

No live browser, profile, provider, supervisor, installed generation, or
accounting state was changed by this source repair. The next runtime action is
another transactional install from the exact integrated candidate. It remains
subject to a fresh doctor and runtime-census admission check.
