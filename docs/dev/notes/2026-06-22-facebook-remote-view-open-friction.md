# Facebook Remote View Open Friction

Date: 2026-06-22

## Summary

Opening Facebook through agent-browser should have been this one-liner:

```bash
agent-browser --json remote-view open 'https://www.facebook.com/' --provider rdp_gateway
```

It was not a one-liner in practice during the last30days Facebook dogfood run.
The route stack was ultimately healthy, but I took the long path by mixing the
direct remote-headed launch posture, a named runtime profile, and route-bound
remote-view open. That created profile locks and confusing allocator errors
before the route-bound default-session command succeeded.

## What I Tried

1. Verified route readiness first:

```bash
agent-browser doctor remote-view --json
```

The second doctor run reported `status=ready`, `remoteControl.status=ready`,
`manyToMany.status=ready`, route A on display `:11`, Guacamole schema/login and
connection permissions ready, and viewer prerequisites ready.

2. Tried a direct remote-headed launch with the last30days Facebook profile:

```bash
agent-browser --json \
  --session last30days-facebook \
  --runtime-profile last30days-facebook \
  --browser-host remote_headed \
  --view-stream-provider rdp_gateway \
  --control-input-provider manual_attached_desktop \
  --display-isolation private_virtual_display \
  --leave-open \
  open 'https://www.facebook.com/search/posts?q=OpenAI'
```

This failed first with:

```text
Daemon failed to start (socket: /run/user/1000/agent-browser/last30days-facebook.sock)
```

After the daemon came up, plain navigation on that session worked, but it was
not the route-bound operator open path. Facebook search returned `Not Found`
while the profile was logged out. Base Facebook loaded the login page and
reported `navigator.webdriver=false`.

3. Tried route-bound open with profile flags after the direct launch:

```bash
agent-browser remote-view open 'https://www.facebook.com/' \
  --runtime-profile last30days-facebook \
  --provider rdp_gateway \
  --json
```

This tried the default socket instead of the intended named session and failed:

```text
Daemon failed to start (socket: /run/user/1000/agent-browser/default.sock)
```

4. Moved `--runtime-profile` and `--session` before the subcommand:

```bash
agent-browser --runtime-profile last30days-facebook \
  --session last30days-facebook \
  --json \
  remote-view open 'https://www.facebook.com/' \
  --provider rdp_gateway
```

This reached the route allocator but failed with:

```text
route_pool_unavailable: no available route pool entry for display allocation 'display:private_virtual_display:session-last30days-facebook'
```

5. Tried the default route-bound command while the named profile was still
locked by the direct launch. It failed with:

```text
Chrome profile /home/ecochran76/.agent-browser/runtime-profiles/last30days-facebook/user-data is already in use by PID 653092
```

6. Closed the named session that I had started during the failed path:

```bash
agent-browser --json \
  --session last30days-facebook \
  --runtime-profile last30days-facebook \
  close
```

7. Retried the route-bound default-session one-liner:

```bash
agent-browser --json remote-view open 'https://www.facebook.com/' --provider rdp_gateway
```

This succeeded.

## Successful Evidence

The successful response opened Facebook through `session:default` with a
service tab using `profileId=last30days-facebook` and
`runtimeProfile=last30days-facebook`.

Key returned evidence:

- `status=opened`
- `browserId=session:default`
- `routeId=guacamole:3`
- `routePoolEntryId=guacamole-rdp-a`
- `displayAllocationId=remote-view-display:11`
- `displayName=:11`
- `provider=rdp_gateway`
- `visibleWindowProof.state=ready`
- `visibleWindowProof.displayContent.state=browser_window_visible`
- public operator URL:
  `https://agent-browser.ecochran.dyndns.org/guacamole/#/client/MwBjAHBvc3RncmVzcWw=`

Follow-up DOM checks against the opened tab showed:

```text
url=https://www.facebook.com/
title=Facebook
navigator.webdriver=false
```

The page was the Facebook login screen with email, password, and Log In controls
visible. Search URLs still returned `Not Found` while logged out, so this proved
browser opening and operator visibility, not authenticated Facebook scraping.

## Trouble Pattern

The route was ready, but the operator path was not obvious because there are
three similar-looking launch surfaces:

- direct `open` with remote-headed posture;
- `remote-view open` with flags after the subcommand;
- `remote-view open` with global profile/session flags before the subcommand.

Those surfaces do not behave equivalently:

- direct `open` can create a browser/profile lock without checking out the
  operator route;
- subcommand-position `--runtime-profile` did not steer the expected session in
  this run;
- global named-session route-bound open requested a display allocation that had
  no route-pool entry;
- the default route-bound command worked only after the named profile process
  was closed.

The practical lesson is that, when `doctor remote-view` says
`remoteControl.status=ready`, the first attempt for an operator-visible page
should be the default route-bound one-liner. Do not start a separate
remote-headed named-profile browser first unless the task explicitly requires a
separate browser identity.

## Product Follow-Up

Make the expected one-liner easier to discover and harder to derail:

- `remote-view open --help` should show the remote-view-specific flags and
  working examples instead of the generic CLI help.
- The documented profile/session flag placement should match parser behavior.
- If the user asks to open a URL on the default operator route, the CLI should
  prefer route-bound retained-tab acquisition and avoid launching an independent
  remote-headed browser on the same profile.
- When `route_pool_unavailable` is caused by a named-session display allocation
  mismatch, the error should recommend the default route-bound command or show
  the available route allocation IDs.
- When a profile lock blocks route-bound open, the error should identify the
  owning agent-browser session and suggest the exact close or retained-tab reuse
  command.

