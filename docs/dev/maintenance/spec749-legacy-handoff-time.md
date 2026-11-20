# Legacy handoff retention time finding

Product lane: platform

Source finding for the single spec749 final review. This note is not runtime acceptance or authority to extend a retained link.

`cli/src/native/stream/remote_view_presentation.rs` (`native_continuation_location` and redirect validation) and `cli/src/native/stream/native_browser_mode.rs` (mode read and callback validation) pass guest `SystemTime` milliseconds to `RemoteViewTabHandoff::check_link_at`. The model in `crates/agent-browser-service-model/src/remote_view_tab_handoff.rs` compares that value against optional `expires_at_ms`. Creation and retention setters accept caller-provided milliseconds without a retained qualified time domain. A guest jump can therefore falsely expire a link or admit a link beyond its intended lifetime. Perpetual links have no time-dependent expiry check.

Link retention is separate from viewer authentication, browser idle expiry and process custody. Disabling finite expiry or treating legacy guest starts as StableTime evidence would broaden policy. Merely substituting current service UTC cannot qualify a legacy start. `set_link_lifetime` and `extend_link_retention` currently check existing expiry first, so an explicit current-owner repair must be considered together with qualified mint/check semantics. Preserve the same durable handoff identity and independent authentication; do not issue a new provider URL or silently rewrite old expiry.

The bounded final remediation should distinguish qualified future link evidence, unqualified legacy finite retention, an explicitly authorized same-link owner renewal, and unrelated perpetual-link/read operations. StableTime helper failure is not proof of Remote View service outage. Native input freshness repair in ticket761 does not qualify these link timestamps. The profile-recovery expiry finding belongs in the same final clock review. No fix for these link-retention boundaries is claimed by this note.
