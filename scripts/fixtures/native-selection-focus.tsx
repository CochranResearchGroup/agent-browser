import React, { useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { WorkspaceSelectionPanel } from '@/components/workspace-selection-panel';
import { buildSelectedWorkspaceContext } from '@/lib/selected-workspace-context';
import { managedWorkspaceSources } from '@/lib/managed-workspace';
import type { ServiceManagedWorkspace } from '@agent-browser/client/service-observability';

/** Actual-provider acceptance: inventory and Focus both use the real owner HTTP API. */
function Fixture() {
  const [workspace, setWorkspace] = useState<ServiceManagedWorkspace>();
  const [target, setTarget] = useState('');
  const [error, setError] = useState('');
  async function refresh() {
    try {
      const response = await fetch('/api/service/status');
      const payload = await response.json();
      if (!response.ok || !payload.success) throw Error('Owner inventory unavailable');
      setWorkspace(payload.data.browserRuntime.workspace);
    } catch { setError('Owner inventory unavailable'); }
  }
  useEffect(() => { void refresh(); }, []);
  if (!workspace) return <p>{error || 'Loading actual owner inventory'}</p>;
  const tabs = Object.values(workspace.tabs).filter(tab => tab.presence?.state === 'live');
  const tab = tabs.find(item => item.targetId === target) ?? tabs[0];
  if (!tab) return <p>No current owned tab</p>;
  const session = workspace.sessions[tab.sessionId];
  const browser = workspace.browsers[tab.browserId];
  const selection = { workspaceId: `browser:${browser.id}`, browserId: browser.id,
    sessionId: session.id, tabId: tab.id, profileId: browser.profileId, jobId: null };
  const sources = managedWorkspaceSources(workspace);
  const context = buildSelectedWorkspaceContext({ selection, serviceBrowsers: sources.browsers,
    serviceSessions: sources.sessions, serviceTabs: sources.tabs });
  return <main><nav>{tabs.map(item => <button key={item.id} onClick={() => { setTarget(item.targetId); void refresh(); }}>Inspect {item.title}</button>)}</nav>
    <WorkspaceSelectionPanel context={context} managedWorkspace={workspace} presenceCurrent={true} onRefresh={refresh} />
  </main>;
}
createRoot(document.getElementById('root')!).render(<React.StrictMode><Fixture /></React.StrictMode>);
