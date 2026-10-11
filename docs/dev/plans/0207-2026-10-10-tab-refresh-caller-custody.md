# P207: preserve caller custody during tab refresh

Product lane: PL-BUGFIX
Work item: [issue 175](https://github.com/CochranResearchGroup/agent-browser/issues/175)
Baseline: `4cbea18ecf0879b71a578e508cc3770ab1e719b2`
Branch: `fix/p207-current-custody`
State: IMPLEMENTING

When two tasks intentionally share a profile, refreshing one task's closed tab
must preserve the other task's tab. Blank pages and matching origins are discovery
hints, not ownership evidence. Issue 175 also requires explicit cleanup-attempt
fields so consumers can accept a repair without guessing.

This repair adapts the retained PR 184 implementation to current main. It owns
only refresh reuse, canonical replacement persistence, duplicate selection and
response evidence. It does not revive the archived platform requirements or
change Remote View desktop allocation. PR 191 remains separate.

Acceptance:

1. A closed caller handle cannot adopt an unrelated blank or same-origin target.
2. Reuse returns the compatible target's canonical caller-bound handle.
3. A new replacement persists caller identity and is usable by subsequent actions.
4. Duplicate cleanup closes only compatible targets with matching canonical
   session, principal, work lease and profile-access custody.
5. Every structured refresh response reports duplicate and peer cleanup attempts.
6. A disposable real-browser workflow preserves the peer target while opening a
   replacement and closing a caller-owned duplicate.

Validation starts with a failing fixture exercising current main's actual
selection function, followed by focused Rust tests, client contracts, formatting,
strict workspace Clippy and the isolated browser workflow. Review Standards and
Spec separately against the pinned baseline and issue 175.

The clean checkout is the primary writer for these surfaces. The dirty primary
checkout and retained PR 184 branch are preserved. The open operator Facebook
profile, production services and other agents' desktops remain outside the test.
No reboot or memory write is authorized by this packet.
