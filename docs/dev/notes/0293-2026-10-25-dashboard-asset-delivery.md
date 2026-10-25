# P293: Paired dashboard asset delivery measurement

Date: 2026-10-25
Product lane: PL-BUGFIX
Work item: https://github.com/CochranResearchGroup/agent-browser/issues/422
Disposition: diagnostic packet complete; issue remains open, symptom not reproduced.

## Measurement

Five serial paired rounds requested production HTML plus all16 directly referenced static resources:12 JavaScript chunks, two stylesheets and two fonts. The public and loopback order alternated by round. Each curl process used a fresh connection, compressed transfer, a five-second connection limit and a12-second total deadline. There were170 measured requests plus one loopback discovery read. Public negotiated HTTP/2; loopback used HTTP/1.1. No credentials or authentication changes were needed for these static resources.

All170 measured requests returned200 with nonempty content; each public/local pair had identical decompressed SHA256. Static MIME types were JavaScript, CSS and WOFF2, with immutable one-year cache headers. There were no redirects, transport failures or content mismatches. The production listener retained PID2373552 and kernel start24036340 before/after; its executing binary SHA256 remained `cf65dd76422b0d05d1c4332d431a6f2448128e8570bd8402b1d3420f793bd17c`.

| Metric | Loopback median / maximum | Public median / maximum |
| --- | --- | --- |
| First byte | 6.25 /119.51ms | 72.74 /1002.98ms |
| Body transfer | 0.66 /22.76ms | 7.02 /326.23ms |
| Total | 7.45 /123.59ms | 87.88 /1031.53ms |

The slowest public request was a font. TCP connect completed at943.42ms, TLS at967.09ms, first byte at1002.98ms and body completion at1031.53ms. This sample contains connection delay; it does not establish the cause of the original timeout or ChunkLoadError. Do not conflate per-resource timing with full dashboard rail or Native-viewer readiness.

## Scope and remaining evidence

This finite run did not reproduce P289's timeout. No repair is justified from this measurement. Keep422 open with its original evidence. Fresh serial curl connections do not model browser HTTP/2 multiplexing, browser cache state, dynamically imported resources, JavaScript execution or a post-install race. No application account, native desktop, browser process or service lifecycle was changed. No build, deployment, ingress edit or CI activation occurred.

The repeatable private diagnostic and structured timings are under `~/.local/state/agent-browser/p293-assets-20261025/`: `probe.py`, `requests.jsonl`, `summary.json`, `identity-before.json`, `identity-after.json` and `asset-paths.json`. Run `python3 ~/.local/state/agent-browser/p293-assets-20261025/probe.py` for another explicitly bounded five-round diagnostic; each run appends request history and replaces its latest summary. Raw state stays outside Git. The command exits nonzero for transport/HTTP failure, pair mismatch or serving-generation drift; its failure branch was not exercised in this successful run.

The next useful signal is a failed browser request's exact asset path and request/header/body stage, correlated with serving-generation identity. Preserve original failures and avoid guessing ingress, authentication or browser-engine causes. Standards/Spec review confirms the bounded read-only scope and explicit non-reproduction; this is evidence integration, not a claim that422 is fixed.
