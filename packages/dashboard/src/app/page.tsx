"use client";

import { type FormEvent, useCallback, useEffect, useState } from "react";
import { useAtomValue, useSetAtom } from "jotai/react";
import { activePortAtom, sessionsAtom, newSessionDialogAtom } from "@/store/sessions";
import { useSessionsSync } from "@/store/sessions";
import { useStreamSync, hasConsoleErrorsAtom, consoleLogsAtom } from "@/store/stream";
import { useActivitySync } from "@/store/activity";
import { activeExtensionsAtom } from "@/store/sessions";
import { useChatStatusSync } from "@/store/chat";
import { useMediaQuery } from "@/hooks/use-media-query";
import { useSelectedWorkspaceContext } from "@/hooks/use-selected-workspace-context";
import { useWorkspaceViewPreferences } from "@/hooks/use-workspace-view-preferences";
import { Viewport } from "@/components/viewport";
import { WorkspaceRemoteViewport } from "@/components/workspace-remote-viewport";
import { WorkspaceSelectionPanel } from "@/components/workspace-selection-panel";
import { ActivityFeed } from "@/components/activity-feed";
import { ChatPanel } from "@/components/chat-panel";
import { ConsolePanel } from "@/components/console-panel";
import { StoragePanel } from "@/components/storage-panel";
import { ExtensionsPanel } from "@/components/extensions-panel";
import { NetworkPanel } from "@/components/network-panel";
import { WorkspaceNavigator } from "@/components/workspace-navigator";
import { AppShell, type DashboardSection } from "@/components/app-shell";
import {
  DASHBOARD_WORKSPACE_SELECTION_EVENT,
  dashboardWorkspaceSelectionHasValue,
  readDashboardWorkspaceUrlSelection,
  writeDashboardWorkspaceUrlSelection,
} from "@/lib/workspace-url-selection";
import {
  hashOpaqueIdentifier,
  installDashboardFetchFailureInstrumentation,
  reportDashboardFailure,
} from "@/lib/failure-observation";
import { fetchDashboardAuthStatus } from "@/lib/dashboard-auth-status";
import {
  ServiceDetailInspector,
  ServicePanel,
  type ServiceInspectorActions,
  type ServiceInspectorSelection,
} from "@/components/service-panel";
import {
  ResizablePanelGroup,
  ResizablePanel,
  ResizableHandle,
} from "@/components/ui/resizable";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import {
  AlertTriangle,
  PanelLeftClose,
  PanelLeftOpen,
  PanelRightClose,
  PanelRightOpen,
  Plus,
  ShieldCheck,
} from "lucide-react";

const LEFT_PANE_COLLAPSED_KEY = "agent-browser-dashboard-left-pane-collapsed";
const RIGHT_PANE_COLLAPSED_KEY = "agent-browser-dashboard-right-pane-collapsed";
const SECTION_PATHS: Record<DashboardSection, string> = {
  overview: "/",
  browsers: "/browsers",
  service: "/service",
  activity: "/activity",
};
type MobileDashboardPanel = "workspaces" | "viewport" | "activity" | "service";
type RightPaneTab = "workspace" | "chat" | "activity" | "console" | "network" | "storage" | "extensions";
type DashboardAuthUser = {
  username: string;
  displayName?: string;
  role?: string;
};

type DashboardAuthStatus = {
  authenticated: boolean;
  user?: DashboardAuthUser | null;
};

type RemoteViewHandoffResolution = {
  status?: string;
  resolved?: boolean;
  reopenRequired?: boolean;
  handoffId?: string;
  handoffUrl?: string | null;
  browserId?: string | null;
  sessionName?: string | null;
  tabId?: string | null;
  targetId?: string | null;
  viewStreamProvider?: string | null;
  requiredViewStreamProvider?: string | null;
  presentationGeneration?: number | null;
  presentationReceipt?: {
    generation?: number | null;
    dashboardDeploymentGeneration?: string | null;
    logicalBrowserId?: string | null;
    daemonOwnerGeneration?: number | null;
    processInstanceDigest?: string | null;
    targetId?: string | null;
    requiredStreamProvider?: string | null;
    observedStreamProvider?: string | null;
    state?: string | null;
  } | null;
  message?: string | null;
  tab?: Record<string, unknown> | null;
  open?: Record<string, unknown> | null;
};

function durableHandoffPresentationReady(resolution: RemoteViewHandoffResolution): boolean {
  const receipt = resolution.presentationReceipt;
  if (!receipt) return false;
  return resolution.resolved === true
    && resolution.status === "ready"
    && Number.isInteger(resolution.presentationGeneration)
    && Number(resolution.presentationGeneration) > 0
    && receipt.generation === resolution.presentationGeneration
    && Boolean(receipt.dashboardDeploymentGeneration)
    && receipt.logicalBrowserId === resolution.browserId
    && Number.isInteger(receipt.daemonOwnerGeneration)
    && Number(receipt.daemonOwnerGeneration) > 0
    && Boolean(receipt.processInstanceDigest)
    && Boolean(resolution.targetId)
    && receipt.targetId === resolution.targetId
    && Boolean(resolution.viewStreamProvider)
    && receipt.requiredStreamProvider === resolution.viewStreamProvider
    && receipt.observedStreamProvider === receipt.requiredStreamProvider
    && receipt.state === "ready";
}

type RemoteViewHandoffApiResponse = {
  success: boolean;
  data?: RemoteViewHandoffResolution;
  error?: string | null;
};

type RuntimeManifest = {
  schemaVersion?: string;
  runtimeEnvironment?: "production" | "development";
  packageVersion?: string;
  serviceContractVersion?: string;
  supportedUiFeatures?: string[];
  dashboard?: {
    sha256?: string;
    assetCount?: number;
  };
  executable?: {
    path?: string | null;
    sha256?: string | null;
  };
};

type RuntimeManifestState = {
  loading: boolean;
  manifest: RuntimeManifest | null;
  issue: string | null;
};

type RuntimeHealthIssue = {
  code?: string;
  severity?: string;
  message?: string;
  sessions?: string[];
  recommendedAction?: string;
};

type DashboardHealthFinding = {
  code: string;
  blocking: boolean;
  message: string;
};

type DashboardHealthAxes = {
  schemaVersion: "agent-browser.dashboard-health.v1";
  runtime: {
    state: "ready" | "degraded" | "blocked" | "unknown";
    ready: boolean;
    findings: DashboardHealthFinding[];
  };
  convergence: {
    state: "ready" | "degraded" | "blocked" | "unknown";
    ready: boolean;
    findings: DashboardHealthFinding[];
  };
  access: {
    state: "allowed" | "attention" | "denied" | "unknown";
    findings: DashboardHealthFinding[];
  };
  acquisition: {
    state: "available" | "waiting" | "denied" | "unknown";
    requestScoped: true;
    findings: DashboardHealthFinding[];
  };
};

type SessionSupervisorHealth = {
  ready?: boolean;
  count?: number;
  degradedCount?: number;
  sessions?: Array<{
    session?: string;
    state?: string;
    ready?: boolean;
    streamPort?: number;
    publishedStreamPort?: number;
    executableMatches?: boolean;
  }>;
  issues?: RuntimeHealthIssue[];
};

type RuntimeMultiplicityHealth = {
  state?: string;
  steadyState?: boolean;
  convergenceWindow?: {
    active?: boolean;
    state?: string;
    transactionId?: string;
    deadline?: string | null;
  } | null;
  counts?: {
    dashboardProcesses?: number;
    runtimeHosts?: number;
    legacyDaemons?: number;
    executableGenerations?: number;
  };
  issues?: string[];
};

type RuntimeMonitorHealth = {
  ready?: boolean;
  state?: string;
  fresh?: boolean;
  ageSeconds?: number | null;
  receipt?: {
    consecutiveFailures?: number;
    effects?: {
      service?: {
        resources?: {
          summary?: {
            candidateRssBytes?: number;
            protectedRssBytes?: number;
            observedRssBytes?: number;
            totalRssBytes?: number;
          };
        };
        cleanupObligations?: {
          trackedCount?: number;
          missingCount?: number;
        };
      };
      generations?: {
        removed?: unknown[];
        retained?: unknown[];
      };
    };
    incident?: {
      type?: string;
      failureCount?: number;
    } | null;
  };
};

type RuntimeHealth = {
  schemaVersion?: string;
  state?: "ready" | "degraded" | string;
  ready?: boolean;
  observedAtEpochMs?: number;
  staleRuntimeCount?: number;
  staleSessions?: string[];
  sessionSupervisors?: SessionSupervisorHealth;
  issues?: RuntimeHealthIssue[];
  dashboardHealth?: DashboardHealthAxes;
  workstationConvergence?: {
    schemaVersion?: string;
    state?: string;
    ready?: boolean;
    executableNextAction?: string | null;
    dashboardHealth?: DashboardHealthAxes;
  };
  runtimeMultiplicity?: RuntimeMultiplicityHealth;
  runtimeMonitor?: RuntimeMonitorHealth;
  runtimeLifecycle?: {
    ready?: boolean;
    state?: string;
    multiplicity?: RuntimeMultiplicityHealth | null;
    lifecycle?: {
      available?: boolean;
      ownerCount?: number;
      recordCount?: number;
    };
    reconciliation?: RuntimeMonitorHealth | null;
    resources?: {
      summary?: {
        candidateRssBytes?: number;
        protectedRssBytes?: number;
        observedRssBytes?: number;
        totalRssBytes?: number;
      };
    } | null;
    cleanupObligations?: { trackedCount?: number; missingCount?: number } | null;
    retention?: {
      generations?: { removed?: unknown[]; retained?: unknown[] } | null;
    };
    incident?: { type?: string; failureCount?: number } | null;
  };
  workstationUpgrade?: {
    selectedGenerationId?: string | null;
    admissionDraining?: boolean;
    latestTransaction?: {
      transactionId?: string;
      state?: string;
      oldGenerationId?: string | null;
      candidateGenerationId?: string;
      runtimeMigrations?: Array<{
        logicalBrowserId?: string;
        classification?: string;
        disposition?: string;
        receipted?: boolean;
        reasonCodes?: string[];
      }>;
      terminalResult?: string | null;
      stopReason?: string | null;
    } | null;
  };
};

type RuntimeHealthState = {
  loading: boolean;
  health: RuntimeHealth | null;
  issue: string | null;
};

const REQUIRED_RUNTIME_FEATURES = [
  "workspace.detectedBrowsers",
  "workspace.foreignCdpBorrow",
  "workspace.noRetainedLiveRail",
] as const;

const REQUIRED_RUNTIME_CONTRACT = "service-ui-runtime.v1";

function workstationConvergenceIssue(health: RuntimeHealth): string | null {
  const axes = health.dashboardHealth;
  if (!axes) return null;
  const acquisition = health.dashboardHealth?.acquisition;
  if (acquisition?.requestScoped !== true) {
    return "The runtime health contract did not preserve request-scoped acquisition state.";
  }
  if (axes.runtime.ready && axes.convergence.ready) return null;
  const finding = axes.runtime.findings.find((candidate) => candidate.blocking)
    ?? axes.convergence.findings.find((candidate) => candidate.blocking);
  const message = finding?.message || "The installed runtime has not reached its selected convergence state.";
  const action = health.workstationConvergence?.executableNextAction;
  return action ? `${message} Next action: ${action}.` : message;
}

function dashboardSectionFromPath(pathname: string): DashboardSection {
  const segments = pathname.split("/").filter(Boolean);
  const segment = segments[segments.length - 1];
  if (segment === "browsers" || segment === "service" || segment === "activity") return segment;
  return "overview";
}

function dashboardSectionUrl(section: DashboardSection): string {
  if (typeof window === "undefined") return SECTION_PATHS[section];
  const params = new URLSearchParams(window.location.search);
  const search = params.toString();
  return `${SECTION_PATHS[section]}${search ? `?${search}` : ""}${window.location.hash}`;
}

function readWorkspaceViewportRoute(): boolean {
  if (typeof window === "undefined") return false;
  const params = new URLSearchParams(window.location.search);
  const view = params.get("view");
  if (view === "workspace:tile") return true;
  if (view !== "workspace:control" && view !== "workspace:view") return false;
  return dashboardWorkspaceSelectionHasValue(readDashboardWorkspaceUrlSelection());
}

function remoteViewHandoffIdFromPath(pathname: string): string | null {
  const match = pathname.match(/^\/remote-view\/([^/]+)\/?$/);
  if (!match?.[1]) return null;
  try {
    return decodeURIComponent(match[1]);
  } catch {
    return null;
  }
}

function remoteViewResolutionString(
  record: Record<string, unknown> | null | undefined,
  key: string,
): string | null {
  const value = record?.[key];
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

function readStoredBoolean(key: string, fallback: boolean): boolean {
  if (typeof window === "undefined") return fallback;
  const value = window.localStorage.getItem(key);
  if (value === null) return fallback;
  return value === "true";
}

function writeStoredBoolean(key: string, value: boolean): void {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(key, String(value));
}

function runtimeManifestIssue(manifest: RuntimeManifest): string | null {
  if (manifest.schemaVersion !== "agent-browser.runtime-manifest.v1") {
    return "The installed binary is not reporting the dashboard runtime manifest contract.";
  }
  if (manifest.serviceContractVersion !== REQUIRED_RUNTIME_CONTRACT) {
    return `The installed binary reports ${manifest.serviceContractVersion || "no runtime contract"}; this UI expects ${REQUIRED_RUNTIME_CONTRACT}.`;
  }
  if (!manifest.dashboard?.sha256) {
    return "The installed binary did not report an embedded dashboard bundle identity.";
  }
  const features = new Set(manifest.supportedUiFeatures ?? []);
  const missing = REQUIRED_RUNTIME_FEATURES.filter((feature) => !features.has(feature));
  if (missing.length > 0) {
    return `The installed binary is missing UI feature support: ${missing.join(", ")}.`;
  }
  return null;
}

export default function DashboardPage({
  initialSection = "overview",
}: {
  initialSection?: DashboardSection;
} = {}) {
  return <DashboardAuthGate initialSection={initialSection} />;
}

function DashboardAuthGate({ initialSection }: { initialSection: DashboardSection }) {
  const [checking, setChecking] = useState(true);
  const [user, setUser] = useState<DashboardAuthUser | null>(null);

  useEffect(() => {
    let cancelled = false;
    async function checkAuth() {
      try {
        const response = await fetchDashboardAuthStatus();
        const payload = (await response.json()) as DashboardAuthStatus;
        if (cancelled) return;
        setUser(payload.authenticated ? payload.user ?? null : null);
      } catch {
        if (!cancelled) setUser(null);
      } finally {
        if (!cancelled) setChecking(false);
      }
    }
    void checkAuth();
    return () => {
      cancelled = true;
    };
  }, []);

  const handleLogin = useCallback((nextUser: DashboardAuthUser) => {
    setUser(nextUser);
    if (typeof window === "undefined") return;
    const next = new URLSearchParams(window.location.search).get("next");
    if (next?.startsWith("/") && !next.startsWith("//")) {
      if (next.startsWith("/guacamole/")) {
        window.location.assign(next);
        return;
      }
      window.history.replaceState({ dashboardAuth: true }, "", next);
    } else if (window.location.pathname === "/login") {
      window.history.replaceState({ dashboardAuth: true }, "", "/");
    }
  }, []);

  const handleLogout = useCallback(async () => {
    await fetch("/api/dashboard-auth/logout", {
      method: "POST",
      credentials: "same-origin",
    }).catch(() => undefined);
    setUser(null);
    if (typeof window !== "undefined") {
      window.history.replaceState({ dashboardAuth: false }, "", "/login");
    }
  }, []);

  if (checking) {
    return <DashboardLoginScreen busy />;
  }

  if (!user) {
    return <DashboardLoginScreen onAuthenticated={handleLogin} />;
  }

  return (
    <RemoteViewHandoffGate
      initialSection={initialSection}
      user={user}
      onLogout={handleLogout}
    />
  );
}

function RemoteViewHandoffGate({
  initialSection,
  user,
  onLogout,
}: {
  initialSection: DashboardSection;
  user: DashboardAuthUser;
  onLogout: () => void;
}) {
  const handoffId = typeof window === "undefined"
    ? null
    : remoteViewHandoffIdFromPath(window.location.pathname);
  const [resolution, setResolution] = useState<RemoteViewHandoffResolution | null>(null);
  const [resolving, setResolving] = useState(Boolean(handoffId));
  const [error, setError] = useState("");

  const resolveHandoff = useCallback(async (allowReopenClosed: boolean) => {
    if (!handoffId) return;
    setResolving(true);
    setError("");
    try {
      const response = await fetch("/api/service/request", {
        method: "POST",
        credentials: "same-origin",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "service_remote_view_handoff_resolve",
          serviceName: "agent-browser-dashboard",
          agentName: user.username || "operator",
          taskName: "durable-remote-view-handoff",
          params: { handoffId, allowReopenClosed },
          serviceStateLockTimeoutMs: 30_000,
          jobTimeoutMs: 90_000,
        }),
      });
      const payload = (await response.json()) as RemoteViewHandoffApiResponse;
      if (!response.ok || !payload.success || !payload.data) {
        throw new Error(payload.error || "The remote-view handoff could not be resolved.");
      }
      let nextResolution = payload.data;
      const presentationMayStillConverge = nextResolution.resolved === true
        || nextResolution.status === "ready"
        || nextResolution.status === "converging";
      if (presentationMayStillConverge && !durableHandoffPresentationReady(nextResolution)) {
        nextResolution = {
          ...nextResolution,
          status: "converging",
          resolved: false,
          message: "The retained browser is attached, but its authenticated presentation generation is still converging.",
        };
      }
      setResolution(nextResolution);
      if (!nextResolution.resolved || nextResolution.status !== "ready") return;

      const tab = nextResolution.tab ?? null;
      const open = nextResolution.open ?? null;
      const intent = open?.intent && typeof open.intent === "object"
        ? open.intent as Record<string, unknown>
        : null;
      const serviceTabHandle = tab?.serviceTabHandle && typeof tab.serviceTabHandle === "object"
        ? tab.serviceTabHandle as Record<string, unknown>
        : null;
      const browserId = nextResolution.browserId
        ?? remoteViewResolutionString(tab, "browserId")
        ?? remoteViewResolutionString(serviceTabHandle, "browserId");
      const sessionName = nextResolution.sessionName
        ?? remoteViewResolutionString(tab, "sessionId")
        ?? remoteViewResolutionString(serviceTabHandle, "sessionName");
      const targetId = nextResolution.targetId
        ?? remoteViewResolutionString(tab, "targetId")
        ?? remoteViewResolutionString(serviceTabHandle, "targetId");
      const tabId = nextResolution.tabId
        ?? remoteViewResolutionString(tab, "tabId")
        ?? remoteViewResolutionString(tab, "id")
        ?? remoteViewResolutionString(serviceTabHandle, "tabId")
        ?? (targetId ? `target:${targetId}` : null);
      const profileId = remoteViewResolutionString(tab, "profileId")
        ?? remoteViewResolutionString(tab, "runtimeProfile")
        ?? remoteViewResolutionString(serviceTabHandle, "profileId")
        ?? remoteViewResolutionString(intent, "runtimeProfile")
        ?? remoteViewResolutionString(intent, "profile");

      const params = new URLSearchParams(window.location.search);
      params.delete("next");
      if (nextResolution.viewStreamProvider) {
        params.set("view-provider", nextResolution.viewStreamProvider);
      }
      params.set("view", "workspace:control");
      const search = params.toString();
      window.history.replaceState(
        { ...(window.history.state ?? {}), remoteViewHandoff: handoffId },
        "",
        `${window.location.pathname}${search ? `?${search}` : ""}`,
      );
      writeDashboardWorkspaceUrlSelection({
        workspaceId: browserId ? `browser:${browserId}` : null,
        browserId,
        sessionId: sessionName,
        tabId,
        profileId,
        jobId: handoffId,
      }, "replace");
    } catch (cause) {
      void hashOpaqueIdentifier(handoffId).then((handoffIdHash) => reportDashboardFailure({
        category: "handoff_link",
        stage: "resolve",
        code: "handoff_unusable",
        summary: "The authenticated dashboard could not resolve the durable handoff into a usable view.",
        action: "service_remote_view_handoff_resolve",
        handoffIdHash,
      }));
      setError(cause instanceof Error ? cause.message : "The remote-view handoff could not be resolved.");
    } finally {
      setResolving(false);
    }
  }, [handoffId, user.username]);

  useEffect(() => {
    if (handoffId) void resolveHandoff(false);
  }, [handoffId, resolveHandoff]);

  useEffect(() => {
    if (resolution?.status !== "converging") return;
    const retry = window.setTimeout(() => void resolveHandoff(false), 1_000);
    return () => window.clearTimeout(retry);
  }, [resolution?.status, resolveHandoff]);

  if (!handoffId) {
    return <DashboardExperience initialSection={initialSection} user={user} onLogout={onLogout} />;
  }

  if (resolving) {
    return <DashboardLoginScreen busy />;
  }

  if (resolution?.status === "closed" && resolution.reopenRequired) {
    return (
      <main className="flex min-h-screen items-center justify-center bg-background p-6">
        <section className="w-full max-w-lg space-y-4 rounded-xl border bg-card p-6 shadow-sm">
          <AlertTriangle className="size-6 text-amber-500" />
          <h1 className="text-xl font-semibold">This browser tab was closed</h1>
          <p className="text-sm text-muted-foreground">
            {resolution.message || "Opening it again requires an explicit operator action."}
          </p>
          <div className="flex gap-3">
            <Button onClick={() => void resolveHandoff(true)}>Reopen tab</Button>
            <Button variant="outline" onClick={onLogout}>Sign out</Button>
          </div>
        </section>
      </main>
    );
  }

  if (resolution?.status === "converging") {
    return (
      <main className="flex min-h-screen items-center justify-center bg-background p-6">
        <section className="w-full max-w-lg space-y-4 rounded-xl border bg-card p-6 shadow-sm">
          <h1 className="text-xl font-semibold">Restoring remote view</h1>
          <p className="text-sm text-muted-foreground">
            {resolution.message || "The requested presentation is still converging."}
          </p>
          <div className="flex gap-3">
            <Button onClick={() => void resolveHandoff(false)}>Retry now</Button>
            <Button variant="outline" onClick={onLogout}>Sign out</Button>
          </div>
        </section>
      </main>
    );
  }

  if (error || resolution?.status === "not_found") {
    return (
      <main className="flex min-h-screen items-center justify-center bg-background p-6">
        <section className="w-full max-w-lg space-y-4 rounded-xl border bg-card p-6 shadow-sm">
          <AlertTriangle className="size-6 text-destructive" />
          <h1 className="text-xl font-semibold">Remote view unavailable</h1>
          <p className="text-sm text-muted-foreground">
            {error || resolution?.message || "The handoff no longer exists."}
          </p>
          <div className="flex gap-3">
            {error ? <Button onClick={() => void resolveHandoff(false)}>Retry</Button> : null}
            <Button variant="outline" onClick={onLogout}>Sign out</Button>
          </div>
        </section>
      </main>
    );
  }

  return <DashboardExperience initialSection={initialSection} user={user} onLogout={onLogout} />;
}

function DashboardLoginScreen({
  busy = false,
  onAuthenticated,
}: {
  busy?: boolean;
  onAuthenticated?: (user: DashboardAuthUser) => void;
}) {
  const [username, setUsername] = useState("admin");
  const [password, setPassword] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (busy || submitting || !onAuthenticated) return;
    setSubmitting(true);
    setError(null);
    try {
      const response = await fetch("/api/dashboard-auth/login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "same-origin",
        body: JSON.stringify({ username, password }),
      });
      const payload = await response.json() as DashboardAuthStatus & { error?: string };
      if (!response.ok || !payload.authenticated || !payload.user) {
        setError(payload.error || "Invalid username or password.");
        return;
      }
      onAuthenticated(payload.user);
    } catch {
      setError("Dashboard auth is not reachable.");
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="dashboard-root dashboard-login-root">
      <div className="dashboard-aurora dashboard-aurora-one" />
      <div className="dashboard-aurora dashboard-aurora-two" />
      <main className="dashboard-login-main">
        <form className="dashboard-login-panel" onSubmit={submit}>
          <div className="dashboard-login-mark">
            <ShieldCheck className="size-5" />
          </div>
          <div className="dashboard-login-heading">
            <p className="dashboard-login-title">Agent Browser</p>
            <p className="dashboard-login-subtitle">Superuser access required</p>
          </div>
          <label className="dashboard-login-field">
            <span>Username</span>
            <input
              autoComplete="username"
              disabled={busy || submitting}
              value={username}
              onChange={(event) => setUsername(event.target.value)}
            />
          </label>
          <label className="dashboard-login-field">
            <span>Password</span>
            <input
              autoComplete="current-password"
              disabled={busy || submitting}
              type="password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
            />
          </label>
          {error && <p className="dashboard-login-error">{error}</p>}
          <Button
            type="submit"
            className="dashboard-primary-action dashboard-login-submit"
            disabled={busy || submitting || !username || !password}
          >
            <ShieldCheck className="size-4" />
            {busy ? "Checking" : submitting ? "Signing in" : "Sign in"}
          </Button>
        </form>
      </main>
    </div>
  );
}

function RuntimeManifestNotice({ state }: { state: RuntimeManifestState }) {
  if (!state.issue) return null;
  return (
    <div
      className="dashboard-runtime-notice"
      role="status"
      data-runtime-manifest-warning="true"
    >
      <AlertTriangle className="size-4 shrink-0" />
      <div className="min-w-0">
        <p>Runtime contract drift</p>
        <span>
          {state.issue} Run <code>pnpm publish:local-dashboard -- --expect-marker "&lt;changed-ui-marker&gt;" --json</code>.
        </span>
      </div>
    </div>
  );
}

function RuntimeHealthNotice({ state }: { state: RuntimeHealthState }) {
  if (!state.issue) return null;
  const sessions = state.health?.staleSessions ?? [];
  return (
    <div
      className="dashboard-runtime-notice"
      role="status"
      data-runtime-health-warning="true"
    >
      <AlertTriangle className="size-4 shrink-0" />
      <div className="min-w-0">
        <p>Runtime convergence action required</p>
        <span>
          {state.issue}
          {sessions.length > 0 ? ` Affected sessions: ${sessions.join(", ")}.` : ""}
          {" "}Active sessions are never restarted automatically.
        </span>
      </div>
    </div>
  );
}

function RuntimeHealthSummary({ state }: { state: RuntimeHealthState }) {
  const lifecycle = state.health?.runtimeLifecycle;
  const multiplicity = lifecycle?.multiplicity ?? state.health?.runtimeMultiplicity;
  const monitor = lifecycle?.reconciliation ?? state.health?.runtimeMonitor;
  if (!multiplicity && !monitor) return null;
  const counts = multiplicity?.counts;
  const cleanup = lifecycle?.cleanupObligations
    ?? monitor?.receipt?.effects?.service?.cleanupObligations;
  const pressure = lifecycle?.resources?.summary
    ?? monitor?.receipt?.effects?.service?.resources?.summary;
  const generations = lifecycle?.retention?.generations
    ?? monitor?.receipt?.effects?.generations;
  const convergenceWindow = multiplicity?.convergenceWindow;
  const access = state.health?.dashboardHealth?.access;
  const healthy = multiplicity?.steadyState === true
    && monitor?.ready === true
    && (cleanup?.missingCount ?? 0) === 0;
  return (
    <div
      className="dashboard-runtime-notice"
      role="status"
      data-runtime-health-summary={healthy ? "ready" : "attention"}
    >
      {healthy
        ? <ShieldCheck className="size-4 shrink-0" />
        : <AlertTriangle className="size-4 shrink-0" />}
      <div className="min-w-0">
        <p>{healthy ? "Runtime healthy" : "Runtime convergence"}</p>
        <span>
	          Dashboard {counts?.dashboardProcesses ?? "?"}, host {counts?.runtimeHosts ?? "?"}, legacy daemons {counts?.legacyDaemons ?? "?"}, generations {counts?.executableGenerations ?? "?"}.
	          {convergenceWindow?.active ? ` Convergence window ${convergenceWindow.state ?? "active"}${convergenceWindow.transactionId ? ` (${convergenceWindow.transactionId})` : ""}.` : ""}
          {cleanup ? ` Cleanup obligations ${cleanup.trackedCount ?? 0} tracked, ${cleanup.missingCount ?? 0} missing.` : ""}
          {pressure ? ` RSS ${formatRuntimeBytes(pressure.protectedRssBytes)} protected, ${formatRuntimeBytes(pressure.candidateRssBytes)} reclaimable, ${formatRuntimeBytes(pressure.observedRssBytes)} unowned.` : ""}
          {generations ? ` Last retention pass removed ${generations.removed?.length ?? 0} and retained ${generations.retained?.length ?? 0}.` : ""}
          {monitor?.state ? ` Monitor ${monitor.state}${monitor.ageSeconds == null ? "" : ` (${monitor.ageSeconds}s old)`}.` : ""}
          {(lifecycle?.incident ?? monitor?.receipt?.incident)?.type ? ` Blocking incident ${(lifecycle?.incident ?? monitor?.receipt?.incident)?.type} after ${(lifecycle?.incident ?? monitor?.receipt?.incident)?.failureCount ?? "?"} failures.` : ""}
          {access ? ` Access ${access.state}${access.findings.length > 0 ? `: ${access.findings.map((finding) => finding.message).join(" ")}` : "."}` : ""}
        </span>
      </div>
    </div>
  );
}

function formatRuntimeBytes(value?: number): string {
  if (!Number.isFinite(value)) return "?";
  const bytes = Math.max(0, Number(value));
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KiB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MiB`;
  return `${(bytes / 1024 ** 3).toFixed(1)} GiB`;
}

function DashboardExperience({
  initialSection = "overview",
  user,
  onLogout,
}: {
  initialSection?: DashboardSection;
  user: DashboardAuthUser;
  onLogout: () => void;
}) {
  const [activeSection, setActiveSection] = useState<DashboardSection>(() => {
    if (typeof window === "undefined") return initialSection;
    return dashboardSectionFromPath(window.location.pathname);
  });
  const [leftPaneCollapsed, setLeftPaneCollapsed] = useState(() =>
    readStoredBoolean(LEFT_PANE_COLLAPSED_KEY, false),
  );
  const [rightPaneCollapsed, setRightPaneCollapsed] = useState(() =>
    readStoredBoolean(RIGHT_PANE_COLLAPSED_KEY, true),
  );
  const [mobilePanel, setMobilePanel] = useState<MobileDashboardPanel>(() => {
    if (initialSection === "service") return "service";
    if (initialSection === "activity") return "activity";
    return "viewport";
  });
  const [serviceInspectorSelection, setServiceInspectorSelection] = useState<ServiceInspectorSelection | null>(null);
  const [serviceInspectorActions, setServiceInspectorActions] = useState<ServiceInspectorActions>({});
  const [hasWorkspaceViewportRoute, setHasWorkspaceViewportRoute] = useState(() => readWorkspaceViewportRoute());
  const [sidePanelTab, setSidePanelTab] = useState<RightPaneTab>("chat");
  const [runtimeManifest, setRuntimeManifest] = useState<RuntimeManifestState>({
    loading: true,
    manifest: null,
    issue: null,
  });
  const [runtimeHealth, setRuntimeHealth] = useState<RuntimeHealthState>({
    loading: true,
    health: null,
    issue: null,
  });
  const activePort = useAtomValue(activePortAtom);
  useStreamSync(activePort);
  useSessionsSync();
  useActivitySync();
  useChatStatusSync();

  useEffect(() => installDashboardFetchFailureInstrumentation(), []);

  const sessions = useAtomValue(sessionsAtom);
  const hasSessions = sessions.length > 0;
  const setNewSessionDialog = useSetAtom(newSessionDialogAtom);
  const isDesktop = useMediaQuery("(min-width: 768px)");
  const hasConsoleErrors = useAtomValue(hasConsoleErrorsAtom);
  const activeExtensions = useAtomValue(activeExtensionsAtom);
  const selectedWorkspaceContextEnabled = !rightPaneCollapsed ||
    hasWorkspaceViewportRoute ||
    mobilePanel === "viewport" ||
    mobilePanel === "activity";
  const workspaceViewPreferences = useWorkspaceViewPreferences();
  const selectedWorkspace = useSelectedWorkspaceContext(
    selectedWorkspaceContextEnabled,
    workspaceViewPreferences.snapshot,
  );
  const changeDashboardSection = useCallback((section: DashboardSection) => {
    setActiveSection(section);
    if (typeof window === "undefined") return;
    const nextUrl = dashboardSectionUrl(section);
    const currentUrl = `${window.location.pathname}${window.location.search}${window.location.hash}`;
    if (nextUrl !== currentUrl) {
      window.history.pushState({ dashboardSection: section }, "", nextUrl);
    }
  }, []);
  const openNewSession = useCallback(() => {
    if (!isDesktop) setMobilePanel("workspaces");
    setNewSessionDialog(true);
  }, [isDesktop, setNewSessionDialog]);

  useEffect(() => {
    const onPopState = () => {
      setActiveSection(dashboardSectionFromPath(window.location.pathname));
      setHasWorkspaceViewportRoute(readWorkspaceViewportRoute());
    };
    window.addEventListener("popstate", onPopState);
    return () => window.removeEventListener("popstate", onPopState);
  }, []);
  useEffect(() => {
    let cancelled = false;
    const checkRuntimeHealth = async () => {
      try {
        const response = await fetch("/api/runtime/health", {
          cache: "no-store",
          credentials: "same-origin",
        });
        if (!response.ok) {
          throw new Error(`HTTP ${response.status}`);
        }
        const health = await response.json() as RuntimeHealth;
        const issue = workstationConvergenceIssue(health);
        if (!cancelled) {
          setRuntimeHealth({ loading: false, health, issue });
        }
      } catch (error) {
        if (!cancelled) {
          const message = error instanceof Error ? error.message : String(error);
          setRuntimeHealth({
            loading: false,
            health: null,
            issue: `Live runtime health is unavailable (${message}).`,
          });
        }
      }
    };
    void checkRuntimeHealth();
    const interval = window.setInterval(checkRuntimeHealth, 10_000);
    return () => {
      cancelled = true;
      window.clearInterval(interval);
    };
  }, []);
  useEffect(() => {
    const onWorkspaceSelection = () => setHasWorkspaceViewportRoute(readWorkspaceViewportRoute());
    window.addEventListener(DASHBOARD_WORKSPACE_SELECTION_EVENT, onWorkspaceSelection);
    return () => window.removeEventListener(DASHBOARD_WORKSPACE_SELECTION_EVENT, onWorkspaceSelection);
  }, []);
  useEffect(() => {
    if (activeSection === "service" || activeSection === "activity") {
      setMobilePanel(activeSection);
    }
  }, [activeSection]);
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const response = await fetch("/api/runtime/manifest", { cache: "no-store" });
        if (!response.ok) {
          throw new Error(`HTTP ${response.status}`);
        }
        const manifest = await response.json() as RuntimeManifest;
        if (!cancelled) {
          setRuntimeManifest({
            loading: false,
            manifest,
            issue: runtimeManifestIssue(manifest),
          });
        }
      } catch (error) {
        if (!cancelled) {
          const message = error instanceof Error ? error.message : String(error);
          setRuntimeManifest({
            loading: false,
            manifest: null,
            issue: `The dashboard could not read the installed binary runtime manifest (${message}).`,
          });
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const openRightPane = useCallback(() => {
    setRightPaneCollapsed(false);
    writeStoredBoolean(RIGHT_PANE_COLLAPSED_KEY, false);
  }, []);
  useEffect(() => {
    const onWorkspaceSelection = () => {
      setSidePanelTab("workspace");
      openRightPane();
    };
    window.addEventListener(DASHBOARD_WORKSPACE_SELECTION_EVENT, onWorkspaceSelection);
    return () => window.removeEventListener(DASHBOARD_WORKSPACE_SELECTION_EVENT, onWorkspaceSelection);
  }, [openRightPane]);
  useEffect(() => {
    const onConsoleSendToChat = () => {
      setSidePanelTab("chat");
      openRightPane();
    };
    window.addEventListener("agent-browser-dashboard-console-send-to-chat", onConsoleSendToChat);
    return () => window.removeEventListener("agent-browser-dashboard-console-send-to-chat", onConsoleSendToChat);
  }, [openRightPane]);
  const inspectServiceSelection = useCallback((selection: ServiceInspectorSelection) => {
    setServiceInspectorSelection(selection);
    openRightPane();
  }, [openRightPane]);
  const toggleLeftPane = () => {
    const next = !leftPaneCollapsed;
    setLeftPaneCollapsed(next);
    writeStoredBoolean(LEFT_PANE_COLLAPSED_KEY, next);
  };
  const toggleRightPane = () => {
    const next = !rightPaneCollapsed;
    setRightPaneCollapsed(next);
    writeStoredBoolean(RIGHT_PANE_COLLAPSED_KEY, next);
  };
  const leftPaneToggle = (
    <Button
      type="button"
      variant="ghost"
      size="icon"
      className="dashboard-pane-toggle dashboard-pane-toggle-left"
      aria-label={leftPaneCollapsed ? "Show workspace pane" : "Collapse workspace pane"}
      title={leftPaneCollapsed ? "Show workspace pane" : "Collapse workspace pane"}
      onClick={toggleLeftPane}
    >
      {leftPaneCollapsed ? <PanelLeftOpen className="size-4" /> : <PanelLeftClose className="size-4" />}
    </Button>
  );
  const rightPaneToggle = (
    <Button
      type="button"
      variant="ghost"
      size="icon"
      className="dashboard-pane-toggle dashboard-pane-toggle-right"
      aria-label={rightPaneCollapsed ? "Show detail pane" : "Collapse detail pane"}
      title={rightPaneCollapsed ? "Show detail pane" : "Collapse detail pane"}
      onClick={toggleRightPane}
    >
      {rightPaneCollapsed ? <PanelRightOpen className="size-4" /> : <PanelRightClose className="size-4" />}
    </Button>
  );
  const primaryPanel = activeSection === "service"
    ? (
      <ServicePanel
        onInspectSelection={inspectServiceSelection}
        onInspectorActionsChange={setServiceInspectorActions}
      />
    )
    : activeSection === "activity"
      ? <ActivityFeed />
      : (
        <WorkspaceRemoteViewport
          fallback={<Viewport />}
          selectedWorkspaceContext={selectedWorkspace.context}
          projection={selectedWorkspace.projection}
          onRefresh={selectedWorkspace.refresh}
          onSelectStream={workspaceViewPreferences.write}
        />
      );
  const serviceInspectorPanel = (
    <ServiceDetailInspector selection={serviceInspectorSelection} actions={serviceInspectorActions} />
  );

  const sidePanel = (
    <Tabs value={sidePanelTab}
      onValueChange={(value) => setSidePanelTab(value as RightPaneTab)}
      className="flex h-full flex-col"
      data-selected-workspace-context={selectedWorkspace.context.node ? "ready" : selectedWorkspace.context.state}
      data-selected-workspace-id={selectedWorkspace.context.node?.id ?? ""}
      data-selected-workspace-state={selectedWorkspace.context.state}
      data-selected-workspace-source={selectedWorkspace.context.source}
    >
      <div className="shrink-0 px-2 pt-1">
        <TabsList variant="line" className="dashboard-right-tabs h-7 w-full">
          <TabsTrigger value="workspace" className="text-[11px]">Workspace</TabsTrigger>
          <TabsTrigger value="chat" className="text-[11px]">Chat</TabsTrigger>
          <TabsTrigger value="activity" className="text-[11px]">Activity</TabsTrigger>
          <TabsTrigger value="console" className="text-[11px]">
            Console
            {hasConsoleErrors && (
              <span className="ml-1 inline-flex size-1.5 rounded-full bg-destructive" />
            )}
          </TabsTrigger>
          <TabsTrigger value="network" className="text-[11px]">Network</TabsTrigger>
          <TabsTrigger value="storage" className="text-[11px]">Storage</TabsTrigger>
          <TabsTrigger value="extensions" className="text-[11px]">
            Extensions
            {activeExtensions.length > 0 && (
              <span className="ml-1 text-[9px] tabular-nums text-muted-foreground">{activeExtensions.length}</span>
            )}
          </TabsTrigger>
        </TabsList>
      </div>
      <TabsContent value="workspace" className="min-h-0 flex-1 overflow-auto">
        <WorkspaceSelectionPanel
          context={selectedWorkspace.context}
          loading={selectedWorkspace.loading}
          error={selectedWorkspace.error}
          onRefresh={() => void selectedWorkspace.refresh()}
        />
      </TabsContent>
      <TabsContent value="activity" className="min-h-0 flex-1 overflow-hidden">
        <ActivityFeed selectedWorkspaceContext={selectedWorkspace.context} />
      </TabsContent>
      <TabsContent value="console" className="min-h-0 flex-1 overflow-hidden">
        <ConsolePanel selectedWorkspaceContext={selectedWorkspace.context} />
      </TabsContent>
      <TabsContent value="network" className="min-h-0 flex-1 overflow-hidden">
        <NetworkPanel selectedWorkspaceContext={selectedWorkspace.context} />
      </TabsContent>
      <TabsContent value="storage" className="min-h-0 flex-1 overflow-hidden">
        <StoragePanel selectedWorkspaceContext={selectedWorkspace.context} />
      </TabsContent>
      <TabsContent value="extensions" className="min-h-0 flex-1 overflow-hidden">
        <ExtensionsPanel selectedWorkspaceContext={selectedWorkspace.context} />
      </TabsContent>
      <TabsContent value="chat" className="min-h-0 flex-1 overflow-hidden">
        <ChatPanel selectedWorkspaceContext={selectedWorkspace.context} authenticatedUser={user} />
      </TabsContent>
    </Tabs>
  );
  const runtimeNotice = runtimeManifest.issue || runtimeHealth.issue || runtimeHealth.health ? (
    <>
      <RuntimeManifestNotice state={runtimeManifest} />
      <RuntimeHealthNotice state={runtimeHealth} />
      <RuntimeHealthSummary state={runtimeHealth} />
    </>
  ) : null;
  const appShellProps = {
    activeSection,
    onSectionChange: changeDashboardSection,
    onNewSessionRequest: openNewSession,
    authenticatedUser: user.displayName || user.username,
    onLogout,
    runtimeEnvironment: runtimeManifest.manifest?.runtimeEnvironment ?? "production",
    runtimeNotice,
  };

  if (isDesktop) {
    if (!hasSessions && activeSection !== "service" && !hasWorkspaceViewportRoute) {
      return (
        <AppShell {...appShellProps}>
          <ResizablePanelGroup
            orientation="horizontal"
            className="dashboard-panel-grid"
          >
            {!leftPaneCollapsed && (
              <>
                <ResizablePanel id="sessions" defaultSize="20%" minSize="14%" maxSize="34%">
                  <div className="dashboard-pane dashboard-pane-left dashboard-pane-with-toggle">
                    {leftPaneToggle}
                    <WorkspaceNavigator />
                  </div>
                </ResizablePanel>
                <ResizableHandle />
              </>
            )}
            <ResizablePanel id="empty" defaultSize="85%">
              <div className="dashboard-empty-state dashboard-pane-with-rails">
                {leftPaneCollapsed && leftPaneToggle}
                <div className="dashboard-empty-card">
                  <div className="dashboard-empty-orb">
                    <Plus className="size-6" />
                  </div>
                  <div className="space-y-2">
                    <p className="text-xl font-black tracking-[-0.04em] text-foreground">
                      No active sessions
                    </p>
                    <p className="max-w-sm text-sm leading-6 text-muted-foreground">
                      Start a managed browser workspace to inspect pages, stream a headed session, and prepare the service control plane.
                    </p>
                  </div>
                  <Button
                    size="lg"
                    className="dashboard-primary-action"
                    onClick={openNewSession}
                  >
                    <Plus className="size-4" />
                    New session
                  </Button>
                </div>
              </div>
            </ResizablePanel>
          </ResizablePanelGroup>
        </AppShell>
      );
    }

    if (!hasSessions && activeSection === "service") {
      return (
        <AppShell {...appShellProps}>
          <ResizablePanelGroup
            orientation="horizontal"
            className="dashboard-panel-grid"
          >
            {!leftPaneCollapsed && (
              <>
                <ResizablePanel id="sessions" defaultSize="20%" minSize="14%" maxSize="34%">
                  <div className="dashboard-pane dashboard-pane-left dashboard-pane-with-toggle">
                    {leftPaneToggle}
                    <WorkspaceNavigator />
                  </div>
                </ResizablePanel>
                <ResizableHandle />
              </>
            )}
            <ResizablePanel id="service" defaultSize={rightPaneCollapsed ? "85%" : "55%"} minSize="30%">
              <div className="dashboard-pane dashboard-pane-viewport dashboard-pane-with-rails">
                {leftPaneCollapsed && leftPaneToggle}
                {rightPaneCollapsed && rightPaneToggle}
                <ServicePanel
                  onInspectSelection={inspectServiceSelection}
                  onInspectorActionsChange={setServiceInspectorActions}
                />
              </div>
            </ResizablePanel>
            {!rightPaneCollapsed && (
              <>
                <ResizableHandle />
                <ResizablePanel id="service-inspector" defaultSize="30%" minSize="18%" maxSize="50%">
                  <div className="dashboard-pane dashboard-pane-right dashboard-pane-with-toggle">
                    {rightPaneToggle}
                    {serviceInspectorPanel}
                  </div>
                </ResizablePanel>
              </>
            )}
          </ResizablePanelGroup>
        </AppShell>
      );
    }

    return (
      <AppShell {...appShellProps}>
        <ResizablePanelGroup
          orientation="horizontal"
          className="dashboard-panel-grid"
        >
          {!leftPaneCollapsed && (
            <>
              <ResizablePanel id="sessions" defaultSize="20%" minSize="14%" maxSize="34%">
                <div className="dashboard-pane dashboard-pane-left dashboard-pane-with-toggle">
                  {leftPaneToggle}
                  <WorkspaceNavigator />
                </div>
              </ResizablePanel>
              <ResizableHandle />
            </>
          )}
          <ResizablePanel id="viewport" defaultSize={rightPaneCollapsed ? "85%" : "55%"} minSize="30%">
            <div className="dashboard-pane dashboard-pane-viewport dashboard-pane-with-rails">
              {leftPaneCollapsed && leftPaneToggle}
              {rightPaneCollapsed && rightPaneToggle}
              {primaryPanel}
            </div>
          </ResizablePanel>
          {!rightPaneCollapsed && (
            <>
              <ResizableHandle />
              <ResizablePanel id="activity" defaultSize="30%" minSize="15%" maxSize="50%">
                <div className="dashboard-pane dashboard-pane-right dashboard-pane-with-toggle">
                  {rightPaneToggle}
                  {activeSection === "service" ? serviceInspectorPanel : sidePanel}
                </div>
              </ResizablePanel>
            </>
          )}
        </ResizablePanelGroup>
      </AppShell>
    );
  }

  return (
    <AppShell {...appShellProps}>
      <Tabs
        value={mobilePanel}
        onValueChange={(value) => {
          if (value === "workspaces" || value === "viewport") {
            setMobilePanel(value);
            changeDashboardSection("overview");
          } else if (value === "service" || value === "activity") {
            setMobilePanel(value);
            changeDashboardSection(value);
          }
        }}
        className="dashboard-mobile-tabs min-h-0 flex-1"
      >
        <div className="dashboard-mobile-tabs-list shrink-0 px-3 pt-3">
          <TabsList className="w-full rounded-2xl bg-white/60 p-1 shadow-sm ring-1 ring-foreground/10 backdrop-blur-xl dark:bg-white/5">
            <TabsTrigger value="workspaces">Workspaces</TabsTrigger>
            <TabsTrigger value="viewport">Viewport</TabsTrigger>
            <TabsTrigger value="activity">Activity</TabsTrigger>
            <TabsTrigger value="service">Service</TabsTrigger>
          </TabsList>
        </div>
        <TabsContent value="workspaces" className="dashboard-mobile-panel min-h-0 overflow-hidden p-3">
          <WorkspaceNavigator />
        </TabsContent>
        <TabsContent value="viewport" className="dashboard-mobile-panel min-h-0 overflow-hidden p-3">
          <WorkspaceRemoteViewport
            fallback={<Viewport />}
            selectedWorkspaceContext={selectedWorkspace.context}
            projection={selectedWorkspace.projection}
            onRefresh={selectedWorkspace.refresh}
            onSelectStream={workspaceViewPreferences.write}
          />
        </TabsContent>
        <TabsContent value="activity" className="dashboard-mobile-panel min-h-0 overflow-hidden p-3">
          {sidePanel}
        </TabsContent>
        <TabsContent value="service" className="dashboard-mobile-panel min-h-0 overflow-hidden p-3">
          <div className="dashboard-pane">
            <ServicePanel />
          </div>
        </TabsContent>
      </Tabs>
    </AppShell>
  );
}
