import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { WorkspaceSelectionPanel } from '@/components/workspace-selection-panel';
import { buildSelectedWorkspaceContext } from '@/lib/selected-workspace-context';
import { managedWorkspaceSources } from '@/lib/managed-workspace';
import type { ServiceManagedWorkspace } from '@agent-browser/client/service-observability';

function Fixture() {
  const [mode, select] = useState('B');
  const workspace = {
    schemaVersion: 'agent-browser.managed-workspace.v1', source: 'managed-owner', observedAtMs: Date.now(),
    browsers: { browser: { id: 'browser', profileId: 'profile', pid: 123, state: 'ready', presence: { state: 'live' }, activeSessionIds: ['session'], desktop: { desktop_id: 'desktop' } } },
    sessions: { session: { id: 'session', name: 'owned-task', browserId: 'browser', profileId: 'profile' } },
    tabs: { A: { id: 'A', browserId: 'browser', sessionId: 'session', targetId: 'target-A', title: 'First tab', presence: { state: 'live' } }, B: { id: 'B', browserId: 'browser', sessionId: 'session', targetId: 'target-B', title: 'Second tab', presence: { state: 'live' } } },
  } as unknown as ServiceManagedWorkspace;
  if (mode === 'stale') workspace.observedAtMs -= 31_000;
  if (mode === 'foreign') workspace.tabs.B.browserId = 'peer';
  if (mode === 'unknown') workspace.tabs.B.presence = { state: 'unknown' };
  if (mode === 'absent') workspace.tabs.B.presence = { state: 'absent' };
  const selection = { workspaceId: 'browser:browser', browserId: 'browser', sessionId: 'session', tabId: mode === 'A' ? 'A' : mode === 'missing' ? 'missing' : 'B', profileId: mode === 'wrong-profile' ? 'peer' : 'profile', jobId: null };
  const sources = managedWorkspaceSources(workspace);
  const context = buildSelectedWorkspaceContext({ selection, serviceBrowsers: sources.browsers, serviceSessions: sources.sessions, serviceTabs: sources.tabs });
  // Model the current backend-advertised Focus action; its presence must not
  // bypass the component's exact current owner/target checks.
  context.actions = [{ id: 'focus', label: 'Focus', enabled: true }];
  return <main><nav>{['A', 'B', 'missing', 'foreign', 'stale', 'unknown', 'absent', 'wrong-profile'].map(value => <button key={value} onClick={() => select(value)}>Select {value}</button>)}</nav>
    <WorkspaceSelectionPanel context={context} managedWorkspace={workspace} presenceCurrent={true} />
  </main>;
}
const root = createRoot(document.getElementById('root')!);
root.render(<React.StrictMode><Fixture /></React.StrictMode>);
Object.assign(window, { dispose: () => root.unmount() });
