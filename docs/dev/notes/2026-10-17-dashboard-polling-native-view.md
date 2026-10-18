# Dashboard polling and native embedding repair

Product lane: PL-BUGFIX. Execution lane: UX301. Work item: #301.
Disposition: accepted-evidence
Owning plan or work item: CochranResearchGroup/agent-browser#301
Related lanes: Remote View plan0132 / #456; Cooper native workspace ingress

## Outcome and scope

Restore the existing workflow: discover a live browser promptly, select its tab,
and keep the native Remote View desktop embedded. Live inventory must not wait
for full service reconciliation and workstation diagnostics. The native viewer
continues to own desktop operation and access controls.

The production Chrome reproducer reaches focus and handoff success, receives
the opaque locator redirect, then loses the iframe and reports failure. The
installed public origin points at a separate Remote View host, but workspace
framing requires a shared origin. Production ingress has only a legacy
Guacamole path; the development native ingress configuration is not installed
for production. Earlier substituted viewer documents did not qualify this seam.

Three consecutive live status responses also report no verified live browser.
Forced summary reads take about four seconds and the completion-driven poll
waits another three seconds. The owner state read takes about ten milliseconds.

## Bounded repair and acceptance

1. Give the existing status endpoint an explicit workspace projection which
   reads the redacted owner inventory and bounded native/CDP observations without
   service reconciliation or workstation diagnostics. Use it in the native UI.
2. Resolve iframe navigation onto native workspace routes at the dashboard's
   shared origin. Standalone opaque links retain native continuation.
3. Install the corresponding production native ingress paths using the existing
   provider and authentication. Preserve unrelated operational checkout changes.
4. Compare native browser windows with owner records; distinguish unowned
   observed windows from managed automation tasks. Empty desktops stay excluded.

Acceptance requires actual HTTP latency and Chrome embedding evidence, including
the real native framing policy. Synthetic viewer replacement cannot establish
installed iframe success. Preserve live browsers, tabs, profiles and handoffs;
do not renew TTLs, recreate tasks or change authentication policy for this repair.

Private runtime evidence stays outside the repository. Source checks and installed
proof remain separate. The source checkpoints below retain the original failures and qualification boundaries. Installed acceptance follows them.

## Source checkpoint

Authenticated production Chrome confirms native sign-in and both task requests
succeed, then CSP blocks the cross-origin iframe. The native browser inventory
endpoint is absent (404). Existing polling completes roughly every six to seven
seconds. This is a real native document boundary, without a substituted viewer.

The dedicated HTTP workspace regression first fails when it enters the diagnostic
pipeline, then passes in 0.01 seconds after bypassing launch-admission, integrity,
backup and reconciliation work. Operational-status redaction checks still pass.
Managed handoff continuation, compact UI, routing, type checks and native-window
identity tests pass. The actual native-window components/hooks preserve one
same-desktop connection across Alpha/Beta selection, issue three exact explicit
window effects, refuse changed generations and invoke no managed task API.
That UI fixture has a synthetic provider boundary and is not installed proof.

Remote View #456 owns native origin aliases, same-host reauthorization and a
read-only browser-window endpoint using its existing discovery and native probes.
Only current browser-class non-dialog windows enter the rail. A failed peer does
not hide a verified browser. Legacy daemon history stays in diagnostics. Native
selection calls Remote View's existing connected helper, preserving its input
lock and generation checks; no managed session or tab is invented.

## Restart continuity

After production upgrade, the original browser processes, CDP targets, profiles
and handoffs remain intact, but a cold runtime has no in-memory browser manager.
Ordinary task commands already reconnect that exact recorded browser; operator
focus omitted the check and returned `browser_session_runtime_browser_missing`.
The restart regression reproduces that failure. Operator focus now uses the same
recorded-process/CDP identity check and background reattachment before applying
the existing signed focus proof and native effect fence. An unavailable recorded
browser returns an error without launching a replacement or changing owner state.


## Installed acceptance, October 18

PR309 integrated the fast workspace and native embedding consumer; PR310
integrated recorded-browser reattachment at
`f583c0d3b93dc3312e11ad7ab579c4d1746d3b30`. Native Remote View PR458, PR459,
PR462 and PR463 supply current native observations and shared-origin delivery;
Cooper PR15 supplies the production native routes. Authentication and native
input fences retain their existing ownership.

Sealed source `68241fd35436522e3e3afb0cca0d8fd77ce397e7` produced candidate
`candidate-64f2739fffb8e2a9-1b4d1cac8b156945`, SHA256
`1b4d1cac8b1569454bf930a42adf3024c8fa2fed0a87d1ce3e814049ad9afe6c`.
The interrupted build was recovered through its exact abandoned operation after
positive owner-process absence. Its replacement seal, source ancestry and
production-shaped artifact review pass. These exact bytes first passed isolated
development publication, install doctor, three launch/read/close/residue cycles,
an authenticated public native iframe with rendered pixels, and physical
Alpha/Beta/Alpha target visibility and focus read independently through CDP.

Production transaction `upgrade-29b907f9-1636-4dee-bd81-7d16fa53ab0e`
completed ready. Installed SHA256 matches the seal and install doctor passes.
The first dashboard selection after installation reaches the actual connected
Native workspace with nonuniform canvas pixels and no CSP rejection. A second
Alpha/Beta/Alpha cycle returns six successful focus/handoff responses; independent
CDP observations confirm each original target is visible and focused. Poll
observation records five workspace and five native inventory reads with zero
legacy diagnostic reads.

The public rail becomes visible in 731 ms. Five public workspace requests take
92, 50, 54, 58 and 48 ms at approximately 112 KB each. First selected desktop
readiness takes 10.071 seconds, including focus, handoff and stream connection;
that remaining cold-path delay is distinct from the fast rail and refreshes.
Five authenticated native inventory reads each return nonempty desktop groups
and zero unverified desktops. These are bounded samples rather than a sustained
latency guarantee.

Before/after evidence preserves the synthetic browser process and birth, both
original targets, retained browser and handoff record sets, and all 25 recorded
Native processes. The host rebooted before this resumed acceptance: prior
browser processes had exited, and thirteen historical managed browsers correctly
report absent. Preservation claims apply to this installation window. Existing
account authentication and private application workflows were not exercised.

The first development disposable attempt retains an unpublished launch claim;
replay correctly requests readback. A fresh named synthetic profile qualified
the managed Native path. The old default MCP transport also rejected implicit
refresh after the upgrade; a current executable using the exact task transport
completed scoped cleanup. Both owned test tasks and browser parent processes
are closed, and their handoff rows are absent. The inert named test profile and
failed launch claim remain retained. No broad process or state cleanup ran.

An initial selection harness matched an observed native-window row during
managed-presence warm-up. The corrected harness addresses only managed tab
controls. Its initial timeout remains retained separately. The real provider
and framing policy were used throughout acceptance. Human-use acknowledgement
was deliberately withheld, so this receipt qualifies display and operator
selection rather than human-input or account-login acceptance. Wider dashboard
work and those separate contracts retain their own owners.
