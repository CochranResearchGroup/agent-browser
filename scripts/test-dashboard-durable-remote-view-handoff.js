#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const dashboardPage = readFileSync('packages/dashboard/src/app/page.tsx', 'utf8');
const streamStore = readFileSync('packages/dashboard/src/store/stream.ts', 'utf8');
const viewport = readFileSync('packages/dashboard/src/components/workspace-remote-viewport.tsx', 'utf8');
const viewPreferences = readFileSync('packages/dashboard/src/hooks/use-workspace-view-preferences.ts', 'utf8');
const coordinator = readFileSync('cli/src/native/remote_view/open/coordinator.rs', 'utf8');
const handoff = readFileSync('cli/src/native/remote_view_handoff.rs', 'utf8');
const developmentProvider = readFileSync('scripts/lib/development-presentation-provider.js', 'utf8');
const developmentProviderDeployment = readFileSync(
  'scripts/lib/development-presentation-provider-deployment.js',
  'utf8',
);

assert.match(
  dashboardPage,
  /remoteViewHandoffIdFromPath\([\s\S]*remote-view[\s\S]*function RemoteViewHandoffGate\([\s\S]*action: "service_remote_view_handoff_resolve"/,
  'authenticated dashboard routes must resolve opaque remote-view handoff IDs through the service queue',
);

assert.match(
  dashboardPage,
  /next\.startsWith\("\/guacamole\/"\)[\s\S]*window\.location\.assign\(next\)/,
  'post-auth forwarding to a direct Guacamole path must perform a real navigation',
);

assert.match(
  dashboardPage,
  /resolveHandoff\(true\)[\s\S]*Reopen tab/,
  'a deliberately closed handoff target must require an explicit reopen action',
);

assert.match(
  dashboardPage,
  /Remote view unavailable[\s\S]*resolveHandoff\(false\)[\s\S]*Retry/,
  'a transient provider reacquisition failure must keep the durable handoff retryable',
);

assert.match(
  dashboardPage,
  /service_remote_view_handoff_resolve[\s\S]*serviceStateLockTimeoutMs: 30_000[\s\S]*jobTimeoutMs: 90_000/,
  'durable handoff resolution must reserve a bounded Service State contention budget inside its job timeout',
);

assert.match(
  dashboardPage,
  /presentationMayStillConverge[\s\S]*nextResolution\.status === "ready"[\s\S]*nextResolution\.status === "converging"[\s\S]*status: "converging"/,
  'an incoherent ready response must enter the bounded presentation retry loop',
);

assert.match(
  streamStore,
  /dashboardStreamWebSocketUrl[\s\S]*\/api\/stream\/\$\{encodeURIComponent\(port\)\}[\s\S]*location\.protocol === "https:" \? "wss:" : "ws:"/,
  'external dashboard CDP streams must use the authenticated same-origin WebSocket proxy',
);

assert.doesNotMatch(
  streamStore,
  /new WebSocket\(`ws:\/\/localhost:\$\{port\}`\)/,
  'the global dashboard stream hook must not hard-code a loopback WebSocket for external clients',
);

assert.match(
  dashboardPage,
  /params\.set\("view-provider", nextResolution\.viewStreamProvider\)[\s\S]*params\.set\("view", "workspace:control"\)/,
  'successful handoff resolution must preserve the intended view provider and open workspace control',
);

assert.match(
  dashboardPage,
  /durableHandoffPresentationReady\([\s\S]*presentationGeneration[\s\S]*dashboardDeploymentGeneration[\s\S]*logicalBrowserId[\s\S]*daemonOwnerGeneration[\s\S]*processInstanceDigest[\s\S]*requiredStreamProvider[\s\S]*observedStreamProvider[\s\S]*state === "ready"/,
  'the dashboard must require a matching authenticated presentation generation before rendering',
);

assert.doesNotMatch(
  dashboardPage,
  /providerFallbackUrl[\s\S]*window\.location\.assign/,
  'durable handoff resolution must never redirect to an unverified provider route',
);

assert.doesNotMatch(
  coordinator,
  /providerFallbackUrl|ProviderFallback|provider_fallback/,
  'the durable resolver must not expose a raw-provider fallback outcome',
);

assert.match(
  handoff,
  /durableResolutionMode[\s\S]*reacquire_only[\s\S]*preferredTargetId/,
  'normal durable resolution must select the retained target in reacquire-only mode',
);

assert.match(
  handoff,
  /for key in \[[\s\S]*"url"[\s\S]*"routePoolEntryId"/,
  'normal durable resolution must remove stored navigation and ephemeral route selectors',
);

assert.match(
  dashboardPage,
  /status: "converging"[\s\S]*window\.setTimeout\(\(\) => void resolveHandoff\(false\), 1_000\)/,
  'a missing or stale presentation receipt must remain on the durable URL and retry convergence',
);

assert.match(
  viewPreferences,
  /get\("view-provider"\)[\s\S]*selectedProvider[\s\S]*readSelectedProvider/,
  'workspace control must carry the provider encoded by the durable handoff route into view preferences',
);

assert.match(
  developmentProvider,
  /publicOperatorUrl:\s*externalIngress\.publicOperatorUrl/,
  'the development provider must source its operator URL from reviewed external ingress',
);

assert.doesNotMatch(
  developmentProvider,
  /publicOperatorUrl:\s*`http:\/\/127\.0\.0\.1/,
  'the development provider must never expose loopback as a public operator URL',
);

assert.match(
  developmentProviderDeployment,
  /externalIngressBindingSha256:\s*descriptor\.externalIngress\.bindingSha256/,
  'development route authority must retain the deterministic external-ingress binding',
);

console.log('dashboard durable remote-view handoff checks passed');
