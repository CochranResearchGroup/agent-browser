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
waits. `browser_type` adds keyboard-style text entry for fields that should be
exercised through typed input instead of direct value assignment.
`browser_press` adds key and key-chord support for submit, tab traversal,
escape, and shortcut-driven flows. `browser_hover` adds pointer movement for
menus and controls that reveal state on hover. `browser_select` adds dropdown
and multi-select option selection for account, tenant, role, and region
switchers. `browser_get_text`, `browser_get_value`,
`browser_get_attribute`, `browser_count`, and `browser_get_box` add targeted
page-state reads without requiring full snapshots, including repeated-element
and geometry checks. `browser_is_visible` and `browser_is_enabled` add
read-side readiness checks before agents attempt focus, typing, clicks, or
control mutation. `browser_check`, `browser_is_checked`, and `browser_uncheck`
cover checkbox and radio controls for consent, remember-me, terms, and
preference flows, including read-side assertions before and after state
changes.
`browser_scroll` covers page and container scrolling for long forms,
offscreen controls, and infinite-scroll workflows. `browser_scroll_into_view`
targets specific offscreen elements without requiring callers to guess pixel
distances. `browser_focus` prepares fields and controls for keyboard-driven
interaction, autocomplete, passkey prompts, and other focus-sensitive browser
flows. `browser_clear` resets stale form values before agents use real typing
or fill operations. More invasive browser tools should wait until profile and
session policy are first-class in the service model, and they should always
supply job caller context.

## Live Validation

`pnpm test:mcp-live` launches an isolated temp-home browser session, calls
`browser_snapshot`, `browser_get_url`, `browser_get_title`, `browser_tabs`, and
`browser_screenshot` over MCP stdio, calls `browser_click`, `browser_fill`,
`browser_type`, `browser_press`, `browser_hover`, `browser_select`,
`browser_get_text`, `browser_get_value`, `browser_get_attribute`,
`browser_count`, `browser_get_box`, `browser_is_visible`,
`browser_is_enabled`, `browser_check`, `browser_is_checked`, `browser_uncheck`,
`browser_scroll`,
`browser_scroll_into_view`, `browser_focus`, `browser_clear`, and
`browser_wait`, verifies the read-side state checks, verifies the mutations
through a follow-up snapshot or post-scroll function wait, and verifies that
the retained `snapshot`, `url`, `title`, `tab_list`, `screenshot`, `click`,
`fill`, `type`, `press`, `hover`, `select`, `gettext`, `inputvalue`,
`getattribute`, `count`, `boundingbox`, `isvisible`, `isenabled`, `check`,
`ischecked`, `uncheck`, `scroll`, `scrollintoview`, `focus`, `clear`, and
`wait` service jobs record `serviceName`, `agentName`, and `taskName`.
