# Policy | Model Selection And Calibration

## Policy

- Select model, reasoning effort, context scope, and delegation topology together to minimize total allocation consumed per accepted milestone while meeting required correctness, safety, and delivery-time constraints. Count orchestration, workers, failed attempts, evaluation, repair, and integration. Treat token counts and API-price estimates as labeled proxies when measured allocation is unavailable.
- Define the milestone before routing substantial work. It must describe usable behavior at its intended integration boundary and a stable acceptance check. Worker completion, prerequisite repair, passing unrelated tests, and document volume do not establish milestone completion.
- Keep a dated repo-local mapping from task tiers to available model and reasoning configurations. Start routine work on the calibrated economical default; use a cheaper tier for mechanical, readily verified work; route material policy design, architectural tradeoffs, or difficult consequential reasoning directly to a designated specialist tier when justified. Select reasoning effort separately. Model novelty and task length alone do not justify an upgrade.
- Reassess routing at non-trivial task start, material replanning, failed acceptance, conflicting evidence, and the configured no-progress interval. Reassessment is a brief primary-agent decision within the existing checkpoint; it does not itself require another model call. Distinguish reasoning limits from missing inputs, authority, unavailable tools, and environmental failure.
- Escalate only when stronger reasoning is likely to resolve a specific obstacle. Delegate the smallest useful decision or diagnostic task with evidence, attempted approaches, acceptance check, write scope, remaining budget, and stop condition. Return ordinary execution to its configured default when that task concludes. Record requested and runtime-reported effective model and effort; report unknown effective configuration explicitly.
- Model upgrades, reasoning changes, prompt edits, tool substitutions, successor plans, and worker replacement inherit cumulative milestone accounting. Reassessment intervals are not renewable budgets. Exhaustion cannot be bypassed by renaming an approach or opening another worker.
- Delegate when expected gains in expertise, independence, context isolation, or elapsed time justify setup and reconciliation cost. Do deterministic mechanical work with existing tools before purchasing model work for it. A compact specialist brief is preferred when full-history inheritance adds no value. The primary integrates returned evidence without repeating the worker's investigation.
- Calibrate complete workflows, not isolated responses. Before starting, freeze representative inputs, acceptance checks, baseline and candidate configurations, quality floor, sample size, retry allowance, resource ceiling, evaluator, and promotion/stop rules. Include failed and timed-out attempts, retain difficult regressions, and use held-out examples when tuning on earlier samples.
- Record the sample/date, workload identity, model and effort, context and tools, topology, accepted count and denominator, defects, interventions, elapsed time, cumulative agent effort, and measured allocation or labeled proxy. Keep elapsed wall time separate from summed worker effort. Do not attribute shared-account consumption to one configuration when concurrent use prevents attribution.
- Stop calibration at its predeclared sample or resource ceiling, or at a defined critical-quality failure. Small samples yield provisional routing only. Do not enlarge the experiment, weaken acceptance, or retry away failures to obtain a favorable result.
- Promote the least costly configuration that meets frozen quality and delivery requirements. An expensive configuration must show a task-relevant benefit that justifies its added consumption. Revert a regressed default promptly and retain specialist use only where justified. Recalibrate after material configuration changes or repeated observed failures with a bounded scheduled sample, not before every task.
- Deterministic audits establish wiring and record validity; they do not prove model quality, allocation savings, or runtime stopping. Repos that operate a controller must test aggregate counters and stop behavior at its real transition boundary. Policy-only adoption must identify calibration and runtime enforcement as unverified.

## Provisional Routing (2026-09-08)

Use deterministic tools and repository evidence first. The current tool surface
offers `gpt-6-astra`, `gpt-5.6-sol`, `gpt-5.6-terra`, `gpt-5.6-luna`, and
`gpt-5.5`; effective runtime model and effort are unknown unless reported.
This table is provisional and makes no price or savings claim. Check the current
tool surface before spawning; do not assume these names or efforts remain
available. When explicit routing is unavailable, record that limitation rather
than claiming a cheaper model was used.

| Work tier | Initial route | Scope and stop rule |
| --- | --- | --- |
| Mechanical or readily verified | `gpt-5.6-luna`, low or medium | Narrow mechanical edits, bounded fixture repair from an explicit reproducer, targeted log/result extraction, or documentation synchronization. Give a small brief, exact file scope, acceptance check, and stop condition. |
| Bounded normal implementation | `gpt-5.6-sol` or `gpt-5.6-terra`, medium | Implement a clearly specified slice with a stable acceptance check and bounded context. |
| Ambiguous or consequential | Current strongest available primary, currently `gpt-6-astra` when available | Diagnose ambiguous failures, make architecture or material policy decisions, handle production custody, or decide acceptance. |

After one failed bounded attempt, the primary must first distinguish missing
inputs, authority, tools, or environment from a model deficiency. Escalate only
for a named obstacle that stronger reasoning could resolve; do not run
unlimited cheap retries. Cheap workers receive a compact brief rather than a
full-history fork, and return evidence for primary integration.

Delegate only when useful independent primary work can proceed alongside it,
or when a permitted bounded consultation supplies expertise the primary needs.
Do not create a worker merely to wait on a build or run a trivial command.
Ask for a patch or compact result with exact evidence, not a second project
history. Inspect the diff and decisive checks without repeating the whole
investigation. Production effects and final acceptance remain with the primary.

Do not add an automatic blanket reviewer or a synthetic benchmark project.
Observe the next real tasks and record runtime-reported model/effort,
acceptance, defects, interventions, elapsed time, and available allocation
evidence before revising this routing.

## Adoption Notes

Use this module for any repo where agents choose among model or reasoning configurations. Trivial one-step work needs no durable routing or calibration record. Keep provider names, prices, available efforts, configuration syntax, exact intervals, and calibration sample sizes in repo-local policy.
