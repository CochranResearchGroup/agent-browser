"use client";

import { useEffect, useState, type ReactNode } from "react";
import { fetchDashboardAuthStatus } from "@/lib/dashboard-auth-status";

export type WorkspaceUser = { username: string; displayName: string; role: string; authenticationSource?: "ingress" | "local" };

/** Workspace sign-in belongs to ingress. Local and machine sessions remain API contracts. */
export function WorkspaceAuthGate({ children }: {
  children: (user: WorkspaceUser, logout: () => Promise<void>) => ReactNode;
}) {
  const [user, setUser] = useState<WorkspaceUser | null>(null);
  const [checking, setChecking] = useState(true);
  const [failure, setFailure] = useState("Sign in through the authenticated workspace URL, then retry. Local clients can establish a supported dashboard API session.");
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    let cancelled = false;
    setChecking(true);
    void (async () => {
      try {
        const response = await fetchDashboardAuthStatus();
        if (!response.headers.get("content-type")?.includes("application/json")) throw new Error("Workspace returned an unexpected sign-in response. Reopen its authenticated public URL.");
        const payload = await response.json().catch(() => { throw new Error("Workspace authentication returned invalid data. Check service access, then retry."); });
        if (cancelled) return;
        if (response.status === 403) setFailure("Access denied. Ask the workspace administrator to check ingress permissions and your workspace role.");
        else if (payload.authenticated && typeof payload.user?.username === "string" && payload.user.username
          && typeof payload.user.role === "string") setUser(payload.user);
        else if (payload.authenticated) throw new Error("Workspace authentication returned an invalid identity. Check service access, then retry.");
        else setFailure("Sign in through the authenticated workspace URL, then retry. Local clients can establish a supported dashboard API session.");
      } catch (error) {
        if (!cancelled) setFailure(error instanceof Error ? error.message : "Workspace authentication service unavailable. Retry when service access is restored.");
      } finally {
        if (!cancelled) setChecking(false);
      }
    })();
    return () => { cancelled = true; };
  }, [attempt]);
  const logout = async () => {
    // This ends only a local dashboard session; ingress owns its independent sign-out.
    try {
      const response = await fetch("/api/dashboard-auth/logout", { method: "POST", credentials: "same-origin" });
      if (!response.ok) throw new Error("Local session could not be ended. Retry after restoring service access.");
    } catch {
      setUser(null);
      setFailure("Local session could not be ended. Retry after restoring service access.");
      return;
    }
    setUser(null);
    setFailure("Local session ended. Workspace sign-in is managed by the ingress provider.");
  };
  if (user) return children(user, logout);
  return <main className="dashboard-login-main" aria-live="polite">
    <section className="dashboard-login-panel">
      <h1>Agent Browser workspace</h1>
      <p>{checking ? "Restoring workspace access…" : failure}</p>
      {!checking && <><button onClick={() => setAttempt(value => value + 1)}>Retry workspace access</button>
        <button onClick={() => window.location.reload()}>Open workspace sign-in</button></>}
    </section>
  </main>;
}
