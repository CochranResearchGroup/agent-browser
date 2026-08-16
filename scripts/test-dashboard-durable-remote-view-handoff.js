#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const dashboardPage = readFileSync('packages/dashboard/src/app/page.tsx', 'utf8');
const viewport = readFileSync('packages/dashboard/src/components/workspace-remote-viewport.tsx', 'utf8');
const viewPreferences = readFileSync('packages/dashboard/src/hooks/use-workspace-view-preferences.ts', 'utf8');
const coordinator = readFileSync('cli/src/native/remote_view/open/coordinator.rs', 'utf8');
const handoff = readFileSync('cli/src/native/remote_view_handoff.rs', 'utf8');

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

console.log('dashboard durable remote-view handoff checks passed');
