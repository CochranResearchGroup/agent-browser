# Service Client Example

This example shows the software-client workflow for agent-browser service mode:

- request one intent-based service tab with `requestServiceTab`
- read the matching service trace with `getServiceTrace`
- optionally cancel a queued job with `cancelServiceJob`
- keep `serviceName`, `agentName`, and `taskName` attached to both calls

## Dry Run

```bash
pnpm --filter agent-browser-service-client-example dry-run
```

The dry run validates imports and prints the request and trace query without
contacting a running agent-browser service.

The repo-level live smoke validates the same example against an isolated
daemon and browser session:

```bash
pnpm test:service-client-example-live
```

## Live Run

Start or identify an agent-browser stream port, then pass it as the base URL:

```bash
pnpm --filter agent-browser-service-client-example exec node service-request-trace.mjs \
  --base-url http://127.0.0.1:<stream-port> \
  --url https://example.com \
  --service-name JournalDownloader \
  --agent-name article-probe-agent \
  --task-name probeACSwebsite \
  --site-id example \
  --login-id example
```

You can also set `AGENT_BROWSER_SERVICE_BASE_URL` instead of passing
`--base-url`.

The script prints the command result, trace counts, and the latest retained
jobs so software projects can confirm that the request and trace metadata are
connected.

Pass `--cancel-job-id <job-id>` when your software already knows a queued job
that should be cancelled. The script calls `cancelServiceJob` and prints the
cancellation result alongside the tab request and trace output. Use
`pnpm test:service-job-naming-live` for the repo-local live smoke that creates
and cancels a queued job end to end.
