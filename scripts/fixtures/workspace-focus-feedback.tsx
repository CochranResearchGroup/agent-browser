// Synthetic projection for the actual viewport component.
// These tests do not qualify provider readiness or native focus ordering.
import React from "react";
import { createRoot } from "react-dom/client";
import { WorkspaceRemoteViewport } from "@/components/workspace-remote-viewport";
import { projectWorkspaceViews } from "@/lib/workspace-view-projection";

const root = createRoot(document.getElementById("root")!);
const stream = {
  id: "cdp-a", provider: "cdp_screencast", controlInput: "cdp_input",
  url: "/stream/", readiness: { state: "ready" },
};
const authority = {
  subjectKey: "browser:browser-a", authoritySource: "service-status-compatibility",
  browserId: "browser-a", workspaceId: "browser:browser-a",
  inventoryClass: "service-owned-controllable-browser",
  lifecycle: { state: "controllable", live: true, retained: false, health: "ready" },
  routeBoundOwnership: { state: "finalized", reason: null },
  operatorVisibleProof: { state: "ready", reason: null },
  lifecycleActions: [], diagnostics: [],
  presentationActionCeilings: {
    view: { allowed: true }, control: { allowed: true },
    stream: { allowed: true }, screenshot: { allowed: true },
  },
};

(window as any).select = (id: string) => {
  history.replaceState(null, "", `/?view=workspace:control&browser=browser-a&tab=${id}`);
  window.dispatchEvent(new Event("popstate"));
  const selection = {
    workspaceId: null, browserId: "browser-a", sessionId: null,
    tabId: id, profileId: null, jobId: null,
  };
  const projection = projectWorkspaceViews({
    sources: {
      serviceBrowsers: [{
        id: "browser-a", health: "ready", activeSessionIds: ["session-a"],
        viewStreams: [stream],
      }],
      daemonSessions: [{ session: "session-a", port: 9223, engine: "chrome" }],
      serviceTabs: [
        { id: "alice", browserId: "browser-a", targetId: "target-alice",
          lifecycle: "active", title: "Alice", url: "https://example.test/alice" },
        { id: "bob", browserId: "browser-a", targetId: "target-bob",
          lifecycle: "active", title: "Bob", url: "https://example.test/bob" },
      ],
    },
    authorityLedger: { "browser:browser-a": authority },
    intent: { selection, mode: "control", dashboardHref: location.href },
  } as any);
  root.render(
    <WorkspaceRemoteViewport
      fallback={<p>fallback</p>}
      projection={projection}
      onRefresh={async () => {}}
      onSelectStream={() => {}}
    />,
  );
};
(window as any).select("alice");
