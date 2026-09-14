# Agent Browser Notes Index

Updated: 2026-09-14

This index consolidates reusable and actionable note families without moving,
renaming, rewriting, or deleting historical evidence. There are more than 250
notes in this directory. Most are acceptance receipts or closed field records,
not active work.

New notes should include these fields near the top:

```text
Product lane: PL-BUGFIX | PL-AUTH | PL-CHALLENGE | PL-RECIPES | PL-PLATFORM
Disposition: active-input | accepted-evidence | deferred | superseded | historical
Owning plan or work item: <stable locator or none>
Related lanes: <IDs or none>
```

An `active-input` note must be adopted by one bounded plan or explicitly marked
`deferred`. A note never authorizes implementation, live effects, retry, or
integration by itself.

## PL-BUGFIX | Current Defect Evidence

- [Native confirm timeout](0160-2026-09-08-soylei-native-confirm-timeout-report.md)
- [Service State lock repair evidence](0167-2026-09-11-plan-0167-service-state-lock-repair-evidence.md)
- [Retained tab identity and profile-path launch conflict](0168-2026-09-12-retained-tab-session-identity-conflict-and-profile-path-launch.md)
- [Wrong-tab close and skipped release](0156-2026-09-06-tab-release-skipped-close-and-wrong-tab-close.md)
- [Exclusive-profile lease divergence](0134-2026-08-26-exclusive-profile-lease-holder-reuse-divergence.md)

Incident notes stay in this lane until triage assigns the repair to a product
owner. Cross-cutting architectural changes then move to `PL-PLATFORM` while the
bugfix lane retains the reproducer and regression evidence.

## PL-AUTH | Credentials and Authentication

- [Passkey and two-factor fieldwork](0110-f1-2026-08-23-passkey-and-two-factor-authentication-fieldwork.md)
- [CDP-free Google seeding gap](0135-2026-08-28-cdp-free-google-dashboard-seeding-gap-handoff.md)
- [Authentication Run source acceptance](0138-2026-08-29-authentication-run-provider-free-source-acceptance.md)
- [BILL saved-credential fieldwork](0138-f1-2026-08-29-bill-saved-credential-fieldwork-delta.md)
- [Deterministic site login and password save](0161-2026-09-09-first-class-deterministic-site-login-and-password-save.md)
- [BILL sealed authentication provider acceptance](0161-2026-09-10-bill-sealed-authentication-provider-source-acceptance.md)
- [Site-login state machine](0162-2026-09-10-site-login-state-machine-source-slice.md)
- [Public Authentication Run custody](0163-2026-09-10-public-authentication-run-custody.md)
- [Unattended authentication handoff](2026-08-29-books-receipts-unattended-authentication-handoff.md)

The open product gap is deterministic identifier entry, credential submission,
password-manager policy, second-factor handling, and exact-account verification
through one resume-safe Authentication Run. Profile repair and reset remain
platform dependencies even when an authentication reset is the selected action.

## PL-CHALLENGE | Challenge and Anti-Automation Evidence

- [Browser-external prompt perception](0110-4-2026-08-12-browser-external-prompt-perception-source-acceptance.md)
- [Controlled X11 provider acceptance series](0131-1-2026-08-25-controlled-x11-provider-source-acceptance.md)
- [Operator-visible window focus gap](0133-2026-08-25-operator-visible-window-focus-gap-handoff.md)
- [Stealth Chromium instrumentation boundary](0114-2026-08-14-chromium-instrumentation-boundary-and-install.md)

The current Turnstile and CAPTCHA roadmap sources live on
`feature/turnstile-desktop-challenge` at `17791566`. Plan 0169 is the active
challenge proof. The roadmap currently labeled P173 must receive a new unique
plan identifier before integration because canonical P173 already names the
closed retained-browser repair.

## PL-RECIPES | Reusable Automation

- [Access-plan service-request handoff](2026-05-09-access-plan-service-request-handoff.md)
- [Access-plan monitor recipe](2026-05-10-access-plan-monitor-run-due-recipe.md)
- [Managed-profile acquisition summary](2026-05-10-managed-profile-flow-acquisition-summary.md)
- [Research.gov deterministic automation fieldwork](0150-f1-2026-09-02-research-gov-deterministic-automation-fieldwork.md)
- [Last30Days Reddit handoff-link failures](0157-f1-2026-09-02-last30days-reddit-handoff-link-errors.md)
- [Last30Days acquisition-contract struggles](0157-f2-2026-09-03-last30days-reddit-acquisition-contract-struggles.md)
- [Site-login state machine](0162-2026-09-10-site-login-state-machine-source-slice.md)

The site-login note is shared evidence: `PL-AUTH` owns secret-bearing and
account-state semantics, while `PL-RECIPES` owns deterministic composition and
reuse after those contracts are frozen.

## PL-PLATFORM | Architecture and Infrastructure

- [Service roadmap](2026-04-22-agent-browser-service-roadmap.md)
- [Runtime process identity acceptance](0108-2026-08-10-runtime-process-identity-test-receipt.md)
- [Runtime lifecycle acceptance series](0117-1-2026-08-20-runtime-lifecycle-slice-a-source-acceptance.md)
- [Desktop and presentation infrastructure series](0124-1-2026-08-23-scalable-desktop-evidence-slice-a-source-acceptance.md)
- [Development runtime isolation](0125-2026-08-23-development-runtime-isolation-acceptance.md)
- [Pre-development runtime safety](0126-2026-08-23-pre-development-runtime-safety-and-browser-launch-acceptance.md)
- [Service State contention acceptance series](0143-2026-08-29-plan-0142-structured-client-recourse-checkpoint.md)
- [Frozen-candidate stress campaign](0158-1-2026-09-02-historical-failure-registry.md)
- [Service State lock repair evidence](0167-2026-09-11-plan-0167-service-state-lock-repair-evidence.md)
- [Lease Authority baseline admission](0181-1-2026-09-13-lease-authority-baseline-admission.md)
- [Lease Authority invariant ledger](0181-2-2026-09-13-lease-authority-invariant-ledger.md)
- [Lease Authority build measurement](0181-3-2026-09-14-lease-authority-build-measurement.md)

[Plan 0161](../plans/0161-2026-09-09-first-class-profile-repair-and-reset-plan.md)
is assigned to this lane. It owns the profile aggregate, sealed repair and reset
transactions, lifecycle joins, and shared adapters. Authentication consumes its
target-scoped reset and readiness outputs.

## Historical Navigation

Numbered note families normally correspond to their plan number. Series such as
`0117-*`, `0124-*`, and `0158-*` are cumulative acceptance evidence and should
be read from the plan before opening individual receipts. Older date-only notes
predate consistent plan numbering; route them by subject through
[the product-lane definitions](../product-lanes.md).

Do not bulk-edit old notes merely to add lane metadata. Add metadata when a
note becomes an active input or when a new plan cites it. This preserves hashes,
links, and historical meaning while preventing the directory from becoming the
active backlog.
