import { requestServiceRemoteViewHandoff } from '@agent-browser/client/service-request';
import type { ServiceManagedWorkspace } from '@agent-browser/client/service-observability';
import type { DashboardWorkspaceUrlSelection } from './workspace-url-selection';
import { managedWorkspaceViewport } from './managed-workspace';

export type NativeTaskFocusTarget = {
  browserId: string;
  sessionName: string;
  targetId: string;
  displayAllocationId: string;
};

/** Reuse the viewport's exact owner join, with positive current presence for an
 * explicit inspector action. Retained identity alone cannot dispatch focus. */
export function selectedWorkspaceNativeFocus(
  workspace: ServiceManagedWorkspace | undefined,
  selection: DashboardWorkspaceUrlSelection,
  presenceCurrent: boolean,
): NativeTaskFocusTarget | null {
  if (!presenceCurrent || workspace?.schemaVersion !== 'agent-browser.managed-workspace.v1'
    || workspace.source !== 'managed-owner' || !selection.browserId || !selection.tabId) return null;
  const tab = workspace.tabs[selection.tabId.replace(/^target:/, '')];
  const browser = workspace.browsers[selection.browserId];
  const session = tab && workspace.sessions[tab.sessionId];
  if (!tab || !browser || !session || browser.presence?.state !== 'live' || tab.presence?.state !== 'live'
    || (selection.profileId && selection.profileId !== browser.profileId)
    || (selection.sessionId && selection.sessionId !== session.id && selection.sessionId !== session.name)) return null;
  const target = managedWorkspaceViewport(workspace, selection.browserId, selection.tabId, false).target;
  return target && Object.values(target).every(value => typeof value === 'string' && value.trim()) ? target : null;
}

/** Existing-task handoff preparation performs signed native focus. Its response
 * acknowledges that operation; callers must not claim observed physical focus. */
export function requestNativeTaskFocus(target: NativeTaskFocusTarget, signal?: AbortSignal) {
  return requestServiceRemoteViewHandoff({
    browserId: target.browserId, sessionName: target.sessionName,
    displayAllocationId: target.displayAllocationId, baseUrl: window.location.origin,
    serviceName: 'agent-browser-dashboard', agentName: 'operator',
    taskName: 'Select native browser task', jobTimeoutMs: 10000,
    params: { targetId: target.targetId, maximize: true }, signal,
  });
}
