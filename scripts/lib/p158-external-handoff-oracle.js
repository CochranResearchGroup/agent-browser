import { createHash } from 'node:crypto';
import { isIP } from 'node:net';

export const EXTERNAL_URL_ROLES = Object.freeze([
  'starting_handoff',
  'location_header',
  'iframe_src',
  'form_action',
  'websocket_endpoint',
  'reconnect_target',
  'copied_action',
  'error_action',
  'provider_external_url',
  'route_binding',
  'local_embed_url',
  'dashboard_embed_url',
  'health_url',
]);

export const EXTERNAL_HANDOFF_FINDING_CODES = Object.freeze([
  'invalid_handoff_url',
  'non_https_url',
  'loopback_url_leak',
  'private_network_url_leak',
  'link_local_url_leak',
  'local_domain_url_leak',
  'raw_guacamole_url_leak',
  'forbidden_role_url_leak',
  'external_vantage_unproven',
  'dns_failure',
  'tls_failure',
  'redirect_failure',
  'cookie_failure',
  'iframe_failure',
  'form_action_failure',
  'websocket_failure',
  'reconnect_failure',
  'operator_not_ready',
  'pixels_before_ready',
  'visible_identity_mismatch',
  'handoff_changed',
  'duplicate_cold_launch',
  'capture_gap',
]);

export const RETAINED_IDENTITY_FIELDS = Object.freeze([
  'browserId',
  'profileId',
  'sessionId',
  'tabId',
  'targetId',
  'visibleUrl',
  'pageMarker',
  'pixelHash',
]);

const FORBIDDEN_URL_ROLES = new Set([
  'provider_external_url',
  'route_binding',
  'local_embed_url',
  'dashboard_embed_url',
  'health_url',
]);

const CHECK_FINDINGS = Object.freeze({
  dns: 'dns_failure',
  tls: 'tls_failure',
  redirect: 'redirect_failure',
  cookies: 'cookie_failure',
  cookie: 'cookie_failure',
  iframe: 'iframe_failure',
  form_action: 'form_action_failure',
  form: 'form_action_failure',
  websocket: 'websocket_failure',
  reconnect: 'reconnect_failure',
});

export const REQUIRED_INGRESS_CHECKS = Object.freeze([
  'dns',
  'tls',
  'redirect',
  'cookie',
  'websocket',
  'iframe',
  'form_action',
  'reconnect',
]);

function clone(value) {
  return structuredClone(value);
}

function canonicalize(value, seen = new Set()) {
  if (value === null || typeof value === 'string' || typeof value === 'boolean') return value;
  if (typeof value === 'number') return Number.isFinite(value) ? value : String(value);
  if (typeof value === 'bigint') return value.toString();
  if (value instanceof Uint8Array) return { $bytes: Buffer.from(value).toString('base64') };
  if (typeof value !== 'object') return String(value);
  if (seen.has(value)) return '[circular]';
  seen.add(value);
  const result = Array.isArray(value)
    ? value.map((entry) => canonicalize(entry, seen))
    : Object.fromEntries(
        Object.keys(value)
          .sort()
          .filter((key) => value[key] !== undefined)
          .map((key) => [key, canonicalize(value[key], seen)]),
      );
  seen.delete(value);
  return result;
}

export function stableHandoffHash(value) {
  return createHash('sha256').update(JSON.stringify(canonicalize(value))).digest('hex');
}

function normalizeHostname(hostname) {
  return hostname.replace(/^\[|\]$/g, '').replace(/\.$/, '').toLowerCase();
}

function parseIpv4(hostname) {
  if (isIP(hostname) !== 4) return null;
  return hostname.split('.').map(Number);
}

export function classifyHost(hostname) {
  const host = normalizeHostname(hostname);
  if (host === 'localhost' || host.endsWith('.localhost')) return 'localhost_literal';
  if (host.endsWith('.local')) return 'local_domain';
  const ipv4 = parseIpv4(host);
  if (ipv4) {
    if (ipv4[0] === 127 || ipv4.every((octet) => octet === 0)) return 'ipv4_loopback';
    if (
      ipv4[0] === 10 ||
      (ipv4[0] === 172 && ipv4[1] >= 16 && ipv4[1] <= 31) ||
      (ipv4[0] === 192 && ipv4[1] === 168)
    ) return 'rfc1918';
    if (ipv4[0] === 169 && ipv4[1] === 254) return 'link_local';
    return 'public';
  }
  if (isIP(host) === 6) {
    if (host === '::1' || host === '::') return 'ipv6_loopback';
    if (/^f[cd][0-9a-f]{2}:/i.test(host)) return 'rfc1918';
    if (/^fe[89ab][0-9a-f]:/i.test(host)) return 'link_local';
    const mapped = host.match(/^::ffff:(\d+\.\d+\.\d+\.\d+)$/i);
    return mapped ? classifyHost(mapped[1]) : 'public';
  }
  return 'public';
}

function isRawGuacamole(parsed) {
  return (
    /(^|\/)guacamole(\/|$)/i.test(parsed.pathname) ||
    /(^|\/)client\/[a-z0-9_-]+/i.test(parsed.pathname) ||
    /#\/?client\//i.test(parsed.hash) ||
    parsed.searchParams.has('guac-data')
  );
}

function parseUrl(value, baseUrl) {
  try {
    return new URL(value, baseUrl);
  } catch {
    return null;
  }
}

export function classifyOperatorUrl(url, { role = 'location_header', baseUrl, resolvedAddresses = [] } = {}) {
  const parsed = parseUrl(url, baseUrl);
  if (!parsed) {
    return {
      valid: false,
      normalizedUrl: null,
      role,
      hostClass: 'invalid',
      resolvedHostClasses: [],
      findingCodes: role === 'starting_handoff' ? ['invalid_handoff_url'] : ['non_https_url'],
    };
  }
  const hostClass = isRawGuacamole(parsed) ? 'raw_guacamole' : classifyHost(parsed.hostname);
  const resolvedHostClasses = [...new Set(resolvedAddresses.map(classifyHost))].sort();
  const findingCodes = [];
  const allowedProtocols = role === 'websocket_endpoint' ? ['wss:'] : ['https:'];
  if (!allowedProtocols.includes(parsed.protocol)) findingCodes.push('non_https_url');
  const allHostClasses = new Set([hostClass, ...resolvedHostClasses]);
  if (allHostClasses.has('localhost_literal') || allHostClasses.has('ipv4_loopback') || allHostClasses.has('ipv6_loopback')) {
    findingCodes.push('loopback_url_leak');
  }
  if (allHostClasses.has('rfc1918')) findingCodes.push('private_network_url_leak');
  if (allHostClasses.has('link_local')) findingCodes.push('link_local_url_leak');
  if (allHostClasses.has('local_domain')) findingCodes.push('local_domain_url_leak');
  if (allHostClasses.has('raw_guacamole')) findingCodes.push('raw_guacamole_url_leak');
  if (FORBIDDEN_URL_ROLES.has(role)) findingCodes.push('forbidden_role_url_leak');
  if (role === 'starting_handoff') {
    const segments = parsed.pathname.split('/').filter(Boolean);
    const validPath =
      segments.length === 2 &&
      segments[0] === 'remote-view' &&
      segments[1].length > 0 &&
      !parsed.username &&
      !parsed.password;
    if (!validPath) findingCodes.push('invalid_handoff_url');
  }
  const uniqueCodes = [...new Set(findingCodes)].sort();
  return {
    valid: uniqueCodes.length === 0,
    normalizedUrl: parsed.href,
    role,
    hostClass,
    resolvedHostClasses,
    findingCodes: uniqueCodes,
  };
}

function observedAt(value) {
  const raw = value?.observedAt ?? value?.timestamp ?? value?.at;
  const parsed = typeof raw === 'number' ? raw : Date.parse(raw);
  return Number.isFinite(parsed) ? parsed : null;
}

function addFinding(findings, finding) {
  const normalized = {
    code: finding.code,
    severity: finding.severity ?? 'blocking',
    observationIds: [...new Set(finding.observationIds ?? [])].sort(),
    checkIds: [...new Set(finding.checkIds ?? [])].sort(),
    reconnectIds: [...new Set(finding.reconnectIds ?? [])].sort(),
    field: finding.field ?? null,
    message: finding.message,
    expected: clone(finding.expected ?? null),
    observed: clone(finding.observed ?? null),
  };
  const key = `${normalized.code}\0${stableHandoffHash(normalized.observed)}`;
  if (!findings.some((entry) => entry.key === key)) findings.push({ key, ...normalized });
}

function findingMessage(code) {
  const messages = {
    invalid_handoff_url: 'The starting URL is not an authenticated opaque durable handoff.',
    non_https_url: 'A client-visible endpoint uses a non-secure scheme.',
    loopback_url_leak: 'A client-visible URL resolves to loopback.',
    private_network_url_leak: 'A client-visible URL resolves to an RFC 1918 or unique-local address.',
    link_local_url_leak: 'A client-visible URL resolves to a link-local address.',
    local_domain_url_leak: 'A client-visible URL uses a .local hostname.',
    raw_guacamole_url_leak: 'A raw Guacamole route was exposed to the external client.',
    forbidden_role_url_leak: 'A diagnostic-only URL role was exposed as an operator action.',
    external_vantage_unproven: 'The capture does not prove an off-host, external-network vantage.',
    dns_failure: 'Public DNS resolution did not succeed from the external client.',
    tls_failure: 'TLS validation did not succeed from the external client.',
    redirect_failure: 'The external redirect chain did not complete successfully.',
    cookie_failure: 'Authenticated cookie handling did not complete successfully.',
    iframe_failure: 'The external iframe did not load successfully.',
    form_action_failure: 'An external form action did not complete successfully.',
    websocket_failure: 'The external WebSocket upgrade or stream failed.',
    reconnect_failure: 'A planned durable-handoff reconnect failed.',
    operator_not_ready: 'operatorVisible did not reach ready before visibility was claimed.',
    pixels_before_ready: 'Usable pixels were recorded before operatorVisible reached ready.',
    visible_identity_mismatch: 'Visible pixels and retained browser identity do not agree.',
    handoff_changed: 'A reconnect did not use the original durable handoff.',
    duplicate_cold_launch: 'Reconnection created a duplicate or cold-launched browser.',
    capture_gap: 'Required external-client evidence is missing or partial.',
  };
  return messages[code];
}

function normalizeChecks(session) {
  if (Array.isArray(session.ingressChecks)) return session.ingressChecks;
  const checks = [];
  for (const [kind, value] of Object.entries(session.ingressChecks ?? session.networkChecks ?? {})) {
    checks.push(typeof value === 'object' ? { kind, ...value } : { kind, success: value === true });
  }
  return checks;
}

function normalizeUrlObservations(session) {
  const observations = [...(session.urlObservations ?? [])];
  if (session.initialHandoffUrl && !observations.some((entry) => entry.role === 'starting_handoff')) {
    observations.unshift({
      observationId: 'initial-handoff',
      role: 'starting_handoff',
      url: session.initialHandoffUrl,
      authenticated: session.authenticated ?? session.handoffAuthenticated,
    });
  }
  return observations;
}

function identityDifferences(expected, observed) {
  const differences = [];
  for (const field of RETAINED_IDENTITY_FIELDS) {
    if (expected?.[field] === undefined) continue;
    if (observed?.[field] !== expected[field]) {
      differences.push({ field, expected: expected[field], observed: observed?.[field] ?? null });
    }
  }
  return differences;
}

export function auditExternalHandoffSession({ session, options = {} }) {
  if (!session || typeof session !== 'object') throw new TypeError('session is required');
  const evidence = clone(session);
  const inputSha256 = stableHandoffHash(evidence);
  const findings = [];
  const urlResults = [];
  const urlObservations = normalizeUrlObservations(evidence);
  for (const observation of urlObservations) {
    const result = classifyOperatorUrl(observation.url, {
      role: observation.role,
      baseUrl: evidence.initialHandoffUrl,
      resolvedAddresses: observation.resolvedAddresses ?? [],
    });
    urlResults.push({
      observationId: observation.observationId ?? observation.id ?? null,
      ...result,
    });
    for (const code of result.findingCodes) {
      addFinding(findings, {
        code,
        observationIds: [observation.observationId ?? observation.id].filter(Boolean),
        field: observation.role,
        message: findingMessage(code),
        expected: { role: observation.role, publicSecureUrl: true },
        observed: { role: observation.role, url: observation.url, hostClass: result.hostClass },
      });
    }
  }

  const parsedInitial = parseUrl(evidence.initialHandoffUrl);
  const initialPathHandoffId = parsedInitial?.pathname.split('/').filter(Boolean)[1] ?? null;
  if (initialPathHandoffId !== evidence.initialHandoffId) {
    addFinding(findings, {
      code: 'invalid_handoff_url',
      observationIds: urlObservations
        .filter((observation) => observation.role === 'starting_handoff')
        .map((observation) => observation.observationId)
        .filter(Boolean),
      field: 'initialHandoffId',
      message: findingMessage('invalid_handoff_url'),
      expected: evidence.initialHandoffId,
      observed: initialPathHandoffId,
    });
  }

  const vantage = evidence.externalVantage ?? evidence.vantage ?? {};
  if (
    vantage.outsideServiceHost !== true ||
    vantage.outsideServiceNetworkNamespace !== true ||
    vantage.publicEgressObserved !== true
  ) {
    addFinding(findings, {
      code: 'external_vantage_unproven',
      field: 'externalVantage',
      message: findingMessage('external_vantage_unproven'),
      expected: {
        outsideServiceHost: true,
        outsideServiceNetworkNamespace: true,
        publicEgressObserved: true,
      },
      observed: vantage,
    });
  }

  const checks = normalizeChecks(evidence);
  for (const check of checks) {
    const kind = String(check.kind ?? check.check ?? check.type ?? '').toLowerCase();
    const code = CHECK_FINDINGS[kind];
    const checkId = check.checkId ?? check.observationId ?? check.id;
    const targetClassification = classifyOperatorUrl(check.targetUrl, {
      role: kind === 'websocket' ? 'websocket_endpoint' : 'location_header',
      baseUrl: evidence.initialHandoffUrl,
      resolvedAddresses: check.resolvedAddresses ?? [],
    });
    for (const leakCode of targetClassification.findingCodes.filter((findingCode) =>
      findingCode.endsWith('_url_leak'),
    )) {
      addFinding(findings, {
        code: leakCode,
        checkIds: [checkId].filter(Boolean),
        field: 'targetUrl',
        message: findingMessage(leakCode),
        expected: { hostClass: 'public' },
        observed: {
          hostClass: targetClassification.hostClass,
          resolvedHostClasses: targetClassification.resolvedHostClasses,
        },
      });
    }
    if (code && check.success !== true && check.state !== 'passed') {
      addFinding(findings, {
        code,
        checkIds: [checkId].filter(Boolean),
        field: kind,
        message: findingMessage(code),
        expected: { state: 'passed' },
        observed: check,
      });
    }
  }
  for (const requiredCheck of options.requiredIngressChecks ?? REQUIRED_INGRESS_CHECKS) {
    if (!checks.some((check) => String(check.kind ?? check.check ?? check.type).toLowerCase() === requiredCheck)) {
      const code = CHECK_FINDINGS[requiredCheck] ?? 'capture_gap';
      addFinding(findings, {
        code,
        field: requiredCheck,
        message: findingMessage(code),
        expected: { check: requiredCheck, success: true },
        observed: null,
      });
    }
  }

  const visibility = Array.isArray(evidence.operatorVisibleObservations)
    ? evidence.operatorVisibleObservations
    : evidence.operatorVisibleState
      ? [{ state: evidence.operatorVisibleState, observedAt: evidence.readyObservedAt }]
      : [];
  const readyObservations = visibility.filter((entry) => entry.state === 'ready');
  const readyAt = readyObservations.map(observedAt).filter((value) => value !== null).sort((a, b) => a - b)[0] ?? null;
  if (readyObservations.length === 0) {
    addFinding(findings, {
      code: 'operator_not_ready',
      field: 'operatorVisibleState',
      message: findingMessage('operator_not_ready'),
      expected: { state: 'ready' },
      observed: visibility,
    });
  }
  const pixelObservations =
    evidence.pixelObservations ??
    evidence.pixels ??
    (evidence.firstUsablePixelsAt
      ? [{ observationId: 'first-usable-pixels', usable: true, observedAt: evidence.firstUsablePixelsAt }]
      : []);
  const usablePixels = pixelObservations.filter((entry) => entry.usable !== false);
  for (const pixels of usablePixels) {
    const pixelsAt = observedAt(pixels);
    if (readyAt === null || pixelsAt === null || pixelsAt < readyAt) {
      addFinding(findings, {
        code: 'pixels_before_ready',
        observationIds: [pixels.observationId ?? pixels.id].filter(Boolean),
        field: 'firstUsablePixelsAt',
        message: findingMessage('pixels_before_ready'),
        expected: { readyAtOrBefore: pixelsAt },
        observed: { readyAt, pixelsAt },
      });
    }
  }

  const expectedIdentity = evidence.expectedIdentity ?? evidence.initialIdentity ?? null;
  const identityObservations = [
    ...(evidence.visibleIdentity ? [{ observationId: 'visible-identity', identity: evidence.visibleIdentity }] : []),
    ...usablePixels.map((entry) => ({
      observationId: entry.observationId ?? entry.id,
      identity: entry.identity ?? entry.visibleIdentity,
    })),
  ];
  for (const observation of identityObservations) {
    if (!observation.identity) continue;
    const differences = identityDifferences(expectedIdentity, observation.identity);
    if (differences.length > 0) {
      addFinding(findings, {
        code: 'visible_identity_mismatch',
        observationIds: [observation.observationId].filter(Boolean),
        field: differences[0].field,
        message: findingMessage('visible_identity_mismatch'),
        expected: expectedIdentity,
        observed: { identity: observation.identity, differences },
      });
    }
  }

  const expectedHandoff = classifyOperatorUrl(evidence.initialHandoffUrl, {
    role: 'starting_handoff',
  }).normalizedUrl;
  const reconnectResults = [];
  for (const reconnect of evidence.reconnects ?? []) {
    const reconnectUrl = reconnect.handoffUrl ?? reconnect.url;
    const normalizedReconnect = classifyOperatorUrl(reconnectUrl, {
      role: 'reconnect_target',
      baseUrl: evidence.initialHandoffUrl,
      resolvedAddresses: reconnect.resolvedAddresses ?? [],
    });
    const differences = identityDifferences(expectedIdentity, reconnect.identity ?? reconnect.visibleIdentity);
    const handoffChanged =
      normalizedReconnect.normalizedUrl !== expectedHandoff ||
      (reconnect.handoffId !== undefined && reconnect.handoffId !== evidence.initialHandoffId);
    const duplicateColdLaunch =
      reconnect.coldLaunch === true ||
      reconnect.newBrowserCreated === true ||
      Number(
        reconnect.physicalBrowserLaunchCount ??
          reconnect.browserLaunchCount ??
          reconnect.newBrowserCount ??
          0,
      ) > 0;
    const reconnectPassed = reconnect.state === 'passed' || reconnect.success === true;
    const reconnectReadyAt = observedAt({ observedAt: reconnect.readyObservedAt });
    const reconnectPixelsAt = observedAt({ observedAt: reconnect.firstUsablePixelsAt });
    reconnectResults.push({
      reconnectId: reconnect.reconnectId ?? reconnect.id ?? null,
      trigger: reconnect.trigger ?? null,
      success: reconnectPassed,
      handoffChanged,
      duplicateColdLaunch,
      identityDifferences: differences,
    });
    if (!reconnectPassed) {
      addFinding(findings, {
        code: 'reconnect_failure',
        reconnectIds: [reconnect.reconnectId ?? reconnect.id].filter(Boolean),
        field: 'state',
        message: findingMessage('reconnect_failure'),
        expected: { success: true },
        observed: reconnect,
      });
    }
    if (handoffChanged) {
      addFinding(findings, {
        code: 'handoff_changed',
        reconnectIds: [reconnect.reconnectId ?? reconnect.id].filter(Boolean),
        field: 'handoffUrl',
        message: findingMessage('handoff_changed'),
        expected: { handoffId: evidence.initialHandoffId, handoffUrl: expectedHandoff },
        observed: { handoffId: reconnect.handoffId ?? null, handoffUrl: reconnectUrl ?? null },
      });
    }
    if (differences.length > 0) {
      addFinding(findings, {
        code: 'visible_identity_mismatch',
        reconnectIds: [reconnect.reconnectId ?? reconnect.id].filter(Boolean),
        field: differences[0].field,
        message: findingMessage('visible_identity_mismatch'),
        expected: expectedIdentity,
        observed: { identity: reconnect.identity ?? reconnect.visibleIdentity ?? null, differences },
      });
    }
    if (duplicateColdLaunch) {
      addFinding(findings, {
        code: 'duplicate_cold_launch',
        reconnectIds: [reconnect.reconnectId ?? reconnect.id].filter(Boolean),
        field: 'physicalBrowserLaunchCount',
        message: findingMessage('duplicate_cold_launch'),
        expected: { browserLaunchCount: 0, newBrowserCreated: false },
        observed: reconnect,
      });
    }
    if (reconnectPassed && reconnect.operatorVisibleState !== 'ready') {
      addFinding(findings, {
        code: 'operator_not_ready',
        reconnectIds: [reconnect.reconnectId ?? reconnect.id].filter(Boolean),
        field: 'operatorVisibleState',
        message: findingMessage('operator_not_ready'),
        expected: 'ready',
        observed: reconnect.operatorVisibleState,
      });
    }
    if (
      reconnectPassed &&
      reconnectPixelsAt !== null &&
      (reconnectReadyAt === null || reconnectPixelsAt < reconnectReadyAt)
    ) {
      addFinding(findings, {
        code: 'pixels_before_ready',
        reconnectIds: [reconnect.reconnectId ?? reconnect.id].filter(Boolean),
        field: 'firstUsablePixelsAt',
        message: findingMessage('pixels_before_ready'),
        expected: { readyAtOrBefore: reconnectPixelsAt },
        observed: { readyAt: reconnectReadyAt, pixelsAt: reconnectPixelsAt },
      });
    }
  }

  for (const gap of evidence.captureGaps ?? []) {
    addFinding(findings, {
      code: 'capture_gap',
      observationIds: [gap.gapId ?? gap.observationId ?? gap.id].filter(Boolean),
      field: gap.evidenceClass ?? null,
      message: findingMessage('capture_gap'),
      expected: { captureState: 'complete' },
      observed: gap,
    });
  }

  findings.sort(
    (left, right) =>
      left.code.localeCompare(right.code) ||
      stableHandoffHash(left.observed).localeCompare(stableHandoffHash(right.observed)),
  );
  const finalFindings = findings.map(({ key: _key, ...finding }, index) => ({
    findingId: `handoff-finding-${String(index + 1).padStart(6, '0')}`,
    ...finding,
    repairAttempted: false,
  }));
  const auditedAt =
    options.auditedAt ??
    [
      ...visibility.map(observedAt),
      ...usablePixels.map(observedAt),
      ...(evidence.reconnects ?? []).map(observedAt),
    ].filter((value) => value !== null).sort((a, b) => b - a)[0];
  return {
    schemaVersion: 'agent-browser.p158-external-handoff-oracle-report.v1',
    planId: 'P158',
    auditId: options.auditId ?? `p158-handoff-audit:${inputSha256.slice(0, 24)}`,
    fixtureId: evidence.fixtureId ?? evidence.sessionId ?? `p158-session:${inputSha256.slice(0, 16)}`,
    inputSha256,
    auditedAt:
      typeof auditedAt === 'number'
        ? new Date(auditedAt).toISOString()
        : auditedAt ?? '1970-01-01T00:00:00.000Z',
    repairAttempted: false,
    passed: finalFindings.length === 0,
    summary: {
      urlObservationCount: urlResults.length,
      ingressCheckCount: checks.length,
      reconnectCount: reconnectResults.length,
      findingCount: finalFindings.length,
      findingCounts: Object.fromEntries(
        EXTERNAL_HANDOFF_FINDING_CODES.map((code) => [
          code,
          finalFindings.filter((finding) => finding.code === code).length,
        ]),
      ),
    },
    urlClassifications: urlResults.map((result) => {
      const source = urlObservations.find(
        (observation) =>
          (observation.observationId ?? observation.id) === result.observationId,
      );
      const parsed = parseUrl(source?.url, evidence.initialHandoffUrl);
      return {
        observationId: result.observationId,
        role: result.role,
        url: source?.url ?? result.normalizedUrl ?? 'invalid:',
        scheme: parsed ? parsed.protocol.replace(/:$/, '') : null,
        hostClass: result.hostClass,
        operatorSafe: result.valid,
      };
    }),
    findings: finalFindings,
  };
}

export function auditExternalHandoff({ fixtureSet, options = {} }) {
  if (!fixtureSet || typeof fixtureSet !== 'object') throw new TypeError('fixtureSet is required');
  const sessions = Array.isArray(fixtureSet.fixtures)
    ? fixtureSet.fixtures
    : Array.isArray(fixtureSet.sessions)
      ? fixtureSet.sessions
      : [fixtureSet];
  return sessions.map((session) => auditExternalHandoffSession({
    session: {
      ...session,
      externalVantage: session.externalVantage ?? fixtureSet.externalVantage,
    },
    options,
  }));
}
