import React from "react";
import { createRoot } from "react-dom/client";
import { NativeHandoffDrawer } from "@/components/native-handoff-drawer";
import type { ServiceManagedHandoff } from "@agent-browser/client/service-observability";

const record = (handoffId: string, fields: Partial<ServiceManagedHandoff>): ServiceManagedHandoff => ({
  handoffId, handoffUrl: `/remote-view/${handoffId}`, sessionId: "synthetic-session", tabId: "synthetic-tab",
  sessionName: null, profileId: null, browserId: null, createdAtMs: 1792279612379, expiresAtMs: null,
  linkState: "active", targetState: "reported", taskMetadata: null, tabLifetime: null,
  pendingHuman: { handoffId, requestedAtMs: 1792279612379, expiresAtMs: null }, ...fields,
});
const records = [
  record("missing-labels", { profileId: "qbo-example", sessionName: "qbo-operator", taskMetadata: { attribution: "supplied", jobId: "synthetic-job" } }),
  record("attributed", { profileId: "example-profile", sessionName: "example-session", taskMetadata: { taskName: "Open accounts", serviceName: "codex", agentName: "Codex", originKind: "agent", attribution: "supplied" } }),
  record("legacy", {}),
];
createRoot(document.getElementById("root")!).render(<NativeHandoffDrawer open onOpenChange={() => {}} busy={false} error={null} sharedLink={null} onReturnFocus={() => {}} records={records} onRefresh={async () => {}} />);
