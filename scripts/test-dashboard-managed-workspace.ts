import assert from 'node:assert/strict';
import { tabDescription } from '../packages/dashboard/src/lib/tab-description.ts';
import { managedWorkspaceSources } from '../packages/dashboard/src/lib/managed-workspace.ts';

const sources = managedWorkspaceSources({
  schemaVersion: 'agent-browser.managed-workspace.v1', source: 'managed-owner',
  browsers: { b: { id: 'b', profileId: 'p', pid: 42, desktop: { desktop_id: 'desktop-a', generation: 7, friendly_route_label: 'Research desktop' }, activeSessionIds: ['s'], state: 'reported' } },
  sessions: { s: { id: 's', name: 'alice', profileId: 'p', browserId: 'b', createdAtMs: 100, lastActivityAtMs: 100, expiresAtMs: 0, currentTabId: 't' } },
  tabs: { t: { id: 't', targetId: 'native-target', browserId: 'b', sessionId: 's', createdAtMs: 100, url: 'https://example.test/task', taskMetadata: { taskName: 'Research task', agentName: 'Supplied label' }, lifetime: null, pendingHuman: null } },
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
  schemaVersion: 'agent-browser.managed-workspace.v1', source: 'managed-owner',
  browsers: {}, sessions: {}, tabs: {
    t: { id: 't', targetId: 'native-target', browserId: 'b', sessionId: 's', createdAtMs: 100,
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
console.log('Managed owner inventory dashboard adapter checks passed');
