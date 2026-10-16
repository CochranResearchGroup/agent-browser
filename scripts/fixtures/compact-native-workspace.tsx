import type { ServiceManagedHandoff } from "@agent-browser/client/service-observability";
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
  { id: "data-page", browserId: "carol-browser", sessionId: "data-session", targetId: "data-target", title: null, url: "data:text/html,<title>PRIVATE-HTML-CANARY</title>", lifecycle: "active" },
  { id: "alice", managedTask: true, browserId: "alice-browser", sessionId: "alice-session", targetId: "target-alice", profileId: "shared-profile", desktopId: "desktop-a", taskMetadata: { attribution: "supplied", serviceName: "Synthetic billing", agentName: "Ada worker", humanName: "Review operator", originKind: "agent", taskName: "Review invoice", purpose: "Check supplied invoice totals", createdAtMs: 1000, jobId: "job-review", requestId: "request-review" }, title: "Alice invoice", url: "https://example.test/alice", lifecycle: "active" },
  { id: "bob", managedTask: true, browserId: "alice-browser", sessionId: "bob-session", targetId: "target-bob", title: "Bob invoice", url: "https://example.test/bob", lifecycle: "active" },
  { id: "carol", browserId: "carol-browser", sessionId: "carol-session", targetId: "target-carol", title: "Carol report", url: "https://example.test/carol", lifecycle: "active" },
  { id: "closed", browserId: "alice-browser", targetId: "target-closed", title: "Closed invoice", lifecycle: "closed" },
];
const handoffs: ServiceManagedHandoff[] = [{ handoffId: "retained-old", handoffUrl: "/remote-view/retained-old", sessionId: "ended-session", tabId: "expired-original",
  sessionName: "old-task", profileId: "archived-profile", browserId: null, createdAtMs: 1000, expiresAtMs: null,
  linkState: "active", targetState: "expired", taskMetadata: { taskName: "Retained review", serviceName: "Audit service", humanName: "Casey operator", originKind: "agent", attribution: "supplied" },
  tabLifetime: { inactivityMs: 30000 }, pendingHuman: null }];
function fixtureHandoffs() {
  const deleted: string[] = JSON.parse(sessionStorage.getItem("fixture-deleted-handoffs") || "[]");
  return handoffs.filter(record => !deleted.includes(record.handoffId));
}
function fixtureTabs() {
  const closed: string[] = JSON.parse(sessionStorage.getItem("fixture-closed-tabs") || "[]");
  return tabs.filter(tab => !closed.includes(tab.id));
}
function Fixture() {
  const [selection, setSelection] = useState(() => readDashboardWorkspaceUrlSelection());
  const [availableTabs, setAvailableTabs] = useState(fixtureTabs);
  const [refreshing, setRefreshing] = useState(false);
  const [availableHandoffs, setAvailableHandoffs] = useState<typeof handoffs | undefined>(fixtureHandoffs);
  const failHandoffRefresh = useRef(false);
  const pauseRefresh = useRef(false);
  const releaseRefresh = useRef<(() => void) | null>(null);
  const refresh = async () => {
    setRefreshing(true);
    try {
      if (pauseRefresh.current) await new Promise<void>(resolve => { releaseRefresh.current = resolve; });
      if (failHandoffRefresh.current) throw new Error("SYNTHETIC-PRIVATE-INVENTORY-DETAIL");
      setAvailableTabs(fixtureTabs());
      setAvailableHandoffs(fixtureHandoffs());
    } finally { setRefreshing(false); }
  };
  useEffect(() => {
    const empty = () => setAvailableHandoffs([]);
    const unavailable = () => setAvailableHandoffs(undefined);
    const fail = () => { failHandoffRefresh.current = true; };
    const live = () => setAvailableHandoffs([{ ...handoffs[0], targetState: "reported", browserId: "alice-browser" }]);
    const rearmed = () => setAvailableHandoffs([{ ...handoffs[0], targetState: "reported", browserId: "alice-browser", taskMetadata: { ...handoffs[0].taskMetadata, firstHumanUseAtMs: 2000 }, pendingHuman: { handoffId: "retained-old", requestedAtMs: 3000, expiresAtMs: null } }]);
    const pending = () => setAvailableHandoffs([{ ...handoffs[0], pendingHuman: { handoffId: "retained-old", requestedAtMs: 1000, expiresAtMs: null } }]);
    const used = () => setAvailableHandoffs([{ ...handoffs[0], taskMetadata: { ...handoffs[0].taskMetadata, firstHumanUseAtMs: 2000 }, pendingHuman: null }]);
    const route = () => setAvailableHandoffs([{ ...handoffs[0], handoffUrl: "https://provider.invalid/internal?token=PRIVATE-ROUTE-CANARY" }]);
    const repair = () => { failHandoffRefresh.current = false; };
    window.addEventListener("fixture-handoffs-empty", empty);
    window.addEventListener("fixture-handoffs-unavailable", unavailable);
    window.addEventListener("fixture-handoffs-fail", fail);
    window.addEventListener("fixture-handoffs-live", live);
    window.addEventListener("fixture-handoffs-rearmed", rearmed);
    window.addEventListener("fixture-handoffs-route", route);
    window.addEventListener("fixture-handoffs-pending", pending);
    window.addEventListener("fixture-handoffs-used", used);
    window.addEventListener("fixture-handoffs-repair", repair);
    return () => {
      window.removeEventListener("fixture-handoffs-empty", empty);
      window.removeEventListener("fixture-handoffs-unavailable", unavailable);
      window.removeEventListener("fixture-handoffs-fail", fail);
      window.removeEventListener("fixture-handoffs-live", live);
      window.removeEventListener("fixture-handoffs-rearmed", rearmed);
      window.removeEventListener("fixture-handoffs-route", route);
      window.removeEventListener("fixture-handoffs-pending", pending);
      window.removeEventListener("fixture-handoffs-used", used);
      window.removeEventListener("fixture-handoffs-repair", repair);
    };
  }, []);
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
  return <CompactNativeWorkspace nodes={nodes as any} handoffs={availableHandoffs as any} tabs={availableTabs} selection={selection} userLabel="Synthetic operator" onLogout={() => {}} onRefresh={refresh}>
    <NativeTaskViewport hasSelection={Boolean(selection.tabId)} loading={refreshing} onRefresh={refresh} target={selectedTab ? {
      browserId: selection.browserId!, sessionName: selection.sessionId!, targetId: `target-${selection.tabId}`,
      displayAllocationId: selection.browserId === "carol-browser" ? "desktop-b" : "desktop-a",
    } : null} />
  </CompactNativeWorkspace>;
}
createRoot(document.getElementById("root")!).render(<Fixture />);
