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
