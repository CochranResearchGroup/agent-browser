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
import {
  projectWorkspaceViews,
  applyStatusObservationsToWorkspaceSources,
  type WorkspaceStatusProjection,
  type WorkspaceViewAuthorityLedger,
  type WorkspaceViewBrowserSource,
  type WorkspaceViewProjection,
} from "@/lib/workspace-view-projection";
import type { ServiceViewStream } from "@/lib/service-view-streams";
import type { WorkspaceViewPreferenceSnapshot } from "@/hooks/use-workspace-view-preferences";
import type {
  TabInfo,
} from "@/types";
import type {
  WorkspaceServiceBrowser,
  WorkspaceManualBrowser,
  WorkspaceBrowserSessionAuthority,
  WorkspaceServiceIncident,
  WorkspaceServiceJob,
  WorkspaceServiceProfileAllocation,
  WorkspaceResourceRecord,
  WorkspaceServiceSession,
  WorkspaceServiceTab,
  WorkspaceNode,
} from "@/lib/service-workspaces";
import { deriveWorkspaceNodes, deriveWorkspaceViewAuthorityLedger } from "@/lib/service-workspaces";

type ServiceStatusData = {
  service_state?: {
    browsers?: Record<string, WorkspaceServiceBrowser>;
    sessions?: Record<string, WorkspaceServiceSession>;
    tabs?: Record<string, WorkspaceServiceTab>;
    jobs?: Record<string, WorkspaceServiceJob>;
    incidents?: WorkspaceServiceIncident[];
    remoteViewRoutes?: Record<string, ServiceViewStream>;
  };
  profileAllocations?: WorkspaceServiceProfileAllocation[];
  manualBrowsers?: WorkspaceManualBrowser[];
  browserSessionAuthority?: WorkspaceBrowserSessionAuthority | null;
  statusProjection?: WorkspaceStatusProjection | null;
};

type ServiceResourcesData = {
  resources?: WorkspaceResourceRecord[];
};

type ApiResponse<T> = {
  success: boolean;
  data?: T;
  error?: string | null;
};

export type UseSelectedWorkspaceContextResult = {
  context: SelectedWorkspaceContext;
  projection: WorkspaceViewProjection;
  sourceSnapshotIdentity: number;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
};

export function useSelectedWorkspaceContext(
  enabled = true,
  preferences?: WorkspaceViewPreferenceSnapshot,
): UseSelectedWorkspaceContextResult {
  const daemonSessions = useAtomValue(sessionsAtom);
  const getTabsForPort = useAtomValue(tabsForPortAtom);
  const getEngineForPort = useAtomValue(engineForPortAtom);
  const [selection, setSelection] = useState(() => readDashboardWorkspaceUrlSelection());
  const [serviceStatus, setServiceStatus] = useState<ServiceStatusData | null>(null);
  const [serviceResources, setServiceResources] = useState<ServiceResourcesData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [refreshedAt, setRefreshedAt] = useState(() => Date.now());

  const refresh = useCallback(async () => {
    if (!enabled) return;
    setLoading(true);
    try {
      const [statusResponse, resourcesResponse] = await Promise.all([
        fetch(`${SERVICE_API_BASE}/status`, { cache: "no-store" }),
        fetch(`${SERVICE_API_BASE}/resources`, { cache: "no-store" }).catch(() => null),
      ]);
      if (!statusResponse.ok) throw new Error(`HTTP ${statusResponse.status}`);
      const json = (await statusResponse.json()) as ApiResponse<ServiceStatusData>;
      const resourcesJson = resourcesResponse?.ok
        ? ((await resourcesResponse.json()) as ApiResponse<ServiceResourcesData>)
        : null;
      if (!json.success) throw new Error(json.error || "Service status unavailable");
      setServiceStatus(json.data ?? null);
      setServiceResources(resourcesJson?.success ? resourcesJson.data ?? null : null);
      setError(null);
      setRefreshedAt(Date.now());
    } catch (err) {
      setError(err instanceof Error ? err.message : "Service status unavailable");
      setServiceResources(null);
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

  const { context, projection } = useMemo(() => {
    const serviceBrowsers = Object.values(serviceStatus?.service_state?.browsers ?? {});
    const serviceTabs = Object.values(serviceStatus?.service_state?.tabs ?? {});
    const nodeInput = {
      daemonSessions,
      daemonTabsByPort,
      daemonEngineByPort,
      serviceBrowsers,
      serviceSessions: Object.values(serviceStatus?.service_state?.sessions ?? {}),
      serviceTabs,
      profileAllocations: serviceStatus?.profileAllocations ?? [],
      manualBrowsers: serviceStatus?.manualBrowsers ?? [],
      jobs: Object.values(serviceStatus?.service_state?.jobs ?? {}),
      incidents: serviceStatus?.service_state?.incidents ?? [],
      resources: serviceResources?.resources ?? [],
      browserSessionAuthority: serviceStatus?.browserSessionAuthority ?? null,
      remoteViewRoutes: serviceStatus?.service_state?.remoteViewRoutes ?? {},
    };
    const canonicalAuthority = deriveWorkspaceViewAuthorityLedger(nodeInput);
    const nodes = deriveWorkspaceNodes({ ...nodeInput, includeRetained: true, includeHidden: true });
    const baseContext = buildSelectedWorkspaceContext({
      ...nodeInput,
      selection,
      nodes,
      refreshedAt,
    });
    const authorityLedger = workspaceAuthorityLedger(nodes, canonicalAuthority);
    const selectedSubjectKey = baseContext.node?.id ?? "";
    const projection = projectWorkspaceViews({
      sources: applyStatusObservationsToWorkspaceSources({
        serviceBrowsers: serviceBrowsers.map(workspaceProjectionBrowser),
        serviceTabs,
        remoteViewRoutes: serviceStatus?.service_state?.remoteViewRoutes ?? {},
        daemonSessions,
        selectedContext: {
          node: baseContext.node,
          stream: baseContext.stream ? {
            ...baseContext.stream,
            provider: baseContext.stream.provider ?? undefined,
          } : null,
        },
      }, serviceStatus?.statusProjection),
      authorityLedger,
      intent: {
        selection,
        mode: "view",
        dashboardHref: typeof window === "undefined" ? null : window.location.href,
        preferences: {
          selected: selectedSubjectKey
            ? {
                subjectKey: selectedSubjectKey,
                provider: preferences?.selectedProvider ?? null,
                streamKey: baseContext.browser?.id
                  ? preferences?.byBrowserId?.[baseContext.browser.id]?.streamKey ?? null
                  : null,
              }
            : null,
          byBrowserId: preferences?.byBrowserId,
        },
      },
    });
    return {
      projection,
      context: {
        ...baseContext,
        projectedView: projection.selected,
        projectionSnapshotIdentity: refreshedAt,
        stream: projection.selected?.stream
          ? projectedNodeStream(projection.selected.stream, projection.selected.canView, projection.selected.canControl)
          : baseContext.stream,
        viewable: projection.selected?.canView ?? baseContext.viewable,
        controllable: projection.selected?.canControl ?? baseContext.controllable,
      },
    };
  }, [
    daemonEngineByPort,
    daemonSessions,
    daemonTabsByPort,
    refreshedAt,
    selection,
    serviceResources,
    serviceStatus,
    preferences?.revision,
  ]);

  return { context, projection, sourceSnapshotIdentity: refreshedAt, loading, error, refresh };
}

function workspaceProjectionBrowser(browser: WorkspaceServiceBrowser): WorkspaceViewBrowserSource {
  return {
    ...browser,
    viewStreams: (browser.viewStreams ?? []).map((stream) => ({
      ...stream,
      id: stream.id ?? undefined,
      provider: stream.provider ?? undefined,
      readOnly: stream.readOnly ?? undefined,
    })),
  };
}

function workspaceAuthorityLedger(
  nodes: WorkspaceNode[],
  canonical: WorkspaceViewAuthorityLedger,
): WorkspaceViewAuthorityLedger {
  return {
    ...Object.fromEntries(nodes.filter((node) => !canonical[node.id]).map((node) => {
    const action = (id: string) => node.actions.find((candidate) => candidate.id === id);
    const ceiling = (id: string, fallback = false) => {
      const candidate = action(id);
      return { allowed: candidate?.enabled ?? fallback, reason: candidate?.reason ?? null };
    };
    return [node.id, {
      subjectKey: node.id,
      authoritySource: node.source === "service-browser" ? "service-status-compatibility" as const : "daemon-detection" as const,
      browserId: node.browserId ?? null,
      workspaceId: node.id,
      inventoryClass: node.inventoryClass,
      inventoryPlacement: node.inventoryPlacement,
      lifecycle: {
        state: node.state,
        live: node.live,
        retained: node.retained,
        health: node.health ?? null,
      },
      routeBoundOwnership: node.routeBoundOwnership,
      operatorVisibleProof: node.viewStream ? {
        state: node.viewStream.operatorVisibleState,
        reason: node.viewStream.operatorVisibleReason ?? null,
        routeId: node.viewStream.routeId ?? null,
        displayAllocationId: node.viewStream.displayAllocationId ?? null,
      } : null,
      lifecycleActions: node.actions.filter((candidate) => ["close", "kill", "add-tab"].includes(candidate.id)),
      presentationActionCeilings: {
        view: ceiling("view", Boolean(node.viewStream?.embeddable)),
        control: ceiling("control", false),
        stream: ceiling("stream", Boolean(node.viewStream?.embeddable)),
        screenshot: ceiling("screenshot", false),
      },
      diagnostics: node.diagnostics,
    }];
    })),
    ...canonical,
  };
}

function projectedNodeStream(stream: ServiceViewStream, embeddable: boolean, controllable: boolean) {
  return {
    provider: stream.provider ?? null,
    url: stream.url ?? stream.frameUrl ?? stream.externalUrl ?? null,
    routeId: stream.routeId ?? null,
    displayAllocationId: stream.displayAllocationId ?? null,
    routePoolEntryId: null,
    connectionId: stream.connectionId ?? null,
    connectionName: stream.connectionName ?? null,
    routeSource: stream.routeSource ?? null,
    providerMode: stream.providerMode ?? null,
    viewerLeaseIds: stream.viewerLeaseIds ?? [],
    controllerLeaseId: stream.controllerLeaseId ?? null,
    embeddable,
    controllable,
    readOnly: stream.readOnly === true || !stream.controlInput,
    controlInput: stream.controlInput ?? null,
    operatorVisibleState: embeddable ? "ready" : "unavailable",
    operatorVisibleReason: null,
    routeSummary: stream.routeId ?? stream.connectionName ?? stream.connectionId ?? "unrouted",
  };
}
