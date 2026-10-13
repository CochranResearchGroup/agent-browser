// Synthetic projection for the actual viewport component.
// Controller admission and viewer transport are synthetic; focus reaches native HTTP.
import React from "react";
import { createRoot } from "react-dom/client";
import { WorkspaceRemoteViewport } from "@/components/workspace-remote-viewport";
import { projectWorkspaceViews } from "@/lib/workspace-view-projection";

const config = (window as any).P237_UI_CONFIG;
const root = createRoot(document.getElementById("root")!);
const stream = {
  id: "cdp-a", provider: "rdp_gateway", controlInput: "manual_attached_desktop",
  routeId: config.routeId, displayAllocationId: config.displayAllocationId,
  viewerLeaseIds: [config.controllerLeaseId], controllerLeaseId: config.controllerLeaseId,
  url: "/guacamole/client", frameUrl: "/guacamole/client", readiness: { state: "ready" },
};
const authority = {
  subjectKey: `browser:${config.browserId}`, authoritySource: "service-status-compatibility",
  browserId: config.browserId, workspaceId: `browser:${config.browserId}`,
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
  history.replaceState(null, "", `/?view=workspace:control&browser=${config.browserId}&tab=${id}`);
  window.dispatchEvent(new Event("popstate"));
  const selection = {
    workspaceId: null, browserId: config.browserId, sessionId: null,
    tabId: id, profileId: null, jobId: null,
  };
  const projection = projectWorkspaceViews({
    sources: {
      serviceBrowsers: [{
        id: config.browserId, health: "ready", activeSessionIds: [config.sessionName],
        viewStreams: [stream],
      }],
      daemonSessions: [{ session: config.sessionName, port: 9223, engine: "chrome" }],
      serviceTabs: [
        { id: "alice", browserId: config.browserId, targetId: config.tabs[0][0],
          lifecycle: "active", title: config.tabs[0][1], url: "https://example.test/alice" },
        { id: "bob", browserId: config.browserId, targetId: config.tabs[1][0],
          lifecycle: "active", title: config.tabs[1][1], url: "https://example.test/bob" },
      ],
    },
    authorityLedger: { [`browser:${config.browserId}`]: authority },
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
