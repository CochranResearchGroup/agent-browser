import assert from 'node:assert/strict';
import { tabDescription } from '../packages/dashboard/src/lib/tab-description.ts';
import { managedWorkspaceSources, managedWorkspaceViewport } from '../packages/dashboard/src/lib/managed-workspace.ts';

const sources = managedWorkspaceSources({
  schemaVersion: 'agent-browser.managed-workspace.v1', source: 'managed-owner', observedAtMs: Date.now(),
  browsers: { b: { id: 'b', profileId: 'p', pid: 42, presence: { state: 'live' }, desktop: { desktop_id: 'desktop-a', generation: 7, friendly_route_label: 'Research desktop' }, activeSessionIds: ['s'], state: 'reported' } },
  sessions: { s: { id: 's', name: 'alice', profileId: 'p', browserId: 'b', createdAtMs: 100, lastActivityAtMs: 100, expiresAtMs: 0, currentTabId: 't' } },
  tabs: { t: { id: 't', targetId: 'native-target', presence: { state: 'live' }, browserId: 'b', sessionId: 's', createdAtMs: 100, url: 'https://example.test/task', taskMetadata: { taskName: 'Research task', agentName: 'Supplied label' }, lifetime: null, pendingHuman: null } },
});
assert.equal(sources.browsers[0].id, 'b');
assert.equal(sources.browsers[0].displayAllocationId, 'desktop-a');
assert.equal(sources.browsers[0].operatorVisibleProof, undefined, 'owner inventory must not invent native viewer readiness');
assert.equal(sources.browsers[0].cdpEndpoint, undefined);
assert.deepEqual(sources.browsers[0].activeSessionIds, ['s']);
assert.equal(sources.tabs[0].targetId, 'native-target');
assert.equal(sources.tabs[0].sessionId, 's');
assert.equal(sources.tabs[0].ownerSessionId, 'alice', 'native focus routes by the exact owning lane name');
assert.equal(sources.tabs[0].title, null, 'a supplied task label is not the current page title');
assert.deepEqual(sources.tabs[0].taskMetadata, { taskName: 'Research task', agentName: 'Supplied label' });
assert.equal(sources.tabs[0].profileId, 'p');
assert.equal(sources.tabs[0].desktopId, 'desktop-a');
assert.equal(sources.tabs[0].inventoryState, 'reported', 'owner inventory must not imply native viewer readiness');
assert.equal(sources.sessions[0].agentName, 'Supplied label');
assert.deepEqual(managedWorkspaceSources(undefined), { browsers: [], sessions: [], tabs: [] });
const titled = managedWorkspaceSources({
  schemaVersion: 'agent-browser.managed-workspace.v1', source: 'managed-owner', observedAtMs: Date.now(),
  browsers: {}, sessions: {}, tabs: {
    t: { id: 't', targetId: 'native-target', presence: { state: 'live' }, browserId: 'b', sessionId: 's', createdAtMs: 100,
      url: 'data:text/html,<title>P237 installed dev inspection</title>',
      title: 'P237 installed dev inspection', taskMetadata: null, lifetime: null, pendingHuman: null },
  },
});
assert.equal(titled.tabs[0].title, 'P237 installed dev inspection', 'preserve the owner-observed title instead of exposing HTML as a tab description');
assert.equal(tabDescription(titled.tabs[0]), 'P237 installed dev inspection');
assert.equal(tabDescription({url:'data:text/html,<title>Untrusted</title>'}), 'HTML document');
assert.equal(tabDescription({url:'data:application/pdf;base64,private-payload'}), 'Data document');
assert.equal(tabDescription({url:'blob:https://example.test/private-id'}), 'Browser document');
assert.equal(tabDescription({taskLabel:'Review task',url:'data:text/html,code'}), 'Review task');
assert.equal(tabDescription({title:'Observed title',taskLabel:'Supplied task'}), 'Observed title');
assert.equal(tabDescription({}), 'Untitled tab');

const retained = {
  schemaVersion: 'agent-browser.managed-workspace.v1', source: 'managed-owner', observedAtMs: Date.now(),
  browsers: { b: { id: 'b', profileId: 'p', pid: 42, presence: { state: 'live' }, desktop: null, activeSessionIds: ['s'], state: 'reported' } },
  sessions: { s: { id: 's', name: 'alice', profileId: 'p', browserId: 'b' } },
  tabs: { t: { id: 't', targetId: 'exact-target', presence: { state: 'live' }, browserId: 'b', sessionId: 's' } },
} as any;
assert.deepEqual(managedWorkspaceViewport(retained, 'b', 't', false), { target: null, desktopUnavailable: true, presenceUnavailable: null, useNativeViewport: true }, 'present retained tab without a desktop needs availability guidance, not missing-tab feedback');
assert.deepEqual(managedWorkspaceViewport(retained, 'b', 't', true), { target: null, desktopUnavailable: false, presenceUnavailable: null, useNativeViewport: false }, 'an available stream remains usable for a browser without a desktop');
assert.deepEqual(managedWorkspaceViewport(retained, 'b', 'missing', false), { target: null, desktopUnavailable: false, presenceUnavailable: null, useNativeViewport: true }, 'a missing tab without a desktop still needs missing-target guidance');
retained.browsers.b.desktop = { desktop_id: 'owned-desktop', generation: 1 };
assert.deepEqual(managedWorkspaceViewport(retained, 'b', 'target:t', true).target, { browserId: 'b', sessionName: 'alice', targetId: 'exact-target', displayAllocationId: 'owned-desktop' });
assert.equal(managedWorkspaceViewport(retained, 'b', 't', true).useNativeViewport, true, 'native desktop takes precedence over compatibility stream');
assert.equal(managedWorkspaceViewport(retained, 'b', 'missing', false).desktopUnavailable, false, 'missing selected tab remains a distinct error');
retained.sessions.s.browserId = 'peer';
assert.equal(managedWorkspaceViewport(retained, 'b', 't', true).target, null, 'never route a tab through a different session owner');

retained.sessions.s.browserId = 'b';
retained.browsers.b.presence.state = 'absent';
assert.equal(managedWorkspaceSources(retained).tabs[0].presence?.state, 'absent');
assert.equal(managedWorkspaceViewport(retained, 'b', 't', true).target, null, 'stale selection cannot open a dead browser');
assert.equal(managedWorkspaceViewport(retained, 'b', 't', true).presenceUnavailable, 'absent');
retained.browsers.b.presence.state = 'live';
retained.tabs.t.presence.state = 'unknown';
assert.equal(managedWorkspaceSources(retained).tabs[0].lifecycle, 'unknown');
assert.equal(managedWorkspaceViewport(retained, 'b', 't', true).presenceUnavailable, 'unknown');
retained.tabs.t.presence.state = 'live';
retained.observedAtMs = Date.now() - 31_000;
assert.equal(managedWorkspaceSources(retained).tabs[0].presence?.state, 'unknown', 'stale successful polls must not leave ghost tabs live');
assert.equal(managedWorkspaceViewport(retained, 'b', 't', true).target, null);
retained.observedAtMs = Date.now();
assert.equal(managedWorkspaceSources(retained).tabs[0].lifecycle, 'ready', 'fresh success restores exact existing target without recreation');
assert.equal(managedWorkspaceViewport(retained, 'b', 't', true).target?.targetId, 'exact-target');
retained.browsers.b.activeSessionIds = [];
assert.equal(managedWorkspaceSources(retained, true).tabs[0].presence?.state, 'unknown', 'an inactive session must not revive a tab');
retained.browsers.b.activeSessionIds = ['s'];
retained.sessions.s.profileId = 'peer-profile';
assert.equal(managedWorkspaceViewport(retained, 'b', 't', true, true).target, null, 'a profile mismatch must not open an apparently live target');
console.log('Managed owner inventory dashboard adapter checks passed');
