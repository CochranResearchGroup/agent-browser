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
  /window\.location\.replace\(`\/api\/remote-view\/\$\{encodeURIComponent\(handoffId\)\}\/presentation`\)/,
  'authenticated handoffs must navigate directly through the current presentation endpoint',
);
const gate = dashboardPage.slice(dashboardPage.indexOf('function RemoteViewHandoffGate('),
  dashboardPage.indexOf('function DashboardSessionRestoreScreen('));
assert.doesNotMatch(gate, /fetch\(|setTimeout|Open desktop|writeDashboardWorkspaceUrlSelection/,
  'handoffs must not duplicate resolution, poll, select a dashboard workspace or require another click');

assert.match(
  streamStore,
  /dashboardStreamWebSocketUrl[\s\S]*\/api\/stream\/\$\{encodeURIComponent\(port\)\}[\s\S]*location\.protocol === "https:" \? "wss:" : "ws:"/,
  'external dashboard CDP streams must use the authenticated same-origin WebSocket proxy',
);

assert.match(streamStore, /export function useStreamSync\(port: number, enabled = true\)/);
assert.match(streamStore, /if \(!enabled\) return;[\s\S]*new WebSocket\(dashboardStreamWebSocketUrl\(port\)\)/);
assert.match(
  dashboardPage,
  /useStreamSync\(activePort, !hasWorkspaceViewportRoute\)/,
  'durable RDP and snapshot handoffs must not also start the legacy CDP stream socket',
);

assert.doesNotMatch(
  streamStore,
  /new WebSocket\(`ws:\/\/localhost:\$\{port\}`\)/,
  'the global dashboard stream hook must not hard-code a loopback WebSocket for external clients',
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
