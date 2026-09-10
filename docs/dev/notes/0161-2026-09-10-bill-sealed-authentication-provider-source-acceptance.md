# BILL Sealed Authentication Provider Source Acceptance

Date: 2026-09-10

State: SOURCE ACCEPTED

Branch: `feature/p0240-sealed-auth`

Effect posture: provider-free and runtime-neutral

## Accepted Outcome

The durable Authentication Run now has a closed BILL provider for the
identifier, password, SMS OTP, and authenticated-company states. A public
recipe-status read returns the repository-owned `bill-login-v1` digest. Start
binds the exact retained service tab, profile, browser, session, organization,
vault account, challenge-provider tenant and account, and recipe digest.

Every browser or challenge-provider effect is recorded as pending before it is
executed. If the process stops between effect and receipt, later resume calls
return `authentication_run_effect_outcome_unknown` and do not replay. The IM
Receipts watch is live before password submission can trigger SMS. The sole
post-fence code crosses only a protected loopback response, is submitted in the
same daemon operation, and is absent from command requests, durable state,
public status, generated clients, CLI output, and MCP results.

## Repository And Tenant Boundary

The tracked `config/site-login-recipes/bill-login-v1.json` contains only
tenant-neutral origins, form selectors, semantic labels, and route shape. The
organization, account, profile, tab, and IM Receipts bindings remain
tenant-owned input supplied by Books Receipts. No private tenant values or
provider material were copied into this repository.

The authenticated receipt proves the exact BILL company route in the exact
retained profile and tab, tied to the vault account used by the run. It does
not claim that BILL exposes an email address in the authenticated DOM.

## Deliberate MVP Boundary

Chrome password-manager save or update prompts are native browser chrome and
remain a separately modeled provider boundary. If one is observed, the run
stops with `authentication_run_browser_chrome_provider_unavailable`; it does
not guess coordinates or report success. Routine BILL reauthentication does
not require that optional persistence action because the retained profile and
vault account already provide the credential source.

## Validation

- `pnpm generate:service-client` and
  `pnpm generate:service-client --check` passed.
- `scripts/ci/cargo-safe.sh fmt --all -- --check` passed.
- `scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml --bin agent-browser authentication_run`
  returned 22 passed and 0 failed.
- The focused closed-request and stable-contract metadata tests each passed
  through `scripts/ci/cargo-safe.sh`.
- `git diff --check` passed.

The regression suite includes a wrong-challenge attempt that performs no
effect and does not consume its operation ID, followed by one successful use of
that same operation ID after the binding is corrected.

## Effect Audit And Remaining Gate

No browser, retained profile, credential store, IM Receipts service, message,
SMS, OTP, BILL tenant, or accounting provider was accessed. No runtime was
installed or restarted. Source integration, development-runtime validation,
and an explicitly governed live run remain under Books Receipts Plan 0240.
