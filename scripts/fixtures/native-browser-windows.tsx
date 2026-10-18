import React from "react";
import { createRoot } from "react-dom/client";
import { CompactNativeWorkspace } from "@/components/compact-native-workspace";
import { NativeWindowViewport } from "@/components/native-window-viewport";
import { useNativeBrowserWindows } from "@/hooks/use-native-browser-windows";

function Fixture() {
  const native = useNativeBrowserWindows();
  return <CompactNativeWorkspace nodes={[]} tabs={[]} selection={{ browserId: null, tabId: null }} userLabel="Synthetic operator"
    nativeWindows={native.windows} nativeSelection={native.selectedKey} nativeIssue={native.issue} onSelectNativeWindow={native.select}
    onLogout={() => {}} onRefresh={() => native.refresh(true)} loading={native.loading}>
    {native.selectedKey ? <NativeWindowViewport target={native.windows.find(window => window.key === native.selectedKey) ?? null} loading={native.loading} /> : <p>Select a browser window.</p>}
  </CompactNativeWorkspace>;
}
createRoot(document.getElementById("root")!).render(<Fixture />);
