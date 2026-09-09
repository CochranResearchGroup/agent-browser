# Plan 0158 W4 External Handoff Oracle

Date: 2026-09-02

Plan: `docs/dev/plans/0158-2026-09-02-frozen-candidate-historical-failure-stress-campaign.md`

State: COMPLETE

## Outcome

W4 adds a provider-free external-ingress and durable-handoff oracle at
`scripts/lib/p158-external-handoff-oracle.js`. It does not contact a network or
open a browser. It adjudicates evidence captured by the later E2 runner and
fails closed unless that evidence proves a public authenticated HTTPS ingress,
off-host vantage, operator readiness before pixels, exact retained identity,
unchanged durable handoff, and zero reconnect cold launches.

The input and report authorities are:

- `docs/dev/contracts/p158-external-handoff-fixtures.v1.schema.json`;
- `docs/dev/contracts/p158-external-handoff-oracle-report.v1.schema.json`; and
- `docs/dev/fixtures/p158/external-handoff-sessions.v1.json`.

The frozen corpus contains 36 fully synthetic sessions covering 23 finding
codes, 13 URL roles, nine host classes, six independent visible-identity axes,
and one clean full-ingress reconnect path.

## URL And Ingress Contract

The only valid starting operator URL is public HTTPS with exactly
`/remote-view/<handoff-id>` and no embedded user information. The ID in the
path must equal the declared durable handoff ID.

The oracle scans these roles independently:

- starting handoff;
- redirect `Location`;
- iframe source;
- form action;
- WebSocket endpoint;
- reconnect target;
- copied action;
- client-visible error action;
- provider external URL;
- route binding;
- local embed URL;
- dashboard embed URL; and
- health URL.

It rejects literal localhost, localhost subdomains, IPv4 and IPv6 loopback,
RFC 1918 IPv4, unique-local IPv6, IPv4 and IPv6 link-local, `.local`, raw
Guacamole paths and fragments, DNS names resolving to a forbidden address,
non-HTTPS or non-WSS schemes, and every diagnostic-only URL role. A public
spelling does not override private DNS resolution.

Production-shaped audit calls require all eight ingress observations by
default: DNS, TLS, redirects, authenticated cookie handling, WebSocket, iframe,
form action, and reconnect. A missing check produces the corresponding failure.
Isolated fixtures may suppress unrelated checks only to prove one defect at a
time; the E2 runner cannot.

## Visibility And Continuity Contract

The external vantage must independently prove it is outside the Service host
and its network namespace and that public egress was observed. Usable pixels
cannot precede `operatorVisible.state=ready`.

The visible and expected identities compare exactly across:

- browser;
- Profile;
- session;
- tab;
- CDP target;
- visible URL;
- page marker; and
- pixel hash.

Every reconnect must use the same normalized durable URL and handoff ID. It
must retain the same eight-field identity, reach ready before pixels, and show
zero new physical browser launches. A healthy provider route, dashboard, or
CDP target does not compensate for a failed external observation.

## Integration Correction

Primary-agent review found that the initial core checked ingress evidence only
when a check happened to be present. The default is now the complete eight
check set, and a dedicated negative test removes TLS from an otherwise clean
session and requires `tls_failure`. Fixture classification passes an explicit
empty required-check set only when isolating a different finding.

## Validation

`pnpm test:p158-external-handoff-oracle` passes more than 40 provider-free
assertions. It proves:

- strict Ajv validation of all inputs and outputs;
- exact classification of every synthetic session and all 23 findings;
- coverage of every URL role and host family, including DNS rebinding to
  loopback or private addresses;
- no loopback fallback despite otherwise healthy ingress evidence;
- required public HTTPS, external vantage, and all eight ingress checks;
- ready-before-pixels ordering;
- unchanged durable handoff across reconnects;
- exact eight-field visible identity;
- wrong-browser and all other identity-axis detection;
- duplicate or cold-launch detection; and
- deterministic output, no input mutation, and `repairAttempted=false`.

No installed runtime, network, browser, service, provider, route, display,
Profile, or production state was touched.

## Remaining Gate

W5 is next. It must build the provider-free dashboard truth and performance
probes with dense immutable fixtures. Rendered rows, warnings, selection,
actions, accessibility, and timing must be correlated to one authoritative
snapshot barrier. Source strings and Service rows alone are not UI proof.
