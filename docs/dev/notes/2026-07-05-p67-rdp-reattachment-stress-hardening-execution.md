# P67 RDP Reattachment Stress Hardening Execution

Date: 2026-07-05

Plan: `docs/dev/plans/0067-2026-07-05-rdp-reattachment-stress-hardening-plan.md`

## Summary

P67 added a shared stress harness for two-route RDP operation, no-launch model
fixtures for reattachment edge states, and live modes covering route churn,
restart/reconcile, profile identity, viewer contention, rollback/close, and
dashboard rail persistence.

The strengthened profile identity mode found a real remaining gap: a service
access-plan default could inject another browser's `runtimeProfile` even when
the request carried an explicit top-level `profileId`. The repair treats
`profileId` as explicit runtime identity during launch planning and prevents
planned defaults from overriding it.

## Live Route Pool

- Selected Guacamole routes: `guacamole:4` and `guacamole:5`
- Selected displays: `:10` and `:11`
- Route-pool readiness: ready after reopening route displays and removing stale
  route-viewer process `24022`

## Validation

- `node --check scripts/test-p67-rdp-stress-hardening-live.js`
- `pnpm test:p67-rdp-stress-fixtures`
- `cargo test --manifest-path cli/Cargo.toml explicit_profile -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml stress -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm test:p67-rdp-profile-identity-live`
- `pnpm test:p67-rdp-route-churn-soak-live`
- `pnpm test:p67-rdp-restart-reconcile-live`
- `pnpm test:p67-rdp-viewer-contention-live`
- `pnpm test:p67-rdp-rollback-and-close-live`
- `pnpm test:p67-rdp-dashboard-rail-persistence-live`

## Artifacts

- `/tmp/agent-browser-p67-rdp-stress-profile-identity-2026-07-05T21-39-05-574Z`
- `/tmp/agent-browser-p67-rdp-stress-route-churn-soak-2026-07-05T21-15-46-707Z`
- `/tmp/agent-browser-p67-rdp-stress-restart-reconcile-2026-07-05T21-31-41-571Z`
- `/tmp/agent-browser-p67-rdp-stress-viewer-contention-2026-07-05T21-18-28-392Z`
- `/tmp/agent-browser-p67-rdp-stress-rollback-and-close-2026-07-05T21-17-06-229Z`
- `/tmp/agent-browser-p67-rdp-stress-dashboard-rail-persistence-2026-07-05T21-17-43-504Z`
