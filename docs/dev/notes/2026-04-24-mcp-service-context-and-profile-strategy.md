# MCP Service Context And Profile Strategy

Date: 2026-04-24

## Context

`agent-browser` is moving toward a service model where multiple services and
agents can share the same browser control plane. MCP calls should not assume a
single anonymous caller. Services and agents should identify themselves and the
task being performed so operators can debug behavior, trace decisions, and
attribute browser activity.

Example caller context:

- `serviceName`: `JournalDownloader`
- `agentName`: `article-probe-agent`
- `taskName`: `probeACSwebsite`

## Interface Guidance

MCP tools accept optional caller context fields when the caller can provide
them:

- `serviceName`
- `agentName`
- `taskName`

The service preserves this context in retained job metadata when commands
provide it. Read-only resources can remain global for now, but mutation tools
should include caller context before browser control tools are added. Audit
events should carry the same context as event ownership matures.

## Profile Strategy

A browser profile represents one browser identity per site. Different services
may need different profile isolation levels:

- shared service profile: lower resource use, faster warm starts, but shared
  cookies and identity state;
- profile per service: clearer ownership and fewer identity collisions, but
  more Chrome processes and memory pressure when profiles must run
  independently;
- profile per site or identity: strongest isolation for anti-bot, login, and
  keyring policy, but highest operational overhead.

Profile allocation should be explicit policy, not an accidental default.
Future service profile records should include profile ownership, keyring
policy, credential provider posture, site policy bindings, and whether
multiple services may share the profile.

## Near-Term Decision

Start MCP mutation support with low-risk service control tools before browser
control tools. `service_job_cancel` is the first tool because it already has
CLI, HTTP, daemon, queue, and cancellation semantics. `browser_snapshot` is the
first browser-control tool because it exercises the live browser queue path
without navigation or input mutation. `browser_get_url` extends the same
read-only MCP browser-control pattern to active page URL inspection.
`browser_get_title` adds the same pattern for active page title inspection.
`browser_tabs` applies the pattern to open tab inspection, with optional
verbose target metadata. `browser_screenshot` adds a visual inspection path
that saves the image to disk instead of returning inline bytes. `browser_click`
is the first mutating MCP browser control and is limited to the stable CLI
click contract of selector or cached ref plus optional new-tab behavior.
`browser_fill` extends mutating MCP control to field entry with required
selector and value. `browser_wait` adds the post-mutation synchronization
primitive for selector, text, URL, function, load-state, and fixed-duration
waits. More invasive browser tools should wait until profile and session policy
are first-class in the service model, and they should always supply job caller
context.

## Live Validation

`pnpm test:mcp-live` launches an isolated temp-home browser session, calls
`browser_snapshot`, `browser_get_url`, `browser_get_title`, `browser_tabs`, and
`browser_screenshot` over MCP stdio, calls `browser_click`, `browser_fill`, and
`browser_wait`, verifies the mutations through a follow-up snapshot, and
verifies that the retained `snapshot`, `url`, `title`, `tab_list`, `screenshot`,
`click`, `fill`, and `wait` service jobs record `serviceName`, `agentName`, and
`taskName`.
