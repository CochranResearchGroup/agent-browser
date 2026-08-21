# Service control-plane attestation source acceptance

Date: 2026-08-21

## Outcome

Service action `diagnostics` now returns
`controlPlaneAttestation` using schema
`agent-browser.service-control-plane-attestation.v1`. The attestation is
complete only when one persisted service snapshot proves all of the following:

- the current ready runtime owner generation is effect-capable for the exact
  logical browser and daemon session
- the retained browser PID and process start identity hash match that owner
- the exact browser, session, tab, profile record, and exclusive lease agree
  without a conflicting exclusive profile holder
- the current owner generation has a hash-bound accepted commit or reverse
  handoff receipt

Missing or conflicting proof remains explicit in `missingProofs`; callers must
fail closed before effect-capable input. The response does not expose profile
paths, executable paths, credentials, or private browser content.

## Validation

- focused control-plane and owner-attestation Rust tests passed
- generated service client contract and TypeScript checks passed
- service request client and example tests passed
- service API and MCP parity passed
- route-confusion no-launch gates passed
- documentation build and remote-view handoff documentation checks passed
- Rust formatting and clippy with warnings denied passed
- JSON schema parse and diff hygiene passed

The full Rust suite reached 2,246 passed and retained 11 failures in existing
connection, install, dashboard fallback, and session-cleanup tests. Each failed
group reproduced in isolation. None of those failing files is changed by this
slice; the focused attestation tests and all selected contract gates pass.

## Runtime boundary

This acceptance covers source and provider-neutral fixtures only. It does not
install a new runtime generation, restart a service, adopt a browser, or send
browser input. An installed runtime must expose the new diagnostics schema
before a downstream client can rely on the attestation.
