# Workspace sign-in

The ordinary workspace reuses existing ingress sign-in without a second password
form. This is separate from website account authentication and Native viewer access.

Configure all three settings in the backend's reviewed environment:

- `AGENT_BROWSER_DASHBOARD_INGRESS_VERIFY_URL`: trusted Authelia owner URL with
  exact path `/api/authz/forward-auth`; HTTPS or numeric-loopback HTTP only.
- `AGENT_BROWSER_DASHBOARD_INGRESS_WORKSPACE_URL`: fixed public HTTPS workspace
  GET/ACL target. Review owner ACL and verifier peer-IP semantics. Caller Host
  and forwarded headers cannot select it.
- `AGENT_BROWSER_DASHBOARD_INGRESS_USERS`: JSON exact owner-principal to existing
  dashboard username mapping, such as `{"operator":"admin","reader":"codex"}`.
  Current private-store roles apply; no automatic superuser grant. Prefer
  one-to-one mappings for distinct actor attribution.

Only a direct owner HTTP200 with one Remote-User and an explicit mapping
establishes identity. The backend sends the session cookie and creates
ForwardAuth metadata from the reviewed URL. Arbitrary caller Remote-* headers
never authenticate. Redirects are not followed or exposed as navigation targets.
Owner302/303/401 requires sign-in; denied/unmapped identities return403;
malformed/unavailable owner verification returns503. Partial/invalid config
fails closed. Recovery retains the selected browser URL and distinguishes
sign-in, denial, redirected content and service failure.

With all three settings absent, supported local/machine login API and signed
sessions remain available. Existing auth-directory/auth-file overrides remain
supported. The private store defaults admin to superuser and codex to observer.
A valid local session takes precedence and renews its thirty-day cookie on
status read. Ingress identities never mint local cookies. The returned user's
`authenticationSource` is ingress/local. End local session ends only the local
cookie; ingress provider sign-out remains separate.

Never read credentials into logs, tracked docs or command arguments. Configure
or mutate installed ingress only within its explicit authority. Source fixtures
prove neither installed cookie delivery nor actual Native workflow acceptance.

## Manual workspace tools

Use the compact top-toolbar Open browser tool for Chrome/Chromium, exact installed build, existing/new durable/disposable profile, CDP-free or connected mode, and optional HTTP/HTTPS/blank URL. Disposable plus CDP-free is the default. Manual tasks stay open until explicitly closed. Compatible connected sharing creates an independent task/tab; CDP-free has one owned window. Return to that task or choose another profile. Use Profile Lab for independent copies. Changing an original closed profile build requires explicit migration/sign-in risk confirmation.

A lost launch reply retains the original operation. Inspect it without creation replay. Only a confirmed refusal with no effects permits revised choices. A ready allocation selects exact browser/task/tab IDs; Native visibility remains independently qualified by the existing Remote View consumer. Remote View retains input, recording and narration controls.

Profiles and SiteGuides stores multiple intended site/account/organization associations. Intended labels, observed identity and authentication freshness remain distinct. Explicit profile choice is honored. Automatic routing needs a unique compatible exact intended identity; ambiguity/conflict requires a choice. Public policy filters profile read/use/write, and association edits preserve authentication evidence.
