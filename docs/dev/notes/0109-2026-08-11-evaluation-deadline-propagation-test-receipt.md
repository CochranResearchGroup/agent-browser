# Evaluation Deadline Propagation Test Receipt

Date: 2026-08-11
Plan: P109 / Plan 0109
Source base: `baaed508a7b884ec6382816eddf27c47589e56ab`

## Failure Signature

The installed Last30Days Facebook attempt supplied a 45-second agent-browser
job deadline but the final evaluation returned `agent_browser_error` after
31.810 seconds. Source inspection found two smaller legacy limits: a
24.75-second renderer deadline and the CDP client's fixed 30-second transport
deadline.

## Red Receipt

A focused socket-backed test delayed a valid `Runtime.evaluate` response until
31 virtual seconds under a 45-second caller budget. Before the repair it failed
by unwrapping:

```text
CDP command timed out: Runtime.evaluate
```

The permanent suite does not retain that paused-time socket fixture because
Tokio virtual-time advancement cannot deterministically order kernel-backed
WebSocket delivery. The red failure remains the source-bound defect receipt.

## Focused Green Receipt

The production path now builds one request contract containing both the
renderer parameters and transport deadline, then passes both to
`send_command_with_timeout`.

Validated assertions:

- a 45,000-millisecond caller produces renderer timeout 44,750 milliseconds;
- the same request produces transport timeout 45,000 milliseconds;
- a 6,000-millisecond caller produces renderer timeout 5,750 milliseconds;
- the fake-CDP integration observes the renderer deadline on the actual
  `Runtime.evaluate` command;
- all 35 `native::browser::tests` passed twice with one test thread.

## Source Gate

- repository validation selection completed for the exact base and required
  patch hygiene, Rust formatting, strict Clippy, and focused Rust coverage;
- patch hygiene, formatting, and strict production Clippy passed;
- the canonical Rust runner passed its 1,042 parallel-safe tests with 57
  ignored, then passed every serialized environment-mutating partition;
- no source or test change remains outside `cli/src/native/browser.rs`.

Broader source, artifact, install, and downstream receipts will be appended at
the terminal checkpoint.
