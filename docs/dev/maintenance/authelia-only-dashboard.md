# Authelia-only workspace authentication

Product lane: PL-PLATFORM
Work item: #591
Status: IN_PROGRESS

The operator requires Agent Browser and its Remote View dependency to share
Authelia authentication until compatible native authentication exists across
both products. A native Agent Browser cookie must never mask an unauthenticated
Remote View dependency. Preserve explicit workspace roles and the private
provider bootstrap. Missing or unavailable ingress verification denies access.

Implement ingress-only dashboard authentication, reject native password login
and existing native sessions, update workspace sign-in messaging and migrate
authenticated fixtures to a synthetic Authelia owner. Check anonymous, valid,
expired, spoofed, denied and unavailable-owner behavior on HTTP boundaries.
Review Cooper ingress inventory and dependency guard coverage independently.

Reuse the retained dashboard checkout without adding another worktree. Preserve
sealed candidate custody. PR508 owns access diagnostics; PR588 owns idle handoff.
This lane owns authentication and sign-in paragraphs. Source qualification,
review, merge and installed acceptance remain separate evidence boundaries.

## Evidence and remaining gate

The prior executable accepts valid native credentials in an isolated home and
renews its own cookie without an ingress owner. The same public probe with the
required disabled-auth expectation fails against that executable. The source
repair refuses native login, ignores even valid legacy cookies, and prevents an
old administrator cookie from overriding an ingress observer. The former native
login endpoint no longer reads credentials or enters its failed-login journal.

The dashboard selection passes 156 tests plus isolation. Strict workspace
Clippy, Rustfmt, mounted sign-in/entry checks, auth-status transport checks,
role contracts, development-runtime, policy/link/handoff checks and dashboard and
docs builds pass. The initial HTTP regression attempts timed out in the former
failed-login journal path; preserve them as harness failures, not successful
RED evidence. An initial stream run passed 266 cases and failed one fixture
that omitted its owner configuration. Its corrected full run and exact CLI
qualification are pending at this source custody checkpoint.

The complete public ingress guard is published in Cooper PR38. Existing ACLs and
the private Remote View bootstrap are retained. Anonymous and native-cookie-only
requests redirect to the same Cooper realm; one independent sign-in renders
both the workspace and Remote View desktop chooser. This proves authenticated
application access, not desktop pixels, control or wider Remote View acceptance.

The new backend binary is not installed at this checkpoint. Source review,
production-shaped sealing and preserving installation remain separate gates.
The two existing producer-driver tests remain ignored; no test was newly ignored
or deleted to pass a gate. Their fixture authentication has been migrated but
actual producer-composed acceptance is not claimed.

## Integration checkpoint

The corrected stream compartment passes 267 cases and five child fixtures with
two existing ignored producer drivers. The exact debug CLI refuses valid native
credentials and fails closed for a legacy cookie without owner configuration.
Its initial five-second listener allowance was too short; the retained replay
measured about seven seconds with a bounded thirty-second startup allowance.
The source-free installer fixture passes and proves owned-host cleanup.

PR588 entered main while the first production-shaped build was running. Stop
that exact compiler before sealing or installation, retain its failed/superseded
receipt, and compose main89852310a through merge9e2d66a4. Shared source and guide
changes merge without conflicts. Requalify Rust quality and the stream seam on
the composition, then seal a new candidate. Do not install the superseded build.

An additional source-string journal check fails on its pre-existing assertion
that page.tsx contains handoff_unusable. Both that page and the assertion are
unchanged from base08f1dc2d6. This unrelated baseline failure is retained and is
not represented as an authentication regression or a passing full-project suite.
