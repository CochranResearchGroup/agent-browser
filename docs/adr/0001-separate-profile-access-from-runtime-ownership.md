# Separate Profile Access From Runtime Ownership

Status: accepted

Agent Browser will model profile access as a revisioned permission system that is separate from leases and exact runtime ownership proof. A trusted single-user local runtime defaults profiles to `shared-local`: a self-declared client subject may reuse the managed browser and receive attributable tabs without constructing cryptographic proof of the browser process. Restricted and exclusive use remain explicit policies, while remote or multi-tenant ingress supplies authenticated subjects.

Permissions are live and inheritable. Widening takes effect immediately. Narrowing an occupied shared profile uses a drain-and-restrict transition that fences new admissions, leaves the previous policy effective while compatible work drains, and commits only after incompatible occupancy reaches zero. Forced eviction is a separate permission and explicit operation. Human takeover remains a controller lease rather than a policy rewrite, and exact runtime proof remains mandatory for process adoption, transfer, termination, and full shutdown after access policy authorizes the caller.

Every request carries immutable request provenance from ingress to its response, job, event, trace, and incident projections. Access denials name the subject, assurance level, resource, operation, policy revision, missing permission, blocking occupancy, and executable recourse. Internal owner generations, profile paths, capability bearers, and process digests are not client remediation requirements.

One deep in-process Profile acquisition module owns the joined decision from
Service State and lease evidence through an executable result or exact denial.
It contains identity joins, access evaluation, dominant-blocker selection,
lifecycle replacement, route derivation, and stale-history rejection. Service
Access, Profile recovery, and action runtime consume that result rather than
reinterpreting its serialized representation. The module is extracted behind
current public contracts before permission semantics change; no new crate is
introduced until the in-process seam and dependency direction are proven.

Installation remains a separate deep module. Its Rust implementation owns
desired state, observed state, the sealed convergence plan, and the terminal
receipt. Privileged shell code is an effect adapter only. Dashboard runtime
health consumes the typed convergence result and distinguishes profile-access
ambiguity, request-scoped acquisition denial, and actual runtime convergence
failure.

## Consequences

- Presets provide the ordinary interface: `shared-local`, `restricted`, and `exclusive`. Granular permissions remain available beneath them.
- A shared profile deliberately shares cookies, local storage, extensions, downloads, and other profile-wide effects. Tab attribution does not promise profile-state isolation.
- Stable client subjects receive durable policy grants, while service-generated connection instances own live commands and coordination leases.
- Child browser, session, tab, and view policy inherits from the profile and may narrow but cannot silently exceed parent authority.
- Already-dispatched atomic commands may finish under their admitted policy revision. Queued commands and long-lived control are fenced when authority changes.
- Profile permission changes do not themselves kill a browser, close a tab, or erase occupancy. Those effects require their own authority and receipts.
- Legacy ambiguity becomes observation or migration evidence, never a permanent installation or ordinary shared-use blocker.
- One semantic contract oracle proves the same acquisition decision across
  HTTP, MCP, generated clients, dashboard projections, and execution admission.
- The canonical lease-authority kernel remains singular. New permission and
  lifecycle operations use one cohesive protected client interface rather than
  adding operation-shaped transport helpers.
- Domain response rendering may live beside the contracts it understands, but
  this decision does not authorize a broad `output.rs` extraction without
  measured duplication and a proven seam.
