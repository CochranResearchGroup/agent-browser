# Domain documentation

Use the existing single domain authority, CONTEXT.md, and applicable docs/adr entries. Preserve this convention; do not create a competing GLOSSARY.md or context map. Rust crates and dashboard packages are implementation boundaries, not grounds for inventing multiple domain glossaries.

The governing product contract is docs/dev/policies/0054-browser-product-contract.md. Agent Browser owns browser operation, profiles, sessions and integration. Remote View owns desktop allocation, operation and the native viewer. Read current ROADMAP.md, RUNBOOK.md and the active plan index. Archived plans are evidence for a named question only.

Before changing domain terms, reconcile the existing definitions and ADRs. Preserve distinctions between durable/disposable profiles, client subjects, access policy, runtime ownership, profile occupancy, desktop capacity and handoff lifetime. Load relevant current governance policy packets rather than the whole policy directory.
