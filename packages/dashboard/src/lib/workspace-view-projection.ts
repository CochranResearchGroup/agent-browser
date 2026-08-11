import {
  canEmbedViewStream,
  canOpenControlViewStream,
  canOpenViewStream,
  viewStreamDashboardFrameUrl,
  viewStreamExternalUrl,
  viewStreamOpenTitle,
  viewStreamReadinessEvidence,
  viewStreamReadinessLabel,
  viewStreamRouteSummary,
  type ServiceViewStream,
} from "./service-view-streams.ts";
import type { DashboardWorkspaceUrlSelection } from "./workspace-url-selection.ts";

export type WorkspaceViewAuthoritySource =
  | "canonical-inventory"
  | "service-status-compatibility"
  | "daemon-detection";

export type WorkspaceViewAuthorityEntry = {
  subjectKey: string;
  authoritySource: WorkspaceViewAuthoritySource;
  browserId: string | null;
  workspaceId: string | null;
  inventoryClass: string;
  inventoryPlacement?: unknown;
  lifecycle: {
    state: string;
    live: boolean;
    retained: boolean;
    health: string | null;
    reason?: string | null;
  };
  routeBoundOwnership?: unknown;
  operatorVisibleProof?: {
    state: string;
    reason: string | null;
    routeId: string | null;
    displayAllocationId: string | null;
  } | null;
  lifecycleActions?: readonly unknown[];
  presentationActionCeilings: {
    view: { allowed: boolean; reason: string | null };
    control: { allowed: boolean; reason: string | null };
    stream: { allowed: boolean; reason: string | null };
    screenshot: { allowed: boolean; reason: string | null };
  };
  diagnostics?: readonly unknown[];
};

export type WorkspaceViewAuthorityLedger = Readonly<Record<string, WorkspaceViewAuthorityEntry>>;

export type WorkspaceViewBrowserSource = {
  id: string;
  authoritySubjectKey?: string;
  displayName?: string | null;
  profileId?: string | null;
  host?: string | null;
  health?: string | null;
  browserBuild?: string | null;
  cdpEndpoint?: string | null;
  displayAllocationId?: string | null;
  viewStreams?: ServiceViewStream[];
  /** Canonical streams retained when a lower-authority daemon source is merged. */
  canonicalViewStreams?: readonly ServiceViewStream[];
  attachability?: unknown;
  activeSessionIds?: string[];
};

export type WorkspaceViewTabSource = {
  id: string;
  browserId?: string | null;
  targetId?: string | null;
  title?: string | null;
  url?: string | null;
  lifecycle?: string | null;
};

export type WorkspaceViewDaemonSource = {
  session: string;
  port: number;
  engine?: string;
  provider?: string;
  ownership?: string;
  detected?: boolean;
  pending?: boolean;
  closing?: boolean;
  cdpPort?: number;
};

export type WorkspaceViewSelectedContextSource = {
  node?: {
    id: string;
    browserId?: string | null;
    daemonSession?: string | null;
    serviceSessionId?: string | null;
    profileId?: string | null;
    label?: string | null;
    host?: string | null;
    health?: string | null;
    browserBuild?: string | null;
  } | null;
  stream?: (ServiceViewStream & {
    embeddable?: boolean;
    controllable?: boolean;
    operatorVisibleState?: string | null;
    operatorVisibleReason?: string | null;
    routeSummary?: string | null;
  }) | null;
};

export type WorkspaceViewPreferenceScope = {
  selected?: {
    subjectKey: string;
    provider?: string | null;
    streamKey?: string | null;
  } | null;
  byBrowserId?: Readonly<Record<string, { streamKey?: string | null }>>;
};

export type WorkspaceViewSources = {
  serviceBrowsers?: WorkspaceViewBrowserSource[];
  serviceTabs?: WorkspaceViewTabSource[];
  remoteViewRoutes?: Record<string, ServiceViewStream>;
  daemonSessions?: WorkspaceViewDaemonSource[];
  selectedContext?: WorkspaceViewSelectedContextSource | null;
};

export type WorkspaceStatusProjection = {
  schemaVersion?: number | null;
  observations?: {
    state?: "complete" | "partial" | "unavailable" | string;
    validUntil?: string | null;
    viewStreams?: Array<{
      browserId?: string | null;
      streamId?: string | null;
      state?: "observed" | "timed_out" | "unsupported" | "unavailable" | "failed" | string;
      validUntil?: string | null;
      routePresentation?: {
        frameUrl?: string | null;
        externalUrl?: string | null;
        source?: string | null;
      } | null;
      displayContent?: unknown;
    }>;
  } | null;
};

/**
 * Applies only current, typed host presentation observations to a P99 source
 * snapshot. It cannot add a browser, change authority, or upgrade view or
 * control action ceilings. Unknown projection versions fail closed.
 */
export function applyStatusObservationsToWorkspaceSources(
  sources: WorkspaceViewSources,
  projection?: WorkspaceStatusProjection | null,
  currentTimeMs: number = Date.now(),
): WorkspaceViewSources {
  if (projection?.schemaVersion !== 1) return sources;
  const observations = projection.observations?.viewStreams ?? [];
  if (!sources.serviceBrowsers?.length || observations.length === 0) return sources;
  const byStream = new Map(
    observations
      .filter((observation) => {
        if (observation.state !== "observed" || !observation.browserId || !observation.streamId) return false;
        const validUntil = Date.parse(observation.validUntil ?? "");
        return Number.isFinite(validUntil) && currentTimeMs <= validUntil;
      })
      .map((observation) => [`${observation.browserId}\u0000${observation.streamId}`, observation]),
  );
  if (byStream.size === 0) return sources;
  return {
    ...sources,
    serviceBrowsers: sources.serviceBrowsers.map((browser) => ({
      ...browser,
      viewStreams: browser.viewStreams?.map((stream) => {
        const observation = byStream.get(`${browser.id}\u0000${stream.id ?? ""}`);
        if (!observation) return stream;
        return {
          ...stream,
          frameUrl: stream.frameUrl ?? observation.routePresentation?.frameUrl ?? null,
          externalUrl: stream.externalUrl ?? observation.routePresentation?.externalUrl ?? null,
          displayContent: stream.displayContent ?? observation.displayContent ?? null,
        };
      }),
    })),
  };
}

export type WorkspaceViewIntent = {
  selection?: DashboardWorkspaceUrlSelection | null;
  mode: "view" | "control" | "tile" | "inspect";
  dashboardHref?: string | null;
  preferences?: WorkspaceViewPreferenceScope;
  tileLimit?: number;
};

export type ProjectedWorkspaceTab = {
  tab: WorkspaceViewTabSource | null;
  tabIndex: number | null;
  recoveredFromStaleSelection: boolean;
  staleSelectionId: string | null;
  selectionEvidence: "none" | "selected-live" | "selected-live-blank" | "selected-missing" | "selected-closed";
};

export type ProjectedWorkspaceView = {
  authoritySubjectKey: string;
  authorityPreservation: "preserved" | "missing";
  authority: WorkspaceViewAuthorityEntry;
  browser: WorkspaceViewBrowserSource;
  streamChoices: readonly ServiceViewStream[];
  streamChoiceKeys: readonly string[];
  stream: ServiceViewStream | null;
  selectionReason: "explicit-provider" | "persisted-key" | "automatic" | "unavailable";
  tabSelection: ProjectedWorkspaceTab;
  frameUrl: string | null;
  externalUrl: string | null;
  routeKey: string | null;
  routeSummary: string;
  sharedRoute: boolean;
  canEmbed: boolean;
  canView: boolean;
  canControl: boolean;
  readiness: {
    state: string;
    reason: string | null;
    source: "authority" | "stream";
    recoveryAction: "service_remote_view_browser_reattach" | "service_remote_view_route_switch" | null;
  };
};

export type WorkspaceViewProjectionInput = {
  sources: WorkspaceViewSources;
  authorityLedger: WorkspaceViewAuthorityLedger;
  intent: WorkspaceViewIntent;
};

export type WorkspaceViewProjection = {
  selected: ProjectedWorkspaceView | null;
  candidates: readonly ProjectedWorkspaceView[];
  tiles: readonly ProjectedWorkspaceView[];
};

const TERMINAL_HEALTH = new Set([
  "cdp_disconnected",
  "closed",
  "disconnected",
  "faulted",
  "not_started",
  "process_exited",
  "unreachable",
]);

/**
 * Projects all workspace view decisions from one immutable source snapshot.
 * Callers render this result and never rescore streams or reconstruct routes.
 */
export function projectWorkspaceViews(input: WorkspaceViewProjectionInput): WorkspaceViewProjection {
  const selection = input.intent.selection ?? null;
  const serviceBrowsers = (input.sources.serviceBrowsers ?? []).map(normalizeServiceBrowser);
  const daemonBrowsers = (input.sources.daemonSessions ?? [])
    .map(daemonBrowser)
    .filter((browser): browser is WorkspaceViewBrowserSource => Boolean(browser));
  const selectedContextBrowser = browserFromSelectedContext(
    input.sources.selectedContext,
    input.sources.remoteViewRoutes,
  );
  const selectedService = selectServiceBrowser(serviceBrowsers, selection);
  const selectedDaemon = selectDaemonBrowser(daemonBrowsers, selection)
    ?? (selectedService
      ? daemonBrowsers.find((browser) => browsersShareSession(selectedService, browser)) ?? null
      : null);
  const selectedBrowser = mergeSelectedBrowser(
    selectedService,
    selectedContextBrowser ?? selectedDaemon,
  );

  const candidatesById = new Map<string, WorkspaceViewBrowserSource>();
  for (const browser of serviceBrowsers) candidatesById.set(browser.id, browser);
  for (const browser of daemonBrowsers) {
    const matched = serviceBrowsers.find((candidate) => browsersShareSession(candidate, browser));
    if (matched) {
      candidatesById.set(matched.id, mergeSelectedBrowser(matched, browser) ?? matched);
    } else {
      candidatesById.set(browser.id, browser);
    }
  }
  if (selectedContextBrowser && !candidatesById.has(selectedContextBrowser.id)) {
    candidatesById.set(selectedContextBrowser.id, selectedContextBrowser);
  }
  if (selectedBrowser) candidatesById.set(selectedBrowser.id, selectedBrowser);

  const canonicalIds = new Set(serviceBrowsers.map((browser) => browser.id));
  const projected = [...candidatesById.values()].map((browser) => projectBrowser({
    browser,
    canonical: canonicalIds.has(browser.id)
      || serviceBrowsers.some((candidate) => browsersShareSession(candidate, browser)),
    authorityLedger: input.authorityLedger,
    intent: input.intent,
    tabs: input.sources.serviceTabs ?? [],
    selected: Boolean(selectedBrowser && browser.id === selectedBrowser.id),
  }));
  const selected = selectedBrowser
    ? projected.find((candidate) => candidate.browser.id === selectedBrowser.id) ?? null
    : null;
  const tileCandidates = projected
    .map((candidate) => candidate.selectedPreferenceFreeProjection ?? candidate)
    .sort(compareProjectedViews);
  const routeCounts = new Map<string, number>();
  for (const candidate of tileCandidates) {
    if (candidate.routeKey) routeCounts.set(candidate.routeKey, (routeCounts.get(candidate.routeKey) ?? 0) + 1);
  }
  const tiles = tileCandidates.slice(0, input.intent.tileLimit ?? 2).map((candidate) => ({
    ...candidate,
    sharedRoute: Boolean(candidate.routeKey && (routeCounts.get(candidate.routeKey) ?? 0) > 1),
  }));

  return {
    selected: selected ? stripInternalProjection(selected) : null,
    candidates: projected.map(stripInternalProjection),
    tiles: tiles.map(stripInternalProjection),
  };
}

type InternalProjectedWorkspaceView = ProjectedWorkspaceView & {
  selectedPreferenceFreeProjection?: ProjectedWorkspaceView;
};

function projectBrowser({
  browser,
  canonical,
  authorityLedger,
  intent,
  tabs,
  selected,
}: {
  browser: WorkspaceViewBrowserSource;
  canonical: boolean;
  authorityLedger: WorkspaceViewAuthorityLedger;
  intent: WorkspaceViewIntent;
  tabs: WorkspaceViewTabSource[];
  selected: boolean;
}): InternalProjectedWorkspaceView {
  const subjectKey = authoritySubjectKey(browser);
  const authority = authorityLedger[subjectKey] ?? missingAuthority(subjectKey, browser);
  const choices = streamChoices(browser.viewStreams);
  const canonicalChoices = canonical
    ? streamChoices(browser.canonicalViewStreams ?? browser.viewStreams)
    : [];
  const automaticChoices = canonicalChoices.length > 0 ? canonicalChoices : choices;
  const selectedPreference = selected && intent.preferences?.selected?.subjectKey === subjectKey
    ? intent.preferences.selected
    : null;
  const persistedKey = intent.preferences?.byBrowserId?.[browser.id]?.streamKey ?? null;
  const explicit = selectedPreference?.provider
    ? choices.find((stream) => normalize(stream.provider) === normalize(selectedPreference.provider)) ?? null
    : null;
  const keyed = findPreferredStream(choices, selectedPreference?.streamKey ?? persistedKey);
  const stream = explicit ?? keyed ?? automaticChoices[0] ?? null;
  const selectionReason = explicit
    ? "explicit-provider"
    : keyed
      ? "persisted-key"
      : stream
        ? "automatic"
        : "unavailable";
  const frameUrl = stream ? viewStreamDashboardFrameUrl(stream, intent.dashboardHref) : null;
  const externalUrl = stream ? viewStreamExternalUrl(stream) : null;
  const authorityReady = authority.authoritySource !== "daemon-detection" || authority.lifecycle.live;
  const canView = authorityReady && authority.presentationActionCeilings.view.allowed && canOpenViewStream(stream);
  const canControl = authorityReady && authority.presentationActionCeilings.control.allowed && canOpenControlViewStream(stream);
  const tabSelection = selectTab(tabs, browser.id, intent.selection);
  const readiness = projectReadiness({ authority, stream, canView });
  const projection: InternalProjectedWorkspaceView = {
    authoritySubjectKey: subjectKey,
    authorityPreservation: authorityLedger[subjectKey] ? "preserved" : "missing",
    authority,
    browser,
    streamChoices: choices,
    streamChoiceKeys: choices.map((choice, index) => streamKeys(choice, index)[0]),
    stream,
    selectionReason,
    tabSelection,
    frameUrl,
    externalUrl,
    routeKey: stream ? routeKey(stream) : null,
    routeSummary: viewStreamRouteSummary(stream),
    sharedRoute: false,
    canEmbed: Boolean(stream && canEmbedViewStream(stream)),
    canView,
    canControl,
    readiness,
  };
  if (selectedPreference?.provider || selectedPreference?.streamKey) {
    const tileStream = findPreferredStream(choices, persistedKey) ?? automaticChoices[0] ?? null;
    const tileCanView = authorityReady
      && authority.presentationActionCeilings.view.allowed
      && canOpenViewStream(tileStream);
    const tileCanControl = authorityReady
      && authority.presentationActionCeilings.control.allowed
      && canOpenControlViewStream(tileStream);
    projection.selectedPreferenceFreeProjection = {
      ...projection,
      stream: tileStream,
      selectionReason: tileStream && persistedKey && findPreferredStream(choices, persistedKey)
        ? "persisted-key"
        : tileStream
          ? "automatic"
          : "unavailable",
      frameUrl: tileStream ? viewStreamDashboardFrameUrl(tileStream, intent.dashboardHref) : null,
      externalUrl: tileStream ? viewStreamExternalUrl(tileStream) : null,
      routeKey: tileStream ? routeKey(tileStream) : null,
      routeSummary: viewStreamRouteSummary(tileStream),
      canEmbed: Boolean(tileStream && canEmbedViewStream(tileStream)),
      canView: tileCanView,
      canControl: tileCanControl,
      readiness: projectReadiness({ authority, stream: tileStream, canView: tileCanView }),
    };
  }
  return projection;
}

function stripInternalProjection(candidate: InternalProjectedWorkspaceView): ProjectedWorkspaceView {
  const { selectedPreferenceFreeProjection: _selectedPreferenceFreeProjection, ...projection } = candidate;
  return projection;
}

function compareProjectedViews(left: ProjectedWorkspaceView, right: ProjectedWorkspaceView): number {
  const ready = Number(right.canView) - Number(left.canView);
  if (ready !== 0) return ready;
  const score = streamScore(right.stream) - streamScore(left.stream);
  if (score !== 0) return score;
  return left.browser.id.localeCompare(right.browser.id);
}

function streamChoices(streams?: readonly ServiceViewStream[]): ServiceViewStream[] {
  return [...(streams ?? [])].sort((left, right) => streamScore(right) - streamScore(left));
}

function streamScore(stream?: ServiceViewStream | null): number {
  if (!stream) return 0;
  const provider = normalize(stream.provider);
  const routeSource = normalize(stream.routeSource);
  const providerMode = normalize(stream.providerMode);
  const displayAllocationId = normalize(stream.displayAllocationId);
  let score = 0;
  if (canOpenViewStream(stream)) score += 80;
  if (provider === "rdp_gateway") score += 20;
  if (canOpenControlViewStream(stream)) score += 15;
  if (stream.routeId || stream.connectionId || stream.connectionName) score += 20;
  if (displayAllocationId) score += 10;
  if (displayAllocationId && !displayAllocationId.includes("shared")) score += 35;
  if (["pool", "generated", "discovered"].includes(routeSource)) score += 40;
  if (providerMode === "simultaneous_view") score += 20;
  if (providerMode === "single_controller") score += 10;
  if (viewStreamReadinessLabel(stream) === "ready") score += 10;
  return score;
}

function findPreferredStream(streams: readonly ServiceViewStream[], key?: string | null): ServiceViewStream | null {
  if (!key) return null;
  return streams.find((stream, index) => streamKeys(stream, index).includes(key)) ?? null;
}

function streamKeys(stream: ServiceViewStream, index: number): string[] {
  const stable = stream.id?.trim()
    ? `id:${stream.id.trim()}`
    : `provider:${normalize(stream.provider) || "unknown"}|route:${stream.routeId?.trim() || stream.connectionId?.trim() || "unrouted"}|index:${index}`;
  return [
    stable,
    stream.provider?.trim() ?? "",
    normalize(stream.provider),
    stream.routeId?.trim() ?? "",
    stream.connectionId?.trim() ?? "",
    String(index),
  ].filter(Boolean);
}

function projectReadiness({
  authority,
  stream,
  canView,
}: {
  authority: WorkspaceViewAuthorityEntry;
  stream: ServiceViewStream | null;
  canView: boolean;
}): ProjectedWorkspaceView["readiness"] {
  if (!authority.presentationActionCeilings.view.allowed) {
    return {
      state: "blocked",
      reason: authority.presentationActionCeilings.view.reason,
      source: "authority",
      recoveryAction: recoveryAction(stream?.attachability),
    };
  }
  if (!stream || !canView) {
    const evidence = viewStreamReadinessEvidence(stream);
    return {
      state: evidence.state ?? (stream ? viewStreamReadinessLabel(stream).replaceAll(" ", "_") : "unavailable"),
      reason: evidence.reason ?? (stream ? viewStreamOpenTitle(stream) : "No workspace view stream is available."),
      source: "stream",
      recoveryAction: recoveryAction(stream?.attachability),
    };
  }
  return {
    state: "ready",
    reason: recoveryReason(stream?.attachability),
    source: "stream",
    recoveryAction: recoveryAction(stream?.attachability),
  };
}

function recoveryReason(attachability: unknown): string | null {
  const reason = record(attachability)?.reason;
  return typeof reason === "string" && reason.trim() ? reason.trim() : null;
}

function recoveryAction(attachability: unknown): ProjectedWorkspaceView["readiness"]["recoveryAction"] {
  const action = record(attachability)?.recommendedAction;
  return action === "service_remote_view_route_switch"
    ? "service_remote_view_route_switch"
    : action === "service_remote_view_browser_reattach"
      ? "service_remote_view_browser_reattach"
      : null;
}

function selectTab(
  tabs: WorkspaceViewTabSource[],
  browserId: string,
  selection?: DashboardWorkspaceUrlSelection | null,
): ProjectedWorkspaceTab {
  const rows = tabs.filter((tab) => tab.browserId === browserId || browserId === `browser:${tab.browserId}`);
  if (rows.length === 0) {
    return {
      tab: null,
      tabIndex: null,
      recoveredFromStaleSelection: false,
      staleSelectionId: selection?.tabId ?? null,
      selectionEvidence: selection?.tabId ? "selected-missing" : "none",
    };
  }
  const selected = selection?.tabId
    ? rows.find((tab) => tab.id === selection.tabId || tab.targetId === selection.tabId || `target:${tab.targetId}` === selection.tabId)
    : null;
  const liveRows = rows.filter(isLiveTab);
  const best = [...liveRows].sort((left, right) => tabScore(right) - tabScore(left))[0] ?? rows[0];
  const selectedLive = Boolean(selected && isLiveTab(selected));
  const selectedBlank = Boolean(selected && isBlankTab(selected));
  const tab = selectedLive && selected ? selected : best;
  const indexRows = liveRows.length > 0 ? liveRows : rows;
  const tabIndex = indexRows.findIndex((candidate) => candidate.id === tab.id);
  const stale = Boolean(selection?.tabId && (!selectedLive || selectedBlank));
  const recovered = Boolean(selection?.tabId && !selectedLive);
  return {
    tab,
    tabIndex: tabIndex >= 0 ? tabIndex : null,
    recoveredFromStaleSelection: recovered,
    staleSelectionId: stale ? selection?.tabId ?? null : null,
    selectionEvidence: !selection?.tabId
      ? "none"
      : !selected
        ? "selected-missing"
        : !isLiveTab(selected)
          ? "selected-closed"
          : selectedBlank
            ? "selected-live-blank"
            : "selected-live",
  };
}

function tabScore(tab: WorkspaceViewTabSource): number {
  if (!isLiveTab(tab)) return -1000;
  const lifecycle = normalize(tab.lifecycle);
  return (lifecycle === "active" ? 400 : lifecycle === "loading" ? 320 : 300)
    + (isBlankTab(tab) ? 0 : 200)
    + (tab.targetId ? 25 : 0);
}

function isLiveTab(tab: WorkspaceViewTabSource): boolean {
  return ["active", "ready", "loading"].includes(normalize(tab.lifecycle));
}

function isBlankTab(tab: WorkspaceViewTabSource): boolean {
  const url = normalize(tab.url);
  const title = normalize(tab.title);
  return (!url || url === "about:blank" || url === "chrome://newtab/")
    && (!title || title === "about:blank" || title === "new tab");
}

function browserFromSelectedContext(
  context?: WorkspaceViewSelectedContextSource | null,
  routes?: Record<string, ServiceViewStream>,
): WorkspaceViewBrowserSource | null {
  const node = context?.node;
  const stream = context?.stream;
  if (!node || !stream) return null;
  const route = stream.routeId ? routes?.[stream.routeId] : null;
  const completed = completeRouteStream(node.id, stream, route);
  if (!completed) return null;
  return {
    id: node.browserId ?? (node.daemonSession ? `daemon:${node.daemonSession}` : node.id),
    authoritySubjectKey: node.id,
    displayName: node.label,
    profileId: node.profileId,
    host: node.host,
    health: node.health,
    browserBuild: node.browserBuild,
    displayAllocationId: completed.displayAllocationId,
    viewStreams: [completed],
    activeSessionIds: [node.daemonSession, node.serviceSessionId].filter((value): value is string => Boolean(value)),
  };
}

function completeRouteStream(
  nodeId: string,
  stream: NonNullable<WorkspaceViewSelectedContextSource["stream"]>,
  route?: ServiceViewStream | null,
): ServiceViewStream | null {
  const url = stream.url ?? route?.url ?? route?.frameUrl ?? route?.localEmbedUrl ?? route?.dashboardEmbedUrl ?? route?.publicOperatorUrl;
  if (!url) return null;
  return {
    ...route,
    ...stream,
    id: stream.id ?? route?.id ?? `selected:${nodeId}:${stream.provider ?? route?.provider ?? "stream"}`,
    provider: stream.provider ?? route?.provider,
    url,
    frameUrl: stream.frameUrl ?? route?.frameUrl ?? route?.localEmbedUrl ?? url,
    externalUrl: stream.externalUrl ?? route?.externalUrl ?? route?.publicOperatorUrl ?? url,
    routeDescriptor: { ...route?.routeDescriptor, ...stream.routeDescriptor },
    routeId: stream.routeId ?? route?.routeId ?? route?.id ?? null,
    readiness: stream.readiness ?? route?.readiness ?? {
      state: stream.operatorVisibleState,
      reason: stream.operatorVisibleReason ?? stream.routeSummary ?? undefined,
    },
    remoteReadiness: stream.remoteReadiness ?? route?.remoteReadiness,
    attachability: stream.attachability ?? route?.attachability,
    displayContent: stream.displayContent ?? route?.displayContent,
  };
}

function daemonBrowser(session: WorkspaceViewDaemonSource): WorkspaceViewBrowserSource | null {
  if (session.pending || session.closing || session.port <= 0) return null;
  const foreign = session.detected === true || session.ownership === "foreign_cdp";
  const url = foreign ? `/api/session-screenshot?port=${encodeURIComponent(String(session.port))}` : `http://127.0.0.1:${session.port}/`;
  return {
    id: `daemon:${session.session}`,
    displayName: session.session,
    host: "daemon-session",
    health: "ready",
    browserBuild: session.provider ?? session.engine,
    activeSessionIds: [session.session],
    viewStreams: [{
      id: foreign ? `foreign-cdp-snapshot:${session.session}` : `daemon-stream:${session.session}`,
      provider: foreign ? "cdp_snapshot" : "cdp_screencast",
      controlInput: foreign ? null : "cdp_input",
      url,
      frameUrl: url,
      externalUrl: url,
      routeId: foreign ? `foreign-cdp:${session.session}` : `daemon:${session.session}`,
      connectionName: session.session,
      routeSource: foreign ? "foreign-cdp" : "daemon-session",
      providerMode: foreign ? "read_only_snapshot_poll" : "single_controller",
      readOnly: foreign,
      readiness: { state: "ready", reason: foreign ? `foreign CDP snapshot ${session.port}` : `daemon stream ${session.port}` },
    }],
  };
}

function normalizeServiceBrowser(browser: WorkspaceViewBrowserSource): WorkspaceViewBrowserSource {
  const cdpPort = portFromUrl(browser.cdpEndpoint);
  if (!cdpPort) return browser;
  return {
    ...browser,
    viewStreams: (browser.viewStreams ?? []).map((stream) => {
      if (
        normalize(stream.provider) !== "cdp_screencast"
        || stream.frameUrl
        || stream.url
        || stream.externalUrl
      ) {
        return stream;
      }
      const url = `/api/session-screenshot?port=${encodeURIComponent(String(cdpPort))}`;
      const priorReadiness = viewStreamReadinessEvidence(stream);
      return {
        ...stream,
        id: stream.id ?? `service-cdp-snapshot:${browser.id}`,
        provider: "cdp_snapshot",
        url,
        frameUrl: url,
        externalUrl: url,
        routeId: stream.routeId ?? `service-cdp-snapshot:${browser.id}`,
        connectionName: stream.connectionName ?? browser.id,
        routeSource: "service-cdp-snapshot",
        providerMode: "read_only_snapshot_poll",
        controllerLeaseId: null,
        readOnly: true,
        controlInput: null,
        readiness: {
          state: "ready",
          reason: priorReadiness.state
            ? `CDP snapshot fallback replaces ${priorReadiness.state}.`
            : "CDP snapshot fallback is available.",
        },
        remoteReadiness: null,
      };
    }),
  };
}

function mergeSelectedBrowser(
  canonical: WorkspaceViewBrowserSource | null,
  fallback: WorkspaceViewBrowserSource | null,
): WorkspaceViewBrowserSource | null {
  if (!canonical) return fallback;
  if (!fallback || !browsersShareSession(canonical, fallback)) return canonical;
  return {
    ...fallback,
    ...canonical,
    viewStreams: mergeStreams(canonical.viewStreams, fallback.viewStreams),
    canonicalViewStreams: canonical.viewStreams ?? [],
    activeSessionIds: [...new Set([...(canonical.activeSessionIds ?? []), ...(fallback.activeSessionIds ?? [])])],
  };
}

function mergeStreams(primary?: ServiceViewStream[], secondary?: ServiceViewStream[]): ServiceViewStream[] {
  const result: ServiceViewStream[] = [];
  const indexByIdentity = new Map<string, number>();
  for (const stream of [...(primary ?? []), ...(secondary ?? [])]) {
    const identity = streamKeys(stream, 0)[0];
    const existingIndex = indexByIdentity.get(identity);
    if (existingIndex !== undefined) {
      const existing = result[existingIndex];
      result[existingIndex] = {
        ...stream,
        ...existing,
        url: existing.url ?? stream.url,
        frameUrl: existing.frameUrl ?? stream.frameUrl,
        externalUrl: existing.externalUrl ?? stream.externalUrl,
        localEmbedUrl: existing.localEmbedUrl ?? stream.localEmbedUrl,
        publicOperatorUrl: existing.publicOperatorUrl ?? stream.publicOperatorUrl,
        dashboardEmbedUrl: existing.dashboardEmbedUrl ?? stream.dashboardEmbedUrl,
        routeDescriptor: { ...stream.routeDescriptor, ...existing.routeDescriptor },
      };
      continue;
    }
    indexByIdentity.set(identity, result.length);
    result.push(stream);
  }
  return result;
}

function selectServiceBrowser(
  browsers: WorkspaceViewBrowserSource[],
  selection?: DashboardWorkspaceUrlSelection | null,
): WorkspaceViewBrowserSource | null {
  if (!selection) return null;
  const browserId = stripPrefix(selection.browserId, "browser:") ?? stripPrefix(selection.workspaceId, "browser:");
  if (browserId) {
    const exact = browsers.find((browser) => browser.id === browserId || browser.id === `browser:${browserId}`);
    if (exact) return exact;
  }
  const sessionId = stripPrefix(selection.sessionId, "session:") ?? stripPrefix(selection.workspaceId, "daemon-session:");
  return sessionId ? browsers.find((browser) => browser.activeSessionIds?.includes(sessionId)) ?? null : null;
}

function selectDaemonBrowser(
  browsers: WorkspaceViewBrowserSource[],
  selection?: DashboardWorkspaceUrlSelection | null,
): WorkspaceViewBrowserSource | null {
  if (!selection) return null;
  const sessionId = stripPrefix(selection.sessionId, "session:") ?? stripPrefix(selection.workspaceId, "daemon-session:");
  return sessionId
    ? browsers.find((browser) => browser.id === `daemon:${sessionId}` || browser.activeSessionIds?.includes(sessionId)) ?? null
    : null;
}

function browsersShareSession(left: WorkspaceViewBrowserSource, right: WorkspaceViewBrowserSource): boolean {
  const leftIds = new Set([...(left.activeSessionIds ?? []), stripPrefix(left.id, "daemon:")].filter((value): value is string => Boolean(value)));
  return [...(right.activeSessionIds ?? []), stripPrefix(right.id, "daemon:")]
    .filter((value): value is string => Boolean(value))
    .some((value) => leftIds.has(value));
}

function authoritySubjectKey(browser: WorkspaceViewBrowserSource): string {
  if (browser.authoritySubjectKey) return browser.authoritySubjectKey;
  if (browser.id.startsWith("browser:") || browser.id.startsWith("daemon:")) return browser.id;
  return `browser:${browser.id}`;
}

function missingAuthority(subjectKey: string, browser: WorkspaceViewBrowserSource): WorkspaceViewAuthorityEntry {
  return {
    subjectKey,
    authoritySource: "service-status-compatibility",
    browserId: browser.id,
    workspaceId: subjectKey,
    inventoryClass: "service-owned-diagnostic-browser",
    lifecycle: {
      state: "needs-attention",
      live: !TERMINAL_HEALTH.has(normalize(browser.health)),
      retained: TERMINAL_HEALTH.has(normalize(browser.health)),
      health: browser.health ?? null,
    },
    presentationActionCeilings: {
      view: { allowed: false, reason: "Canonical workspace authority is unavailable." },
      control: { allowed: false, reason: "Canonical workspace authority is unavailable." },
      stream: { allowed: false, reason: "Canonical workspace authority is unavailable." },
      screenshot: { allowed: false, reason: "Canonical workspace authority is unavailable." },
    },
    diagnostics: [],
  };
}

function routeKey(stream: ServiceViewStream): string {
  return stream.routeId || stream.connectionId || stream.frameUrl || stream.externalUrl || stream.url || "unrouted";
}

function stripPrefix(value: string | null | undefined, prefix: string): string | null {
  const trimmed = value?.trim();
  if (!trimmed) return null;
  return trimmed.startsWith(prefix) ? trimmed.slice(prefix.length) : trimmed;
}

function portFromUrl(value?: string | null): number | null {
  if (!value) return null;
  try {
    const port = new URL(value).port;
    return port ? Number(port) : null;
  } catch {
    const match = value.match(/:(\d{2,5})(?:\/|$)/);
    return match ? Number(match[1]) : null;
  }
}

function normalize(value?: string | null): string {
  return value?.trim().toLowerCase().replaceAll("-", "_") ?? "";
}

function record(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" && !Array.isArray(value)
    ? value as Record<string, unknown>
    : null;
}
