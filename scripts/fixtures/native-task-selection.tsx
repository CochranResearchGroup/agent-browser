import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import { managedWorkspaceViewport } from "@/lib/managed-workspace";
import { NativeTaskViewport } from "@/components/native-task-viewport";
function Fixture() {
  const [targetId, select] = useState("alpha");
  const workspace = {
    schemaVersion: "agent-browser.managed-workspace.v1", source: "managed-owner", observedAtMs: Date.now(),
    browsers: { browser: { id: "browser", profileId: "profile", pid: 42, presence: { state: "unknown" }, desktop: { desktop_id: "desktop", generation: 1 }, activeSessionIds: ["session-id"] } },
    sessions: { "session-id": { id: "session-id", name: "session", browserId: "browser", profileId: "profile" } },
    tabs: { selected: { id: "selected", browserId: "browser", sessionId: "session-id", targetId, presence: { state: "unknown" } } },
  } as any;
  const viewport = managedWorkspaceViewport(workspace, "browser", "selected", false, true);
  return <><button onClick={() => select("alpha")}>Alpha</button><button onClick={() => select("beta")}>Beta</button>
    <NativeTaskViewport target={viewport.target} />
  </>;
}
createRoot(document.getElementById("root")!).render(<Fixture />);
