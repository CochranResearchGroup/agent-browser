# Fullscreen handoff mode control

Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#538
Owner: ecochran76
Branch: fix/fullscreen-mode-538
State: COMPLETE

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
node --test tests/browser_mode_toolbar.cjs tests/browser_mode_fullscreen.cjs
scripts/ci/rust-tests.sh --focused native_mode_callback
cargo test --locked --offline --lib browser_mode_toolbar_uses_native_auth_exact_page_and_private_owner_callback -- --test-threads=1
```

The first command runs in Remote View; the second runs in Agent Browser. The
third runs in Remote View through Agent Browser's WSL Cargo admission wrapper.
Initial callback reds returned 400 instead of 403 for missing authority, 503 instead
of 200 for valid retained routing, and 400 instead of 200 for opaque operation IDs.
Gateway reds refused the missing route, omitted its asset and retained the wrong
handoff selector. Final local qualification is recorded below after completion.

The actual-client fixture uses headless Linux Chrome and the pinned Guacamole
1.5.5 asset. It observes its exact browser process tree exiting. Synthetic owner
and provider stand-ins do not prove production authentication or browser relaunch.
No production installation, provider mutation or hosted CI is part of this slice.

## Owner source qualification

Owner source integrated through PR #540 at
`5c5b0f90660484e3e98cbaec903a2b479a75388a`. Executable validation binds to
`bb71467e0a30601247365e41537ccb04dbbc621d`; the integration merge adds only
P302 documentation from main. Remote View consumer integration is tracked by
CochranResearchGroup/remote-view#786. This completes owner source delivery;
issue #538 closes after the dependent consumer lands.

Local commands passed: `scripts/ci/rust-tests.sh --compartment cli-native-stream`
(257 passed, 2 ignored), the `browser_mode_operation` integration binary (16
passed), workspace format check and strict workspace clippy. Also passed API/MCP
parity, generated service-client contracts and JavaScript types, docs build,
policy wiring (11 passed), changed documentation links, handoff documentation
and patch hygiene. No hosted runner was used. Synthetic qualification and Linux
Chrome do not establish target-platform parity or installed runtime acceptance.

The two consumer UI files passed 11 checks and the existing viewer suites passed
84. Set `PLAYWRIGHT_MODULE` to the reviewed Playwright module and
`REMOTE_VIEW_GUACAMOLE_JS` to the hash-verified Guacamole 1.5.5 asset before the
UI command. Cargo commands in this note must use the repository's cargo-safe
wrapper on WSL. No install, provider change, website login or browser relaunch
was performed.
