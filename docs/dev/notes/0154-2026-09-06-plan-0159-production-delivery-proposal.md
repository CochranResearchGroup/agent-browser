# Plan 0159 production delivery proposal

Date: 2026-09-06. Decision: operator approved the exact production delivery
proposal with “ok go”, then explicitly directed full production runtime shutdown.
Production is stopped; the candidate is not activated. Whole-branch promotion
and formal release remain outside this delivery.

## Exact proposed composition

Source commit: `b3c6b6119a805ff87723c4c6701e83516f27dec9`.
Release binary SHA-256:
`367c1006318811324faa64a2bbf0c3e016a76902971fadde677ab39d9452337e`.
Expected production support manifest SHA-256:
`27306db23f455e196d1700b550f7478ea9e3b2266009632ea28198c044408c9b`.
Expected generation: `0.28.0-367c10063188-27306db23f45`.
Dashboard: 80 freshly built embedded assets; asset-list SHA-256
`8cf26fe2f06796cb046a6c0abfa7b7fdf40c454fe7cef53145ae9bddadafdf0b`.

The binary and materialized support bundle are private under
`~/.local/state/agent-browser/campaigns/p159/production-candidate-v2/`.
`expected-production-support/` contains the reviewed production-path unit files
and manifest. Its identity was derived from the isolated materialization after
verifying exact manifest serialization; the real installer transaction must
match it. It is not an installed production generation.

Deliver the runtime, embedded dashboard, controller support, pinned Guacamole
bundle and units together. The [repair inventory](0153-2026-09-06-plan-0159-repair-delivery-inventory.md)
names the coupled client/backend/provider repairs and their proof boundaries.
The takeover helper correction is included in source; JavaScript clients using
that helper must consume the corrected client source separately from the CLI.
The extension source hash is
`afe1387803908700c2fa90420c67ed592a6f4560ce145d6d43bc4a48619ab009`.
Its Compose label changes the Guacamole web configuration so changed extension
code is reloaded. PostgreSQL and guacd configuration hashes were unchanged by
that correction. Web-container recreation can interrupt viewers; retain browser
identity and reconnect through the same durable URL.

## Migration review and decision boundary

The supported production dry run completed without mutation. The earlier orphan
reader rejection is cleared. A second review staged a private copy of current
Service State through the exact binary and deliberately stopped at the isolated
migration-validation checkpoint. The original copied state remained unchanged;
no production service or state was mutated.

The actual staged diff contains:

| Change | Disposition |
| --- | --- |
| 71 browser placeholders, all `not_started`, with no PID or CDP endpoint | Inert historical reference repair; no browser launch or new control authority proved by these rows. |
| 72 session rows, all released | Inert reference preservation. |
| Two browser `tabHandles` fields and nine tab `serviceTabHandle` fields | Handle serialization; include in retained-handle outcome checks. |
| 77 profile `accessPolicy` fields, previously absent | Requires an explicit profile-policy delivery decision; see below. |
| Principal registry, profile capabilities and runtime-owner registry | Preserved in the reviewed class diff. |
| Existing displays, routes, handoffs and viewer/acquisition leases | Preserved in the reviewed class diff; no protected record removals. |

Each new policy uses `shared-local`, active state, revision 1 and no explicit
grants. Default permissions are `profile_use`, `policy_read`, `policy_write`,
`tab_create`, `tab_observe`, `tab_control_own`, `tab_close_own`, `view_open`, and
`drain`. The candidate intentionally materializes this legacy default. This
review does not establish equivalence to every production caller's effective
permissions. Plan 0159 excludes production ACL mutation, so these writes must
not be silently included in an ordinary repair deployment. This is a bounded
P157 policy-migration decision, not a newly authorized remediation campaign.
The schema-level `not_required` label does not mean the staged bytes are unchanged.

Private before/candidate snapshots, transaction and field review are retained
under `production-candidate-v2/migration-review/` and
`migration-field-review.json`. Fresh runtime census, state snapshot and exact
diff review remain mandatory at any later approved apply.

## Proposed guarded update, conditional on the decision

After authority explicitly covers the proposed composition and profile-policy
materialization, invoke the frozen candidate:

```sh
~/.local/state/agent-browser/campaigns/p159/production-candidate-v2/agent-browser install workstation --apply --json --dashboard-port 4848 --guacamole-port 8092
```

Use the default preserve-runtime policy. Do not add a full-shutdown or
browserless override, evict clients, enter credentials, mutate tenant ACLs beyond
a separately approved exact policy packet, or run a formal release. Stop on a
changed composition, ambiguous census, pending effect, failed state backup,
failed retained presentation proof, or transaction identity mismatch. Preserve
the transaction evidence before any recovery action. The dry run found one
eligible retained handoff; bootstrap readiness still requires the candidate's
post-staging presentation proof and does not authorize capture of private pages.

## Compatibility override and rollback

Retain production generation `0.28.0-4a92c42517e1-6121fd69672b`, binary SHA-256
`4a92c42517e1441f5e30b6fcf52857123efa7eb8273a8b126fc504de966333f7`, and support
manifest SHA-256 `6121fd69672bd18e7fa66bd3ea1abe3594aa0679d214b579992d4b0b4068d5c8`.
Private `production-rollback-custody/retained-generation-copy/` independently
matches all 32 generation files. The installed metadata omits its source commit;
no exact source diff from production is claimed.

Preserve the exact bytes of
`50-current-generation-socket-directory.conf`, SHA-256
`aba367de7198cc1f284caee7bf93c9712ff019e0d9c7d517a6a0a02e26c29509`.
It pins an old generation's socket directory. Retire or supersede it only after
new-generation host startup proves the source directory-creation fix and the
supported transaction can retain original client handles. Retain the old file
for restoration with rollback. Until those checks and authority exist, leave it
in place. Source tests alone do not authorize its removal.

For a failed approved update, inspect the exact transaction using
`install transactions inspect --transaction-id <recorded-id> --json`.
Use that readback's current revision, candidate generation and census digest with
`install transactions rollback`. Never invent those arguments, flip the selector
manually, or copy an old state snapshot over newer effects. The transaction owns
state restoration and admission reconciliation. Recheck old-generation identity,
its compatibility drop-in, original handles and retained presentations afterward.

## Installed outcome gates and retained findings

After an approved update, verify selected binary/support identity and
`agent-browser install doctor`; verify current attached clients and runtime hosts,
not only future launch configuration. Confirm original authorized handles remain
usable and selected denial/actor/journal joins retain their exact evidence.
Verify the served Guacamole extension, then confirm a prepared synthetic durable
URL has `operatorVisible.state=ready`, correct external pixels, trusted mouse and
keyboard acknowledgment, same-URL reopen and retained identity under the selected
concurrent view. Use the protected manual external lane only with its exact
synthetic capture bindings and no unchanged retry. Existing P159 outcomes do not
prove these future production outcomes.

The accepted external screenshots still show runtime-convergence notices and a
workspace attention message. Disposition: retained delivery caveat requiring
current status reconciliation during an approved update; the bounded functional
view passed despite these notices. They neither prove a clean dashboard nor
start a general diagnosis queue. P158 endurance and untested transitions remain
unaccepted.

Retain the P158 synthetic browser, fixture server and owned warm provider for the
pending delivery review, with their exact private receipts. Close only the owned
fixture through Service after that review or before a new P158 install, then
verify each captured PID/start identity separately. If its PID is reused, do not
signal it. Isolated materialization and migration-review roots are private
rollback/review evidence and launch no browser. Earlier cleanup receipts remain
part of the acceptance audit; no foreign cleanup is authorized.

## Authorized delivery execution: shutdown complete

The fresh preflight matched the reviewed 77-policy diff and exact candidate,
rollback copy and compatibility drop-in. The preserve-runtime installer stopped
at `blocked_ambiguous_runtime` before activation, with two prior-boot browser
records lacking current identity evidence. Both recorded browser PIDs were
absent. The subsequent full-shutdown planner also depended on stale host and
supervisor evidence. No unchanged installer retry was performed.

The operator explicitly directed full production shutdown. Actual production
systemd ownership was used to stop the runtime host, dashboard ingress/backend,
and reconciliation service/timer. The exact production Guacamole, guacd and
PostgreSQL containers were stopped cleanly. Readback proves those units inactive,
containers stopped, zero production-generation executable processes and zero
browser processes under the production runtime-profile root. Profile directories
and database volumes were preserved. The isolated P158 units remain active.

Private `production-delivery-approved/` retains the plans, blocked transaction,
shutdown intent, before/after ownership and final readback. Candidate payload
staging occurred, but the production generation selector was not switched and
profile-policy migration was not committed. A cold activation of the approved
candidate remains unfinished; do not claim deployment from shutdown alone.
