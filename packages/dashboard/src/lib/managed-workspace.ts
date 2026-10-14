import type { ServiceManagedWorkspace } from '@agent-browser/client/service-observability';
import type { WorkspaceServiceBrowser, WorkspaceServiceSession, WorkspaceServiceTab } from './service-workspaces.ts';

/** Adapt the public owner snapshot without inventing viewer or child authority. */
export function managedWorkspaceSources(workspace?: ServiceManagedWorkspace): {
  browsers: WorkspaceServiceBrowser[]; sessions: WorkspaceServiceSession[]; tabs: WorkspaceServiceTab[];
} {
  if (!workspace || workspace.schemaVersion !== 'agent-browser.managed-workspace.v1' || workspace.source !== 'managed-owner') {
    return { browsers: [], sessions: [], tabs: [] };
  }
  const text = (value: unknown): string | null => typeof value === 'string' && value.trim() ? value : null;
  return {
    browsers: Object.values(workspace.browsers).map(browser => ({
      id: browser.id, profileId: browser.profileId, pid: browser.pid,
      displayAllocationId: browser.desktop?.desktop_id ?? null,
      displayName: browser.profileId,
      activeSessionIds: browser.activeSessionIds,
      health: browser.state === 'sleeping' ? 'not_started' : 'reported',
      // No operatorVisibleProof or controllable stream follows from inventory.
    })),
    sessions: Object.values(workspace.sessions).map(session => {
      const tabs = Object.values(workspace.tabs).filter(tab => tab.sessionId === session.id);
      const metadata = tabs.find(tab => tab.taskMetadata)?.taskMetadata;
      return {
        id: session.id, profileId: session.profileId, browserIds: [session.browserId],
        tabIds: tabs.map(tab => tab.id),
        serviceName: text(metadata?.serviceName), agentName: text(metadata?.agentName), taskName: text(metadata?.taskName),
      };
    }),
    tabs: Object.values(workspace.tabs).map(tab => ({
      id: tab.id, browserId: tab.browserId, targetId: tab.targetId,
      sessionId: tab.sessionId, ownerSessionId: workspace.sessions[tab.sessionId]?.name ?? null,
      lifecycle: 'ready', url: tab.url, title: null, taskLabel: text(tab.taskMetadata?.taskName),
    })),
  };
}
