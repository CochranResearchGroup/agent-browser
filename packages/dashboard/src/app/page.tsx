"use client";

import { managedWorkspaceViewport } from "@/lib/managed-workspace";
import { useEffect, useState } from "react";
import { useAtomValue } from "jotai/react";
import { activePortAtom } from "@/store/sessions";
import { useSessionsSync } from "@/store/sessions";
import { useStreamSync, hasConsoleErrorsAtom } from "@/store/stream";
import { useActivitySync } from "@/store/activity";
import { activeExtensionsAtom } from "@/store/sessions";
import { useChatStatusSync } from "@/store/chat";
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
import { NativeTaskViewport } from "@/components/native-task-viewport";
import { NativeWindowViewport } from "@/components/native-window-viewport";
import { useNativeBrowserWindows } from "@/hooks/use-native-browser-windows";
import { CompactNativeWorkspace } from "@/components/compact-native-workspace";
import type { DashboardSection } from "@/components/app-shell";
import {
  DASHBOARD_WORKSPACE_SELECTION_EVENT,
  dashboardWorkspaceSelectionHasValue,
  readDashboardWorkspaceUrlSelection,
} from "@/lib/workspace-url-selection";
import {
  hashOpaqueIdentifier,
  installDashboardFetchFailureInstrumentation,
  reportDashboardFailure,
} from "@/lib/failure-observation";
import { WorkspaceAuthGate } from "@/components/workspace-auth-gate";
import { fetchSharedRuntimeHealth } from "@/lib/dashboard-api";
import { startCompletionDrivenDashboardPoll } from "@/lib/dashboard-read-coordinator";
import { summarizeRuntimeAccess } from "@/lib/runtime-health-summary";
import {
  ServiceDetailInspector,
  ServicePanel,
  type ServiceInspectorActions,
  type ServiceInspectorSelection,
} from "@/components/service-panel";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import {
  AlertTriangle,
  ShieldCheck,
} from "lucide-react";

const SECTION_PATHS: Record<DashboardSection, string> = {
  overview: "/",
  browsers: "/browsers",
  service: "/service",
  activity: "/activity",
};
type DashboardAuthUser = {
  username: string;
  displayName?: string;
  role?: string;
  authenticationSource?: "ingress" | "local";
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

function readDashboardSection(fallback: DashboardSection): DashboardSection {
  if (typeof window === "undefined") return fallback;
  const view = new URLSearchParams(window.location.search).get("view");
  if (view?.startsWith("workspace:")) return "overview";
  if (view?.startsWith("service:")) return "service";
  return dashboardSectionFromPath(window.location.pathname);
}

function dashboardSectionUrl(section: DashboardSection): string {
  if (typeof window === "undefined") return SECTION_PATHS[section];
  const params = new URLSearchParams(window.location.search);
  if (section === "service") {
    if (!params.get("view")?.startsWith("service:")) params.set("view", "service:status");
  } else if (section === "activity") {
    params.delete("view");
  } else {
    params.set("view", "workspace:control");
  }
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
  return <WorkspaceAuthGate>{(user, onLogout) => <RemoteViewHandoffGate initialSection={initialSection} user={user} onLogout={onLogout} />}</WorkspaceAuthGate>;
}

/** Navigate an authenticated durable handoff directly to the provider viewer. */
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

  useEffect(() => {
    if (!handoffId) return;
    // The endpoint restores the retained browser and redirects to its native desktop.
    window.location.replace(`/api/remote-view/${encodeURIComponent(handoffId)}/presentation`);
  }, [handoffId]);

  if (handoffId) return <DashboardSessionRestoreScreen message="Opening remote view" />;
  return <DashboardExperience initialSection={initialSection} user={user} onLogout={onLogout} />;
}

function DashboardSessionRestoreScreen({
  message = "Restoring session",
}: {
  message?: string;
}) {
  return (
    <div className="dashboard-root dashboard-login-root">
      <div className="dashboard-aurora dashboard-aurora-one" />
      <div className="dashboard-aurora dashboard-aurora-two" />
      <main className="dashboard-login-main">
        <section className="dashboard-login-panel" aria-busy="true" aria-live="polite">
          <div className="dashboard-login-mark">
            <ShieldCheck className="size-5" />
          </div>
          <div className="dashboard-login-heading">
            <p className="dashboard-login-title">Agent Browser</p>
            <p className="dashboard-login-subtitle">{message}</p>
          </div>
        </section>
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
  const accessSummary = summarizeRuntimeAccess(access);
  const healthy = multiplicity?.steadyState === true
    && monitor?.ready === true
    && (cleanup?.missingCount ?? 0) === 0
    && accessSummary.ready;
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
          {accessSummary.text ? ` ${accessSummary.text}` : ""}
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

/** One browser workspace is the default; diagnostics retain separate tools. */
function DashboardExperience({
  initialSection = "overview", user, onLogout,
}: {
  initialSection?: DashboardSection;
  user: DashboardAuthUser;
  onLogout: () => void;
}) {
  const [section, setSection] = useState(() => readDashboardSection(initialSection));
  useEffect(() => installDashboardFetchFailureInstrumentation(), []);
  useEffect(() => {
    const update = () => setSection(readDashboardSection(initialSection));
    window.addEventListener("popstate", update);
    window.addEventListener(DASHBOARD_WORKSPACE_SELECTION_EVENT, update);
    return () => {
      window.removeEventListener("popstate", update);
      window.removeEventListener(DASHBOARD_WORKSPACE_SELECTION_EVENT, update);
    };
  }, [initialSection]);
  return section === "service" || section === "activity"
    ? <DashboardDiagnostics section={section} user={user} onLogout={onLogout} />
    : <NativeBrowserWorkspace user={user} onLogout={onLogout} />;
}

function useDashboardSessionSource() {
  const activePort = useAtomValue(activePortAtom);
  // Workspace transports own their selected stream; avoid a second daemon socket.
  useStreamSync(activePort, !readWorkspaceViewportRoute());
  useSessionsSync();
}

function NativeBrowserWorkspace({ user, onLogout }: {
  user: DashboardAuthUser;
  onLogout: () => void;
}) {
  const [runtimeEnvironment, setRuntimeEnvironment] = useState<RuntimeManifest["runtimeEnvironment"]>();
  useEffect(() => {
    const controller = new AbortController();
    void fetch("/api/runtime/manifest", { cache: "no-store", signal: AbortSignal.any([controller.signal, AbortSignal.timeout(5000)]) })
      .then(response => response.ok ? response.json() : null)
      .then((manifest: RuntimeManifest | null) => {
        const environment = manifest?.runtimeEnvironment;
        if (!controller.signal.aborted && (environment === "development" || environment === "production")) setRuntimeEnvironment(environment);
      }).catch(() => { /* Unknown metadata must not claim a production or development identity. */ });
    return () => controller.abort();
  }, []);
  const workspaceViewPreferences = useWorkspaceViewPreferences();
  const selectedWorkspace = useSelectedWorkspaceContext(true, workspaceViewPreferences.snapshot, true);
  const activePort = useAtomValue(activePortAtom);
  const inventory = selectedWorkspace.managedWorkspace;
  const native = useNativeBrowserWindows();
  const managedPids = new Set(Object.values(inventory?.browsers ?? {})
    .filter(browser => selectedWorkspace.presenceCurrent && browser.presence?.state === "live").map(browser => browser.pid));
  const nativeWindows = native.windows.filter(window => window.key === native.selectedKey || window.pid === null || !managedPids.has(window.pid));
  const browserId = selectedWorkspace.context.selection.browserId;
  const tabId = selectedWorkspace.context.selection.tabId?.replace(/^target:/, "");
  const viewport = managedWorkspaceViewport(inventory, browserId, tabId ?? null,
    selectedWorkspace.projection.selected?.browser.id === browserId && selectedWorkspace.projection.selected.canView, selectedWorkspace.presenceCurrent);
  useStreamSync(activePort, Boolean(selectedWorkspace.context.node) && !viewport.useNativeViewport && !readWorkspaceViewportRoute());
  return <CompactNativeWorkspace runtimeEnvironment={runtimeEnvironment} nodes={selectedWorkspace.nodes} tabs={selectedWorkspace.tabs}
    handoffs={inventory?.handoffs ? Object.values(inventory.handoffs) : undefined}
    selection={native.selectedKey ? { browserId: null, tabId: null } : selectedWorkspace.context.selection} userLabel={user.displayName || user.username}
    nativeWindows={nativeWindows} nativeSelection={native.selectedKey} nativeIssue={native.issue} onSelectNativeWindow={native.select}
    onLogout={onLogout} canEndLocalSession={user.authenticationSource !== "ingress"} onRefresh={async () => { await Promise.all([selectedWorkspace.refresh(), native.refresh(true)]); }} loading={selectedWorkspace.loading || native.loading} error={selectedWorkspace.error}
    diagnosticsHref={dashboardSectionUrl("service")}>
    {native.selectedKey ? <NativeWindowViewport target={native.windows.find(window => window.key === native.selectedKey) ?? null} loading={native.loading} /> : viewport.useNativeViewport ? <NativeTaskViewport target={viewport.target} unverifiedTarget={viewport.unverifiedTarget} desktopUnavailable={viewport.desktopUnavailable} presenceUnavailable={viewport.presenceUnavailable} hasSelection={Boolean(tabId)} loading={selectedWorkspace.loading} onRefresh={selectedWorkspace.refresh} /> : <WorkspaceRemoteViewport fallback={<Viewport />} selectedWorkspaceContext={selectedWorkspace.context}
      projection={selectedWorkspace.projection} onRefresh={selectedWorkspace.refresh} onSelectStream={workspaceViewPreferences.write} />}
  </CompactNativeWorkspace>;
}

type DiagnosticTool = "service" | "workspace" | "chat" | "activity" | "console" | "network" | "storage" | "extensions";

/** Existing inspection tools remain optional and mount no competing browser rail or viewport. */
function DashboardDiagnostics({ section, user, onLogout }: {
  section: "service" | "activity";
  user: DashboardAuthUser;
  onLogout: () => void;
}) {
  useDashboardSessionSource();
  useActivitySync();
  useChatStatusSync();
  const [tool, setTool] = useState<DiagnosticTool>(section === "activity" ? "activity" : "service");
  const [serviceInspectorSelection, setServiceInspectorSelection] = useState<ServiceInspectorSelection | null>(null);
  const [serviceInspectorActions, setServiceInspectorActions] = useState<ServiceInspectorActions>({});
  const hasConsoleErrors = useAtomValue(hasConsoleErrorsAtom);
  const activeExtensions = useAtomValue(activeExtensionsAtom);
  const workspaceViewPreferences = useWorkspaceViewPreferences();
  const selectedWorkspace = useSelectedWorkspaceContext(true, workspaceViewPreferences.snapshot);
  const [runtimeManifest, setRuntimeManifest] = useState<RuntimeManifestState>({ loading: true, manifest: null, issue: null });
  const [runtimeHealth, setRuntimeHealth] = useState<RuntimeHealthState>({ loading: true, health: null, issue: null });
  useEffect(() => { setTool(section === "activity" ? "activity" : "service"); }, [section]);
  useEffect(() => {
    const showChat = () => setTool("chat");
    window.addEventListener("agent-browser-dashboard-console-send-to-chat", showChat);
    return () => window.removeEventListener("agent-browser-dashboard-console-send-to-chat", showChat);
  }, []);
  useEffect(() => {
    let cancelled = false;
    const checkRuntimeHealth = async () => {
      try {
        const response = await fetchSharedRuntimeHealth();
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
    const stop = startCompletionDrivenDashboardPoll(checkRuntimeHealth, 10_000);
    return () => {
      cancelled = true;
      stop();
    };
  }, []);
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

  return <main className="dashboard-diagnostics flex h-dvh min-h-0 flex-col bg-background text-foreground">
    <header className="native-workspace-toolbar">
      <h1>Diagnostics</h1>
      <a href={dashboardSectionUrl("overview")}>Browsers</a>
      <span className="native-workspace-user">{user.displayName || user.username}</span>
      {user.authenticationSource !== "ingress" && <Button variant="ghost" size="sm" onClick={onLogout}>End local session</Button>}
    </header>
    <RuntimeManifestNotice state={runtimeManifest} />
    <RuntimeHealthNotice state={runtimeHealth} />
    <RuntimeHealthSummary state={runtimeHealth} />
    <Tabs value={tool} onValueChange={value => setTool(value as DiagnosticTool)} className="flex min-h-0 flex-1 flex-col"
      data-selected-workspace-context={selectedWorkspace.context.node ? "ready" : selectedWorkspace.context.state}
      data-selected-workspace-id={selectedWorkspace.context.node?.id ?? ""}
      data-selected-workspace-state={selectedWorkspace.context.state}
      data-selected-workspace-source={selectedWorkspace.context.source}>
      <div className="shrink-0 overflow-x-auto px-2 pt-1">
        <TabsList variant="line" className="h-8 min-w-max">
          <TabsTrigger value="service">Service</TabsTrigger>
          <TabsTrigger value="workspace">Selection</TabsTrigger>
          <TabsTrigger value="activity">Activity</TabsTrigger>
          <TabsTrigger value="chat">Chat</TabsTrigger>
          <TabsTrigger value="console">Console{hasConsoleErrors && <span className="ml-1 inline-flex size-1.5 rounded-full bg-destructive" />}</TabsTrigger>
          <TabsTrigger value="network">Network</TabsTrigger>
          <TabsTrigger value="storage">Storage</TabsTrigger>
          <TabsTrigger value="extensions">Extensions{activeExtensions.length > 0 && <span className="ml-1 text-[9px] tabular-nums">{activeExtensions.length}</span>}</TabsTrigger>
        </TabsList>
      </div>
      <TabsContent value="service" className="min-h-0 flex-1 overflow-auto">
        <div className="grid min-h-full grid-cols-1 xl:grid-cols-[minmax(0,1fr)_22rem]">
          <ServicePanel onInspectSelection={setServiceInspectorSelection} onInspectorActionsChange={setServiceInspectorActions} />
          <aside className="min-h-0 border-l" aria-label="Service detail inspector"><ServiceDetailInspector selection={serviceInspectorSelection} actions={serviceInspectorActions} /></aside>
        </div>
      </TabsContent>
      <TabsContent value="workspace" className="min-h-0 flex-1 overflow-auto"><WorkspaceSelectionPanel context={selectedWorkspace.context} managedWorkspace={selectedWorkspace.managedWorkspace} presenceCurrent={selectedWorkspace.presenceCurrent} loading={selectedWorkspace.loading} error={selectedWorkspace.error} onRefresh={() => void selectedWorkspace.refresh()} /></TabsContent>
      <TabsContent value="activity" className="min-h-0 flex-1 overflow-hidden"><ActivityFeed selectedWorkspaceContext={selectedWorkspace.context} /></TabsContent>
      <TabsContent value="chat" className="min-h-0 flex-1 overflow-hidden"><ChatPanel selectedWorkspaceContext={selectedWorkspace.context} authenticatedUser={user} /></TabsContent>
      <TabsContent value="console" className="min-h-0 flex-1 overflow-hidden"><ConsolePanel selectedWorkspaceContext={selectedWorkspace.context} /></TabsContent>
      <TabsContent value="network" className="min-h-0 flex-1 overflow-hidden"><NetworkPanel selectedWorkspaceContext={selectedWorkspace.context} /></TabsContent>
      <TabsContent value="storage" className="min-h-0 flex-1 overflow-hidden"><StoragePanel selectedWorkspaceContext={selectedWorkspace.context} /></TabsContent>
      <TabsContent value="extensions" className="min-h-0 flex-1 overflow-hidden"><ExtensionsPanel selectedWorkspaceContext={selectedWorkspace.context} /></TabsContent>
    </Tabs>
  </main>;
}
