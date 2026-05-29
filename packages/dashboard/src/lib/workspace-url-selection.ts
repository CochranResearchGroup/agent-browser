export const DASHBOARD_WORKSPACE_SELECTION_EVENT = "agent-browser-dashboard-workspace-selection-change";

export const DASHBOARD_WORKSPACE_QUERY_KEYS = [
  "workspace",
  "browser",
  "session",
  "tab",
  "profile",
  "job",
] as const;

export type DashboardWorkspaceQueryKey = (typeof DASHBOARD_WORKSPACE_QUERY_KEYS)[number];

export type DashboardWorkspaceUrlSelection = {
  workspaceId: string | null;
  browserId: string | null;
  sessionId: string | null;
  tabId: string | null;
  profileId: string | null;
  jobId: string | null;
};

export type DashboardWorkspaceUrlSelectionUpdate = Partial<DashboardWorkspaceUrlSelection>;

function emptyWorkspaceUrlSelection(): DashboardWorkspaceUrlSelection {
  return {
    workspaceId: null,
    browserId: null,
    sessionId: null,
    tabId: null,
    profileId: null,
    jobId: null,
  };
}

export function readDashboardWorkspaceUrlSelection(search?: string): DashboardWorkspaceUrlSelection {
  if (typeof window === "undefined" && search === undefined) return emptyWorkspaceUrlSelection();
  const params = new URLSearchParams(search ?? window.location.search);
  return {
    workspaceId: params.get("workspace"),
    browserId: params.get("browser"),
    sessionId: params.get("session"),
    tabId: params.get("tab"),
    profileId: params.get("profile"),
    jobId: params.get("job"),
  };
}

export function dashboardWorkspaceSelectionHasValue(selection: DashboardWorkspaceUrlSelection): boolean {
  return Boolean(
    selection.workspaceId ||
    selection.browserId ||
    selection.sessionId ||
    selection.tabId ||
    selection.profileId ||
    selection.jobId,
  );
}

function workspaceSelectionQueryValues(
  selection: DashboardWorkspaceUrlSelection,
): Record<DashboardWorkspaceQueryKey, string | null> {
  return {
    workspace: selection.workspaceId,
    browser: selection.browserId,
    session: selection.sessionId,
    tab: selection.tabId,
    profile: selection.profileId,
    job: selection.jobId,
  };
}

function dispatchDashboardWorkspaceSelectionChange(selection: DashboardWorkspaceUrlSelection): void {
  if (typeof window === "undefined") return;
  window.dispatchEvent(new CustomEvent(DASHBOARD_WORKSPACE_SELECTION_EVENT, { detail: selection }));
}

export function writeDashboardWorkspaceUrlSelection(
  selection: DashboardWorkspaceUrlSelection,
  mode: "push" | "replace" = "push",
): DashboardWorkspaceUrlSelection {
  if (typeof window === "undefined") return selection;

  const params = new URLSearchParams(window.location.search);
  const values = workspaceSelectionQueryValues(selection);
  for (const key of DASHBOARD_WORKSPACE_QUERY_KEYS) {
    const value = values[key];
    if (value) {
      params.set(key, value);
    } else {
      params.delete(key);
    }
  }

  const search = params.toString();
  const nextUrl = `${window.location.pathname}${search ? `?${search}` : ""}${window.location.hash}`;
  const currentUrl = `${window.location.pathname}${window.location.search}${window.location.hash}`;
  if (nextUrl !== currentUrl) {
    const state = {
      ...(window.history.state && typeof window.history.state === "object" ? window.history.state : {}),
      dashboardWorkspace: selection.workspaceId,
    };
    if (mode === "replace") {
      window.history.replaceState(state, "", nextUrl);
    } else {
      window.history.pushState(state, "", nextUrl);
    }
  }

  dispatchDashboardWorkspaceSelectionChange(selection);
  return selection;
}

export function updateDashboardWorkspaceUrlSelection(
  update: DashboardWorkspaceUrlSelectionUpdate,
  mode: "push" | "replace" = "push",
): DashboardWorkspaceUrlSelection {
  return writeDashboardWorkspaceUrlSelection(
    {
      ...readDashboardWorkspaceUrlSelection(),
      ...update,
    },
    mode,
  );
}
