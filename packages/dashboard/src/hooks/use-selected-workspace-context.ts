"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { useAtomValue } from "jotai/react";
import { sessionsAtom } from "@/store/sessions";
import { engineForPortAtom, tabsForPortAtom } from "@/store/tabs";
import { SERVICE_API_BASE } from "@/lib/dashboard-api";
import {
  DASHBOARD_WORKSPACE_SELECTION_EVENT,
  readDashboardWorkspaceUrlSelection,
} from "@/lib/workspace-url-selection";
import {
  buildSelectedWorkspaceContext,
  type SelectedWorkspaceContext,
} from "@/lib/selected-workspace-context";
import type {
  TabInfo,
} from "@/types";
import type {
  WorkspaceServiceBrowser,
  WorkspaceServiceIncident,
  WorkspaceServiceJob,
  WorkspaceServiceProfileAllocation,
  WorkspaceServiceSession,
  WorkspaceServiceTab,
} from "@/lib/service-workspaces";

type ServiceStatusData = {
  service_state?: {
    browsers?: Record<string, WorkspaceServiceBrowser>;
    sessions?: Record<string, WorkspaceServiceSession>;
    tabs?: Record<string, WorkspaceServiceTab>;
    jobs?: Record<string, WorkspaceServiceJob>;
    incidents?: WorkspaceServiceIncident[];
  };
  profileAllocations?: WorkspaceServiceProfileAllocation[];
};

type ApiResponse<T> = {
  success: boolean;
  data?: T;
  error?: string | null;
};

export type UseSelectedWorkspaceContextResult = {
  context: SelectedWorkspaceContext;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
};

export function useSelectedWorkspaceContext(enabled = true): UseSelectedWorkspaceContextResult {
  const daemonSessions = useAtomValue(sessionsAtom);
  const getTabsForPort = useAtomValue(tabsForPortAtom);
  const getEngineForPort = useAtomValue(engineForPortAtom);
  const [selection, setSelection] = useState(() => readDashboardWorkspaceUrlSelection());
  const [serviceStatus, setServiceStatus] = useState<ServiceStatusData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [refreshedAt, setRefreshedAt] = useState(() => Date.now());

  const refresh = useCallback(async () => {
    if (!enabled) return;
    setLoading(true);
    try {
      const response = await fetch(`${SERVICE_API_BASE}/status`, { cache: "no-store" });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const json = (await response.json()) as ApiResponse<ServiceStatusData>;
      if (!json.success) throw new Error(json.error || "Service status unavailable");
      setServiceStatus(json.data ?? null);
      setError(null);
      setRefreshedAt(Date.now());
    } catch (err) {
      setError(err instanceof Error ? err.message : "Service status unavailable");
      setRefreshedAt(Date.now());
    } finally {
      setLoading(false);
    }
  }, [enabled]);

  useEffect(() => {
    if (!enabled) return;
    const updateSelection = () => setSelection(readDashboardWorkspaceUrlSelection());
    updateSelection();
    window.addEventListener("popstate", updateSelection);
    window.addEventListener(DASHBOARD_WORKSPACE_SELECTION_EVENT, updateSelection);
    return () => {
      window.removeEventListener("popstate", updateSelection);
      window.removeEventListener(DASHBOARD_WORKSPACE_SELECTION_EVENT, updateSelection);
    };
  }, [enabled]);

  useEffect(() => {
    if (!enabled) return;
    void refresh();
    const interval = window.setInterval(() => {
      void refresh();
    }, 7000);
    return () => window.clearInterval(interval);
  }, [enabled, refresh]);

  const daemonTabsByPort = useMemo(() => {
    const tabsByPort: Record<number, TabInfo[]> = {};
    for (const session of daemonSessions) {
      if (session.port > 0) tabsByPort[session.port] = getTabsForPort(session.port);
    }
    return tabsByPort;
  }, [daemonSessions, getTabsForPort]);

  const daemonEngineByPort = useMemo(() => {
    const engineByPort: Record<number, string> = {};
    for (const session of daemonSessions) {
      if (session.port > 0) engineByPort[session.port] = getEngineForPort(session.port);
    }
    return engineByPort;
  }, [daemonSessions, getEngineForPort]);

  const context = useMemo(() => buildSelectedWorkspaceContext({
    selection,
    daemonSessions,
    daemonTabsByPort,
    daemonEngineByPort,
    serviceBrowsers: Object.values(serviceStatus?.service_state?.browsers ?? {}),
    serviceSessions: Object.values(serviceStatus?.service_state?.sessions ?? {}),
    serviceTabs: Object.values(serviceStatus?.service_state?.tabs ?? {}),
    profileAllocations: serviceStatus?.profileAllocations ?? [],
    jobs: Object.values(serviceStatus?.service_state?.jobs ?? {}),
    incidents: serviceStatus?.service_state?.incidents ?? [],
    refreshedAt,
  }), [
    daemonEngineByPort,
    daemonSessions,
    daemonTabsByPort,
    refreshedAt,
    selection,
    serviceStatus,
  ]);

  return { context, loading, error, refresh };
}
