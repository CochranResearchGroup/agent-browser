# Plan 0086: Remote-View Open Per-Job Timeout

State: CLOSED

Created: 2026-07-31

Lane: post-release v0.28.0 integration successor to Plan 0085

Source evidence:

- last30days Plan 0018 checkpoint C31
- service jobs for Reddit, X, Facebook, and LinkedIn on 2026-07-31

## Goal

Expose the control plane's existing per-request `jobTimeoutMs` contract through
the user-facing `remote-view open` CLI so durable-profile launches are not
silently constrained by the daemon's shorter default job timeout.

## Bounds

- Add one positive-integer `--job-timeout-ms <ms>` option to `remote-view open`.
- Copy the value to the existing command-level `jobTimeoutMs` field.
- Update CLI help and focused parser coverage.
- Preserve the daemon-wide `--service-job-timeout` meaning.
- One implementation pass, one review/rework cycle, and one installed-runtime
  validation before any source successor interval.
- Preserve the operator-owned untracked `--full-page` file.
- No push, release, tag, or publication.

## Acceptance

1. The parser accepts a positive timeout and rejects zero, non-numeric, or
   missing values with contextual usage.
2. The parsed command carries exact `jobTimeoutMs` without changing unrelated
   remote-view routing fields.
3. Focused parser and relevant service/control-plane tests pass.
4. The installed CLI exposes the option and one bounded durable-profile open
   lasts beyond the former 15-second boundary or returns a distinct terminal
   result.

## Next Action

- Implement and validate the parser/help change, install the local successor,
  then return to last30days Plan 0018's newly bounded source successors.

## Result

The CLI now carries the configured timeout into live jobs; a 90-second source
job proved that contract. The remaining inner 15-second stall was classified
separately and passed to Plan 0087.
