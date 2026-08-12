# P110 PoC 1 Display-Bound Frame Capture Source Acceptance

Date: 2026-08-12

Status: SOURCE ACCEPTED | NO LIVE OR INSTALLED PROOF

Plan: `docs/dev/plans/0110-1-2026-08-12-p110-poc1-display-bound-frame-capture-plan.md`

Accepted commit: `853c2d90`

## Outcome

PoC 1 adds one canonical queued `desktop_capture` action. A caller supplies a
service-owned `browserId` and optional `sessionName`; the service resolves the
current browser, RDP stream, route, display allocation, geometry, and
operator-visible proof. A successful immediate response carries typed desktop
context, a typed ephemeral frame receipt, and bounded PNG bytes. Callers cannot
select a raw display, provider route, URL, output path, or input behavior.

CLI, HTTP, generic and dedicated MCP, generated client, schema metadata, help,
README, repo skill, and service-mode docs share the same contract. Desktop
capture does not launch or attach a browser, use CDP, grant display access,
navigate, take over a controller lease, inject input, or persist frame bytes.

## Audit And Remediation

One independent broad audit found five actionable candidates. The primary
agent accepted the four routing, identity, visible-proof, and subprocess-bound
findings as blocking and accepted the attribution-doc finding for repair. One
bounded remediation at `853c2d90` resolved all five and added the missing
provider-free evidence identified by the testing agent. No second broad audit
was opened.

The remediation proves:

- arbitrary browser IDs do not select HTTP or MCP daemon sessions;
- route and display record IDs must agree with their authoritative map keys;
- attached-ready state without `browser_window_visible` evidence fails before
  provider capture;
- missing, ambiguous, terminal, replaced, or mismatched identity fails before
  provider capture;
- bounded provider readers drain stdout and stderr without pipe deadlock;
- missing executable, timeout, oversized output, invalid PNG, and provider
  failure retain stable typed errors without stderr disclosure;
- context ID, geometry epoch, dimensions, hash, byte count, retention, and
  persistence posture are deterministic against the pinned fixture;
- response-only pixels are removed from long-lived broadcasts and persisted
  job results.

## Validation Receipt

Primary-agent closed-world validation after remediation:

- `scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml`: passed;
- `scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings`:
  passed;
- focused `desktop_capture` tests: 26 passed;
- persisted desktop-pixel exclusion: 1 passed;
- `pnpm test:service-client`: passed;
- `pnpm test:service-api-mcp-parity`: passed;
- `node scripts/check-actions-architecture.js --check`: passed;
- `pnpm test:service-contracts-no-launch`: passed and exited cleanly;
- `pnpm test:wsl-cargo-safety`: passed;
- `pnpm --dir docs build`: passed;
- `git diff --check`: passed.

Delegated validation accepted after primary reconciliation:

- focused service-contract metadata: 1 passed;
- remote-view handoff: 46 passed;
- page screenshot: 30 passed and 2 ignored E2E tests remained unrun;
- source-only checks started no Chrome, RDP, Guacamole, real X display, or
  ImageMagick capture.

The full actions architecture wrapper reports a pre-existing exact-count drift
in an unchanged remote-view test file: nine tests exist at planning commit
`f693f82a`, while the checker expects eight. The action definition inventory
changed by this proof is green, so this baseline debt does not block PoC 1
source acceptance.

## Remaining Boundary

No live or installed capture has been proven. The production X11 provider
still depends on `xdpyinfo` and ImageMagick `import` being available inside the
authorized display environment. A later explicitly authorized live proof must
validate that provider and cleanup behavior without using credential-bearing
content. PoC 2 remains provider-free and observation-only.

## Next Recommendation

Write Plan 0110-2 and freeze deterministic fixture-location contracts before
implementing any locator. Do not add input, challenge handling, or live prompt
automation in that plan.
