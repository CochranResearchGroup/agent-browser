/**
 * Normalize one list-shaped route inventory without imposing a compiled size.
 * Provider-specific fields remain opaque to the inventory owner.
 */
export function canonicalRouteInventory(entries) {
  if (!Array.isArray(entries)) {
    throw new Error('route_inventory_must_be_array');
  }
  const normalized = entries.map((entry, index) => {
    if (!entry || typeof entry !== 'object' || Array.isArray(entry)) {
      throw new Error(`route_inventory_entry_invalid:${index}`);
    }
    const routeId = text(entry.routeId) || (text(entry.connectionId) ? `guacamole:${text(entry.connectionId)}` : null);
    const id = text(entry.id) || opaqueInventoryId(routeId || text(entry.connectionId), index);
    if (!id) throw new Error(`route_inventory_id_missing:${index}`);
    return {
      ...entry,
      id,
      ...(routeId ? { routeId } : {}),
      target: entry.target && typeof entry.target === 'object' && !Array.isArray(entry.target)
        ? { ...entry.target }
        : {},
    };
  });
  assertUnique(normalized.map((entry) => entry.id), 'route_inventory_duplicate_id');
  const routeIdentities = normalized
    .map((entry) => text(entry.routeId) || text(entry.connectionId) || text(entry.frameUrl) || text(entry.externalUrl))
    .filter(Boolean);
  assertUnique(routeIdentities, 'route_inventory_duplicate_route_identity');
  return normalized;
}

/**
 * Compatibility adapter for the legacy two-route environment. New callers use
 * AGENT_BROWSER_RDP_ROUTE_POOL_JSON and never synthesize alphabetic entries.
 */
export function legacyTwoRouteInventory(environment = process.env) {
  return canonicalRouteInventory(
    legacyRouteEntries(environment, false)
      .filter((entry) => entry.routeId || entry.connectionId || entry.frameUrl || entry.externalUrl),
  );
}

export function legacyTwoRouteDisplaySubjects(environment = process.env) {
  return canonicalRouteInventory(legacyRouteEntries(environment, true));
}

function legacyRouteEntries(environment, includeDefaultUsers) {
  const route = (label, defaultUser) => {
    const prefix = `AGENT_BROWSER_RDP_ROUTE_${label}_`;
    const connectionId = text(environment[`${prefix}CONNECTION_ID`]);
    const routeId = text(environment[`${prefix}ID`]) || (connectionId ? `guacamole:${connectionId}` : null);
    const frameUrl = text(environment[`${prefix}FRAME_URL`]);
    const externalUrl = text(environment[`${prefix}EXTERNAL_URL`]);
    return {
      id: text(environment[`${prefix}POOL_ENTRY_ID`]) || `pool-${label.toLowerCase()}`,
      routeId,
      connectionId,
      connectionName: text(environment[`${prefix}CONNECTION_NAME`]),
      frameUrl,
      externalUrl,
      providerMode: text(environment[`${prefix}PROVIDER_MODE`]) || 'simultaneous_view',
      viewerSession: text(environment[`${prefix}VIEWER_SESSION`]),
      viewerProfile: text(environment[`${prefix}VIEWER_PROFILE`]),
      viewerExecutable: text(environment[`${prefix}VIEWER_EXECUTABLE`]),
      target: {
        displayName: text(environment[`${prefix}DISPLAY_NAME`]),
        routeUser: text(environment[`${prefix}USERNAME`]) || (includeDefaultUsers ? defaultUser : null),
      },
    };
  };
  return [
    route('A', 'agent-browser-rdp-a'),
    route('B', 'agent-browser-rdp-b'),
  ];
}

/** Select every managed static route deterministically. */
export function selectManagedRouteCandidates(connections) {
  const routeSpecific = connections.filter((connection) =>
    /^Agent Browser RDP Route (?:[AB]|\d+)$/i.test(text(connection.connectionName) || ''),
  );
  const existingUser = connections.filter((connection) =>
    /^Agent Browser RDP Existing User Route (?:[AB]|\d+)$/i.test(text(connection.connectionName) || ''),
  );
  const selected = routeSpecific.length > 0
    ? routeSpecific
    : existingUser.length > 0
      ? existingUser
      : connections;
  const collator = new Intl.Collator('en', { numeric: true, sensitivity: 'base' });
  return [...selected].sort((left, right) => {
    const leftOrdinal = routeOrdinal(text(left.connectionName));
    const rightOrdinal = routeOrdinal(text(right.connectionName));
    if (leftOrdinal !== rightOrdinal) return leftOrdinal - rightOrdinal;
    const byName = collator.compare(text(left.connectionName) || '', text(right.connectionName) || '');
    if (byName !== 0) return byName;
    return collator.compare(text(left.connectionId) || '', text(right.connectionId) || '');
  });
}

export function legacyRouteLabel(index) {
  return index === 0 ? 'A' : index === 1 ? 'B' : null;
}

function opaqueInventoryId(identity, index) {
  if (!identity) return `route-${index + 1}`;
  const normalized = identity.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
  return normalized ? `route-${normalized}` : `route-${index + 1}`;
}

function routeOrdinal(connectionName) {
  const suffix = connectionName?.match(/(?:Route)\s+([AB]|\d+)$/i)?.[1]?.toUpperCase();
  if (suffix === 'A') return 1;
  if (suffix === 'B') return 2;
  const numeric = Number.parseInt(suffix || '', 10);
  return Number.isFinite(numeric) ? numeric : Number.MAX_SAFE_INTEGER;
}

function assertUnique(values, code) {
  if (new Set(values).size !== values.length) throw new Error(code);
}

function text(value) {
  if (typeof value !== 'string') return null;
  const trimmed = value.trim();
  return trimmed || null;
}
