import React from "react";
import { createRoot } from "react-dom/client";
import { useSelectedWorkspaceContext } from "@/hooks/use-selected-workspace-context";

function Fixture() {
  const workspace = useSelectedWorkspaceContext();
  const handoff = workspace.managedWorkspace?.handoffs?.["synthetic-link"];
  return <main>
    <button onClick={() => void workspace.refresh()}>Refresh handoffs</button>
    <output aria-label="Reported expiry">{handoff?.expiresAtMs ?? "Unknown"}</output>
    <span role="status">{workspace.loading ? "Loading" : "Settled"}</span>
    {workspace.error && <p role="alert">{workspace.error}</p>}
  </main>;
}
createRoot(document.getElementById("root")!).render(<Fixture />);
