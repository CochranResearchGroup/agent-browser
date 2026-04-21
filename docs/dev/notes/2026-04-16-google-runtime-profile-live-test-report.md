# Google Runtime Profile Live Test Report

Date: 2026-04-16

## Scope

This report captures the live Chrome launch validation run for the
`google-login` runtime profile on Ubuntu/WSL using the repo-local
`cli/target/debug/agent-browser` binary.

It is a narrower test artifact than
`docs/dev/notes/google-runtime-profile-login.md`, which remains the broader
workflow and continuity note.

## Environment

- Repo: `agent-browser`
- Binary: `cli/target/debug/agent-browser`
- Runtime profile: `google-login`
- Profile path:
  `/home/ecochran76/.agent-browser/runtime-profiles/google-login/user-data`
- Browser: Google Chrome
- OS context: Ubuntu/WSL

## Goal

Verify that the documented two-phase Google login workflow works end to end:

1. manual sign-in without DevTools
2. attachable relaunch on the same runtime profile
3. authenticated automation against Google Account, Gmail, and Calendar

## Commands Run

Phase 1: manual sign-in without DevTools

```bash
cli/target/debug/agent-browser --runtime-profile google-login runtime login https://accounts.google.com
```

Phase 2: attachable relaunch and verification

```bash
cli/target/debug/agent-browser --runtime-profile google-login runtime login https://myaccount.google.com --attachable
cli/target/debug/agent-browser --runtime-profile google-login runtime status
cli/target/debug/agent-browser --runtime-profile google-login get url
cli/target/debug/agent-browser --runtime-profile google-login get title
cli/target/debug/agent-browser --runtime-profile google-login open https://mail.google.com
cli/target/debug/agent-browser --runtime-profile google-login get title
cli/target/debug/agent-browser --runtime-profile google-login open https://calendar.google.com
cli/target/debug/agent-browser --runtime-profile google-login get title
```

## Results

### Phase 1

Pass. Chrome launched headed without DevTools and the user completed Google
sign-in manually, then closed the browser.

Observed launch output:

- `Manual browser launched`
- `Runtime profile: google-login`

### Phase 2

Pass. The same runtime profile relaunched in attachable mode.

Observed relaunch output:

- `Manual browser launched`
- `Runtime profile: google-login`
- `DevTools port: 37527`

Observed runtime status:

- `Browser alive: true`
- `Launch mode: manual-attachable`
- target page `Google Account https://myaccount.google.com/`

Observed authenticated reads:

- `get url` returned `https://myaccount.google.com/`
- `get title` returned `Google Account`

Observed authenticated follow-up navigation:

- Gmail opened at `https://mail.google.com/mail/u/0/#inbox`
- Gmail title was `Inbox (2) - ecochran76@gmail.com - Gmail`
- Calendar opened at `https://calendar.google.com/calendar/u/0/r`
- Calendar title was `Google Calendar - Week of April 12, 2026`

## Issue Observed

The first parallel `get url` and `get title` attempts after attachable relaunch
failed even though the profile itself was healthy.

Observed failures:

- `Daemon failed to start (socket: /run/user/1000/agent-browser/default.sock)`
- `Daemon process exited during startup with no error output. Re-run with --debug for more details.`

This did not indicate lost authentication or a broken profile. A sequential
retry of the same reads succeeded immediately.

## Conclusion

The live Chrome launch validation passed end to end. The two-phase Google
runtime-profile workflow is working with the repo-local binary, and the signed
in profile can be reused for authenticated automation against Google Account,
Gmail, and Calendar.

## Practical Guidance

- Use detached `runtime login` for the initial Google sign-in.
- Use `--attachable` only after the user has signed in and closed Chrome.
- If the first read after attachable relaunch fails, check `runtime status`
  before assuming the profile is broken.
- If `runtime status` shows a live attached browser, retry the read
  sequentially.

## Related Note

For the broader background, root-cause narrative, and canonical workflow, see:

- `docs/dev/notes/google-runtime-profile-login.md`
