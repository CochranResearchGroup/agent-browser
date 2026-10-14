# Failed-upgrade drain preservation evidence

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: P239 / #183
Related lanes: P237, no overlapping source edits
Baseline: 9bd54cfbc711c338dadaec7ad92f4fdd7dfc85b5

The installed release executable has SHA256
`8212e03ca26110f03537ddd409fca79a0922e9208a9edfea77fb9ed4edaf29a4`.
The fixture pins that executable and a reviewed Linux Chromium binary explicitly.
It allocates a disposable home, runtime directory, socket namespace and synthetic
persistent profile; it borrows no production state and uses no external provider.

Run `pnpm test:failed-upgrade-browser-preservation` with
`AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD` and
`AGENT_BROWSER_SMOKE_BROWSER_EXECUTABLE` set to absolute reviewed paths.
Run the underlying script with `--break-preservation` only for its deliberate
negative control; a nonzero result at exact browser preservation is expected.

The first attempt failed its executable oracle because the stable launcher
resolves to a generation binary. The revised check compares the actual executable
to the resolved pin and proves the fixture namespace through process environment.
The first attempt's remaining host was located by its exact fixture home and
Agent Browser home, with the reviewed executable rechecked before SIGTERM. The
second attempt passed browser PID and CDP preservation but failed its lock oracle:
Chromium's singleton lock is a dangling symlink, so `existsSync` cannot establish
its presence. The corrected test compares the exact symlink target, including
the browser PID. Neither failure established a product preservation defect.

The corrected positive check passes: a genuine active drain rejects new browser
work; the exact host exits in 29ms; browser PID/start time, CDP websocket and
profile-control target survive; owned endpoint metadata disappears; drain bytes
remain unchanged. After removing only the fixture drain, the same logical task
continues in the same browser. Explicit task close retires that exact browser.
The negative control fails at the precise browser-preservation assertion. A
fresh namespace/executable census proves zero owned processes after each final
run; production services and the foreign P237 checkout remain untouched.

The wider #183 operation remains unqualified. This script sends an exact graceful
owner signal under a drain; it does not execute a workstation shutdown, synthesize
a failed installer ledger, clear production leases or stop arbitrary generations.
