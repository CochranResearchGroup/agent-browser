# Route Staleness And Operator Handoff Defect

Date: 2026-07-05
Source incident: Texas SOSDirect temporary-login handoff.

## Context

During a Texas SOSDirect temporary-login handoff, the operator needed to fill
out the payment/login form through Guacamole while the agent retained CDP
control for post-login SOSDirect lookup work.

The intended operator-visible page was:

```text
https://direct.sos.state.tx.us/acct/acct-templogin.asp
```

## What Happened

The first browser I opened was a plain agent-browser session:

```text
session: tx-sos-temp
url: https://direct.sos.state.tx.us/acct/acct-templogin.asp
```

That was the wrong first move for this workflow. The user had asked for a
browser they could use, and the workflow involved manual payment and login.
The default should have been an operator-visible remote browser, not a local or
non-visible agent browser that then had to be replaced.

I then opened a route-bound remote-view session on Route A:

```text
session: tx-sos-temp-remote
route: guacamole:4
external URL: https://agent-browser.ecochran.dyndns.org/guacamole/#/client/NABjAHBvc3RncmVzcWw=
```

Agent-side checks reported the SOSDirect temporary login URL and title, but the
operator reported that the browser was not showing at the Route A link. That is
the product failure: agent-browser served or returned a route URL that was not
actually usable by the operator for the current browser handoff.

I opened Route B as a replacement:

```text
session: tx-sos-temp-remote-b
route: guacamole:5
external URL: https://agent-browser.ecochran.dyndns.org/guacamole/#/client/NQBjAHBvc3RncmVzcWw=
```

Route B was verified with:

```text
agent-browser --session tx-sos-temp-remote-b get url
=> https://direct.sos.state.tx.us/acct/acct-templogin.asp

agent-browser --session tx-sos-temp-remote-b get title
=> SOS Direct Temporary Login
```

The stale Route A session was then closed.

## Defect 1: Wrong Default For Operator Handoffs

When the task includes manual login, payment, account challenge handling, or
other operator input, the default agent posture should be route-bound remote
view.

Expected first command shape:

```text
agent-browser remote-view open <url> --runtime-profile <profile> --browser-build stealthcdp_chromium --view-stream-provider rdp_gateway
```

The CLI, skill guidance, and service defaults already describe this as the
preferred posture, but the operational path still made it easy to open the
wrong browser first. For operator handoffs, agent-browser should make the
remote route the natural default and should warn when a non-visible browser is
being opened for a likely manual-login or payment workflow.

## Defect 2: Stale Route Should Be Impossible To Serve As Ready

Route A should not have been handed to the operator if it could not display the
current browser. It is not enough for the agent-side CDP target to have the
right URL and title. The returned public operator URL must be coupled to a live
route, the selected display, the visible browser window, and the selected tab.

The system already has route-health concepts such as `operatorVisible`,
`stale_route_record`, route-pool readiness, display allocation state, and
visible window proof. This incident means one of the following remains true:

- a stale route can still pass the handoff path as usable;
- route readiness can be checked against stale retained state instead of the
  current display and browser;
- a public Guacamole URL can outlive or drift away from the route display it is
  supposed to represent;
- the route can be technically ready while not rendering the selected browser
  for the operator.

Any of those outcomes is a product bug. The handoff command should fail closed
or repair the route before returning the URL.

## Required Product Behavior

For `remote-view open`, the returned operator URL should be emitted only when
all of these are true:

- the route-pool entry is current, not stale retained metadata;
- the route display is reachable by the Guacamole connection behind the public
  URL;
- the selected display contains a visible browser window;
- the visible browser window is attached to the selected CDP target or selected
  service tab;
- the public operator URL points to that same route and display;
- a post-open route preflight still reports the route as ready for operator
  viewing.

If any check fails, the command should return a specific state such as:

```text
operatorVisible.state=stale_route_record
operatorVisible.state=wrong_route_display
operatorVisible.state=guacamole_route_unavailable
operatorVisible.state=wrong_tab
```

It should not return a public handoff URL as ready.

## Suggested Fix Direction

1. Make `remote-view open` the default or strongly recommended path when the
   workflow is a manual-login, payment, or challenge handoff.
2. Add a final no-launch route preflight after tab acquisition and browser
   visibility proof, immediately before returning the public URL.
3. Treat stale retained route records as hard failures unless the command can
   reconcile them to the current route display and browser.
4. Add a regression fixture where a retained Route A record points at an old or
   wrong display while Route B is healthy. The command should not serve the
   stale Route A public URL as ready.
5. Add live validation that a route URL returned to the operator can actually
   render the selected browser, not only that CDP can read the target URL and
   title.

## Acceptance Criteria

- A likely operator-handoff request opens through route-bound remote-view by
  default, or emits an explicit warning before opening a non-visible browser.
- A stale Guacamole route cannot be returned with `operatorVisible.state=ready`.
- `remote-view open` either repairs stale route/display coupling or returns a
  specific non-ready state with remediation.
- The regression suite covers stale route metadata, wrong display coupling, and
  wrong selected tab before the public operator URL is returned.
