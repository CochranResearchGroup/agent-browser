import React from "react";
import { createRoot } from "react-dom/client";
import { CompactNativeWorkspace } from "../../packages/dashboard/src/components/compact-native-workspace";
createRoot(document.getElementById("root")!).render(<CompactNativeWorkspace nodes={[]} tabs={[]} selection={{ browserId: null, tabId: null }} userLabel="Synthetic operator" onLogout={() => {}} onRefresh={async () => {}}><div className="native-task-viewport" data-native-task-state="opened"><iframe title="Native fixture" srcDoc={'<body><span id="connection-status" data-state="connected">Connected</span></body>'} /></div></CompactNativeWorkspace>);
