import type { ServiceManagedWorkspace } from '@agent-browser/client/service-observability';
import type { WorkspaceServiceBrowser, WorkspaceServiceSession, WorkspaceServiceTab } from './service-workspaces.ts';

/** A failed poll must not keep a previous live observation indefinitely. */
function currentPresence(workspace?: ServiceManagedWorkspace): boolean {
  const age = Date.now() - (workspace?.observedAtMs ?? Number.NaN);
  return Number.isFinite(age) && age >= 0 && age <= 30_000;
}

function tabPresence(workspace: ServiceManagedWorkspace, tab: ServiceManagedWorkspace['tabs'][string]): 'live' | 'absent' | 'unknown' {
  if (!currentPresence(workspace)) return 'unknown';
  const browser = workspace.browsers[tab.browserId];
  const session = workspace.sessions[tab.sessionId];
  if (!browser || !session || session.browserId !== browser.id) return 'unknown';
  if (browser.presence?.state === 'absent' || tab.presence?.state === 'absent') return 'absent';
  return browser.presence?.state === 'live' && tab.presence?.state === 'live' ? 'live' : 'unknown';
}

/** Choose a viewer from available presentation evidence, not managed ownership
 * alone. A retained tab without a desktop must not masquerade as a missing tab
 * or acquire a different browser's stream. */
export function managedWorkspaceViewport(workspace: ServiceManagedWorkspace | undefined, browserId: string | null, tabId: string | null, selectedStreamAvailable: boolean) {
  const browser = browserId ? workspace?.browsers[browserId] : null;
  const tab = tabId ? workspace?.tabs[tabId.replace(/^target:/, '')] : null;
  const session = tab ? workspace?.sessions[tab.sessionId] : null;
  const knownTab = tab?.browserId === browser?.id && session?.browserId === browser?.id;
  const presence = workspace && tab ? tabPresence(workspace, tab) : 'unknown';
  const presenceUnavailable = Boolean(browser && knownTab && presence !== 'live');
  const target = presence === 'live' && browser?.desktop && tab?.browserId === browser.id && session?.browserId === browser.id
    ? { browserId: browser.id, sessionName: session.name, targetId: tab.targetId, displayAllocationId: browser.desktop.desktop_id }
    : null;
  const desktopUnavailable = Boolean(!presenceUnavailable && browser && !browser.desktop && tab?.browserId === browser.id && session?.browserId === browser.id && !selectedStreamAvailable);
  return { target, desktopUnavailable, presenceUnavailable: presenceUnavailable && presence !== 'live' ? presence : null, useNativeViewport: Boolean((browser && (browser.desktop || presenceUnavailable || !selectedStreamAvailable)) || (!browserId && !tabId)) };
}

/** Adapt the public owner snapshot without inventing viewer or child authority. */
export function managedWorkspaceSources(workspace?: ServiceManagedWorkspace): {
  browsers: WorkspaceServiceBrowser[]; sessions: WorkspaceServiceSession[]; tabs: WorkspaceServiceTab[];
} {
  if (!workspace || workspace.schemaVersion !== 'agent-browser.managed-workspace.v1' || workspace.source !== 'managed-owner') {
    return { browsers: [], sessions: [], tabs: [] };
  }
  const current = currentPresence(workspace);
  const text = (value: unknown): string | null => typeof value === 'string' && value.trim() ? value : null;
  return {
    browsers: Object.values(workspace.browsers).map(browser => ({
      id: browser.id, profileId: browser.profileId, pid: browser.pid,
      displayAllocationId: browser.desktop?.desktop_id ?? null,
      displayName: browser.profileId,
      activeSessionIds: browser.activeSessionIds,
      health: browser.state === 'sleeping' || browser.presence?.state === 'absent' ? 'not_started' : current && browser.presence?.state === 'live' ? 'reported' : 'unknown',
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
      managedTask: true,
      id: tab.id, browserId: tab.browserId, targetId: tab.targetId,
      sessionId: tab.sessionId, ownerSessionId: workspace.sessions[tab.sessionId]?.name ?? null,
      presence: { state: tabPresence(workspace, tab) },
      lifecycle: tabPresence(workspace, tab) === 'live' ? 'ready' : tabPresence(workspace, tab) === 'absent' ? 'expired' : 'unknown', url: tab.url, title: text(tab.title), taskLabel: text(tab.taskMetadata?.taskName),
      taskMetadata: tab.taskMetadata,
      profileId: workspace.browsers[tab.browserId]?.profileId ?? null,
      desktopId: workspace.browsers[tab.browserId]?.desktop?.desktop_id ?? null,
      inventoryState: workspace.browsers[tab.browserId]?.state ?? null,
    })),
  };
}
