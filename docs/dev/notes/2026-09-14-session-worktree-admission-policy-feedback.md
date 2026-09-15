# Session And Worktree Admission Policy Feedback

Date: 2026-09-14

Product lane: PL-PLATFORM

Disposition: accepted-evidence

Owning plan or work item: CochranResearchGroup/agent-browser#134

Related work item: CochranResearchGroup/agent-browser#132

## Policy Source

- Installed selector bundle: `repo-policy-selector` v0.1.26
- Selected profile: `operations-platform`
- Selection mode: `already-aligned`
- Graph memory assessment: `use`; focused read-only discovery in
  `agent_browser_main` returned no exact current session-admission contract.

## Observed Friction

Four intended top-level development sessions were opened: two bugfix sessions,
one architecture session, and one feature session. The live inventory contained
six non-canonical worktrees. Issue 96 had a second detached review checkout,
and the architecture lane had two additional dirty benchmark worktrees. At the
same time, the active-lane catalog contained stale checkpoints and claimed an
active bugfix worktree that was absent.

The existing policy stated one worktree per lane and a hard repository ceiling
of six. It did not relate the expected population to the user-directed session
portfolio, serialize concurrent worktree creation, or require temporary review
and benchmark checkouts to close within their parent slice. It therefore
described the intended topology without preventing incremental accumulation.

## Local Override

Policy `0052-session-and-worktree-admission.md` makes each worktree transition
serialized and evidence-based without assigning permission power to a
permanent coordinator. Each active top-level development session normally owns
one primary worktree. Auxiliary checkouts remain parent-owned and temporary.
Every admission inspects the complete local worktree population, including
detached and unregistered checkouts, rather than trusting only the active-lane
catalog. Unexpected drift suspends automatic creation and produces advice; the
operator may still direct a specific disposition after reviewing the evidence.

The current dirty or detached worktrees are preserved for reconciliation. This
policy correction does not authorize their deletion, reset, merge, or rewrite.

## Upstream Candidate

The shared policy library should add transactional session and worktree
coordination to its multi-session and worktree modules. A reusable
implementation should provide a repository-scoped transition lock, a portfolio
receipt, complete local inventory classification, supported choices, and exact
closeout receipts. The lock prevents snapshot races; it does not determine
operator permission.
