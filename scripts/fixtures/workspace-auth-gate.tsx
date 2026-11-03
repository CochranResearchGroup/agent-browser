import React from "react";
import { createRoot } from "react-dom/client";
import { WorkspaceAuthGate } from "../../packages/dashboard/src/components/workspace-auth-gate";
createRoot(document.getElementById("root")!).render(<WorkspaceAuthGate>{user => <main>Selected browser retained for {user.username}</main>}</WorkspaceAuthGate>);
