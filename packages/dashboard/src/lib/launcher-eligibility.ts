import { compactWorkspaceViewportReadinessComponents } from "./workspace-viewport-state.ts";

export type LauncherEligibilityStatus = "eligible" | "needs-operator-action" | "blocked";

export type LauncherEligibilityTone = "good" | "warn" | "bad" | "neutral";

export type LauncherReadinessRow = {
  targetServiceId?: string | null;
  loginId?: string | null;
  state?: string | null;
  manualSeedingRequired?: boolean | null;
  evidence?: string | null;
  recommendedAction?: string | null;
};

export type LauncherProfileRecord = {
  id?: string | null;
  name?: string | null;
  defaultBrowserHost?: string | null;
  browserBuild?: string | null;
  targetServiceIds?: string[];
  authenticatedServiceIds?: string[];
  accountIds?: string[];
  sharedServiceIds?: string[];
  manualLoginPreferred?: boolean | null;
  targetReadiness?: LauncherReadinessRow[];
};

export type LauncherProfileAllocation = {
  profileId: string;
  profileName?: string | null;
  browserBuild?: string | null;
  targetServiceIds?: string[];
  authenticatedServiceIds?: string[];
  accountIds?: string[];
  targetReadiness?: LauncherReadinessRow[];
  holderSessionIds?: string[];
  exclusiveHolderSessionIds?: string[];
  waitingJobIds?: string[];
  conflictSessionIds?: string[];
  leaseState?: string | null;
  recommendedAction?: string | null;
  serviceNames?: string[];
  agentNames?: string[];
  taskNames?: string[];
  browserIds?: string[];
  browserSummaries?: Array<{
    browserId?: string | null;
    host?: string | null;
    health?: string | null;
    hasCdpEndpoint?: boolean | null;
    activeSessionIds?: string[];
  }>;
};

export type LauncherBrowserRecord = {
  id: string;
  profileId?: string | null;
  host?: string | null;
  health?: string | null;
  browserBuild?: string | null;
  executableId?: string | null;
  executablePath?: string | null;
  viewStreams?: Array<{
    provider?: string | null;
    url?: string | null;
    readOnly?: boolean | null;
    controlInput?: string | null;
  }>;
};

export type LauncherBrowserCapabilityRegistry = {
  browserHosts?: Array<Record<string, unknown>>;
  browserExecutables?: Array<Record<string, unknown>>;
  browserCapabilities?: Array<Record<string, unknown>>;
  profileCompatibility?: Array<Record<string, unknown>>;
  browserPreferenceBindings?: Array<Record<string, unknown>>;
  validationEvidence?: Array<Record<string, unknown>>;
  generatedAt?: string | null;
};

export type LauncherAccessPlanPreview = {
  comboId?: string;
  profileId?: string | null;
  browserBuild?: string | null;
  selectedProfile?: Record<string, unknown> | null;
  readinessSummary?: Record<string, unknown> | null;
  browserCapabilityEvidence?: Record<string, unknown> | null;
  decision?: Record<string, unknown> | null;
};

export type LauncherDisplayIsolation =
  | "service_default"
  | "private_virtual_display"
  | "shared_display"
  | "ambient_display";

export type LauncherViewStreamPreference =
  | "service_default"
  | "cdp_screencast"
  | "chrome_tab_webrtc"
  | "virtual_display_webrtc"
  | "novnc"
  | "rdp_gateway"
  | "external_url";

export type LauncherControlInputPreference =
  | "service_default"
  | "cdp_input"
  | "webrtc_input"
  | "vnc_input"
  | "manual_attached_desktop";

export type LauncherServiceRequest = {
  action: "tab_new" | "cdp_free_launch";
  params?: Record<string, unknown>;
  [key: string]: unknown;
};

export type LauncherServiceRequestOptions = {
  url?: string;
  displayIsolation?: LauncherDisplayIsolation;
  viewStreamProvider?: LauncherViewStreamPreference;
  controlInputProvider?: LauncherControlInputPreference;
  jobTimeoutMs?: number;
  allowManualAction?: boolean;
};

export type LauncherSessionLaunchOptions = LauncherServiceRequestOptions & {
  sessionName: string;
  executableId?: string | null;
  browserHostId?: string | null;
};

export type LauncherAccessPlanPosture = {
  action: "tab_new" | "cdp_free_launch";
  helper?: string | null;
  route?: string | null;
  profileLeasePolicy?: string | null;
  browserBuild?: string | null;
  url?: string | null;
  displayIsolation?: string | null;
  viewStreamProvider?: string | null;
  controlInputProvider?: string | null;
};

export type LauncherEligibilityInput = {
  profiles?: LauncherProfileRecord[];
  allocations?: LauncherProfileAllocation[];
  browsers?: LauncherBrowserRecord[];
  browserCapabilityRegistry?: LauncherBrowserCapabilityRegistry | null;
  accessPlans?: LauncherAccessPlanPreview[];
  serviceRequestActions?: string[] | Set<string>;
};

export type LauncherEligibilityRow = {
  id: string;
  status: LauncherEligibilityStatus;
  tone: LauncherEligibilityTone;
  profileId: string;
  profileName: string;
  browserBuild: string;
  browserHost: string;
  browserHostId?: string | null;
  browserId?: string | null;
  executableId?: string | null;
  capabilityId?: string | null;
  launchAction: "tab_new" | "cdp_free_launch";
  cdpFree: boolean;
  remoteView: "controllable" | "view-only" | "unavailable";
  reason: string;
  reasonSource: "access-plan" | "readiness" | "capability-registry" | "profile-allocation" | "service-contract";
  targetServiceIds: string[];
  loginIds: string[];
  accountIds: string[];
  serviceNames: string[];
  agentNames: string[];
  taskNames: string[];
  accessPlanFetched: boolean;
  serviceReason?: string | null;
  evidenceSummary: string;
};

export type LauncherEligibilityPreview = {
  rows: LauncherEligibilityRow[];
  summary: {
    total: number;
    eligible: number;
    needsOperatorAction: number;
    blocked: number;
    accessPlanFetched: number;
    registryExecutables: number;
    runtimeProfiles: number;
  };
};

type BrowserOption = {
  id: string;
  browserBuild: string;
  browserHost: string;
  browserHostId?: string | null;
  executableId?: string | null;
  capabilityId?: string | null;
  browserId?: string | null;
  cdpFreeSupported: boolean;
  cdpSupported: boolean;
  remoteView: "controllable" | "view-only" | "unavailable";
  source: "capability-registry" | "runtime-browser" | "profile-default";
};

const SERVICE_DEFAULT_BUILD = "service default";
const MANUAL_READINESS_STATES = new Set([
  "needs_manual_seeding",
  "manual_seeding_required",
  "stale",
  "failed",
  "unverified",
  "seeding_closed_unverified",
]);
const CONFLICT_LEASE_STATES = new Set([
  "blocked",
  "conflict",
  "conflicted",
  "exclusive_conflict",
  "lease_conflict",
]);
const REMOTE_VIEW_FAILED_READINESS_STATES = new Set([
  "blocked",
  "down",
  "error",
  "failed",
  "missing",
  "refused",
  "unavailable",
  "unhealthy",
]);
const REMOTE_VIEW_ACTION_READINESS_STATES = new Set([
  "action_required",
  "auth_required",
  "degraded",
  "expired",
  "stale",
  "unknown",
  "warning",
]);

export function deriveLauncherEligibilityPreview(input: LauncherEligibilityInput): LauncherEligibilityPreview {
  const serviceActions = new Set(
    input.serviceRequestActions instanceof Set
      ? Array.from(input.serviceRequestActions)
      : input.serviceRequestActions ?? [],
  );
  const allocationsByProfile = new Map(
    (input.allocations ?? [])
      .filter((allocation) => allocation.profileId?.trim())
      .map((allocation) => [allocation.profileId, allocation]),
  );
  const profiles = mergedLauncherProfiles(input.profiles ?? [], input.allocations ?? []);
  const browsersByProfile = groupBrowsersByProfile(input.browsers ?? []);
  const accessPlans = input.accessPlans ?? [];
  const registry = input.browserCapabilityRegistry ?? null;
  const registryExecutables = registry?.browserExecutables?.length ?? 0;
  const rows: LauncherEligibilityRow[] = [];

  for (const profile of profiles) {
    const profileId = normalizedProfileId(profile);
    if (!profileId) continue;
    const allocation = allocationsByProfile.get(profileId);
    const browserOptions = deriveBrowserOptions(
      profile,
      allocation,
      browsersByProfile.get(profileId) ?? [],
      registry,
    );

    for (const option of browserOptions) {
      rows.push(deriveLauncherEligibilityRow({
        profile,
        allocation,
        option,
        serviceActions,
        registry,
        accessPlan: findAccessPlan(accessPlans, `launch:${profileId}:${option.id}`, profileId, option.browserBuild),
      }));
    }
  }

  rows.sort((left, right) => {
    const statusOrder = statusSortValue(left.status) - statusSortValue(right.status);
    if (statusOrder !== 0) return statusOrder;
    const profileOrder = left.profileName.localeCompare(right.profileName);
    if (profileOrder !== 0) return profileOrder;
    return left.browserBuild.localeCompare(right.browserBuild);
  });

  return {
    rows,
    summary: {
      total: rows.length,
      eligible: rows.filter((row) => row.status === "eligible").length,
      needsOperatorAction: rows.filter((row) => row.status === "needs-operator-action").length,
      blocked: rows.filter((row) => row.status === "blocked").length,
      accessPlanFetched: rows.filter((row) => row.accessPlanFetched).length,
      registryExecutables,
      runtimeProfiles: profiles.length,
    },
  };
}

export function launcherAccessPlanPosture(accessPlan?: LauncherAccessPlanPreview | null): LauncherAccessPlanPosture | null {
  if (!accessPlan) return null;
  const decision = recordField(accessPlan, "decision");
  const serviceRequest = recordField(decision, "serviceRequest");
  const request = recordField(serviceRequest, "request");
  const params = recordField(request, "params");
  const launchPosture = recordField(decision, "launchPosture");
  if (!request) return null;
  const action = launcherAccessPlanRequiresCdpFree(accessPlan) ? "cdp_free_launch" : "tab_new";
  return {
    action,
    helper: stringField(serviceRequest, "helper"),
    route: stringField(serviceRequest, "route"),
    profileLeasePolicy: stringField(request, "profileLeasePolicy"),
    browserBuild: stringField(request, "browserBuild") || stringField(launchPosture, "browserBuild"),
    url: stringField(request, "url") || stringField(params, "url"),
    displayIsolation: stringField(params, "displayIsolation") || stringField(launchPosture, "displayIsolation"),
    viewStreamProvider: stringField(params, "viewStreamProvider") || stringField(launchPosture, "viewStreamProvider"),
    controlInputProvider: stringField(params, "controlInputProvider") || stringField(launchPosture, "controlInputProvider"),
  };
}

export function createLauncherServiceRequestFromAccessPlan(
  accessPlan: LauncherAccessPlanPreview,
  options: LauncherServiceRequestOptions = {},
): LauncherServiceRequest {
  const decision = requiredRecord(accessPlan, "decision", "access plan decision");
  const serviceRequest = requiredRecord(decision, "serviceRequest", "access plan service request");
  if ((serviceRequest as Record<string, unknown>).available === false) {
    throw new Error(stringField(serviceRequest, "reason") || "Access plan service request is not available.");
  }
  if (!options.allowManualAction && launcherAccessPlanRequiresManualSeeding(accessPlan, decision, serviceRequest)) {
    throw new Error(stringField(serviceRequest, "reason") || "Access plan requires manual profile seeding before launch.");
  }
  const plannedRequest = requiredRecord(serviceRequest, "request", "access plan service request payload");
  const plannedAction = stringField(plannedRequest, "action");
  if (plannedAction !== "tab_new" && plannedAction !== "cdp_free_launch") {
    throw new Error(`Access plan service request action ${plannedAction || "unknown"} is not launchable.`);
  }

  const { action: _plannedAction, params: plannedParams, ...plannedFields } = plannedRequest;
  const params = plainRecordOrEmpty(plannedParams);
  const targetUrl = options.url?.trim();
  const posture = launcherAccessPlanPosture(accessPlan);
  const effectiveDisplayIsolation =
    options.displayIsolation && options.displayIsolation !== "service_default"
      ? options.displayIsolation
      : posture?.displayIsolation || undefined;
  const effectiveViewStreamProvider =
    options.viewStreamProvider && options.viewStreamProvider !== "service_default"
      ? options.viewStreamProvider
      : posture?.viewStreamProvider || undefined;
  const explicitControlInputProvider =
    options.controlInputProvider && options.controlInputProvider !== "service_default"
      ? options.controlInputProvider
      : undefined;
  const effectiveControlInputProvider =
    explicitControlInputProvider ??
    (effectiveViewStreamProvider === "rdp_gateway" ? undefined : posture?.controlInputProvider || undefined);
  const action = launcherAccessPlanRequiresCdpFree(accessPlan) ? "cdp_free_launch" : plannedAction;
  const request: LauncherServiceRequest = {
    ...plannedFields,
    action,
  };

  if (options.allowManualAction === true) request.allowManualAction = true;
  if (Number.isInteger(options.jobTimeoutMs) && Number(options.jobTimeoutMs) > 0) {
    request.jobTimeoutMs = options.jobTimeoutMs;
  }
  if (targetUrl) {
    request.url = targetUrl;
    params.url = targetUrl;
  }
  if (effectiveDisplayIsolation) {
    params.displayIsolation = effectiveDisplayIsolation;
  }
  if (effectiveViewStreamProvider) {
    params.viewStreamProvider = effectiveViewStreamProvider;
    if (effectiveViewStreamProvider === "rdp_gateway") {
      params.browserHost = "remote_headed";
      params.headless = false;
      params.displayIsolation = params.displayIsolation || "shared_display";
      params.controlInputProvider = params.controlInputProvider || "manual_attached_desktop";
    }
  }
  if (effectiveControlInputProvider) {
    params.controlInputProvider = effectiveControlInputProvider;
  }
  if (action === "cdp_free_launch") {
    request.requiresCdpFree = true;
    request.cdpAttachmentAllowed = false;
  }
  if (Object.keys(params).length > 0) {
    request.params = params;
  }
  return request;
}

export function createLauncherSessionArgsFromAccessPlan(
  accessPlan: LauncherAccessPlanPreview,
  options: LauncherSessionLaunchOptions,
): string[] {
  const sessionName = options.sessionName.trim();
  if (!sessionName) {
    throw new Error("A session name is required for workspace launch.");
  }
  const request = createLauncherServiceRequestFromAccessPlan(accessPlan, options);
  const params = plainRecordOrEmpty(request.params);
  const args = ["--session", sessionName];
  const executablePath = launcherExecutablePath(accessPlan, {
    executableId: options.executableId,
    browserHostId: options.browserHostId,
    browserBuild: stringField(request, "browserBuild") || accessPlan.browserBuild || null,
  });

  if (executablePath) {
    args.push("--executable-path", executablePath);
  }
  const runtimeProfile = stringField(request, "runtimeProfile");
  const profile = stringField(request, "profile");
  if (runtimeProfile) {
    args.push("--runtime-profile", runtimeProfile);
  } else if (profile) {
    args.push("--profile", profile);
  }
  const browserHost = stringField(params, "browserHost");
  if (browserHost) {
    args.push("--browser-host", browserHost);
  }
  const viewStreamProvider = stringField(params, "viewStreamProvider");
  if (viewStreamProvider) {
    args.push("--view-stream-provider", viewStreamProvider);
  }
  const controlInputProvider = stringField(params, "controlInputProvider");
  if (controlInputProvider) {
    args.push("--control-input-provider", controlInputProvider);
  }
  const displayIsolation = stringField(params, "displayIsolation");
  if (displayIsolation) {
    args.push("--display-isolation", displayIsolation);
  }
  if (params.headless === false) {
    args.push("--headed");
  }
  args.push("open", stringField(params, "url") || stringField(request, "url") || options.url?.trim() || "about:blank");
  return args;
}

function launcherExecutablePath(
  accessPlan: LauncherAccessPlanPreview,
  selection: { executableId?: string | null; browserHostId?: string | null; browserBuild?: string | null },
): string | null {
  const evidence = accessPlan.browserCapabilityEvidence;
  const executables = Array.isArray(evidence?.browserExecutables)
    ? evidence.browserExecutables.filter((item): item is Record<string, unknown> => item !== null && typeof item === "object")
    : [];
  const executableId = selection.executableId?.trim();
  const browserHostId = selection.browserHostId?.trim();
  const browserBuild = selection.browserBuild?.trim();
  const candidates = [
    executableId ? executables.find((item) => stringField(item, "id") === executableId) : null,
    browserHostId && browserBuild
      ? executables.find(
          (item) => stringField(item, "hostId") === browserHostId && stringField(item, "buildLabel") === browserBuild,
        )
      : null,
    browserBuild ? executables.find((item) => stringField(item, "buildLabel") === browserBuild) : null,
  ];
  for (const candidate of candidates) {
    const executablePath = candidate ? stringField(candidate, "executablePath") : "";
    if (executablePath) return executablePath;
  }
  return null;
}

function deriveLauncherEligibilityRow({
  profile,
  allocation,
  option,
  serviceActions,
  registry,
  accessPlan,
}: {
  profile: LauncherProfileRecord;
  allocation?: LauncherProfileAllocation;
  option: BrowserOption;
  serviceActions: Set<string>;
  registry: LauncherBrowserCapabilityRegistry | null;
  accessPlan?: LauncherAccessPlanPreview;
}): LauncherEligibilityRow {
  const profileId = normalizedProfileId(profile, allocation?.profileId);
  const readinessRows = profile.targetReadiness?.length ? profile.targetReadiness : allocation?.targetReadiness ?? [];
  const targetServiceIds = uniqueStrings([
    ...(profile.targetServiceIds ?? []),
    ...(profile.authenticatedServiceIds ?? []),
    ...(allocation?.targetServiceIds ?? []),
    ...(allocation?.authenticatedServiceIds ?? []),
    ...readinessRows.map((row) => row.targetServiceId),
  ]);
  const loginIds = uniqueStrings(readinessRows.map((row) => row.loginId));
  const accountIds = uniqueStrings([
    ...(profile.accountIds ?? []),
    ...(allocation?.accountIds ?? []),
  ]);
  const serviceNames = uniqueStrings(allocation?.serviceNames ?? []);
  const agentNames = uniqueStrings(allocation?.agentNames ?? []);
  const taskNames = uniqueStrings(allocation?.taskNames ?? []);
  const checks: Array<{
    status: LauncherEligibilityStatus;
    reason: string;
    source: LauncherEligibilityRow["reasonSource"];
    serviceReason?: string | null;
  }> = [];
  const accessPlanCheck = accessPlan ? accessPlanDecisionCheck(accessPlan, option) : null;
  const launchAction = accessPlanCheck?.launchAction ?? (option.browserBuild === "cdp_free_headed" ? "cdp_free_launch" : "tab_new");
  const plannedHost = accessPlan ? accessPlanBrowserHost(accessPlan) : "";
  const plannedRemoteView = accessPlan ? accessPlanRemoteView(accessPlan) : null;
  const effectiveOption: BrowserOption = {
    ...option,
    browserHost: plannedHost || option.browserHost,
    remoteView: plannedRemoteView ?? option.remoteView,
  };

  if (!serviceActions.has(launchAction)) {
    checks.push({
      status: "blocked",
      reason: `Service request contract does not advertise ${launchAction}.`,
      source: "service-contract",
    });
  }

  if (!accessPlan) {
    checks.push({
      status: "needs-operator-action",
      reason: "No access-plan response has been fetched for this combination.",
      source: "access-plan",
    });
  } else if (accessPlanCheck) {
    checks.push(accessPlanCheck);
    const remoteReadinessCheck = accessPlanRemoteViewReadinessCheck(accessPlan);
    if (remoteReadinessCheck) checks.push(remoteReadinessCheck);
  }

  const leaseState = allocation?.leaseState?.trim().toLowerCase() ?? "";
  if (
    (leaseState && CONFLICT_LEASE_STATES.has(leaseState)) ||
    (allocation?.conflictSessionIds?.length ?? 0) > 0 ||
    (allocation?.exclusiveHolderSessionIds?.length ?? 0) > 0
  ) {
    checks.push({
      status: "blocked",
      reason: allocation?.recommendedAction || "Profile allocation reports an exclusive lease conflict.",
      source: "profile-allocation",
      serviceReason: allocation?.recommendedAction,
    });
  }

  for (const row of readinessRows) {
    const state = row.state?.trim().toLowerCase() ?? "";
    if (row.manualSeedingRequired || MANUAL_READINESS_STATES.has(state)) {
      checks.push({
        status: "needs-operator-action",
        reason: row.recommendedAction || `Profile readiness is ${row.state ?? "not ready"}.`,
        source: "readiness",
        serviceReason: row.recommendedAction,
      });
      break;
    }
  }

  const capabilityChecks = capabilityRegistryChecks(profileId, effectiveOption, registry, launchAction);
  checks.push(...capabilityChecks);

  const primaryCheck = selectPrimaryCheck(checks);
  const status = primaryCheck?.status ?? "eligible";
  const reason = primaryCheck?.reason ?? "Access plan, profile readiness, capability evidence, and service request action are aligned.";
  return {
    id: `launch:${profileId}:${option.id}`,
    status,
    tone: status === "eligible" ? "good" : status === "needs-operator-action" ? "warn" : "bad",
    profileId,
    profileName: profileDisplayName(profile.name?.trim() || allocation?.profileName?.trim() || profileId),
    browserBuild: effectiveOption.browserBuild,
    browserHost: effectiveOption.browserHost,
    browserHostId: effectiveOption.browserHostId,
    browserId: effectiveOption.browserId,
    executableId: effectiveOption.executableId,
    capabilityId: effectiveOption.capabilityId,
    launchAction,
    cdpFree: launchAction === "cdp_free_launch",
    remoteView: effectiveOption.remoteView,
    reason,
    reasonSource: primaryCheck?.source ?? "access-plan",
    targetServiceIds,
    loginIds,
    accountIds,
    serviceNames,
    agentNames,
    taskNames,
    accessPlanFetched: Boolean(accessPlan),
    serviceReason: primaryCheck?.serviceReason,
    evidenceSummary: [
      option.source,
      accessPlan ? "access-plan" : "no access-plan",
      registry?.generatedAt ? `registry ${registry.generatedAt}` : null,
    ].filter(Boolean).join(" / "),
  };
}

function accessPlanBrowserHost(accessPlan: LauncherAccessPlanPreview): string {
  const decision = recordField(accessPlan, "decision");
  const launchPosture = recordField(decision, "launchPosture");
  const serviceRequest = recordField(decision, "serviceRequest");
  const request = recordField(serviceRequest, "request");
  const params = recordField(request, "params");
  return stringField(params, "browserHost") || stringField(launchPosture, "browserHost");
}

function accessPlanRemoteView(accessPlan: LauncherAccessPlanPreview): BrowserOption["remoteView"] | null {
  const decision = recordField(accessPlan, "decision");
  const launchPosture = recordField(decision, "launchPosture");
  const serviceRequest = recordField(decision, "serviceRequest");
  const request = recordField(serviceRequest, "request");
  const params = recordField(request, "params");
  const provider = stringField(params, "viewStreamProvider") || stringField(launchPosture, "viewStreamProvider");
  const controlInput = stringField(params, "controlInputProvider") || stringField(launchPosture, "controlInputProvider");
  const remoteViewRecommended = booleanField(launchPosture, "remoteViewRecommended");

  if (!provider && !remoteViewRecommended) return null;
  if (!provider || provider === "cdp_screencast") return null;
  if (provider === "external_url") return "view-only";
  return controlInput && controlInput !== "cdp_input" ? "controllable" : "view-only";
}

function mergedLauncherProfiles(
  profiles: LauncherProfileRecord[],
  allocations: LauncherProfileAllocation[],
): LauncherProfileRecord[] {
  const byId = new Map<string, LauncherProfileRecord>();
  for (const profile of profiles) {
    const profileId = normalizedProfileId(profile);
    if (profileId) byId.set(profileId, profile);
  }
  for (const allocation of allocations) {
    if (!allocation.profileId?.trim() || byId.has(allocation.profileId)) continue;
    byId.set(allocation.profileId, {
      id: allocation.profileId,
      name: allocation.profileName,
      browserBuild: allocation.browserBuild,
      targetServiceIds: allocation.targetServiceIds,
      authenticatedServiceIds: allocation.authenticatedServiceIds,
      accountIds: allocation.accountIds,
      targetReadiness: allocation.targetReadiness,
    });
  }
  return Array.from(byId.values());
}

function deriveBrowserOptions(
  profile: LauncherProfileRecord,
  allocation: LauncherProfileAllocation | undefined,
  browsers: LauncherBrowserRecord[],
  registry: LauncherBrowserCapabilityRegistry | null,
): BrowserOption[] {
  const options: BrowserOption[] = [];
  const hostsById = new Map((registry?.browserHosts ?? []).map((host) => [stringField(host, "id"), host]));
  const capabilitiesByExecutable = new Map<string, Record<string, unknown>>();
  for (const capability of registry?.browserCapabilities ?? []) {
    const executableId = stringField(capability, "executableId");
    if (executableId && !capabilitiesByExecutable.has(executableId)) {
      capabilitiesByExecutable.set(executableId, capability);
    }
  }

  for (const executable of registry?.browserExecutables ?? []) {
    const executableId = stringField(executable, "id");
    const hostId = stringField(executable, "hostId");
    const capability = executableId ? capabilitiesByExecutable.get(executableId) : undefined;
    const host = hostId ? hostsById.get(hostId) : undefined;
    options.push({
      id: `registry:${hostId || "host"}:${executableId || options.length}`,
      browserBuild: stringField(executable, "buildLabel") || SERVICE_DEFAULT_BUILD,
      browserHost: stringField(host, "name") || hostId || "service host",
      browserHostId: hostId,
      executableId,
      capabilityId: stringField(capability, "id"),
      cdpFreeSupported: booleanField(capability, "cdpFreeLaunchSupported"),
      cdpSupported: booleanField(capability, "cdpSupported"),
      remoteView: remoteViewFromCapability(host, capability),
      source: "capability-registry",
    });
  }

  for (const browser of browsers) {
    options.push({
      id: `browser:${browser.id}`,
      browserBuild: browser.browserBuild?.trim() || SERVICE_DEFAULT_BUILD,
      browserHost: browser.host?.trim() || "service host",
      executableId: browser.executableId ?? null,
      browserId: browser.id,
      cdpFreeSupported: false,
      cdpSupported: Boolean(browser.executableId || browser.executablePath),
      remoteView: remoteViewFromStreams(browser.viewStreams),
      source: "runtime-browser",
    });
  }

  const profileBuild = profile.browserBuild?.trim() || allocation?.browserBuild?.trim();
  const profileHost = profile.defaultBrowserHost?.trim() ||
    allocation?.browserSummaries?.find((summary) => summary.host?.trim())?.host?.trim();
  if (profileBuild || profileHost || options.length === 0) {
    options.push({
      id: `profile-default:${profileBuild || SERVICE_DEFAULT_BUILD}:${profileHost || "service-host"}`,
      browserBuild: profileBuild || SERVICE_DEFAULT_BUILD,
      browserHost: profileHost || "service host",
      cdpFreeSupported: profileBuild === "cdp_free_headed",
      cdpSupported: profileBuild !== "cdp_free_headed",
      remoteView: "unavailable",
      source: "profile-default",
    });
  }

  return dedupeBrowserOptions(options);
}

function capabilityRegistryChecks(
  profileId: string,
  option: BrowserOption,
  registry: LauncherBrowserCapabilityRegistry | null,
  launchAction: "tab_new" | "cdp_free_launch",
): Array<{
  status: LauncherEligibilityStatus;
  reason: string;
  source: LauncherEligibilityRow["reasonSource"];
  serviceReason?: string | null;
}> {
  if (!registry || (registry.browserExecutables?.length ?? 0) === 0) {
    return [{
      status: "blocked",
      reason: "No browser capability registry executable evidence is available from service state.",
      source: "capability-registry",
    }];
  }
  if (!option.executableId) {
    return [{
      status: "blocked",
      reason: "No browser executable evidence is attached to this browser option.",
      source: "capability-registry",
    }];
  }

  const checks = [];
  const executable = registry.browserExecutables?.find((row) => stringField(row, "id") === option.executableId);
  const hostId = stringField(executable, "hostId") || option.browserHostId || "";
  const host = registry.browserHosts?.find((row) => stringField(row, "id") === hostId);
  const capability = registry.browserCapabilities?.find((row) => stringField(row, "executableId") === option.executableId);
  const compatibilityRows = (registry.profileCompatibility ?? []).filter((row) =>
    stringField(row, "profileId") === profileId &&
    stringField(row, "executableId") === option.executableId
  );
  const validationRows = (registry.validationEvidence ?? []).filter((row) =>
    stringField(row, "executableId") === option.executableId &&
    (!option.capabilityId || stringField(row, "capabilityId") === option.capabilityId)
  );

  if (host && booleanField(host, "reachable") === false) {
    checks.push({
      status: "blocked" as const,
      reason: `${stringField(host, "name") || hostId || "Browser host"} is not reachable.`,
      source: "capability-registry" as const,
    });
  }
  if (host && stringField(host, "lifecycleOwner") && stringField(host, "lifecycleOwner") !== "agent_browser") {
    checks.push({
      status: "blocked" as const,
      reason: `${stringField(host, "name") || hostId || "Browser host"} is not owned by agent-browser lifecycle control.`,
      source: "capability-registry" as const,
    });
  }
  if (executable && stringField(executable, "executablePath") === "") {
    checks.push({
      status: "blocked" as const,
      reason: `${option.executableId} has no executable path evidence.`,
      source: "capability-registry" as const,
    });
  }
  if (executable && booleanField(executable, "fresh") === false) {
    checks.push({
      status: "blocked" as const,
      reason: `${option.executableId} is marked stale by browser capability evidence.`,
      source: "capability-registry" as const,
    });
  }
  if (!capability) {
    checks.push({
      status: "blocked" as const,
      reason: `${option.executableId} has no browser capability row.`,
      source: "capability-registry" as const,
    });
  }
  if (launchAction === "cdp_free_launch" && capability && booleanField(capability, "cdpFreeLaunchSupported") === false) {
    checks.push({
      status: "blocked" as const,
      reason: `${option.executableId} does not advertise CDP-free launch support.`,
      source: "capability-registry" as const,
    });
  }
  if (option.remoteView === "unavailable") {
    checks.push({
      status: "needs-operator-action" as const,
      reason: "No controllable remote-view evidence is available for this browser host.",
      source: "capability-registry" as const,
    });
  }
  if (compatibilityRows.length === 0) {
    checks.push({
      status: "blocked" as const,
      reason: "No profile compatibility evidence is available for this browser/profile pair.",
      source: "capability-registry" as const,
    });
  }
  for (const row of compatibilityRows) {
    if (booleanField(row, "compatible") === false) {
      checks.push({
        status: "blocked" as const,
        reason: stringField(row, "notes") || stringField(row, "reason") || "Profile compatibility evidence blocks this pair.",
        source: "capability-registry" as const,
        serviceReason: stringField(row, "reason"),
      });
    } else if (booleanField(row, "requiresOperatorOverride")) {
      checks.push({
        status: "needs-operator-action" as const,
        reason: stringField(row, "notes") || "Profile compatibility requires an operator override.",
        source: "capability-registry" as const,
        serviceReason: stringField(row, "reason"),
      });
    }
  }

  if (validationRows.length === 0 || !validationRows.some((row) => stringField(row, "state") === "passed")) {
    checks.push({
      status: "blocked" as const,
      reason: "No passed browser capability validation evidence is available for this executable.",
      source: "capability-registry" as const,
    });
  }
  for (const row of validationRows) {
    const state = stringField(row, "state");
    if (state === "failed" || state === "stale") {
      checks.push({
        status: "blocked" as const,
        reason: stringField(row, "evidence") || `Browser validation evidence is ${state}.`,
        source: "capability-registry" as const,
        serviceReason: state,
      });
    }
  }

  return checks;
}

function accessPlanDecisionCheck(accessPlan: LauncherAccessPlanPreview, option: BrowserOption): {
  status: LauncherEligibilityStatus;
  reason: string;
  source: LauncherEligibilityRow["reasonSource"];
  serviceReason?: string | null;
  launchAction: "tab_new" | "cdp_free_launch";
} {
  const decision = recordField(accessPlan, "decision");
  const serviceRequest = recordField(decision, "serviceRequest");
  const request = recordField(serviceRequest, "request");
  const attention = recordField(decision, "attention");
  const launchPosture = recordField(decision, "launchPosture");
  const readinessSummary = recordField(accessPlan, "readinessSummary");
  const requestedAction = stringField(request, "action");
  const requiresCdpFree =
    booleanField(launchPosture, "requiresCdpFree") ||
    booleanField(serviceRequest, "requiresCdpFree") ||
    option.browserBuild === "cdp_free_headed";
  const launchAction = requestedAction === "cdp_free_launch" || requiresCdpFree ? "cdp_free_launch" : "tab_new";

  if (booleanField(attention, "required")) {
    const attentionSeverity = stringField(attention, "severity").toLowerCase();
    const attentionPresentation = stringField(attention, "presentation").toLowerCase();
    const requestAvailable = booleanField(serviceRequest, "available");
    if (
      requestAvailable &&
      attentionSeverity === "warning" &&
      attentionPresentation === "client_decides"
    ) {
      return {
        status: "eligible",
        reason: stringField(attention, "message") || stringField(attention, "title") || "Access plan advertises a launchable service request with a warning.",
        source: "access-plan",
        serviceReason: stringField(attention, "reason"),
        launchAction,
      };
    }
    return {
      status: "needs-operator-action",
      reason: stringField(attention, "message") || stringField(attention, "reason") || "Access plan requires operator attention.",
      source: "access-plan",
      serviceReason: stringField(attention, "reason"),
      launchAction,
    };
  }
  if (booleanField(readinessSummary, "manualSeedingRequired") || booleanField(serviceRequest, "blockedByManualAction")) {
    return {
      status: "needs-operator-action",
      reason: stringField(serviceRequest, "reason") || "Access plan says manual seeding is required before launch.",
      source: "access-plan",
      serviceReason: stringField(serviceRequest, "reason"),
      launchAction,
    };
  }
  if (requestedAction && requestedAction !== launchAction) {
    return {
      status: "blocked",
      reason: `Access plan service request action is ${requestedAction}, not ${launchAction}.`,
      source: "access-plan",
      serviceReason: requestedAction,
      launchAction,
    };
  }
  return {
    status: "eligible",
    reason: stringField(decision, "recommendedAction") || "Access plan advertises a launchable service request.",
    source: "access-plan",
    launchAction,
  };
}

function accessPlanRemoteViewReadinessCheck(accessPlan: LauncherAccessPlanPreview): {
  status: LauncherEligibilityStatus;
  reason: string;
  source: LauncherEligibilityRow["reasonSource"];
  serviceReason?: string | null;
} | null {
  const readiness = accessPlanRemoteViewReadiness(accessPlan);
  if (!readiness) return null;
  for (const component of compactWorkspaceViewportReadinessComponents(readiness)) {
    const status = stringField(component, "status").toLowerCase();
    if (!status || status === "ready" || status === "ok" || status === "passed") continue;
    const failed = REMOTE_VIEW_FAILED_READINESS_STATES.has(status);
    if (!failed && !REMOTE_VIEW_ACTION_READINESS_STATES.has(status)) continue;
    const componentName = stringField(component, "component") || "remote view";
    const reason = stringField(component, "recovery") ||
      stringField(component, "message") ||
      stringField(component, "nextAction") ||
      stringField(component, "evidence") ||
      `${componentName} readiness is ${status}.`;
    return {
      status: failed ? "blocked" : "needs-operator-action",
      reason: `${componentName.replaceAll("_", " ")}: ${reason}`,
      source: "readiness",
      serviceReason: status,
    };
  }
  return null;
}

function accessPlanRemoteViewReadiness(accessPlan: LauncherAccessPlanPreview): unknown {
  const decision = recordField(accessPlan, "decision");
  const launchPosture = recordField(decision, "launchPosture");
  const readinessSummary = recordField(accessPlan, "readinessSummary");
  const serviceRequest = recordField(decision, "serviceRequest");
  const request = recordField(serviceRequest, "request");
  const params = recordField(request, "params");
  return unknownField(accessPlan, "remoteViewReadiness") ||
    unknownField(accessPlan, "viewStreamReadiness") ||
    unknownField(readinessSummary, "remoteViewReadiness") ||
    unknownField(readinessSummary, "viewStreamReadiness") ||
    unknownField(decision, "remoteViewReadiness") ||
    unknownField(launchPosture, "remoteViewReadiness") ||
    unknownField(params, "remoteViewReadiness");
}

function launcherAccessPlanRequiresManualSeeding(
  accessPlan: LauncherAccessPlanPreview,
  decision: Record<string, unknown>,
  serviceRequest: Record<string, unknown>,
): boolean {
  const readinessSummary = recordField(accessPlan, "readinessSummary");
  const request = recordField(serviceRequest, "request");
  return booleanField(decision, "manualSeedingRequired") ||
    booleanField(serviceRequest, "blockedByManualAction") ||
    booleanField(serviceRequest, "manualSeedingRequired") ||
    booleanField(request, "blockedByManualAction") ||
    booleanField(request, "manualSeedingRequired") ||
    Boolean(readinessSummary && booleanField(readinessSummary, "manualSeedingRequired"));
}

function launcherAccessPlanRequiresCdpFree(accessPlan: LauncherAccessPlanPreview): boolean {
  const decision = recordField(accessPlan, "decision");
  const serviceRequest = recordField(decision, "serviceRequest");
  const request = recordField(serviceRequest, "request");
  const launchPosture = recordField(decision, "launchPosture");
  return (
    stringField(request, "action") === "cdp_free_launch" ||
    booleanField(serviceRequest, "requiresCdpFree") ||
    booleanField(request, "requiresCdpFree") ||
    booleanField(launchPosture, "requiresCdpFree")
  ) && (
    booleanField(serviceRequest, "cdpAttachmentAllowed") !== true &&
    booleanField(request, "cdpAttachmentAllowed") !== true &&
    booleanField(launchPosture, "cdpAttachmentAllowed") !== true
  );
}

function selectPrimaryCheck(checks: Array<{
  status: LauncherEligibilityStatus;
  reason: string;
  source: LauncherEligibilityRow["reasonSource"];
  serviceReason?: string | null;
}>): typeof checks[number] | null {
  return checks.find((check) => check.status === "blocked") ??
    checks.find((check) => check.status === "needs-operator-action") ??
    checks.find((check) => check.status === "eligible") ??
    null;
}

function findAccessPlan(
  plans: LauncherAccessPlanPreview[],
  comboId: string,
  profileId: string,
  browserBuild: string,
): LauncherAccessPlanPreview | undefined {
  return plans.find((plan) => plan.comboId === comboId) ??
    plans.find((plan) =>
      (plan.profileId === profileId || stringField(plan.selectedProfile, "id") === profileId) &&
      (!plan.browserBuild || plan.browserBuild === browserBuild)
    );
}

function normalizedProfileId(profile: LauncherProfileRecord, fallback = ""): string {
  return profile.id?.trim() || fallback;
}

function profileDisplayName(value: string): string {
  if (!value.includes("/")) return value;
  const parts = value.split("/").filter(Boolean);
  return parts.slice(-2).join("/") || value;
}

function groupBrowsersByProfile(browsers: LauncherBrowserRecord[]): Map<string, LauncherBrowserRecord[]> {
  const grouped = new Map<string, LauncherBrowserRecord[]>();
  for (const browser of browsers) {
    const profileId = browser.profileId?.trim();
    if (!profileId) continue;
    const rows = grouped.get(profileId) ?? [];
    rows.push(browser);
    grouped.set(profileId, rows);
  }
  return grouped;
}

function dedupeBrowserOptions(options: BrowserOption[]): BrowserOption[] {
  const seen = new Set<string>();
  const result = [];
  for (const option of options) {
    const key = [option.browserBuild, option.browserHostId, option.browserHost, option.executableId, option.browserId].join("\0");
    if (seen.has(key)) continue;
    seen.add(key);
    result.push(option);
  }
  return result;
}

function remoteViewFromCapability(
  host: Record<string, unknown> | undefined,
  capability: Record<string, unknown> | undefined,
): "controllable" | "view-only" | "unavailable" {
  if (!booleanField(host, "remoteViewSupport") && !booleanField(capability, "streamingSupported")) {
    return "unavailable";
  }
  return booleanField(capability, "streamingSupported") ? "controllable" : "view-only";
}

function remoteViewFromStreams(streams?: LauncherBrowserRecord["viewStreams"]): "controllable" | "view-only" | "unavailable" {
  if (!streams?.length) return "unavailable";
  return streams.some((stream) => stream.url && stream.readOnly !== true && stream.controlInput)
    ? "controllable"
    : "view-only";
}

function statusSortValue(status: LauncherEligibilityStatus): number {
  if (status === "eligible") return 0;
  if (status === "needs-operator-action") return 1;
  return 2;
}

function uniqueStrings(values: Array<string | null | undefined>): string[] {
  return Array.from(new Set(values.map((value) => value?.trim()).filter(Boolean) as string[])).sort((left, right) =>
    left.localeCompare(right),
  );
}

function recordField(source: unknown, key: string): Record<string, unknown> | undefined {
  if (!source || typeof source !== "object") return undefined;
  const value = (source as Record<string, unknown>)[key];
  return value && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : undefined;
}

function unknownField(source: unknown, key: string): unknown {
  return source && typeof source === "object"
    ? (source as Record<string, unknown>)[key]
    : undefined;
}

function requiredRecord(source: unknown, key: string, label: string): Record<string, unknown> {
  const value = recordField(source, key);
  if (!value) throw new Error(`Missing ${label}.`);
  return value;
}

function plainRecordOrEmpty(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value)
    ? { ...(value as Record<string, unknown>) }
    : {};
}

function stringField(source: unknown, key: string): string {
  if (!source || typeof source !== "object") return "";
  const value = (source as Record<string, unknown>)[key];
  return typeof value === "string" ? value.trim() : "";
}

function booleanField(source: unknown, key: string): boolean {
  if (!source || typeof source !== "object") return false;
  return (source as Record<string, unknown>)[key] === true;
}
