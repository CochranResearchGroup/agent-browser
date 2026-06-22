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
} from "@/lib/workspace-url-selection";
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

type RuntimeManifest = {
  schemaVersion?: string;
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

const REQUIRED_RUNTIME_FEATURES = [
  "workspace.detectedBrowsers",
  "workspace.noRetainedLiveRail",
] as const;

const REQUIRED_RUNTIME_CONTRACT = "service-ui-runtime.v1";

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
        const response = await fetch("/api/dashboard-auth/status", {
          cache: "no-store",
          credentials: "same-origin",
        });
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
    <DashboardExperience
      initialSection={initialSection}
      user={user}
      onLogout={handleLogout}
    />
  );
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
  const activePort = useAtomValue(activePortAtom);
  useStreamSync(activePort);
  useSessionsSync();
  useActivitySync();
  useChatStatusSync();

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
  const selectedWorkspace = useSelectedWorkspaceContext(selectedWorkspaceContextEnabled);
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
      : <WorkspaceRemoteViewport fallback={<Viewport />} selectedWorkspaceContext={selectedWorkspace.context} />;
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
  const runtimeNotice = runtimeManifest.issue ? (
    <RuntimeManifestNotice state={runtimeManifest} />
  ) : null;
  const appShellProps = {
    activeSection,
    onSectionChange: changeDashboardSection,
    onNewSessionRequest: openNewSession,
    authenticatedUser: user.displayName || user.username,
    onLogout,
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
          <WorkspaceRemoteViewport fallback={<Viewport />} selectedWorkspaceContext={selectedWorkspace.context} />
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
