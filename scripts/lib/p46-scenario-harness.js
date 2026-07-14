export const P46_SCENARIOS = {
  s0: {
    id: 's0',
    title: 'Baseline Doctor And Reset Readiness',
    roles: [
      { id: 'runtime', type: 'runtime', routeLeases: 0 },
      { id: 'route-pool', type: 'route-pool', routeLeases: 0 },
    ],
    reset: { before: 'optional', after: 'optional' },
    artifactsPrefix: 'agent-browser-p46-s0',
    invariants: [
      'runtime baseline is observable',
      'route pool has two ready entries',
      'active incidents are zero',
    ],
  },
  s1: {
    id: 's1',
    title: 'One UX User, One Profile, One Browser, One Tab',
    roles: [
      { id: 'target-browser', type: 'target-browser', routeLeases: 1 },
      { id: 'operator', type: 'operator', routeLeases: 0 },
    ],
    reset: { before: 'required', after: 'required' },
    artifactsPrefix: 'agent-browser-p46-s1',
    invariants: [
      'one route-bound target browser is opened',
      'browser controls navigate and create a tab',
      'route display visual proof is captured',
    ],
  },
  s2: {
    id: 's2',
    title: 'Two UX Users Viewing The Same Route-Bound Browser',
    roles: [
      { id: 'target-browser', type: 'target-browser', routeLeases: 1 },
      { id: 'operator-a', type: 'viewer-client', routeLeases: 0 },
      { id: 'operator-b', type: 'viewer-client', routeLeases: 0 },
    ],
    reset: { before: 'required', after: 'required' },
    lockRule: { consecutiveFailures: 2, action: 'return_to_chat_planning' },
    artifactsPrefix: 'agent-browser-p46-s2',
    invariants: [
      'one target-browser route lease is used',
      'two viewer clients consume zero route leases',
      'both viewer clients point at the same route-bound target browser',
      'failure audit is required before retry',
    ],
  },
  s3: {
    id: 's3',
    title: 'Default Profile, Multiple Operators, Different Tabs',
    roles: [
      { id: 'target-browser', type: 'target-browser', routeLeases: 1 },
      { id: 'operator-a', type: 'viewer-client', routeLeases: 0 },
      { id: 'operator-b', type: 'viewer-client', routeLeases: 0 },
    ],
    reset: { before: 'required', after: 'required' },
    lockRule: { consecutiveFailures: 2, action: 'return_to_chat_planning' },
    artifactsPrefix: 'agent-browser-p46-s3',
    invariants: [
      'the default runtime profile can host multiple distinct tabs',
      'two viewer clients can target different tab IDs without consuming route leases',
      'tab selection and navigation affect the intended tab only',
      'failure audit is required before retry',
    ],
  },
  's3-open': {
    id: 's3-open',
    title: 'S3 Route-Bound Open Visible-Window Proof',
    roles: [
      { id: 'target-browser', type: 'target-browser', routeLeases: 1 },
    ],
    reset: { before: 'required', after: 'required' },
    lockRule: { consecutiveFailures: 2, action: 'return_to_chat_planning' },
    artifactsPrefix: 'agent-browser-p50-s3-open',
    invariants: [
      'the explicit agent-browser command is recorded before live work',
      'route-bound remote-view open reaches operatorVisible ready',
      'visible-window proof reaches browser_window_visible before dashboard tab stress',
      'failure audit is required before retry',
    ],
  },
  s4: {
    id: 's4',
    title: 'One Profile, Multiple Operators, Different Browser Windows',
    roles: [
      { id: 'window-a', type: 'target-browser', routeLeases: 1 },
      { id: 'window-b', type: 'target-browser', routeLeases: 0 },
      { id: 'operator-a', type: 'viewer-client', routeLeases: 0 },
      { id: 'operator-b', type: 'viewer-client', routeLeases: 0 },
    ],
    reset: { before: 'required', after: 'required' },
    lockRule: { consecutiveFailures: 2, action: 'return_to_chat_planning' },
    artifactsPrefix: 'agent-browser-p46-s4',
    invariants: [
      'one runtime profile is intentionally requested by two browser windows',
      'same-profile windows share one retained browser process and one route lease',
      'two viewer clients target different browser window IDs without consuming route leases',
      'closing one browser window does not corrupt the other window',
      'failure audit is required before retry',
    ],
  },
  s5: {
    id: 's5',
    title: 'Two Profiles Concurrently In Use',
    roles: [
      { id: 'profile-a-browser', type: 'target-browser', routeLeases: 1 },
      { id: 'profile-b-browser', type: 'target-browser', routeLeases: 1 },
      { id: 'operator-a', type: 'viewer-client', routeLeases: 0 },
      { id: 'operator-b', type: 'viewer-client', routeLeases: 0 },
    ],
    reset: { before: 'required', after: 'required' },
    lockRule: { consecutiveFailures: 2, action: 'return_to_chat_planning' },
    artifactsPrefix: 'agent-browser-p46-s5',
    invariants: [
      'two distinct runtime profiles are concurrently route-bound',
      'profile A and profile B consume distinct route leases',
      'two viewer clients target different profile browsers without consuming route leases',
      'closing profile A does not corrupt profile B',
      'failure audit is required before retry',
    ],
  },
  s6: {
    id: 's6',
    title: 'Two UX Users, Two Profiles, Cross-Observation',
    roles: [
      { id: 'profile-a-browser', type: 'target-browser', routeLeases: 1 },
      { id: 'profile-b-browser', type: 'target-browser', routeLeases: 1 },
      { id: 'operator-a', type: 'viewer-client', routeLeases: 0 },
      { id: 'operator-b', type: 'viewer-client', routeLeases: 0 },
    ],
    reset: { before: 'required', after: 'required' },
    lockRule: { consecutiveFailures: 2, action: 'return_to_chat_planning' },
    artifactsPrefix: 'agent-browser-p46-s6',
    invariants: [
      'two distinct runtime profiles are concurrently route-bound',
      'two viewer clients can swap dashboard selection between profiles',
      'swapped viewer clients refresh the selected profile without route leases',
      'selection changes do not mutate the wrong browser',
      'failure audit is required before retry',
    ],
  },
  s7: {
    id: 's7',
    title: 'Route Pool Exhaustion And Queued Demand',
    roles: [
      { id: 'profile-a-browser', type: 'target-browser', routeLeases: 1 },
      { id: 'profile-b-browser', type: 'target-browser', routeLeases: 1 },
      { id: 'profile-c-demand', type: 'target-browser', routeLeases: 1 },
    ],
    reset: { before: 'required', after: 'required' },
    lockRule: { consecutiveFailures: 2, action: 'return_to_chat_planning' },
    artifactsPrefix: 'agent-browser-p46-s7',
    invariants: [
      'two healthy route-bound browsers occupy all route-pool capacity',
      'a third route-bound browser request fails closed with an explicit capacity blocker',
      'failed demand does not create a fake live dashboard row or terminal fallback',
      'after one route is released, one retry succeeds or returns a new explicit blocker',
      'failure audit is required before retry',
    ],
  },
  s8: {
    id: 's8',
    title: 'Permission And Display Recovery',
    roles: [
      { id: 'target-browser', type: 'target-browser', routeLeases: 1 },
      { id: 'display-access-fixture', type: 'runtime', routeLeases: 0 },
    ],
    reset: { before: 'required', after: 'required' },
    lockRule: { consecutiveFailures: 2, action: 'return_to_chat_planning' },
    artifactsPrefix: 'agent-browser-p46-s8',
    invariants: [
      'a controlled display-access denial fails before browser launch',
      'the failure reports an explicit display-access blocker',
      'failed display access leaves no fake retained browser row or terminal fallback',
      'the same route-bound open succeeds after restoring normal display access',
      'failure audit is required before retry',
    ],
  },
  s9: {
    id: 's9',
    title: 'Stale Target And Duplicate Tab Stress',
    roles: [
      { id: 'target-browser', type: 'target-browser', routeLeases: 1 },
      { id: 'operator-a', type: 'viewer-client', routeLeases: 0 },
      { id: 'operator-b', type: 'viewer-client', routeLeases: 0 },
      { id: 'operator-c', type: 'viewer-client', routeLeases: 0 },
    ],
    reset: { before: 'required', after: 'required' },
    lockRule: { consecutiveFailures: 2, action: 'return_to_chat_planning' },
    artifactsPrefix: 'agent-browser-p46-s9',
    invariants: [
      'one route-bound browser hosts blank and duplicate same-origin tabs',
      'dashboard selections for duplicate tabs retain distinct live tab IDs',
      'blank or stale metadata cannot satisfy final selected-target readiness',
      'duplicate tabs remain independently navigable after tab switching',
      'failure audit is required before retry',
    ],
  },
  s10: {
    id: 's10',
    title: 'Foreign CDP Inventory Beside Service-Owned RDP Browsers',
    roles: [
      { id: 'service-owned-browser', type: 'target-browser', routeLeases: 1 },
      { id: 'foreign-cdp-browser', type: 'foreign-cdp-browser', routeLeases: 0 },
      { id: 'operator', type: 'viewer-client', routeLeases: 0 },
    ],
    reset: { before: 'required', after: 'required' },
    lockRule: { consecutiveFailures: 2, action: 'return_to_chat_planning' },
    artifactsPrefix: 'agent-browser-p46-s10',
    invariants: [
      'one service-owned route-bound browser remains fully controllable',
      'one reachable foreign CDP browser is inventoried as non-owned',
      'foreign CDP actions are read-only or explicitly adoption-gated',
      'foreign CDP inventory does not borrow service route or display state',
      'switching between rows preserves selected workspace context',
      'failure audit is required before retry',
    ],
  },
  s11: {
    id: 's11',
    title: 'Dashboard Refresh, Stale URL, And Reconnect Stress',
    roles: [
      { id: 'target-browser', type: 'target-browser', routeLeases: 1 },
      { id: 'operator', type: 'viewer-client', routeLeases: 0 },
    ],
    reset: { before: 'required', after: 'required' },
    lockRule: { consecutiveFailures: 2, action: 'return_to_chat_planning' },
    artifactsPrefix: 'agent-browser-p46-s11',
    invariants: [
      'one route-bound browser survives dashboard reload',
      'stale dashboard tab URLs recover to a live selected target',
      'viewer-client reconnect and viewport refresh controls remain functional',
      'direct Guacamole frame URL remains reachable for the same route',
      'route display visual state remains browser-window-visible',
      'failure audit is required before retry',
    ],
  },
  s12: {
    id: 's12',
    title: 'Repeated Normal-Use Drift And Reset Soak',
    roles: [
      { id: 'target-browser', type: 'target-browser', routeLeases: 1 },
      { id: 'operator', type: 'viewer-client', routeLeases: 0 },
      { id: 'route-pool', type: 'route-pool', routeLeases: 0 },
    ],
    reset: { before: 'required', after: 'required' },
    lockRule: { consecutiveFailures: 2, action: 'return_to_chat_planning' },
    artifactsPrefix: 'agent-browser-p46-s12',
    invariants: [
      'at least ten normal-use cycles open, navigate, switch tabs, refresh, close, and reset',
      'each cycle includes dashboard reload and viewer-client reconnect proof',
      'doctor and incident summaries are captured at every cycle boundary',
      'route-pool state returns to baseline after every reset',
      'memory, process, and profile lease pressure do not trend upward after resets',
      'failure audit is required before retry',
    ],
  },
};

export function supportedScenarioIds() {
  return Object.keys(P46_SCENARIOS);
}

export function scenarioSpec(id) {
  return P46_SCENARIOS[String(id || '').toLowerCase()] ?? null;
}

export function validateScenarioSpec(spec) {
  const failures = [];
  if (!spec?.id) failures.push('scenario id is missing');
  const roles = Array.isArray(spec?.roles) ? spec.roles : [];
  if (roles.length === 0) failures.push('scenario roles are missing');
  const targetBrowsers = roles.filter((role) => role.type === 'target-browser');
  const viewerClients = roles.filter((role) => role.type === 'viewer-client');
  for (const role of roles) {
    if (!role.id) failures.push('scenario role id is missing');
    if (!['runtime', 'route-pool', 'target-browser', 'operator', 'viewer-client', 'foreign-cdp-browser'].includes(role.type)) {
      failures.push(`scenario role ${role.id || '<missing>'} has unsupported type ${role.type || '<missing>'}`);
    }
    if (!Number.isInteger(role.routeLeases) || role.routeLeases < 0) {
      failures.push(`scenario role ${role.id || '<missing>'} has invalid route lease count`);
    }
    if (role.type === 'viewer-client' && role.routeLeases !== 0) {
      failures.push(`viewer-client role ${role.id} must consume zero route leases`);
    }
  }
  if (spec?.id === 's2') {
    if (targetBrowsers.length !== 1) failures.push('S2 must define exactly one target-browser role');
    if (viewerClients.length !== 2) failures.push('S2 must define exactly two viewer-client roles');
    const targetRouteLeases = targetBrowsers.reduce((total, role) => total + role.routeLeases, 0);
    if (targetRouteLeases !== 1) failures.push('S2 target-browser roles must consume exactly one route lease');
  }
  if (spec?.id === 's3') {
    if (targetBrowsers.length !== 1) failures.push('S3 must define exactly one target-browser role');
    if (viewerClients.length !== 2) failures.push('S3 must define exactly two viewer-client roles');
    const targetRouteLeases = targetBrowsers.reduce((total, role) => total + role.routeLeases, 0);
    if (targetRouteLeases !== 1) failures.push('S3 target-browser roles must consume exactly one route lease');
  }
  if (spec?.id === 's3-open') {
    if (targetBrowsers.length !== 1) failures.push('S3-open must define exactly one target-browser role');
    if (viewerClients.length !== 0) failures.push('S3-open must not define viewer-client roles');
    const targetRouteLeases = targetBrowsers.reduce((total, role) => total + role.routeLeases, 0);
    if (targetRouteLeases !== 1) failures.push('S3-open target-browser roles must consume exactly one route lease');
  }
  if (spec?.id === 's4') {
    if (targetBrowsers.length !== 2) failures.push('S4 must define exactly two target-browser roles');
    if (viewerClients.length !== 2) failures.push('S4 must define exactly two viewer-client roles');
    const targetRouteLeases = targetBrowsers.reduce((total, role) => total + role.routeLeases, 0);
    if (targetRouteLeases !== 1) failures.push('S4 target-browser roles must consume exactly one route lease');
  }
  if (spec?.id === 's5') {
    if (targetBrowsers.length !== 2) failures.push('S5 must define exactly two target-browser roles');
    if (viewerClients.length !== 2) failures.push('S5 must define exactly two viewer-client roles');
    const targetRouteLeases = targetBrowsers.reduce((total, role) => total + role.routeLeases, 0);
    if (targetRouteLeases !== 2) failures.push('S5 target-browser roles must consume exactly two route leases');
  }
  if (spec?.id === 's6') {
    if (targetBrowsers.length !== 2) failures.push('S6 must define exactly two target-browser roles');
    if (viewerClients.length !== 2) failures.push('S6 must define exactly two viewer-client roles');
    const targetRouteLeases = targetBrowsers.reduce((total, role) => total + role.routeLeases, 0);
    if (targetRouteLeases !== 2) failures.push('S6 target-browser roles must consume exactly two route leases');
  }
  if (spec?.id === 's7') {
    if (targetBrowsers.length !== 3) failures.push('S7 must define exactly three target-browser roles');
    if (viewerClients.length !== 0) failures.push('S7 must not define viewer-client roles');
    const targetRouteLeases = targetBrowsers.reduce((total, role) => total + role.routeLeases, 0);
    if (targetRouteLeases !== 3) failures.push('S7 target-browser roles must demand three route leases');
  }
  if (spec?.id === 's8') {
    if (targetBrowsers.length !== 1) failures.push('S8 must define exactly one target-browser role');
    if (viewerClients.length !== 0) failures.push('S8 must not define viewer-client roles');
    const targetRouteLeases = targetBrowsers.reduce((total, role) => total + role.routeLeases, 0);
    if (targetRouteLeases !== 1) failures.push('S8 target-browser roles must consume exactly one route lease');
    if (!roles.some((role) => role.id === 'display-access-fixture' && role.type === 'runtime')) {
      failures.push('S8 must define a display-access-fixture runtime role');
    }
  }
  if (spec?.id === 's9') {
    if (targetBrowsers.length !== 1) failures.push('S9 must define exactly one target-browser role');
    if (viewerClients.length !== 3) failures.push('S9 must define exactly three viewer-client roles');
    const targetRouteLeases = targetBrowsers.reduce((total, role) => total + role.routeLeases, 0);
    const viewerRouteLeases = viewerClients.reduce((total, role) => total + role.routeLeases, 0);
    if (targetRouteLeases !== 1) failures.push('S9 target-browser roles must consume exactly one route lease');
    if (viewerRouteLeases !== 0) failures.push('S9 viewer-client roles must consume zero route leases');
  }
  if (spec?.id === 's10') {
    const foreignBrowsers = roles.filter((role) => role.type === 'foreign-cdp-browser');
    if (targetBrowsers.length !== 1) failures.push('S10 must define exactly one target-browser role');
    if (foreignBrowsers.length !== 1) failures.push('S10 must define exactly one foreign-cdp-browser role');
    if (viewerClients.length !== 1) failures.push('S10 must define exactly one viewer-client role');
    const targetRouteLeases = targetBrowsers.reduce((total, role) => total + role.routeLeases, 0);
    const foreignRouteLeases = foreignBrowsers.reduce((total, role) => total + role.routeLeases, 0);
    const viewerRouteLeases = viewerClients.reduce((total, role) => total + role.routeLeases, 0);
    if (targetRouteLeases !== 1) failures.push('S10 target-browser roles must consume exactly one route lease');
    if (foreignRouteLeases !== 0) failures.push('S10 foreign CDP browser roles must consume zero route leases');
    if (viewerRouteLeases !== 0) failures.push('S10 viewer-client roles must consume zero route leases');
  }
  if (spec?.id === 's11') {
    if (targetBrowsers.length !== 1) failures.push('S11 must define exactly one target-browser role');
    if (viewerClients.length !== 1) failures.push('S11 must define exactly one viewer-client role');
    const targetRouteLeases = targetBrowsers.reduce((total, role) => total + role.routeLeases, 0);
    const viewerRouteLeases = viewerClients.reduce((total, role) => total + role.routeLeases, 0);
    if (targetRouteLeases !== 1) failures.push('S11 target-browser roles must consume exactly one route lease');
    if (viewerRouteLeases !== 0) failures.push('S11 viewer-client roles must consume zero route leases');
  }
  if (spec?.id === 's12') {
    if (targetBrowsers.length !== 1) failures.push('S12 must define exactly one target-browser role');
    if (viewerClients.length !== 1) failures.push('S12 must define exactly one viewer-client role');
    if (!roles.some((role) => role.id === 'route-pool' && role.type === 'route-pool')) {
      failures.push('S12 must define a route-pool runtime boundary role');
    }
    const targetRouteLeases = targetBrowsers.reduce((total, role) => total + role.routeLeases, 0);
    const viewerRouteLeases = viewerClients.reduce((total, role) => total + role.routeLeases, 0);
    if (targetRouteLeases !== 1) failures.push('S12 target-browser roles must consume exactly one route lease per cycle');
    if (viewerRouteLeases !== 0) failures.push('S12 viewer-client roles must consume zero route leases');
  }
  return {
    ok: failures.length === 0,
    failures,
  };
}

export function classifyScenarioFailure(failures) {
  const text = failures.join('\n').toLowerCase();
  if (
    text.includes('finalization') ||
    text.includes('remote_view_finalization_incomplete') ||
    text.includes('remote_view_open_acquisition') ||
    text.includes('display_allocation_unavailable')
  ) {
    return 'route_bound_finalization';
  }
  if (
    text.includes('same_profile_multi_process_unsupported') ||
    text.includes('duplicate profile lane') ||
    text.includes('profile lock')
  ) {
    return 'profile_topology';
  }
  if (text.includes('viewer') || text.includes('dashboard')) return 'viewer_client_adapter';
  if (text.includes('route') || text.includes('display') || text.includes('guacamole')) return 'route_display_runtime';
  if (text.includes('incident') || text.includes('service status')) return 'service_state';
  if (text.includes('reset')) return 'reset_protocol';
  return 'unclassified_requires_audit';
}

export function routeBoundFinalizationEvidence({ openJson, statusJson, incidentsJson }) {
  const openData = openJson?.data || {};
  const serviceState = statusJson?.data?.service_state || {};
  const routeId = openData.routeId || openData.routeBinding?.routeId || null;
  const displayAllocationId = openData.displayAllocationId || openData.routeBinding?.displayAllocationId || null;
  const routePoolEntryId = openData.routePoolEntryId || openData.routeBinding?.routePoolEntryId || null;
  const browserId = openData.browserId || null;
  const leaseId = openData.acquisitionLease?.id || null;
  const lease = leaseId ? serviceState.remoteViewAcquisitionLeases?.[leaseId] : null;
  const route = routeId ? serviceState.remoteViewRoutes?.[routeId] : null;
  const displayAllocation = displayAllocationId ? serviceState.displayAllocations?.[displayAllocationId] : null;
  const routePoolEntry = routePoolEntryId ? serviceState.routePool?.[routePoolEntryId] : null;
  const browser = browserId ? serviceState.browsers?.[browserId] : null;
  const incidentRows = incidentsJson?.data?.incidents || serviceState.incidents || [];
  const incidentList = Array.isArray(incidentRows) ? incidentRows : Object.values(incidentRows);
  const relatedIncidents = incidentList.filter((incident) => {
    const id = String(incident?.id || '');
    return [routeId, routePoolEntryId, displayAllocationId, browserId]
      .filter(Boolean)
      .some((value) => id.includes(value) || String(incident?.latestMessage || '').includes(value));
  });
  const routeBindsBrowser = Boolean(route?.browserId === browserId && route?.displayAllocationId === displayAllocationId);
  const displayBindsRoute = Boolean(
    Array.isArray(displayAllocation?.routeIds) &&
    (!routeId || displayAllocation.routeIds.includes(routeId))
  );
  const browserHasRouteStream = Boolean(browser?.viewStreams?.some((stream) => {
    const streamDisplayMatches = stream?.displayAllocationId === displayAllocationId;
    const streamRouteMatches = stream?.routeId
      ? stream.routeId === routeId
      : routeBindsBrowser && displayBindsRoute;
    const streamReady = stream?.remoteReadiness?.state === 'ready' ||
      stream?.readiness?.state === 'ready' ||
      (route?.readiness?.state === 'ready' && displayAllocation?.readiness?.state === 'ready');
    return streamDisplayMatches && streamRouteMatches && streamReady;
  }));
  const finalized = Boolean(
    lease?.state === 'completed' &&
    lease?.phase === 'checked_out' &&
    routePoolEntry?.state === 'checked_out' &&
    routePoolEntry?.currentRouteAllocationId === routeId &&
    displayAllocation?.state === 'ready' &&
    route?.state === 'ready' &&
    browser?.health === 'ready' &&
    browser?.displayAllocationId === displayAllocationId &&
    browserHasRouteStream
  );
  const blockers = [];
  if (leaseId && !(lease?.state === 'completed' && lease?.phase === 'checked_out')) {
    blockers.push(`lease ${leaseId || 'missing'} is ${lease?.state || 'missing'}:${lease?.phase || 'missing'}`);
  }
  if (routePoolEntryId && !(routePoolEntry?.state === 'checked_out' && routePoolEntry?.currentRouteAllocationId === routeId)) {
    blockers.push(`route-pool ${routePoolEntryId} is ${routePoolEntry?.state || 'missing'} with allocation ${routePoolEntry?.currentRouteAllocationId || 'missing'}`);
  }
  if (displayAllocationId && displayAllocation?.state !== 'ready') {
    blockers.push(`display allocation ${displayAllocationId} is ${displayAllocation?.state || 'missing'}`);
  }
  if (routeId && route?.state !== 'ready') {
    blockers.push(`route ${routeId} is ${route?.state || 'missing'}`);
  }
  if (browserId && !(browser?.health === 'ready' && browser?.displayAllocationId === displayAllocationId && browserHasRouteStream)) {
    blockers.push(`browser ${browserId} finalization stream is incomplete`);
  }
  for (const incident of relatedIncidents) {
    if (incident?.latestKind === 'remote_view_finalization_incomplete') {
      blockers.push(`incident ${incident.id} reports remote_view_finalization_incomplete`);
    }
  }
  return {
    finalized,
    blockers,
    ids: {
      browserId,
      displayAllocationId,
      leaseId,
      routeId,
      routePoolEntryId,
    },
    states: {
      browser: browser ? {
        displayAllocationId: browser.displayAllocationId || null,
        health: browser.health || null,
        routeBindingSource: browser.viewStreams?.some((stream) => stream?.routeId === routeId)
          ? 'view_stream_route_id'
          : routeBindsBrowser && displayBindsRoute
            ? 'route_display_binding'
            : null,
        routeStreamReady: browserHasRouteStream,
      } : null,
      displayAllocation: displayAllocation ? {
        readinessState: displayAllocation.readiness?.state || null,
        state: displayAllocation.state || null,
      } : null,
      lease: lease ? {
        phase: lease.phase || null,
        state: lease.state || null,
      } : null,
      route: route ? {
        readinessState: route.readiness?.state || null,
        state: route.state || null,
      } : null,
      routePoolEntry: routePoolEntry ? {
        currentRouteAllocationId: routePoolEntry.currentRouteAllocationId || null,
        readinessState: routePoolEntry.readiness?.state || null,
        state: routePoolEntry.state || null,
      } : null,
    },
    relatedIncidents: relatedIncidents.map((incident) => ({
      id: incident.id,
      latestKind: incident.latestKind,
      latestMessage: incident.latestMessage,
      state: incident.state,
    })),
  };
}
