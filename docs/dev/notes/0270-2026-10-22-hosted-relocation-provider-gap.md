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

## Portable backend feasibility boundary

Xlib [reparenting](https://www.x.org/releases/X11R7.5/doc/libX11/libX11.html) operates within the same screen and rejects a different-screen parent. It is not a transfer between the current independent Native X servers. A candidate for future launches is an opt-in persistent application display session with local presentation clients on Native-owned destination desktops. [Xpra seamless documentation](https://github.com/Xpra-org/xpra/blob/master/docs/Usage/Seamless.md) describes application persistence across client disconnect/reconnect. That supports investigating a provider backend; it does not establish current Chrome/Native identity, input, latency or relocation acceptance.

No Xpra or Xephyr executable is installed in the inspected environment. Introducing that backend changes the provider dependency and launch boundary and cannot retrofit directly attached browsers. Existing direct-X11 browsers must continue to report relocation unsupported. The operator has been asked whether to add this optional portable-launch backend or defer relocation while other acceptance proceeds. No provider source or installed dependency has been changed, no transfer tested, and no new architecture accepted by this note. Any selected backend must remain internal to Remote View and reuse its existing authenticated Native viewer rather than introduce a user-facing viewer or grant ceremony.
