import type { ServiceManagedWorkspace } from '@agent-browser/client/service-observability';
import type { WorkspaceServiceBrowser, WorkspaceServiceSession, WorkspaceServiceTab } from './service-workspaces.ts';

/** A failed poll must not keep a previous live observation indefinitely. */
function currentPresence(workspace?: ServiceManagedWorkspace): boolean {
  const age = Date.now() - (workspace?.observedAtMs ?? Number.NaN);
  return Number.isFinite(age) && age >= 0 && age <= 30_000;
}

function tabPresence(workspace: ServiceManagedWorkspace, tab: ServiceManagedWorkspace['tabs'][string], current = currentPresence(workspace), activeIds?: ReadonlySet<string>): 'live' | 'absent' | 'unknown' {
  if (!current) return 'unknown';
  const browser = workspace.browsers[tab.browserId];
  const session = workspace.sessions[tab.sessionId];
  if (!browser || !session || session.browserId !== browser.id || session.profileId !== browser.profileId
    || !(activeIds ? activeIds.has(session.id) : browser.activeSessionIds.includes(session.id))) return 'unknown';
  if (browser.presence?.state === 'absent' || tab.presence?.state === 'absent') return 'absent';
  return browser.presence?.state === 'live' && tab.presence?.state === 'live' ? 'live' : 'unknown';
}

/** Choose a viewer from available presentation evidence, not managed ownership
 * alone. A retained tab without a desktop must not masquerade as a missing tab
 * or acquire a different browser's stream. */
export function managedWorkspaceViewport(workspace: ServiceManagedWorkspace | undefined, browserId: string | null, tabId: string | null, selectedStreamAvailable: boolean, current = currentPresence(workspace)) {
  const browser = browserId ? workspace?.browsers[browserId] : null;
  const tab = tabId ? workspace?.tabs[tabId.replace(/^target:/, '')] : null;
  const session = tab ? workspace?.sessions[tab.sessionId] : null;
  const knownTab = tab?.browserId === browser?.id && session?.browserId === browser?.id;
  const presence = workspace && tab ? tabPresence(workspace, tab, current) : 'unknown';
  const presenceUnavailable = Boolean(browser && knownTab && presence !== 'live');
  const target = presence === 'live' && browser?.desktop && tab?.browserId === browser.id && session?.browserId === browser.id
    ? { browserId: browser.id, sessionName: session.name, targetId: tab.targetId, displayAllocationId: browser.desktop.desktop_id }
    : null;
  const desktopUnavailable = Boolean(!presenceUnavailable && browser && !browser.desktop && tab?.browserId === browser.id && session?.browserId === browser.id && !selectedStreamAvailable);
  return { target, desktopUnavailable, presenceUnavailable: presenceUnavailable && presence !== 'live' ? presence : null, useNativeViewport: Boolean((browser && (browser.desktop || presenceUnavailable || !selectedStreamAvailable)) || (!browserId && !tabId)) };
}

/** Adapt the public owner snapshot without inventing viewer or child authority. */
export function managedWorkspaceSources(workspace?: ServiceManagedWorkspace, current = currentPresence(workspace)): {
  browsers: WorkspaceServiceBrowser[]; sessions: WorkspaceServiceSession[]; tabs: WorkspaceServiceTab[];
} {
  if (!workspace || workspace.schemaVersion !== 'agent-browser.managed-workspace.v1' || workspace.source !== 'managed-owner') {
    return { browsers: [], sessions: [], tabs: [] };
  }
  const text = (value: unknown): string | null => typeof value === 'string' && value.trim() ? value : null;
  const activeIds = new Map(Object.values(workspace.browsers).map(browser => [browser.id, new Set(browser.activeSessionIds)]));
  const tabsBySession = new Map<string, ServiceManagedWorkspace['tabs'][string][]>();
  for (const tab of Object.values(workspace.tabs)) {
    const group = tabsBySession.get(tab.sessionId) ?? [];
    group.push(tab); tabsBySession.set(tab.sessionId, group);
  }
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
      const tabs = tabsBySession.get(session.id) ?? [];
      const metadata = tabs.find(tab => tab.taskMetadata)?.taskMetadata;
      return {
        id: session.id, profileId: session.profileId, browserIds: [session.browserId],
        tabIds: tabs.map(tab => tab.id),
        serviceName: text(metadata?.serviceName), agentName: text(metadata?.agentName), taskName: text(metadata?.taskName),
      };
    }),
    tabs: Object.values(workspace.tabs).map(tab => {
      const presence = tabPresence(workspace, tab, current, activeIds.get(tab.browserId));
      return {
        managedTask: true,
        id: tab.id, browserId: tab.browserId, targetId: tab.targetId,
        sessionId: tab.sessionId, ownerSessionId: workspace.sessions[tab.sessionId]?.name ?? null,
        presence: { state: presence },
        lifecycle: presence === 'live' ? 'ready' : presence === 'absent' ? 'expired' : 'unknown', url: tab.url, title: text(tab.title), taskLabel: text(tab.taskMetadata?.taskName),
        taskMetadata: tab.taskMetadata,
        profileId: workspace.browsers[tab.browserId]?.profileId ?? null,
        desktopId: workspace.browsers[tab.browserId]?.desktop?.desktop_id ?? null,
        inventoryState: workspace.browsers[tab.browserId]?.state ?? null,
      };
    }),
  };
}
