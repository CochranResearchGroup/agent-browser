import React from 'react';
import { createRoot } from 'react-dom/client';
import { useSelectedWorkspaceContext } from '@/hooks/use-selected-workspace-context';
import { CompactNativeWorkspace } from '@/components/compact-native-workspace';

function Fixture() {
  const workspace = useSelectedWorkspaceContext(true, undefined, true);
  return <CompactNativeWorkspace nodes={workspace.nodes} tabs={workspace.tabs}
    selection={{ browserId: null, tabId: null }} userLabel="Synthetic operator"
    onLogout={() => {}} onRefresh={workspace.refresh} loading={workspace.loading} error={workspace.error}>
    <p>Synthetic startup observation</p>
  </CompactNativeWorkspace>;
}
createRoot(document.getElementById('root')!).render(<Fixture />);
