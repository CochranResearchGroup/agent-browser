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
of six. It did not make the current cap equal the explicitly admitted session
portfolio, reserve worktree creation to the coordinator, or require temporary
review and benchmark checkouts to close within their parent slice. It therefore
described the intended topology without preventing incremental accumulation.

## Local Override

Policy `0052-session-and-worktree-admission.md` makes worktree creation a
serialized coordinator action. Each admitted top-level development session may
own one primary worktree. Auxiliary checkouts remain parent-owned, temporary,
and blocking to further admission if they survive their bounded slice. Every
admission inspects the complete local worktree population, including detached
and unregistered checkouts, rather than trusting only the active-lane catalog.

The current dirty or detached worktrees are preserved for reconciliation. This
policy correction does not authorize their deletion, reset, merge, or rewrite.

## Upstream Candidate

The shared policy library should add transactional session and worktree
admission to its multi-session and worktree modules. A reusable implementation
should provide a repository-scoped admission lock, an admitted-portfolio
receipt, complete local inventory classification, and exact closeout receipts.
