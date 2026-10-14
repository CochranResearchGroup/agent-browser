# SiteGuides and shared website routines

Use one existing taskId for software and agent work. A SiteGuide supplies site
knowledge; routines execute subtasks; playbooks compose routines. A browser
window at a login page is not an authenticated intended account/company.

## Discover and verify

1. `browser_discover({category:"playbooks"})` lists operations.
2. Request `operation:"verify_login"` for its exact schema.
3. `browser_command({taskId,action:"playbook",params:{operation:"verify_login",guideId,accountRef,organizationRef}})` verifies the current task.

`completed` means both intended identities match the current authenticated
application evidence. `waiting` identifies an observed login/MFA state;
`adaptation_required` identifies a stale observation, changed task target or
unrecognized site; `failed` identifies wrong origin/account/company or a blocked
site. This verification routine submits no credential or code. Continue in the
same task using the indicated state rather than creating another browser.

Use `operation:"siteguides"` or `operation:"inspect_guide"` (with guideId) through
browser_command to read the configured guide names or one guide's definition.
The MCP server loads only reviewed guides from its configured directory.

## Software uses the same interface

```js
import {createPlaybookClient} from '@agent-browser/client/playbooks';
const playbooks = createPlaybookClient((name, args) =>
  mcpClient.callTool({name, arguments: args}));
const result = await playbooks.verifyLogin({
  taskId, guideId: 'bill-fixture',
  accountRef: expectedAccount, organizationRef: expectedCompany,
});
```

The callback is your existing connected MCP client. It returns the standard
MCP content envelope; the wrapper returns the same routine data an agent sees.
It creates no transport, task, browser or credential store.

## Minimal reviewed guide

Configure `AGENT_BROWSER_SITEGUIDE_DIRECTORY` when starting the MCP server. Store
one guide per `<guideId>.json`. This example uses an owned synthetic application;
it is not a locator claim about a real BILL account.

```json
{
  "schemaVersion": "agent-browser.siteguide.v1",
  "guideId": "bill-fixture",
  "name": "BILL fixture",
  "allowedOrigins": ["http://127.0.0.1:8080"],
  "authenticatedSelector": "#application",
  "account": {"selector":"#application","attribute":"data-account"},
  "organization": {"selector":"#application","attribute":"data-company"},
  "states": [{"state":"password_required","selector":"input[type=password]"}]
}
```

Require exact origins and a unique visible authenticated application. Identity
fields select one element and compare its nonsecret text or a data/aria-label/title
attribute to the requested identity. Password values and full page bodies are
never routine output. Supported state names are identifier_required,
password_required, mfa_required, login_required and blocked. Unknown variants
require new evidence and a reviewed guide change.

Verification measures the complete observation round trip. Default freshness
is 10000 ms; set maxObservationAgeMs only when the task needs a different
explicit 1 through 60000 ms bound. Task/tab identity is rechecked after reading.

Remote View owns desktop effects. Shared auth helpers own private credential
and MFA operations. These verification calls do not establish helper readiness,
real login acceptance or passkey support.
