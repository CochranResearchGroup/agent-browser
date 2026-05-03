# Google Runtime Profile Login Note

Date: 2026-04-15

## Intended Reader

This note is written so a context-free agent can read this file alone and
understand the current Google runtime-profile behavior, the validated workflow,
and the main failure mode to avoid.

## Finding

Google sign-in can reject a browser when DevTools is enabled during the login ceremony. In live testing on Ubuntu/WSL, Google showed "Couldn't sign you in" when the browser was launched with `--remote-debugging-port=0`, even after removing mock keychain and other common automation flags.

The successful sequence was:

1. Launch the persistent runtime profile as a normal headed Chrome process without DevTools.
2. Sign in manually.
3. Close Chrome.
4. Relaunch the same runtime profile with minimal attachable DevTools flags.
5. Attach automation and verify authenticated state.

## Evidence

The successful no-DevTools manual login command line was:

```text
/opt/google/chrome/chrome --new-window --hide-crash-restore-bubble --user-data-dir=/home/ecochran76/.agent-browser/runtime-profiles/google-login/user-data https://accounts.google.com
```

After sign-in and close, the same profile was relaunched with:

```text
/opt/google/chrome/chrome --new-window --hide-crash-restore-bubble --remote-debugging-port=0 --user-data-dir=/home/ecochran76/.agent-browser/runtime-profiles/google-login/user-data https://myaccount.google.com
```

`agent-browser --runtime-profile google-login get url` returned `https://myaccount.google.com/`, and `get title` returned `Google Account`.

## Additional Finding

During live replay after `close`, immediate attachable relaunches could fail
with connection-refused errors even though Chrome was running. The root cause
was runtime state trusting a DevTools port before that port was actually
reachable.

That was fixed in code by:

1. removing stale `DevToolsActivePort` before detached attachable relaunch
2. waiting until the advertised DevTools port is actually reachable before
   persisting it into runtime state

Post-fix live replay succeeded across:

1. attachable relaunch to `https://myaccount.google.com`
2. `get url` and `get title`
3. `close`
4. immediate attachable relaunch
5. `open https://mail.google.com`
6. `open https://calendar.google.com`

Observed successful titles:

- `Google Account`
- `Inbox (1) - ecochran76@gmail.com - Gmail`
- `Google Calendar - Week of April 12, 2026`

## Follow-up Validation

On 2026-04-16, the same two-phase workflow was re-run successfully against the
same persistent runtime profile using the repo-local binary:

```bash
cli/target/debug/agent-browser --runtime-profile google-login runtime login https://accounts.google.com
cli/target/debug/agent-browser --runtime-profile google-login runtime login https://myaccount.google.com --attachable
cli/target/debug/agent-browser --runtime-profile google-login runtime status
cli/target/debug/agent-browser --runtime-profile google-login get url
cli/target/debug/agent-browser --runtime-profile google-login get title
cli/target/debug/agent-browser --runtime-profile google-login open https://mail.google.com
cli/target/debug/agent-browser --runtime-profile google-login get title
cli/target/debug/agent-browser --runtime-profile google-login open https://calendar.google.com
cli/target/debug/agent-browser --runtime-profile google-login get title
```

Observed successful outputs from the 2026-04-16 replay:

- attachable relaunch reported `DevTools port: 37527`
- `runtime status` showed `Browser alive: true`
- `get url` returned `https://myaccount.google.com/`
- `get title` returned `Google Account`
- Gmail opened with title `Inbox (2) - ecochran76@gmail.com - Gmail`
- Calendar opened with title `Google Calendar - Week of April 12, 2026`

One runtime nuance also showed up during this replay: the first parallel
`get url` and `get title` attempts hit daemon-startup races even though the
browser itself was healthy. Re-running those reads sequentially succeeded
immediately.

Practical implication: after an attachable relaunch, a failed first read does
not necessarily mean the runtime profile or authentication state is broken. If
`runtime status` already shows a live attached browser, retry the read
sequentially before treating it as a profile failure.

## Guidance

For Google, Gmail, and similar SSO flows, do not start with `runtime login --attachable`. Use detached `runtime login` first so the user can sign in without DevTools. After the user closes the browser, relaunch the same runtime profile for automation.

Use `runtime login --attachable` only for sites where DevTools during manual login is known to be accepted, or when the user explicitly needs automation attached to the still-open browser.

## Service Model Implication

For the always-on service roadmap, this workflow means a new Google profile is
not ready for automation just because it exists or is allocatable. It must be
manually seated first.

In service terms, a Google target identity needs a `needs_manual_seating`
readiness state until a user has launched the managed runtime profile without
DevTools, completed sign-in, closed Chrome, and allowed agent-browser to
relaunch the same profile for attachable automation. This state is distinct
from stale auth, a crashed browser, a locked profile, or a lease conflict. The
correct remedy is an operator-facing manual bootstrap, not an automated CDP
login attempt.

Once seated, the profile can still become stale later. Freshness probes should
then answer whether the existing seated profile remains authenticated for the
requested target service or login ID.

## Canonical Workflow

Assume the signed-in runtime profile is named `google-login`.

Important execution detail: for repo-local validation, prefer the repo build at
`cli/target/debug/agent-browser`. Live testing on 2026-04-15 showed the older
globally installed `agent-browser` in `PATH` did not yet support this
`--runtime-profile` workflow correctly.

Phase 1: manual sign-in without DevTools

```bash
cli/target/debug/agent-browser --runtime-profile google-login runtime login https://accounts.google.com
```

The user signs in manually, then closes Chrome.

Phase 2: attachable automation on the same profile

```bash
cli/target/debug/agent-browser --runtime-profile google-login runtime login https://myaccount.google.com --attachable
cli/target/debug/agent-browser --runtime-profile google-login runtime status
cli/target/debug/agent-browser --runtime-profile google-login get url
cli/target/debug/agent-browser --runtime-profile google-login get title
```

Run those post-relaunch reads sequentially. Do not fire the first `get url` and
`get title` in parallel immediately after the attachable relaunch.

If the profile is healthy, expected results are:

- `runtime status` shows `Browser alive: true`
- `get url` returns `https://myaccount.google.com/`
- `get title` returns `Google Account`

Optional authenticated follow-up checks:

```bash
cli/target/debug/agent-browser --runtime-profile google-login open https://mail.google.com
cli/target/debug/agent-browser --runtime-profile google-login get title
cli/target/debug/agent-browser --runtime-profile google-login open https://calendar.google.com
cli/target/debug/agent-browser --runtime-profile google-login get title
```

## What Not To Do

- Do not use `runtime login --attachable` for the initial Google sign-in.
- Do not assume the first observed DevTools port is valid unless the code has
  confirmed it is reachable.
- Do not issue the first post-relaunch reads in parallel.
- Do not tell the user the profile is broken if phase 1 succeeded and only the
  attachable relaunch fails. That likely indicates a runtime-state or DevTools
  readiness bug, not lost authentication.

## Retry Rule

If the first read after attachable relaunch fails but `runtime status` already
shows `Browser alive: true`, retry the read sequentially before treating the
profile as broken.

## Current Validation Status

As of 2026-04-16, this behavior was validated live on Ubuntu/WSL with a real
Google login and a persistent profile at:

`/home/ecochran76/.agent-browser/runtime-profiles/google-login/user-data`

Relevant automated validation also passed:

- `cargo fmt -- --check`
- focused Chrome launch tests for manual-login and DevTools-port parsing
- full Rust test suite: `653 passed; 0 failed; 53 ignored`
- context-free replay from this note alone also passed when run against
  `cli/target/debug/agent-browser`

Relevant live validation also passed on 2026-04-16:

- manual sign-in without DevTools
- attachable relaunch on the same runtime profile
- authenticated access to Google Account, Gmail, and Calendar
- sequential retry success after an initial daemon-startup race on parallel
  reads
