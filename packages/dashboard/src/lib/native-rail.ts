import { tabDescription } from './tab-description.ts';
import type { WorkspaceNode, WorkspaceServiceTab } from './service-workspaces.ts';

/** Index current tabs once. Counts and search use the same qualified rows. */
export function buildNativeRailGroups(nodes: readonly WorkspaceNode[], tabs: readonly WorkspaceServiceTab[], closedTabs: ReadonlySet<string>, query: string) {
  const search = query.trim().toLowerCase();
  const browsers = new Map(nodes.filter(node => node.live && node.browserId).map(node => [node.browserId!, node]));
  const byBrowser = new Map<string, WorkspaceServiceTab[]>();
  let unverifiedTabs = false;
  for (const tab of tabs) {
    if (tab.managedTask && (!tab.presence || tab.presence.state === 'unknown')) unverifiedTabs = true;
    if (!tab.browserId || !browsers.has(tab.browserId) || closedTabs.has(tab.id)
      || tab.lifecycle === 'closed' || tab.lifecycle === 'expired'
      || (tab.managedTask && tab.presence?.state !== 'live')) continue;
    const group = byBrowser.get(tab.browserId) ?? [];
    group.push(tab); byBrowser.set(tab.browserId, group);
  }
  const groups = [...browsers.values()].flatMap(node => {
    const browserTabs = byBrowser.get(node.browserId!) ?? [];
    if (!browserTabs.length) return [];
    const browserMatches = `${node.label} ${node.profileId ?? ''}`.toLowerCase().includes(search);
    const visibleTabs = !search || browserMatches ? browserTabs : browserTabs.filter(tab => `${tabDescription(tab)} ${tab.taskLabel ?? ''} ${/^(?:data|blob|javascript):/i.test(tab.url ?? '') ? '' : tab.url ?? ''}`.toLowerCase().includes(search));
    return visibleTabs.length ? [{ node, browserTabs, visibleTabs }] : [];
  });
  return { groups, browserCount: browsers.size, unverifiedTabs };
}
