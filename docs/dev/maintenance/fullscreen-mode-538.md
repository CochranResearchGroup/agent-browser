# Fullscreen handoff mode control

Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#538
Owner: ecochran76
Branch: fix/fullscreen-mode-538
State: ACTIVE

Provide a CDP on/off control in the Native fullscreen toolbar. Native Remote
View retains viewer and authentication ownership; Agent Browser alone inspects
and changes browser mode through its existing durable conversion journal.
The switch opens review, never immediately closes or launches. Affected task
confirmation and explicit remediation remain in the owner operation. Completion
reconnects the original handoff. Unknown physical mode remains unknown.

Public test seams, exercised one vertical slice at a time:
- Mounted native toolbar and HTTP owner replies: catches missing mode control,
  accidental launch on switch, lost operation identity and wrong reconnect;
  does not prove production desktop or authentication continuity.
- Authenticated Native gateway callback: catches spoofed identity, stale desktop
  generation, absent owner, malformed requests and cookie forwarding;
  does not prove an actual browser relaunch.
- Agent Browser owner callback with retained handoff: catches wrong/expired task
  selection and caller-supplied scope, reuses process mode inspection and durable
  mode operation; provider-free qualification is distinct from installation.

The existing private Native owner capability authenticates the callback. Native
operator identity is forwarded by the gateway; browser cookies and credentials
are never forwarded. A bounded JSON mode endpoint accepts only the original
handoff, current slot/generation, operation identity, mode and offered choice.
Native viewer routing retains the handoff selector; it is a locator, not a grant.
No new viewer, login ceremony, automatic retry or website activity replay.

Source ownership: this branch owns only the Native toolbar and mode callback
adapter in both repos. Existing P302 profile-copy and Remote View audio/Windows
lanes remain separate. Isolated auxiliary checkouts avoid their source custody;
existing checkout populations were inventoried read-only. Preserve all peers.
Close both owned auxiliaries after integration or durable handoff.

Validation: exact red and green commands retained outside Git and summarized
here at closeout. Run local focused UI/HTTP tests, Rust format and strict clippy
for each touched Rust workspace, docs checks and patch hygiene. No hosted CI.
No production relaunch or installation is authorized by synthetic qualification.

## TDD receipts

Provider-free red/green cycles used the public Native toolbar, real gateway HTTP
and the real Agent Browser callback listener. The first UI red lacked a switch;
subsequent reds caught unconfirmed relaunch, lost operation identity, false
CDP-off display, missing remediation, early confirmation and cancellation reuse.
The full Native client red proved fullscreen hid its toolbar; a separate real
Guacamole-client red caught keyboard forwarding during dialog rerender. Busy
operations preserve their exact kind and original mode/operation, without replay.
Opaque operation identities retain punctuation rather than imposing URL syntax.

Repeatable focused commands (private asset locators supplied through environment):

```sh
PLAYWRIGHT_MODULE=<module> REMOTE_VIEW_GUACAMOLE_JS=<verified-1.5.5-asset> node --test tests/browser_mode_toolbar.cjs tests/browser_mode_fullscreen.cjs
scripts/ci/rust-tests.sh --focused native_mode_callback
cargo test --locked --offline --lib browser_mode_toolbar_uses_native_auth_exact_page_and_private_owner_callback -- --test-threads=1
```

The first command runs in Remote View; the second runs in Agent Browser. The
third runs in Remote View through Agent Browser's WSL Cargo admission wrapper.
Initial callback reds returned400 instead of403 for missing authority,503 instead
of200 for valid retained routing, and400 instead of200 for opaque operation IDs.
Gateway reds refused the missing route, omitted its asset and retained the wrong
handoff selector. Final local qualification is recorded below after completion.

The actual-client fixture uses headless Linux Chrome and the pinned Guacamole
1.5.5 asset. It observes its exact browser process tree exiting. Synthetic owner
and provider stand-ins do not prove production authentication or browser relaunch.
No production installation, provider mutation or hosted CI is part of this slice.
