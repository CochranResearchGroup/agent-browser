import React, { useEffect, useState } from "react";
import { DASHBOARD_WORKSPACE_SELECTION_EVENT, readDashboardWorkspaceUrlSelection } from "@/lib/workspace-url-selection";
import { createRoot } from "react-dom/client";
import { NativeTaskViewport } from "@/components/native-task-viewport";
import { CompactNativeWorkspace } from "@/components/compact-native-workspace";

const nodes = [{ id: "browser:alice-browser", browserId: "alice-browser", label: "Research", profileId: "shared-profile", live: true, relatedIds: { serviceSessionIds: ["alice-session"] } }];
const tabs = [
  { id: "alice", browserId: "alice-browser", sessionId: "alice-session", targetId: "target-alice", title: "Alice invoice", url: "https://example.test/alice", lifecycle: "active" },
  { id: "bob", browserId: "alice-browser", sessionId: "bob-session", targetId: "target-bob", title: "Bob invoice", url: "https://example.test/bob", lifecycle: "active" },
  { id: "closed", browserId: "alice-browser", targetId: "target-closed", title: "Closed invoice", lifecycle: "closed" },
];
function Fixture() {
  const [selection, setSelection] = useState(() => readDashboardWorkspaceUrlSelection());
  useEffect(() => {
    const update = () => setSelection(readDashboardWorkspaceUrlSelection());
    window.addEventListener(DASHBOARD_WORKSPACE_SELECTION_EVENT, update);
    return () => window.removeEventListener(DASHBOARD_WORKSPACE_SELECTION_EVENT, update);
  }, []);
  return <CompactNativeWorkspace nodes={nodes as any} tabs={tabs} selection={selection} userLabel="Synthetic operator" onLogout={() => {}} onRefresh={async () => {}}>
    <NativeTaskViewport target={selection.tabId ? {
      browserId: "alice-browser", sessionName: selection.sessionId!, targetId: `target-${selection.tabId}`,
      displayAllocationId: "desktop-a",
    } : null} />
  </CompactNativeWorkspace>;
}
createRoot(document.getElementById("root")!).render(<Fixture />);
