# Same-process relocation provider prerequisite

Product lane: PL-PLATFORM
Disposition: active-input
Owning plan or work item: [P270](../plans/0270-2026-10-22-hosted-continuity.md), CochranResearchGroup/agent-browser#351
Related lanes: Remote View #578, #355

## Verified source boundary

Agent Browser main 2f7135b77 advertises relocation false. Remote View checkout 1ab08691f116b9c0ca24ad1749acb458d5d6dd28 exposes ConsumerRequest in src/consumer_policy.rs with inventory, assignment observation, launch environment, windows, activation, viewing and release, but no transfer operation. Its src/desktop/controller.rs XvfbFixedController launches an X server for the requested display; XorgDummyRandrController provides geometry control. Neither API establishes a move of an already connected browser process between Native desktops.

Changing BrowserHostTaskBinding, choosing another assignment, activating a window or reopening a profile is not evidence of same-process relocation. No existing browser or desktop was changed during this investigation. No transfer experiment or second viewer was introduced.

## Required boundary

[Remote View #578](https://github.com/CochranResearchGroup/remote-view/issues/578) records the provider prerequisite: exact source/destination capability, generation/process/window identity, explicit idempotent intent, current human-control admission, supported physical transfer, uncertain result readback and independent continuity/peer evidence. Agent Browser consumes that capability and updates app handoff routing only after verified transfer. Unsupported destinations preserve the original.

#351 remains incomplete until at least one supported real-provider transfer preserves process/profile/task/tab/cookie/page state. #355 remains dependent on that proof. The parallel reconnect, policy and guidance work may qualify independently; provider-free fixtures cannot close this gap.
