# Plan 0137 source authority map

This map freezes the first profile acquisition recovery authority boundaries.

| State or transition | Propose | Apply | Persist | Project or render |
| --- | --- | --- | --- | --- |
| Profile Acquisition Outcome and Dominant Blocker | `service_profile_recovery.rs` from immutable Service State | none during planning | none during planning | CLI, HTTP, MCP, generated client |
| Sealed terminal-owner Recovery Plan | `plan_terminal_owner_recovery()` | none | caller-held plan only | CLI, HTTP, MCP, generated client |
| Exact terminal-owner supersession and acquisition retry | `RecoveryPlan.actions` | `apply_terminal_owner_recovery()` through the sealed daemon route | `LockedServiceStateRepository` | Recovery Receipt and acquisition outcome |
| Recovery Receipt and status | apply core | apply core only | `ServiceState.profile_recovery_receipts` | authenticated status surfaces only |
| Principal and profile capability authority | `service_principal.rs` | capability-authenticated recovery entry points | `ServiceState.service_principals` without raw capability | redacted principal and profile identifiers |
| Runtime owner generation and lifecycle | `runtime_lifecycle.rs` | runtime lifecycle authority during acquisition retry | runtime owner registry | Service status and recovery postcondition evidence |
| Durable browser id and daemon route | lifecycle owner registry | runtime lifecycle authority | separate fields in Service State | never collapsed into one identity |
| Unproven or inconsistent identity cases | blocker classifier | no effect in the first vertical | original evidence preserved | reviewed recovery class only |
| Fictitious or PID-less retained records | provenance and lifecycle audit | no effect in the first vertical | original evidence preserved | reviewed retirement recourse only |
| CDP-free manual seeding route | later route-bound coordinator | no effect in the first vertical | no new route state | blocked until durable handoff readiness exists |

Raw profile capabilities are ephemeral inputs. They may authenticate and seal a
plan but must not be stored in Service State, receipts, traces, documentation,
or generated-client output. The Odollo contractor portal fixture remains an
identity-reconciliation case and cannot be promoted to terminal-owner
supersession.
