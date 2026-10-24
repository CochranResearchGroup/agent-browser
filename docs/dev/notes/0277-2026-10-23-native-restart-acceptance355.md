# Actual Native restart and explicit reopen (#355)

P277 executes the Native624 producer inside a live Agent Browser fixture. The
ordinary confirmation succeeds; the overall #355 matrix remains OPEN. An earlier
ordinary reopen returned an unknown launch outcome and is unresolved. This is
functional evidence with retained failure, not clean reliability qualification.

## What the successful run proves

Fixture source b5208c07f, Native producer source e3cf45d, Native executable
SHA256 `0a0720e78ddec30dcd0589ca8a261655848babe865ba307f4174b40bca04eae1`,
Agent Browser source16450473a3d117035e0601ec38df3bec751ed9a3 and executable
SHA256 `ff0b22c3bb9399e7aa43bfe8f6cb1a609ddc2dffa061001f9c551c9353bcde75`.
The installed immutable executable was invoked with a fresh private home. No
production service, profile, pool, configuration or authentication was changed.
The reviewed Chromium executable remains the existing150.0.7835.0 build.

One newly owned Native target and one peer were created. Public MCP opened a
fresh managed persistent synthetic task. Its preliminary disconnect/reconnect
preserved human-control intent;20warm public transitions validated with nearest
rank p95 **47.20ms**, strict target250ms, zero failed/unknown warm samples.
That public boundary excludes Native/Bots coordination and viewer focus/pixels.

Native's real Stop/Start retired generation1 processes and independently observed
ready viewing generation2 on the same assignment. The original browser became
positively absent. Report-only reconnect preserved loss without replacement;
explicit rebind recorded recovery intent, then one explicit reopen retained the
exact task/browser/profile/tab IDs and profile directory. The replacement opened
`about:blank` with empty body. Native independently observed its exact PID and
window on viewing generation2. Peer Native/app births and protected files stayed
unchanged. Scoped task closure and independent process/resource cleanup pass.

**Generation domains matter:** public `desktop.generation` is the assignment
lifecycle generation, which remains1. Native `viewingGeneration` changes1 to2.
The public scalar did not advance. Passing viewing2 in the lifecycle field is
correctly refused; a prior fixture did that. This acceptance proves explicit
same-assignment recovery across a real Native viewing-generation change. It does
not prove a public lifecycle-generation replacement or physical relocation.

## Failures and uncertainty

All runs are retained privately under
`~/.local/state/agent-browser/acceptance355-20261023/`:

- Native1: occupied test port; refused before effects, then selected fresh ports.
- Native2/3: preliminary reconnect refused window inventory because the isolated
  consumer omitted `windows`; corrected only that fixture's capability.
- Native4/5/6: admission misread Chromium's flattened process title as argv. A
  small independent probe established the format; exact normalized flag/path
  comparison now accepts both forms and rejects prefixes/ambiguity. Native4's
  abrupt runner exit left one owned host, independently retired using its exact
  environment/executable and pidfd; later failure cleanup awaits the consumer.
- Native7: actual restart succeeded; the oracle passed viewing generation as
  lifecycle generation. Corrected the oracle rather than changing product fences.
- Native8: actual restart and explicit rebind succeeded. One explicit reopen
  returned `browser_session_recovery_launch_readback_required`, with durable
  unpublished launch intent and `browser_private_launch_outcome_unknown` in the
  service failure journal. No retry or inferred replacement. Underlying cause
  remains unproven; the private-launch diagnostic deliberately hides raw errors.
- Native9: a separately admitted synthetic exec shim retained private Chrome
  stderr. Functional recovery passed; this diagnostic launch is separate evidence.
- Native10: one predeclared ordinary confirmation without the shim passed the
  functional checks and warm workload. This pass does not erase Native8 or make
  the wider campaign clean. Further runs require a causal diagnostic question,
  rather than repetition until preferred timing or reliability appears.

No source product bug was established or repaired. New source is reusable test
orchestration, oracle and admission checks. The current fixture always attempts
all cleanup stages even if scoped task closure fails, retains owned identities,
and explicitly labels diagnostic acceptance to avoid ordinary-launch claims.

## Reproduction and remaining work

Run the existing focused ownership/cleanup/reconnect/loss Node tests plus
`scripts/smoke-hosted-browser-restart.test.mjs` (33pass total) and
`/usr/bin/python3 scripts/test-hosted-restart-profile.py` (2pass).
Use `smoke-hosted-browser-restart-native.py --help` for explicit source, binary,
Chrome, Xorg/VNC resource and fresh private result inputs. Only the creating
harness may restart its exact newly owned fixture. Never pass a production home
or borrow a foreign assignment. A diagnostic launcher is explicitly opt-in and
cannot establish ordinary-launch acceptance. Failed result directories are never
reused. Unknown restart/reopen effects are never replayed.

Python compile, Node syntax, policy wiring, planning and changed-link checks pass.
Documentation webpack build passes. Default Turbopack build failed because the
reviewed dependency symlinks point outside its filesystem root; that failure is
retained. No Rust product code changed; focused fixture gates and actual-host
proof were selected for the known isolated test surface. The automatic selector
reports broad for unknown new script paths, which is advice rather than evidence
that product Rust changed. No Rust suite or production deployment is claimed.

#355 still needs a single combined pair covering the wider Bots/Native control,
restriction/artifact/closure matrix and predeclared cold-viewer distribution,
plus disposition of Native8's unknown reopen. Existing P270 Native14 evidence is
retained with its original pair and boundaries. Native578 relocation remains
deferred. Next useful work is launch-stage diagnosis of the unknown reopen, not
another acceptance rerun.
