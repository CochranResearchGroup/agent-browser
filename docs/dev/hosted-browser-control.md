# Hosted browser human control

Hosted viewing uses Native Remote View without changing automation ownership.
An application explicitly calls the public `browser_control` MCP tool to take
control of its task's browser. The existing task/profile/tab remain intact.

Call `browser_control` with `taskId` and `operation: "status"` to read the current
browser-scoped epoch and cooperative controller state. `view` has the same
non-mutating behavior. For `takeover`, supply the current `expectedEpoch`, an
attributable `actor`, a concrete `reason`, and optional `pauseReads: true`.
Takeover blocks new mutating commands across supported MCP, CLI and Service
routes. Default takeover still permits ordinary page observation; `pauseReads`
also blocks page reads and capture. Status and explicit return remain usable.

`return` or deliberate `override` requires the current epoch, actor and reason.
It first enters an awaiting-observation state. A fresh URL read of the same
retained target must succeed before automation resumes. Return never recovers,
reopens or silently replaces a lost browser. Override records attribution using
the caller's existing trusted runtime/operator admission; actor and reason are
descriptive attribution, not proof of authority. The adapter grants no additional
access and creates no viewer grant or permission. Applications must expose override
only through their existing authorized operator surface.

Each admitted mutating command records its request identity before dispatch.
An intervening takeover marks pending work uncertain. The effect may finish;
its later outcome is retained. No website effect is automatically replayed.
If a dispatcher exits with a pending request, return remains blocked until an
explicit `reconcile` request identifies that `requestId`, epoch, actor and reason,
and freshly observes the same retained target. Reconciliation refuses a still-live
same-generation dispatcher. It records observed uncertainty without replay.

The CLI adapter uses the identical contract:

```sh
agent-browser --json mcp host-control TASK status
agent-browser --json mcp host-control TASK takeover 0 operator 'Assist with the current task'
agent-browser --json mcp host-control TASK return 1 operator 'Continue after assistance'
```

The MCP adapter supports `pauseReads`; its CLI adapter uses the default read policy.
State persists across app/daemon restarts without automatic timeout return.
Compact host snapshots/events include current controller ownership, epoch,
pending-outcome count, a bounded outcome sample and the latest attributable change.
The private durable journal retains full request outcomes and change history.

This is a cooperative control contract. Agent Browser coordinates its supported
routes and gives direct controllers current status. It cannot intercept arbitrary
OS input or unmanaged CDP clients. Such controllers must stop dispatch when human
ownership or awaiting-observation is reported. Account authentication, Native
viewer readiness and browser readiness remain independent observations.
