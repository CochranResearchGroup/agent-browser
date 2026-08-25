# Plan 0131 Slice C Development Live Preflight

Date: 2026-08-25

Result: PASS

Authority after this record: DEVELOPMENT RUNTIME EFFECTS FOR SLICE C ONLY

## Exact Identities

- Source commit: `2aa499fbec9abdf296203050469e4af1c658b6a6`
- Candidate path: `cli/target/release/agent-browser`
- Candidate SHA-256:
  `de5888949c2bb9e25ee84f3f1fab1cabdbcdb07bab1ae651835ade32f29a37d3`
- Selected development generation before installation:
  `0.28.0-08de92737c24`
- Development command SHA-256 before installation:
  `803403c426c61ebf13df4d8afacfb286fc32670164064418c84383d41fc878b0`
- Selected production generation:
  `0.28.0-05d9da26035e-7fa3fbcb7248`
- Production command SHA-256:
  `05d9da26035e0e86b55d6b2beaed25ae6dfe45ee6eeb0aa14362ce4ec08b0d10`
- Development rollback selector:
  `/home/ecochran76/.local/lib/agent-browser-dev/generations/0.28.0-08de92737c24`

The release candidate was built through `scripts/ci/cargo-safe.sh` with four
Cargo jobs inside the repository WSL cgroup. The build completed successfully
in 17 minutes 5 seconds. No runtime selector changed during the build.

## Development Runtime And Provider

- `pnpm development-runtime:doctor` passed after synchronizing the repository
  skill into the development pseudo-home only.
- The development presentation provider is configured and ready.
- The provider status field `blocking=true` is the fail-closed provider
  contract. A configured provider always reports blocking so an absent or
  drifted provider cannot silently become optional. It is not a capacity or
  health failure.
- Fresh provider apply preflight is intentionally false because the provider
  manifest, ports, and containers are already installed and listening. No
  provider stop, recreation, or apply is required or authorized for this
  slice.
- Presentation capacity is non-null with four `warm_idle` slots, hard maximum
  six, pressure-admitted maximum six, one human-protected slot, one recovery
  reserve, and zero binding warnings.
- The four ready provider routes map to warm displays `:12` through `:15`.
- There are zero active viewer leases and no route is occupied by a browser.

## Process, Resource, And Retained State

- Development status reports zero GC candidates and zero candidate RSS.
- The one unowned but protected agent-browser process is the separately
  identified production runtime. It is not a development cleanup candidate.
- Development runtime multiplicity reports two generations because its
  read-only host census also observes the production dashboard generation.
  The paths and hashes above make this separation exact.
- One pre-existing development lifecycle record remains `closing` with an
  `owned` cleanup obligation:
  `session:development-presentation-provider-v1-2`, process group `70849`.
  The process group is absent. This historical record is the explicit baseline
  and is not authority for broad cleanup. Slice C must create no additional
  ambiguous cleanup obligation.
- Retained display records are diagnostic evidence. None is apply-safe, and no
  prune or cleanup effect is authorized by this preflight.

## Production Boundary

Production install doctor passed with service readiness, no-launch readiness,
zero resource candidates, zero legacy daemons, zero stale runtimes, and a
converged single production generation. Its configured desktop input remains
`unavailable_pending_plan_0110`.

Slice C may replace only the `agent-browser-dev` selector transactionally,
launch only the repository controlled fixture, and emit only the closed
pointer and keyboard recipe against an exact development route and controller
lease. Any production hash or generation change, ambiguous cleanup state,
missing capacity, or failed after-state verification requires development
rollback to the selector recorded above.

## First Live Attempt And Bounded Remediation

The initial candidate reached two fail-closed outcomes during the controlled
fixture proof:

- The fixture rendered its label over the exact target color, so the strict
  solid-color locator rejected the scene as ambiguous before any input.
- After the fixture target became one solid region, operation
  `p131-success-20260825-03` acknowledged 31 of 32 attempted event keys and
  retained one uncertain `key_down` record. The uncertain event was the hyphen
  in `fixture-ready`. The X11 adapter admitted that character but used the
  literal `-` with `XStringToKeysym`; the registered X11 keysym name is
  `minus`.

The operation was not retried. Its durable uncertainty journal remains under
the private development service state. The fixture browser exited, the viewer
lease was released, the route returned to available, and the development
selector was rolled back to `0.28.0-08de92737c24` before remediation.

The bounded remediation moved the fixture label outside the solid target and
mapped hyphen to the registered `minus` keysym under a focused unit regression.
Strict clippy and fixture compilation passed. The remediated source commit is
`d5f4799ef5d65aabd0f05a91588c761b0eaef38f`; its candidate SHA-256 is
`86026cf08fe76352b92a52312a6b8992c9c0bc8fba455e8cf58d8e461382429b`.
The original preflight candidate above remains historical evidence and is
superseded for the next development installation only.
