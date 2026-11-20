"use client";
import {dashboardLocalFailure} from "@/lib/response-failure";
import {reportDashboardComponentFailure} from "@/lib/failure-observation";


import { useEffect, useState, type ReactNode } from "react";
import { fetchDashboardAuthStatus } from "@/lib/dashboard-auth-status";

export type WorkspaceUser = { username: string; displayName: string; role: string; authenticationSource?: "ingress" | "local" };

/** Workspace sign-in belongs to ingress. Agent Browser and Remote View require the same Authelia guard. */
export function WorkspaceAuthGate({ children }: {
  children: (user: WorkspaceUser, logout: () => Promise<void>) => ReactNode;
}) {
  const [user, setUser] = useState<WorkspaceUser | null>(null);
  const [checking, setChecking] = useState(true);
  const [failure, setFailure] = useState("Sign in through the Authelia-protected workspace URL, then retry.");
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    let cancelled = false;
    setChecking(true);
    void (async () => {
      try {
        const response = await fetchDashboardAuthStatus();
        if (!response.headers.get("content-type")?.includes("application/json")) throw dashboardLocalFailure(new Error("Workspace returned an unexpected sign-in response. Reopen its authenticated public URL."),"dashboard_auth_response_unverified","packages/dashboard/src/components/workspace-auth-gate.tsx","WorkspaceAuthGate");
        const payload = await response.json().catch(() => { throw dashboardLocalFailure(new Error("Workspace authentication returned invalid data. Check service access, then retry."),"dashboard_auth_response_unverified","packages/dashboard/src/components/workspace-auth-gate.tsx","WorkspaceAuthGate"); });
        if (cancelled) return;
        if (response.status === 403) setFailure("Access denied. Ask the workspace administrator to check Authelia ingress permissions.");
        else if (payload.authenticated && payload.user?.authenticationSource === "ingress" && typeof payload.user?.username === "string" && payload.user.username
          && typeof payload.user.role === "string") setUser(payload.user);
        else if (payload.authenticated) throw dashboardLocalFailure(new Error("Workspace authentication returned an invalid identity. Check service access, then retry."),"dashboard_auth_response_unverified","packages/dashboard/src/components/workspace-auth-gate.tsx","WorkspaceAuthGate");
        else setFailure("Sign in through the Authelia-protected workspace URL, then retry.");
      } catch (error) {void reportDashboardComponentFailure(error,"workspace-auth-gate.tsx");
        if (!cancelled) setFailure(error instanceof Error ? error.message : "Workspace authentication service unavailable. Retry when service access is restored.");
      } finally {
        if (!cancelled) setChecking(false);
      }
    })();
    return () => { cancelled = true; };
  }, [attempt]);
  const logout = async () => {
    // Clear obsolete local cookies; Authelia owns the shared session and sign-out.
    try {
      const response = await fetch("/api/dashboard-auth/logout", { method: "POST", credentials: "same-origin" });
      if (!response.ok) throw dashboardLocalFailure(new Error("Could not clear the old dashboard session. Retry after restoring service access."),"dashboard_auth_response_unverified","packages/dashboard/src/components/workspace-auth-gate.tsx","logout");
    } catch (error) {void reportDashboardComponentFailure(error,"workspace-auth-gate.tsx");
      setUser(null);
      setFailure("Could not clear the old dashboard session. Retry after restoring service access.");
      return;
    }
    setUser(null);
    setFailure("Workspace sign-in and sign-out are managed by Authelia for Agent Browser and Remote View.");
  };
  if (user) return children(user, logout);
  return <main className="dashboard-login-main" aria-live="polite" aria-busy={checking}>
    <section className="dashboard-login-panel">
      <h1>Agent Browser workspace</h1>
      <p>{checking ? "Restoring workspace access…" : failure}</p>
      {!checking && <><button onClick={() => setAttempt(value => value + 1)}>Retry workspace access</button>
        <button onClick={() => window.location.reload()}>Open workspace sign-in</button></>}
    </section>
  </main>;
}
