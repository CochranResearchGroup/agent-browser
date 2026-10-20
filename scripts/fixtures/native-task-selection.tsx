import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { managedWorkspaceViewport } from "@/lib/managed-workspace";
import { NativeTaskViewport } from "@/components/native-task-viewport";
function Fixture() {
  const [refreshes, setRefreshes] = useState(0);
  const [targetId, select] = useState("alpha");
  const [census, setCensus] = useState(window.location.search.includes("stale=1") ? "stale" : "current");
  const workspace = {
    schemaVersion: "agent-browser.managed-workspace.v1", source: "managed-owner", observedAtMs: Date.now() - (census === "stale" ? 31_000 : 0),
    browsers: { browser: { id: "browser", profileId: "profile", pid: 42, presence: { state: census === "absent" ? "absent" : "unknown" }, desktop: { desktop_id: "desktop", generation: 1 }, activeSessionIds: ["session-id"] } },
    sessions: { "session-id": { id: "session-id", name: "session", browserId: "browser", profileId: "profile" } },
    tabs: { selected: { id: "selected", browserId: "browser", sessionId: "session-id", targetId, presence: { state: census === "absent" ? "absent" : "unknown" } } },
  } as any;
  const viewport = managedWorkspaceViewport(workspace, "browser", "selected", false);
  return <><button onClick={() => select("alpha")}>Alpha</button><button onClick={() => select("beta")}>Beta</button>
    <button onClick={() => setCensus("stale")}>Expire census</button>
    <button onClick={() => setCensus("current")}>Fresh census</button>
    <button onClick={() => setCensus("absent")}>Confirm absence</button>
    <output aria-label="Refresh count">{refreshes}</output>
    <NativeTaskViewport onRefresh={async () => { setRefreshes(value => value + 1); }} target={viewport.target} unverifiedTarget={viewport.unverifiedTarget} presenceUnavailable={viewport.presenceUnavailable} hasSelection />
  </>;
}
createRoot(document.getElementById("root")!).render(<Fixture />);
