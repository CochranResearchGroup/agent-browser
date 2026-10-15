import React, { useEffect, useRef, useState } from "react";
import { DASHBOARD_WORKSPACE_SELECTION_EVENT, readDashboardWorkspaceUrlSelection } from "@/lib/workspace-url-selection";
import { createRoot } from "react-dom/client";
import { NativeTaskViewport } from "@/components/native-task-viewport";
import { CompactNativeWorkspace } from "@/components/compact-native-workspace";

const nodes = [
  { id: "browser:alice-browser", browserId: "alice-browser", label: "Research", profileId: "shared-profile", live: true, relatedIds: { serviceSessionIds: ["alice-session"] } },
  { id: "browser:carol-browser", browserId: "carol-browser", label: "Analysis", profileId: "independent-profile", live: true, relatedIds: { serviceSessionIds: ["carol-session"] } },
];
const tabs = [
  { id: "alice", managedTask: true, browserId: "alice-browser", sessionId: "alice-session", targetId: "target-alice", profileId: "shared-profile", desktopId: "desktop-a", taskMetadata: { attribution: "supplied", serviceName: "Synthetic billing", agentName: "Ada worker", humanName: "Review operator", originKind: "agent", taskName: "Review invoice", purpose: "Check supplied invoice totals" }, title: "Alice invoice", url: "https://example.test/alice", lifecycle: "active" },
  { id: "bob", managedTask: true, browserId: "alice-browser", sessionId: "bob-session", targetId: "target-bob", title: "Bob invoice", url: "https://example.test/bob", lifecycle: "active" },
  { id: "carol", browserId: "carol-browser", sessionId: "carol-session", targetId: "target-carol", title: "Carol report", url: "https://example.test/carol", lifecycle: "active" },
  { id: "closed", browserId: "alice-browser", targetId: "target-closed", title: "Closed invoice", lifecycle: "closed" },
];
function fixtureTabs() {
  const closed: string[] = JSON.parse(sessionStorage.getItem("fixture-closed-tabs") || "[]");
  return tabs.filter(tab => !closed.includes(tab.id));
}
function Fixture() {
  const [selection, setSelection] = useState(() => readDashboardWorkspaceUrlSelection());
  const [availableTabs, setAvailableTabs] = useState(fixtureTabs);
  const [refreshing, setRefreshing] = useState(false);
  const pauseRefresh = useRef(false);
  const releaseRefresh = useRef<(() => void) | null>(null);
  const refresh = async () => {
    setRefreshing(true);
    try {
      if (pauseRefresh.current) await new Promise<void>(resolve => { releaseRefresh.current = resolve; });
      setAvailableTabs(fixtureTabs());
    } finally { setRefreshing(false); }
  };
  useEffect(() => {
    const removeSelected = () => setAvailableTabs(current => current.filter(tab => tab.id !== readDashboardWorkspaceUrlSelection().tabId));
    const pause = () => { pauseRefresh.current = true; };
    const finish = () => { pauseRefresh.current = false; releaseRefresh.current?.(); releaseRefresh.current = null; };
    window.addEventListener("fixture-remove-selected-tab", removeSelected);
    window.addEventListener("fixture-pause-refresh", pause);
    window.addEventListener("fixture-finish-refresh", finish);
    return () => {
      window.removeEventListener("fixture-remove-selected-tab", removeSelected);
      window.removeEventListener("fixture-pause-refresh", pause);
      window.removeEventListener("fixture-finish-refresh", finish);
    };
  }, []);

  useEffect(() => {
    const update = () => setSelection(readDashboardWorkspaceUrlSelection());
    window.addEventListener(DASHBOARD_WORKSPACE_SELECTION_EVENT, update);
    return () => window.removeEventListener(DASHBOARD_WORKSPACE_SELECTION_EVENT, update);
  }, []);
  const selectedTab = availableTabs.find(tab => tab.id === selection.tabId);
  return <CompactNativeWorkspace nodes={nodes as any} tabs={availableTabs} selection={selection} userLabel="Synthetic operator" onLogout={() => {}} onRefresh={refresh}>
    <NativeTaskViewport hasSelection={Boolean(selection.tabId)} loading={refreshing} onRefresh={refresh} target={selectedTab ? {
      browserId: selection.browserId!, sessionName: selection.sessionId!, targetId: `target-${selection.tabId}`,
      displayAllocationId: selection.browserId === "carol-browser" ? "desktop-b" : "desktop-a",
    } : null} />
  </CompactNativeWorkspace>;
}
createRoot(document.getElementById("root")!).render(<Fixture />);
