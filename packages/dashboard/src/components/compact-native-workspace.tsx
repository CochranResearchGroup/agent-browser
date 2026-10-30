"use client";

import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { ChevronDown, LogOut, Monitor, PanelLeftClose, PanelLeftOpen, RefreshCw, Search, Link2, Wrench } from "lucide-react";
import { buildNativeRailGroups } from "@/lib/native-rail";
import { tabDescription } from "@/lib/tab-description";
import { queueWorkspaceFocusRequest } from "@/lib/workspace-focus-request";
import type { WorkspaceNode, WorkspaceServiceTab } from "@/lib/service-workspaces";
import { writeDashboardWorkspaceUrlSelection } from "@/lib/workspace-url-selection";
import type { ServiceManagedHandoff } from "@agent-browser/client/service-observability";
import { requestServiceRemoteViewHandoff } from "@agent-browser/client/service-request";
import { SubsystemHealthIndicator } from "@/components/subsystem-health";
import { NativeHandoffDrawer } from "@/components/native-handoff-drawer";
import { BrowserModeButton } from "@/components/browser-mode-button";
import { NativeTabActions } from "@/components/native-tab-actions";
import type { NativeBrowserWindow } from "@/lib/native-window-inventory";

type Props = {
  runtimeEnvironment?: "production" | "development";
  nodes: readonly WorkspaceNode[];
  handoffs?: readonly ServiceManagedHandoff[];
  tabs: readonly WorkspaceServiceTab[];
  selection: { browserId: string | null; tabId: string | null };
  userLabel: string;
  diagnosticsHref?: string;
  onLogout: () => void;
  onRefresh: () => Promise<void>;
  children: ReactNode;
  loading?: boolean;
  error?: string | null;
  nativeWindows?: readonly NativeBrowserWindow[];
  nativeSelection?: string | null;
  nativeIssue?: string | null;
  onSelectNativeWindow?: (target: NativeBrowserWindow) => void;
};

/** Compact inventory and native viewer share one authoritative source snapshot. */
export function CompactNativeWorkspace({ runtimeEnvironment, nodes, tabs, handoffs, selection, userLabel, diagnosticsHref = "/service", onLogout, onRefresh, children, loading, error, nativeWindows = [], nativeSelection, nativeIssue, onSelectNativeWindow }: Props) {
  const [handoffsOpen, setHandoffsOpen] = useState(false);
  const [sharedLink, setSharedLink] = useState<{ url: string; label: string } | null>(null);
  const [shareBusy, setShareBusy] = useState(false);
  const sharePending = useRef(false);
  const [shareError, setShareError] = useState<string | null>(null);
  const shareFocus = useRef<HTMLElement | null>(null);
  const shareTab = async (tab: WorkspaceServiceTab, opener: HTMLElement | null) => {
    const { desktopId, targetId, browserId } = tab;
    const sessionName = tab.ownerSessionId || tab.sessionId;
    if (sharePending.current || !tab.managedTask || !desktopId || !targetId || !browserId || !sessionName) return;
    shareFocus.current = opener;
    sharePending.current = true; setShareBusy(true); setShareError(null); setSharedLink(null); setHandoffsOpen(true);
    try {
      // Sharing prepares the same native desktop as selection. Preserve gesture
      // order without spending this request's deadline behind an older focus.
      await queueWorkspaceFocusRequest(JSON.stringify(["native-desktop", desktopId]), async () => {
        const handoff = await requestServiceRemoteViewHandoff({
          baseUrl: window.location.origin, signal: AbortSignal.timeout(15000),
          browserId, sessionName,
          displayAllocationId: desktopId, params: { targetId },
          serviceName: "agent-browser-dashboard", agentName: "operator", taskName: "Share browser task", jobTimeoutMs: 10000,
        });
        const url = new URL(handoff.handoffUrl, window.location.origin);
        if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password || url.search || url.hash
            || !/^\/remote-view\/[A-Za-z0-9_-]+$/.test(url.pathname)) throw new Error('invalid_handoff_url');
        setSharedLink({ url: url.toString(), label: tabDescription(tab) });
      });
    } catch { setShareError("Could not prepare this link. Refresh browsers and check access before trying again."); }
    finally { sharePending.current = false; setShareBusy(false); }
  };
  const [collapsed, setCollapsed] = useState(false);
  const [width, setWidth] = useState(224);
  const [query, setQuery] = useState("");
  const [closedGroups, setClosedGroups] = useState<Set<string>>(new Set());
  const drag = useRef<{ x: number; width: number } | null>(null);
  const root = useRef<HTMLElement>(null);
  const [closedTabs, setClosedTabs] = useState<Set<string>>(new Set());
  const [closeRefreshError, setCloseRefreshError] = useState<string | null>(null);
  const tabClosed = async (tab: WorkspaceServiceTab) => {
    // Only an exact owner receipt reaches this callback. Hide its stale frame
    // immediately while the authoritative inventory refresh completes.
    setClosedTabs(current => new Set(current).add(tab.id));
    setCloseRefreshError(null);
    try { await onRefresh(); }
    catch { setCloseRefreshError("The tab closed, but inventory could not refresh. Refresh browsers."); }
    finally {
      requestAnimationFrame(() => {
        const next = root.current?.querySelector<HTMLButtonElement>(".native-workspace-tab[aria-current]")
          ?? root.current?.querySelector<HTMLButtonElement>(".native-workspace-tab")
          ?? root.current?.querySelector<HTMLButtonElement>('button[aria-label="Refresh browsers"]');
        next?.focus();
      });
    }
  };
  useEffect(() => {
    try {
      setCollapsed(localStorage.getItem("native-workspace-rail-collapsed") === "true");
      const stored = Number(localStorage.getItem("native-workspace-rail-width"));
      if (stored >= 180 && stored <= 400) setWidth(stored);
    } catch { /* Storage denial must not prevent browser operation. */ }
  }, []);
  const saveWidth = (next: number) => {
    const value = Math.max(180, Math.min(400, next));
    setWidth(value);
    try { localStorage.setItem("native-workspace-rail-width", String(value)); } catch { /* Optional preference. */ }
  };
  const toggleRail = () => setCollapsed(current => {
    try { localStorage.setItem("native-workspace-rail-collapsed", String(!current)); } catch { /* Optional preference. */ }
    return !current;
  });
  const selectTab = (node: WorkspaceNode, tab: WorkspaceServiceTab) => {
    // Clear all prior target identity together; never retain a peer session/job.
    const url = new URL(window.location.href);
    url.searchParams.delete("native-window");
    url.searchParams.set("view", "workspace:control");
    history.replaceState(history.state, "", `${url.pathname}${url.search}${url.hash}`);
    writeDashboardWorkspaceUrlSelection({
      workspaceId: node.id, browserId: node.browserId ?? null,
      sessionId: tab.ownerSessionId ?? tab.sessionId ?? null,
      tabId: tab.id, profileId: node.profileId ?? null, jobId: null,
    });
    window.dispatchEvent(new PopStateEvent("popstate", { state: history.state }));
    // writeDashboardWorkspaceUrlSelection dispatches the canonical selection event.
  };
  const search = query.trim().toLowerCase();
  const { groups, browserCount, unverifiedTabs } = useMemo(() => buildNativeRailGroups(nodes, tabs, closedTabs, query), [nodes, tabs, closedTabs, query]);
  const nativeGroups = useMemo(() => {
    const result = new Map<string, NativeBrowserWindow[]>();
    for (const window of nativeWindows) {
      if (search && !`${window.title} ${window.class} ${window.displayName} Desktop ${window.slot}`.toLowerCase().includes(search)) continue;
      const key = `${window.desktopId}:${window.generation}`;
      const group = result.get(key) ?? []; group.push(window); result.set(key, group);
    }
    return [...result.entries()];
  }, [nativeWindows, search]);
  return (
    <main ref={root} className="native-workspace" style={{ gridTemplateColumns: collapsed ? "40px minmax(0, 1fr)" : `${width}px minmax(0, 1fr)` }}>
      <aside className="native-workspace-rail" aria-label="Browsers and tabs">
        <button type="button" aria-label="Handoffs" title="Handoffs" onClick={event => { shareFocus.current = event.currentTarget; setHandoffsOpen(true); }}><Link2 size={15} /></button>
        <div className="native-workspace-rail-header">
          {!collapsed && <span>Browsers</span>}
          <button type="button" aria-label={collapsed ? "Show browser rail" : "Collapse browser rail"} title={collapsed ? "Show browser rail" : "Collapse browser rail"} onClick={toggleRail}>
            {collapsed ? <PanelLeftOpen size={15} /> : <PanelLeftClose size={15} />}
          </button>
        </div>
        {!collapsed && <>
          <label className="native-workspace-search"><Search size={13} /><input type="search" aria-label="Search browsers and tabs" placeholder="Find browser or tab" value={query} onChange={event => setQuery(event.target.value)} /></label>
          <nav className="native-workspace-inventory" aria-label="Browser inventory">
            {groups.map(({ node, browserTabs, visibleTabs }) => {
              const open = search ? true : !closedGroups.has(node.id);
              return <section key={node.id}>
                <div className="native-workspace-browser-row">
                <button type="button" className="native-workspace-browser" aria-expanded={open} onClick={() => setClosedGroups(current => {
                  const next = new Set(current); if (next.has(node.id)) next.delete(node.id); else next.add(node.id); return next;
                })}><ChevronDown size={12} style={{ transform: open ? undefined : "rotate(-90deg)" }} /><Monitor size={13} /><span>{node.label}<small title={node.profileId ?? undefined}>{browserTabs.find(tab => tab.profileProperties)?.profileProperties?.name ?? node.profileId ?? "Unknown profile"}</small></span><small>{browserTabs.length}</small></button><BrowserModeButton label={node.label} profileId={node.profileId} taskId={browserTabs.find(tab => tab.managedTask && tab.sessionId)?.sessionId} sessionName={browserTabs.find(tab => tab.managedTask && tab.sessionId)?.ownerSessionId} inspection={node.browserMode} onRefresh={onRefresh} /></div>
                {open && visibleTabs.map(tab => <NativeTabActions key={tab.id} node={node} tab={tab} onProfileChanged={onRefresh} shareBusy={shareBusy} onShare={opener => void shareTab(tab, opener)} onClosed={() => tabClosed(tab)}><button type="button" className="native-workspace-tab" aria-label={tabDescription(tab)} aria-current={selection.browserId === node.browserId && selection.tabId === tab.id ? "page" : undefined} onClick={() => selectTab(node, tab)} title={tab.url ?? undefined}><span>{tabDescription(tab)}</span></button></NativeTabActions>)}
              </section>;
            })}
            {nativeGroups.map(([id, windows]) => {
              const open = search ? true : !closedGroups.has(`native:${id}`);
              return <section key={`native:${id}`}>
                <button type="button" className="native-workspace-browser" aria-expanded={open} onClick={() => setClosedGroups(current => { const next = new Set(current); if (next.has(`native:${id}`)) next.delete(`native:${id}`); else next.add(`native:${id}`); return next; })}><ChevronDown size={12} style={{ transform: open ? undefined : "rotate(-90deg)" }} /><Monitor size={13} /><span>{windows[0].displayName}<small>Desktop {windows[0].slot} · Remote View</small></span><small>{windows.length}</small></button>
                {open && windows.map(window => <div key={window.key} className="native-workspace-browser-row"><button type="button" className="native-workspace-tab" aria-current={nativeSelection === window.key ? "page" : undefined} title="Observed native browser window; managed task actions are unavailable" onClick={() => onSelectNativeWindow?.(window)}><span>{window.title}</span></button><BrowserModeButton label={window.title} observed onRefresh={onRefresh} /></div>)}
              </section>;
            })}
            {search && (browserCount > 0 || nativeWindows.length > 0) && !groups.length && !nativeGroups.length && <p role="status" className="native-workspace-empty">No matching browsers or tabs</p>}
            {!groups.length && !nativeGroups.length && !search && <p className="native-workspace-empty">{loading ? "Loading browsers…" : unverifiedTabs || nativeIssue || error ? "Browser presence could not be verified. Refresh browsers to check again." : "No active browsers"}</p>}
            {nativeIssue && <p role="status" className="native-workspace-empty">{nativeIssue}</p>}
          </nav>
        </>}
        {!collapsed && <div role="separator" tabIndex={0} aria-label="Browser rail width" aria-orientation="vertical" aria-valuemin={180} aria-valuemax={400} aria-valuenow={width} className="native-workspace-resize" onKeyDown={event => {
          if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
          event.preventDefault(); saveWidth(event.key === "Home" ? 180 : event.key === "End" ? 400 : width + (event.key === "ArrowLeft" ? -8 : 8));
        }} onPointerDown={event => { drag.current = { x: event.clientX, width }; event.currentTarget.setPointerCapture(event.pointerId); }} onPointerMove={event => { if (drag.current) saveWidth(drag.current.width + event.clientX - drag.current.x); }} onPointerUp={() => { drag.current = null; }} onLostPointerCapture={() => { drag.current = null; }} />}
      </aside>
      <section className="native-workspace-main" aria-label="Browser workspace">
        <header className="native-workspace-toolbar"><img src="/branding/agent-browser-logo.png" alt="" width={28} height={28} className="shrink-0" /><span>Agent Browser</span>{runtimeEnvironment === "development" && <span data-runtime-environment={runtimeEnvironment}>Development</span>}<span className="native-workspace-user">{userLabel}</span><SubsystemHealthIndicator inventoryIssue={error} nativeIssue={nativeIssue} loading={loading} /><a className="native-workspace-diagnostics" href={diagnosticsHref} aria-label="Diagnostics" title="Diagnostics"><Wrench size={14} /></a><button type="button" aria-label="Refresh browsers" title="Refresh browsers" onClick={() => void onRefresh()}><RefreshCw size={14} /></button><button type="button" aria-label="Sign out" title="Sign out" onClick={onLogout}><LogOut size={14} /></button></header>
        {closeRefreshError && <div role="alert" className="native-workspace-error">{closeRefreshError}</div>}
        {error && <div role="alert" className="native-workspace-error">{error} <button type="button" onClick={() => void onRefresh()}>Retry</button></div>}
        <div className="native-workspace-desktop">{selection.tabId && closedTabs.has(selection.tabId) ? <p role="status" className="native-task-message">The selected tab was closed. Select another tab to continue.</p> : children}</div>
      </section>
      <NativeHandoffDrawer open={handoffsOpen} onOpenChange={setHandoffsOpen} busy={shareBusy} error={shareError} sharedLink={sharedLink}
        railWidth={collapsed ? 40 : width} records={handoffs} inventoryLoading={loading} inventoryError={error ? "Could not refresh handoffs. Showing the last reported records." : null} onRefresh={onRefresh}
        onReturnFocus={() => shareFocus.current?.focus()} />
    </main>
  );
}
