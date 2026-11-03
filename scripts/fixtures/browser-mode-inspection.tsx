import React from "react";
import { createRoot } from "react-dom/client";
import { CompactNativeWorkspace } from "@/components/compact-native-workspace";
import { NativeTabActions } from "@/components/native-tab-actions";
import { BrowserModeButton } from "@/components/browser-mode-button";

const inspection = { schemaVersion: "agent-browser.browser-mode.v1" as const, browserId: "browser", profileId: "profile", pid: 42,
  mode: "cdp_free" as const, observedAtMs: Date.now(), ownership: "managed" as const, reason: null,
  executable: { name: "chrome", family: "chrome", version: null },
  capabilities: { debugging: "disabled" as const, nativeObservation: "unverified" as const, nativeInput: "unverified" as const } };
createRoot(document.getElementById("root")!).render(<><BrowserModeButton label="Synthetic browser" taskId="task-one" sessionName="synthetic-owner" inspection={inspection} onRefresh={async () => {}}><button>Synthetic browser heading</button></BrowserModeButton><NativeTabActions node={{id:"fixture",profileId:"profile",label:"Synthetic"}} tab={{id:"tab-one",browserId:"browser",sessionId:"task-one",ownerSessionId:"synthetic-owner",managedTask:true,profileId:"profile",title:"Synthetic managed tab"}}><button className="native-workspace-tab">Synthetic managed tab</button></NativeTabActions><CompactNativeWorkspace nodes={[]} tabs={[]} selection={{ browserId: null, tabId: null }} userLabel="Synthetic operator" onLogout={() => {}} onRefresh={async () => {}} nativeWindows={[{ key: "observed", desktopId: "desktop", slot: 1, generation: 1, displayName: "Observed desktop", application: null, windowId: 1, pid: 42, title: "Observed synthetic window", class: "Chrome" }]}><p>Desktop</p></CompactNativeWorkspace></>);
