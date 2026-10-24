---
id: browser-product-contract
title: Agent Browser Product Contract
category: product
status: active
---

# Agent Browser Product Contract

## Purpose

Make real browser work easier to complete, easier to hand to a person, and easier to continue afterward. Agent Browser must earn adoption through less client code, less agent context, fewer failures, and more completed workflows.

## Product contract

Agent Browser launches and automates browsers with the profile appropriate to the task: an existing client profile, a managed persistent profile, or a disposable profile. Existing client profiles are supported, not mandatory. Preserve existing authentication and profile data when the task uses them.

Remote View supplies desktop operation and the native viewer. Agent Browser integrates browser ownership and handoff with that provider. The operator opens a direct desktop link through Remote View's existing authentication, including production Authelia. Human interaction and automation continue in the same browser. Prefer a desktop chooser restricted to Agent Browser-owned desktops.

Browser work must not depend on opening a dashboard, using a copied Remote View service or duplicate viewer, taking an extra Open desktop step, or issuing and renewing an additional Agent Browser viewer grant. Diagnostics should help explain a failure without becoming the normal task workflow.

## Workstation operator authority

The operator is in charge of their own workstation. Agent Browser must not
refuse a valid operator-authorized request because of warnings, upgrade
markers, stale diagnostic state, or concern about interrupting live work.
Warnings are advisory and may present concrete remedial actions. They never
become permission gates or requests for approval already given.

On request, report what Agent Browser currently thinks is live, including
uncertainty, so the operator can decide whether to upgrade. An observation
neither grants nor withholds authority. Upgrade and drain marker files have
no control-plane role and must not be created, consumed or required.

Keep execution truthful: serialize competing writers, bind effects to the
requested processes and resources, and report actual execution failures with
observed effects and runnable remedies. These mechanics must not be presented
as a policy veto over a valid operator request or expand authority to someone
else's resources.

## Why agents choose it

- Less plumbing: consistent launch, navigation, inspection, interaction and closure.
- Useful continuity: task-appropriate profiles and session reuse, with authenticated state preserved when required.
- Seamless human assistance: direct native desktop handoff, followed by continued automation.
- Predictable operation: lifecycle, scoped recovery and task-owned cleanup that preserve other work.
- Clear results: concise observations, actionable failures and explicit remaining work.

Use Agent Browser when these reusable capabilities reduce the total effort of completing the task. A dev-browser scripting workflow may already meet the task's needs. Direct CDP remains appropriate for specialized protocol work. Do not require a migration or add a wrapper ceremony merely to increase Agent Browser usage.

## Change acceptance

Every substantive product change must answer: does this help an agent complete browser work, or help a person take over and return it, with less effort and fewer failures?

Name the concrete workflow and the expected improvement. Match validation to that outcome: useful browser actions, responsive human controls where requested, continued automation, appropriate profile continuity and predictable cleanup. Internal architecture, ceremony, test count and policy compliance do not substitute for the user outcome.

Honor established task authorization. Keep protections for credentials, profiles and peer work proportional to the actual effect; do not require repeated permission for an already authorized action. Express essential implementation constraints as actionable failures and recovery, not new product rituals.

## Evidence and direction

This is the governing product direction, not a claim that every requirement is already implemented or accepted. RUNBOOK owns current evidence and gaps; active plans own bounded delivery. P223 remains paused until explicitly resumed. Archived strategies, historical OPEN states and superseded access ceremonies do not redefine this contract.
