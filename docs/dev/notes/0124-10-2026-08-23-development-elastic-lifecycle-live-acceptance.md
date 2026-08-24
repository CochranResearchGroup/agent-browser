# Plan 0124 Development Elastic Lifecycle Live Acceptance

Date: 2026-08-23

Status: ELASTIC LIFECYCLE LIVE ACCEPTED | CONFIGURED EPISODE ADAPTER PENDING

Plan: `docs/dev/plans/0124-2026-08-23-scalable-desktop-evidence-and-presentation-capacity-plan.md`

Authority: DEVELOPMENT RUNTIME EFFECTS | PRODUCTION READ-ONLY

Installed generation: `0.28.0-8ad31ffa012b`

Source head after repair: `bd5a45f4`

## Outcome

The isolated development presentation provider completed three fully green
4→6→4 cycles. Each scale-out provisioned exactly one descriptor-owned route.
Each scale-in waited through the configured five-second cooldown, found zero
browser, session, viewer, controller, acquisition, handoff, restoration, or
cleanup references, and reclaimed the highest elastic route first. Every
transition recorded `productionUnchanged: true`.

The accepted cycles used warm displays `:12` through `:15`, route 5 on `:16`,
and route 6 on `:17`. Pressure admission retained a maximum of six throughout.
Observed memory availability during accepted scale-out ranged from
44,420,677,632 to 53,466,034,176 bytes. Observed free swap ranged from
8,444,964,864 to 8,660,209,664 bytes. No pressure reason blocked admission.

## Repaired Live Gaps

- exact lifecycle provisioning now opts into a one-route opener and inspector
  mode without weakening the normal many-to-many minimum;
- terminal, cleanup-satisfied runtime owners remain durable history but are no
  longer rehydrated as live effect bindings for a replacement Chrome process;
- provider observation reports active elastic routes instead of truncating its
  process census to the warm minimum;
- reclaim waits for bounded route-user process convergence, so an asynchronous
  systemd session teardown does not create a false quarantine.

The earlier failed and quarantined receipts remain in the runtime receipt
directory as correction history. They are not counted as accepted cycles.

## Accepted Receipt Set

The twelve accepted receipts are stored under
`~/.local/share/agent-browser-dev/presentation-provider/receipts/`:

- cycle 1: `lifecycle-1787538990712-13722-b7b3b468-cda2-4c26-8c77-4009b579f293.json`, `lifecycle-1787539010467-24106-32edfd72-8278-49df-b938-ff8ffd3a180c.json`, `lifecycle-1787539027433-36406-1dd0902c-4fe9-43c4-bf7f-e84677a48bd8.json`, `lifecycle-1787539042659-45349-c928bd46-c709-403a-aa0f-559b327864aa.json`;
- cycle 2: `lifecycle-1787539063619-50004-25896a77-eebf-4a28-af86-6ef6fdff396d.json`, `lifecycle-1787539085016-63755-399d3805-b81e-4dc9-b90e-48f85b3c3429.json`, `lifecycle-1787539100745-76672-b7c001f0-e13b-49da-886a-a8253ba554bb.json`, `lifecycle-1787539115219-84040-aee6645c-da16-4f15-821f-3aeb45d85a09.json`;
- cycle 3: `lifecycle-1787539141047-95078-b73af665-5a14-4267-8409-24ae5f1b8eab.json`, `lifecycle-1787539167941-12787-c1b35944-fd2b-4db7-b59f-7108c690fbec.json`, `lifecycle-1787539193147-32753-3c704bcd-ae2a-4225-b270-ff7ebf93df8e.json`, `lifecycle-1787539209383-45582-c39e262c-9712-46aa-b7a6-66184e853045.json`.

Their SHA-256 digests, in the same order, are:

```text
4a0bd49afde6a716417d48a5e8e9cc3c17dc040daceb7b2375e1cbfcd85ee133
10bb233caee34bec4d69339b91994486588d3d3558b1b31a31ae2ee16f9db291
18e922d835f389a25eebe6ae6e0b8e316f4ca0bc86d434d25a32d837d45977f5
9f9d9dad69d97e5ce4b229d7edb8ceb24c874d045749c88c56a59324571305e6
c04e4fb80730aeba4e9bbd45a87229379b52e07d3afda38921dcedecaac06d03
f0d04161f8899b6cc2630e5c9ad74df2c6628122ced0ee46e3ce685bb43242e4
4d102e7a8979bb7c103c9d0df851a6bc86a9af5db9a5f558eaee30115455c6f5
0f68b2941320bddd0ef210788fe8387f431bda4dffd616c54475f659d2181549
cd3d541b47584addcc3c304d02d89540435ad513590a0cd37eadceed3fe35ef5
2d5c57f0f96f122913fae7950d5c6c0b93e5219138a667da13502358d6e9eae1
c12087d5a25977e41ebd4a76470efb3615579beed1319735d7ed786c805c852c
db4754a84264ae49960d6d3d7781c66008f1ca459b07773b7f42376bbd026d99
```

## Fresh Convergence Census

After cycle 3:

- development doctor passed with no failed checks;
- all three development units used generation `0.28.0-8ad31ffa012b` and were
  active;
- the provider reported exactly four ready displays, `:12` through `:15`;
- route users 5 and 6 had no remaining processes;
- viewer profiles 5 and 6 had no remaining process command lines;
- Guacamole, guacd, and PostgreSQL containers remained up, with guacd and
  PostgreSQL healthy;
- the six durable Guacamole connection records remained distinct and mapped
  to route users 1 through 6;
- the host had 50,401,624,064 bytes of available memory and 8,454,139,904
  bytes of free swap;
- the helper remained v5 with exact-user, supported, and idempotent-when-absent
  termination capabilities.

## Remaining Boundary

This acceptance closes elastic provisioning, exact scale-in, and stale-process
convergence only. `DesktopEvidenceCoordinator::run` still has no configured
provider caller. Therefore human-viewer precedence, two concurrent desktop
observations plus a recovery reservation, CDP-only non-consumption, and
retained authenticated-browser survival cannot yet be claimed live. The next
bounded packet must add one deep observation-only configured adapter and then
run those acceptance cases. Configured production input remains blocked by
Plan 0110.
