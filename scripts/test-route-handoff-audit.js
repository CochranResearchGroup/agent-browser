#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { buildAudit } from './audit-route-handoff.js';
import { assert, parseJsonOutput } from './smoke-utils.js';

const tempDir = mkdtempSync(join(tmpdir(), 'agent-browser-route-handoff-audit-'));
const fixturePath = join(tempDir, 'fixture.json');

function browser(id, fields = {}) {
  return {
    id,
    health: 'ready',
    host: 'remote_headed',
    profileId: `${id}-profile`,
    tabHandles: [],
    viewStreams: [],
    ...fields,
  };
}

const fixture = {
  serviceStatus: {
    success: true,
    data: {
      service_state: {
        browsers: {
          'session:facebook': browser('session:facebook', {
            displayName: ':11',
            displayAllocationId: 'remote-view-display:11',
            tabHandles: [
              {
                tabId: 'target:facebook',
                title: 'Facebook',
                url: 'https://www.facebook.com/',
              },
            ],
            viewStreams: [
              {
                id: 'facebook-stream',
                provider: 'rdp_gateway',
                displayAllocationId: 'remote-view-display:11',
                routeId: 'guacamole:3',
                url: 'https://agent-browser.example/guacamole/',
                controlInput: 'manual_attached_desktop',
              },
            ],
          }),
          'session:litscout': browser('session:litscout', {
            displayName: ':93',
            displayAllocationId: 'display:private_virtual_display:session-litscout',
            tabHandles: [
              {
                tabId: 'target:localhost',
                title: '127.0.0.1',
                url: 'http://127.0.0.1:37525/',
              },
            ],
            viewStreams: [
              {
                id: 'litscout-stream',
                provider: 'rdp_gateway',
                displayAllocationId: 'display:private_virtual_display:session-litscout',
                url: 'https://agent-browser.example/guacamole/',
                controlInput: 'manual_attached_desktop',
              },
            ],
          }),
          'session:terminal': browser('session:terminal', {
            displayName: ':12',
            displayAllocationId: 'remote-view-display:12',
            tabHandles: [
              {
                tabId: 'target:terminal',
                title: 'Terminal route',
                url: 'https://example.com/',
              },
            ],
            viewStreams: [
              {
                id: 'terminal-stream',
                provider: 'rdp_gateway',
                displayAllocationId: 'remote-view-display:12',
                routeId: 'guacamole:4',
                url: 'https://agent-browser.example/guacamole/',
                controlInput: 'manual_attached_desktop',
              },
            ],
          }),
          'session:stale': browser('session:stale', {
            health: 'process_exited',
            displayAllocationId: 'remote-view-display:stale',
            tabHandles: [
              {
                tabId: 'target:stale',
                title: 'Old tab',
                url: 'https://stale.example/',
              },
            ],
          }),
          'foreign:auracall': browser('foreign:auracall', {
            detected: true,
            ownership: 'foreign_cdp',
            host: 'local_headed',
            profileId: 'auracall-chatgpt',
            tabHandles: [
              {
                tabId: 'target:auracall',
                title: 'ChatGPT',
                url: 'https://chatgpt.com/',
              },
            ],
          }),
        },
        tabs: {},
        displayAllocations: {
          'remote-view-display:11': {
            id: 'remote-view-display:11',
            state: 'ready',
            displayName: ':11',
            ownerBrowserId: 'session:facebook',
          },
          'display:private_virtual_display:session-litscout': {
            id: 'display:private_virtual_display:session-litscout',
            state: 'ready',
            displayName: ':93',
            ownerBrowserId: 'session:litscout',
          },
          'remote-view-display:12': {
            id: 'remote-view-display:12',
            state: 'ready',
            displayName: ':12',
            ownerBrowserId: 'session:terminal',
          },
          'remote-view-display:stale': {
            id: 'remote-view-display:stale',
            state: 'released',
            ownerBrowserId: 'session:stale',
          },
        },
        remoteViewRoutes: {
          'guacamole:3': {
            id: 'guacamole:3',
            state: 'ready',
            browserId: 'session:facebook',
            displayAllocationId: 'remote-view-display:11',
            connectionId: '3',
            provider: 'rdp_gateway',
            readiness: {
              state: 'ready',
              displayContent: {
                state: 'browser_window_visible',
                displayName: ':11',
                windows: [
                  { className: 'Chrome', title: 'Facebook - Chromium' },
                  { className: 'XTerm', title: 'agent-browser-rdp-a@host: ~' },
                ],
              },
            },
          },
          'guacamole:4': {
            id: 'guacamole:4',
            state: 'ready',
            browserId: 'session:terminal',
            displayAllocationId: 'remote-view-display:12',
            connectionId: '4',
            provider: 'rdp_gateway',
            readiness: {
              state: 'ready',
              displayContent: {
                state: 'terminal_only',
                displayName: ':12',
                windows: [
                  { className: 'XTerm', title: 'agent-browser-rdp-b@host: ~' },
                ],
              },
            },
          },
        },
        routePool: {
          'guacamole-rdp-a': {
            id: 'guacamole-rdp-a',
            routeId: 'guacamole:3',
            currentRouteAllocationId: 'guacamole:3',
            connectionId: '3',
          },
          'guacamole-rdp-b': {
            id: 'guacamole-rdp-b',
            routeId: 'guacamole:4',
            currentRouteAllocationId: 'guacamole:4',
            connectionId: '4',
          },
        },
        viewerLeases: {
          'viewer:guacamole:3:default': {
            id: 'viewer:guacamole:3:default',
            routeId: 'guacamole:3',
            browserId: 'session:facebook',
          },
        },
      },
    },
  },
  remoteViewDoctor: {
    success: true,
    data: {
      runtimeConvergence: { status: 'converged' },
      runtimeInventory: { status: 'converged', runtimeCount: 1, staleCount: 0 },
      remoteControl: { status: 'ready', ready: true },
    },
  },
};

function rowByBrowser(audit, browserId) {
  return audit.rows.find((row) => row.browserId === browserId);
}

try {
  const audit = buildAudit({
    source: { kind: 'fixture', path: fixturePath },
    serviceStatus: fixture.serviceStatus,
    remoteViewDoctor: fixture.remoteViewDoctor,
    collectionErrors: [],
  });

  assert(rowByBrowser(audit, 'session:facebook')?.classification === 'route_bound_ready', 'Facebook row should be route_bound_ready');
  assert(rowByBrowser(audit, 'session:litscout')?.classification === 'direct_remote_headed', 'LitScout row should be direct_remote_headed');
  assert(rowByBrowser(audit, 'session:terminal')?.classification === 'route_bound_terminal_only', 'Terminal row should be route_bound_terminal_only');
  assert(rowByBrowser(audit, 'session:stale')?.classification === 'stale_or_retained', 'Stale row should be stale_or_retained');
  assert(rowByBrowser(audit, 'foreign:auracall')?.classification === 'foreign_cdp', 'Foreign row should be foreign_cdp');

  writeFileSync(fixturePath, `${JSON.stringify(fixture, null, 2)}\n`);
  const result = spawnSync(process.execPath, ['scripts/audit-route-handoff.js', '--fixture', fixturePath, '--json'], {
    encoding: 'utf8',
    stdio: 'pipe',
  });
  assert(result.status === 0, `fixture CLI failed: ${result.stdout}${result.stderr}`);
  const cliAudit = parseJsonOutput(result.stdout, 'route handoff audit fixture CLI');
  assert(cliAudit.success === true, `fixture CLI did not report success: ${result.stdout}`);
  assert(cliAudit.data?.rows?.length === 5, `fixture CLI returned wrong row count: ${result.stdout}`);
  assert(
    cliAudit.data?.summary?.route_bound_terminal_only === 1,
    `fixture CLI did not summarize terminal-only row: ${result.stdout}`,
  );

  console.log('route handoff audit fixture passed');
} finally {
  rmSync(tempDir, { recursive: true, force: true });
}
