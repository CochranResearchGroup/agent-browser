# Control Plane Worker Upgrade Plan

Date: 2026-04-21

## Scope

This note plans an upgrade to the native daemon so browser control runs through
an explicit worker and queue system.

The goal is to make control-plane races impossible inside one agent-browser
session. Agents should be able to submit many control requests concurrently.
The daemon should dispatch them in a defined order, report queue state, and
recover cleanly when Chrome exits or the CDP connection fails.

This is the first step toward agent-browser becoming an always-available
system service for agentic browser sessions.

## Current Behavior

The daemon currently accepts multiple socket connections and spawns one task
per connection in `cli/src/native/daemon.rs`.

Each connection eventually locks a shared `Arc<tokio::sync::Mutex<DaemonState>>`
and calls `execute_command` in `cli/src/native/actions.rs`.

That means command execution is mostly serialized today, but the serialization
is incidental:

- socket handlers own request lifecycle
- background CDP draining competes for the same state lock
- close and idle shutdown use side channels outside a request queue
- daemon readiness means the socket is reachable, not that the control plane is
  ready to execute browser work
- crash detection exists, but it is spread across command preflight,
  background drain ticks, and CDP command failures

## Target Model

Introduce a per-session control-plane worker.

The worker owns:

- `DaemonState`
- `BrowserManager`
- CDP client access
- periodic CDP event draining
- browser process health checks
- idle timeout handling
- close and shutdown ordering

Socket handlers become thin adapters. They parse a JSON line, create a
`ControlRequest`, send it to the worker over a bounded queue, wait for a
response, and write that response back to the socket.

## Core Types

Add `cli/src/native/control_plane.rs`.

Suggested types:

```rust
pub struct ControlPlaneHandle {
    tx: tokio::sync::mpsc::Sender<ControlRequest>,
    status: std::sync::Arc<ControlPlaneStatus>,
}

pub struct ControlRequest {
    pub id: String,
    pub action: String,
    pub command: serde_json::Value,
    pub priority: ControlPriority,
    pub submitted_at: std::time::Instant,
    pub response_tx: tokio::sync::oneshot::Sender<serde_json::Value>,
}

pub enum ControlPriority {
    Normal,
    Lifecycle,
}

pub enum WorkerState {
    Starting,
    Ready,
    Busy,
    Draining,
    Closing,
    Stopped,
    Faulted,
}
```

Keep the first implementation intentionally small. The worker can still call
the existing `execute_command(&cmd, &mut state).await` internally. The first
slice should change ownership and ordering, not rewrite every command handler.

## Request Ordering

Default behavior should be FIFO per session.

Lifecycle requests may need special handling:

- `close` should run after the active command unless the active command is
  cancelled.
- shutdown should stop accepting new work, drain or fail queued work, close the
  browser, then exit.
- status requests may read the worker status without entering the main browser
  mutation queue once status fields are safe to read.

Do not allow two browser-mutating commands to execute at the same time for the
same session.

## Readiness Barrier

The daemon should distinguish socket readiness from control-plane readiness.

Startup should expose these milestones:

1. socket bound
2. worker queue accepting requests
3. daemon metadata written
4. browser ready, when a command or runtime attach requires a browser

The CLI should treat milestone 2 as daemon-ready. Browser commands that launch
or attach should wait for milestone 4 before returning success.

This directly addresses the Google runtime-profile live test where the browser
was healthy, but first parallel reads raced daemon startup.

## Browser Crash Detection

Crash detection should become a worker responsibility.

The worker should maintain a browser health model:

```rust
pub enum BrowserHealth {
    NotStarted,
    Launching,
    Ready,
    Unreachable,
    ProcessExited,
    CdpDisconnected,
    Closing,
}
```

The worker should update health from three sources:

- process checks through `BrowserManager::has_process_exited`
- CDP reader termination or pending-command channel closure
- explicit CDP liveness probes, such as `Browser.getVersion` or a low-cost
  target query

When Chrome exits, the worker should:

1. mark health as `ProcessExited`
2. clear or detach the browser manager
3. stop screencast and stream state
4. update runtime profile state if the profile is managed
5. fail or retry queued browser-dependent commands according to policy
6. keep non-browser commands available when possible

When CDP disconnects but the process is still alive, the worker should:

1. mark health as `CdpDisconnected`
2. attempt one bounded reconnect if the runtime profile has a known DevTools
   endpoint or port
3. run target discovery after reconnect
4. return a clear error if reconnect fails

This should replace ambiguous errors like `CDP response channel closed` with a
session-level health result that explains whether Chrome exited, CDP
disconnected, or the worker is shutting down.

## Queue Backpressure

Use a bounded queue.

Suggested first limit: 256 pending requests per session.

If the queue is full, return a structured error immediately:

```json
{
  "success": false,
  "error": "Control queue is full",
  "data": {
    "queue_depth": 256,
    "worker_state": "Busy"
  }
}
```

This is better than accepting unbounded work and timing out later.

## Status and Observability

Expose worker state before exposing a full service API.

Add status fields internally first:

- worker state
- browser health
- queue depth
- active command ID and action
- active command duration
- last completed command
- last crash or CDP disconnect reason
- ready milestone reached

Then decide whether these belong in `runtime status`, a new `daemon status`, or
both.

## Implementation Slices

### Slice 1: Add worker plumbing without behavior changes

- Add `control_plane.rs`.
- Add request, handle, worker, and status types.
- Start the worker from `run_socket_server`.
- Keep existing `execute_command` as the command executor.
- Add unit tests for enqueue, response delivery, queue full, and worker close.

### Slice 2: Route socket requests through the worker

- Replace direct state locking in `handle_connection`.
- Enqueue parsed commands and await the response.
- Preserve response JSON shape.
- Preserve current close behavior, but route close through the worker.

### Slice 3: Move periodic work into the worker

- Move CDP event draining into the worker loop.
- Move process-exit checks into the worker loop.
- Move idle timeout reset and shutdown into the worker loop.
- Remove daemon-level state locking.

### Slice 4: Add browser health and crash detection

- Track `BrowserHealth`.
- Treat CDP reader closure as a health transition.
- Convert browser crash and CDP disconnect into structured command errors.
- Add bounded reconnect for attachable runtime profiles.

### Slice 5: Add readiness barriers

- Separate socket readiness from worker readiness.
- Make launch and attach commands return only after browser readiness.
- Add a regression test for parallel reads immediately after attachable
  relaunch.

### Slice 6: Prepare for service mode

- Add a session registry abstraction.
- Keep one worker per session or runtime profile.
- Define service lifecycle commands, such as start, stop, list, and status.
- Defer installation as a system service until the per-session worker is stable.

## Test Plan

Unit tests:

- FIFO execution order
- queue full response
- response delivery through `oneshot`
- dropped client response handling
- close while a command is active
- shutdown with queued commands
- browser health transition from ready to process exited
- CDP disconnect transition and reconnect failure

Integration tests:

- many parallel `get url` and `get title` requests return sane responses
- close waits for the active command or returns a clear cancellation error
- daemon startup reports ready only after the worker queue is available
- runtime profile attach waits for DevTools reachability and target discovery

Live smoke:

- repeat the Google runtime-profile workflow
- after attachable relaunch, send a parallel read storm against Google Account
- navigate to Gmail and Calendar
- kill Chrome during a queued command and verify the worker reports
  `ProcessExited`
- close the CDP WebSocket or block the DevTools port and verify the worker
  reports `CdpDisconnected`

## Non-goals for the First Upgrade

- Do not rewrite all command handlers.
- Do not add a full multi-session system service in the first slice.
- Do not introduce cross-session ordering.
- Do not promise browser crashes are impossible.
- Do not hide Chrome or CDP failures behind retries that obscure the real
  state.

## Open Questions

- Should status requests bypass the mutation queue, or should every request use
  one queue for simpler reasoning?
- Should the first reconnect attempt be automatic for every CDP disconnect, or
  only for managed runtime profiles?
- Should close cancel the active command, wait for it, or support both with a
  flag?
- Should queue priority exist in the first implementation, or should lifecycle
  behavior be encoded only inside the worker shutdown path?

## Recommendation

Implement slices 1 and 2 first. That creates the explicit queue and ownership
boundary while preserving existing command behavior.

Then implement slices 3 and 4 together. Moving periodic work and crash detection
into the worker in the same phase prevents a split-brain model where health is
reported in one place but state cleanup happens somewhere else.

## Implementation Notes

### 2026-04-21

Slices 1 through 3 are now partially implemented:

- `cli/src/native/control_plane.rs` owns `DaemonState`, executes socket
  commands through a bounded worker queue, tracks queue depth, and performs
  periodic CDP draining from the worker loop.
- `cli/src/native/daemon.rs` now starts one control-plane worker per daemon
  session and routes socket commands through `ControlPlaneHandle::submit`.
- The first crash-detection slice is in place. The worker checks
  `BrowserManager::has_process_exited`, marks browser health as
  `ProcessExited`, clears the browser manager, disables screencasting, and
  updates the stream client before later browser commands relaunch.
- A protocol-level `worker_status` action reports `worker_state`,
  `browser_health`, `queue_depth`, and `queue_capacity` directly from atomic
  worker status. This status action intentionally bypasses the mutation queue
  so callers can inspect worker health even when normal work is backed up.
- Queue-depth accounting increments before enqueue and rolls back on send
  failure, so a fast worker cannot decrement the counter before a caller has
  recorded the pending request.

Validation evidence:

- `cd cli && cargo fmt -- --check`
- `cd cli && cargo test native::control_plane`
- `cd cli && cargo test native::daemon`
- `cd cli && cargo test` returned 664 passed, 0 failed, and 53 ignored.
- Full ignored Chrome e2e was retried with an isolated temporary `HOME` and
  `AGENT_BROWSER_SOCKET_DIR`; `cd cli && cargo test e2e -- --ignored
  --test-threads=1` returned 52 passed, 0 failed, and 0 ignored.
- Live temporary-profile smoke: open `https://example.com`, run 10 parallel
  `get title` calls, and close the session. All title reads returned
  `Example Domain`.
- Live crash smoke: open `https://example.com`, read the browser PID, kill only
  that smoke-test Chrome process, then open `https://example.com` again. The
  worker relaunched Chrome and `get title` returned `Example Domain`.
- Live status smoke: open `https://example.com`, send raw socket JSON
  `{"id":"status-smoke","action":"worker_status"}`, and verify
  `browser_health: Ready`, `worker_state: Ready`, `queue_depth: 0`, and
  `queue_capacity: 256`.

Known environmental issue:

- The normal user default profile at `/home/ecochran76/.agent-browser/profile`
  was still locked by an existing headless Chrome process owned by
  `/home/ecochran76/workspace.local/agent-browser/bin/agent-browser-linux-x64`.
  The successful retry avoided that unrelated daemon by isolating `HOME`.
