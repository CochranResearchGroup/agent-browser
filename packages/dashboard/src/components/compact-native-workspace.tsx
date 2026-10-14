"use client";

import { useEffect, useRef, useState, type ReactNode } from "react";
import { ChevronDown, LogOut, Monitor, PanelLeftClose, PanelLeftOpen, RefreshCw, Search } from "lucide-react";
import type { WorkspaceNode, WorkspaceServiceTab } from "@/lib/service-workspaces";
import { writeDashboardWorkspaceUrlSelection } from "@/lib/workspace-url-selection";

type Props = {
  nodes: readonly WorkspaceNode[];
  tabs: readonly WorkspaceServiceTab[];
  selection: { browserId: string | null; tabId: string | null };
  userLabel: string;
  onLogout: () => void;
  onRefresh: () => Promise<void>;
  children: ReactNode;
  loading?: boolean;
  error?: string | null;
};

/** Compact inventory and native viewer share one authoritative source snapshot. */
export function CompactNativeWorkspace({ nodes, tabs, selection, userLabel, onLogout, onRefresh, children, loading, error }: Props) {
  const [collapsed, setCollapsed] = useState(false);
  const [width, setWidth] = useState(224);
  const [query, setQuery] = useState("");
  const [closedGroups, setClosedGroups] = useState<Set<string>>(new Set());
  const drag = useRef<{ x: number; width: number } | null>(null);
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
  const browsers = [...new Map(nodes.filter(node => node.live && node.browserId).map(node => [node.browserId, node])).values()];
  const groups = browsers.map(node => {
    const browserTabs = tabs.filter(tab => tab.browserId === node.browserId && tab.lifecycle !== "closed" && tab.lifecycle !== "expired");
    const browserMatches = `${node.label} ${node.profileId ?? ""}`.toLowerCase().includes(search);
    return { node, browserTabs, visibleTabs: browserTabs.filter(tab => !search || browserMatches || `${tab.title ?? ""} ${tab.taskLabel ?? ""} ${tab.url ?? ""}`.toLowerCase().includes(search)), browserMatches };
  }).filter(group => !search || group.browserMatches || group.visibleTabs.length);
  return (
    <main className="native-workspace" style={{ gridTemplateColumns: collapsed ? "40px minmax(0, 1fr)" : `${width}px minmax(0, 1fr)` }}>
      <aside className="native-workspace-rail" aria-label="Browsers and tabs">
        <div className="native-workspace-rail-header">
          {!collapsed && <span>Browsers</span>}
          <button type="button" aria-label={collapsed ? "Show browser rail" : "Collapse browser rail"} onClick={toggleRail}>
            {collapsed ? <PanelLeftOpen size={15} /> : <PanelLeftClose size={15} />}
          </button>
        </div>
        {!collapsed && <>
          <label className="native-workspace-search"><Search size={13} /><input type="search" aria-label="Search browsers and tabs" placeholder="Find browser or tab" value={query} onChange={event => setQuery(event.target.value)} /></label>
          <nav className="native-workspace-inventory" aria-label="Browser inventory">
            {groups.map(({ node, browserTabs, visibleTabs }) => {
              const open = search ? true : !closedGroups.has(node.id);
              return <section key={node.id}>
                <button type="button" className="native-workspace-browser" aria-expanded={open} onClick={() => setClosedGroups(current => {
                  const next = new Set(current); if (next.has(node.id)) next.delete(node.id); else next.add(node.id); return next;
                })}><ChevronDown size={12} style={{ transform: open ? undefined : "rotate(-90deg)" }} /><Monitor size={13} /><span>{node.label}<small>{node.profileId ?? "Unknown profile"}</small></span><small>{browserTabs.length}</small></button>
                {open && visibleTabs.map(tab => <button key={tab.id} type="button" className="native-workspace-tab" aria-label={tab.title || tab.taskLabel || tab.url || "Untitled tab"} aria-current={selection.browserId === node.browserId && selection.tabId === tab.id ? "page" : undefined} onClick={() => selectTab(node, tab)} title={tab.url ?? undefined}><span>{tab.title || tab.taskLabel || tab.url || "Untitled tab"}</span></button>)}
              </section>;
            })}
            {search && browsers.length > 0 && !groups.length && <p role="status" className="native-workspace-empty">No matching browsers or tabs</p>}
            {!browsers.length && <p className="native-workspace-empty">{loading ? "Loading browsers…" : "No active browsers"}</p>}
          </nav>
        </>}
        {!collapsed && <div role="separator" tabIndex={0} aria-label="Browser rail width" aria-orientation="vertical" aria-valuemin={180} aria-valuemax={400} aria-valuenow={width} className="native-workspace-resize" onKeyDown={event => {
          if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
          event.preventDefault(); saveWidth(event.key === "Home" ? 180 : event.key === "End" ? 400 : width + (event.key === "ArrowLeft" ? -8 : 8));
        }} onPointerDown={event => { drag.current = { x: event.clientX, width }; event.currentTarget.setPointerCapture(event.pointerId); }} onPointerMove={event => { if (drag.current) saveWidth(drag.current.width + event.clientX - drag.current.x); }} onPointerUp={() => { drag.current = null; }} onLostPointerCapture={() => { drag.current = null; }} />}
      </aside>
      <section className="native-workspace-main" aria-label="Browser workspace">
        <header className="native-workspace-toolbar"><span>Agent Browser</span><span className="native-workspace-user">{userLabel}</span><button type="button" aria-label="Refresh browsers" onClick={() => void onRefresh()}><RefreshCw size={14} /></button><button type="button" aria-label="Sign out" onClick={onLogout}><LogOut size={14} /></button></header>
        {error && <div role="alert" className="native-workspace-error">{error} <button type="button" onClick={() => void onRefresh()}>Retry</button></div>}
        <div className="native-workspace-desktop">{children}</div>
      </section>
    </main>
  );
}
