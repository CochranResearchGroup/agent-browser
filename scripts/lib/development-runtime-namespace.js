/** Resolve an optional bounded namespace without changing the legacy development identity. */
export function developmentRuntimeNamespace(env = process.env) {
  const namespace = env.AGENT_BROWSER_DEV_NAMESPACE;
  if (namespace === undefined) return { namespace: null, suffix: '', name: 'agent-browser-dev' };
  if (typeof namespace !== 'string' || !/^[a-z][a-z0-9]{0,7}$/.test(namespace)) {
    throw new Error('AGENT_BROWSER_DEV_NAMESPACE must contain 1 to 8 lowercase letters or digits and start with a letter');
  }
  return { namespace, suffix: `-${namespace}`, name: `agent-browser-dev-${namespace}` };
}

/** A parallel runtime requires all ports explicitly pinned and disjoint from both existing lanes. */
export function requireNamespacedDevelopmentPorts(env = process.env) {
  if (developmentRuntimeNamespace(env).namespace === null) return;
  const names = ['DASHBOARD', 'BACKEND', 'SHADOW', 'LANE_STREAM', 'GUACAMOLE', 'GUACD', 'POSTGRES'];
  const reserved = new Set([3389, 3390, 4822, 5432, 4848, 4849, 8092, 4948, 4949, 4950, 4951, 8093, 4823, 55433]);
  const selected = new Set();
  for (const name of names) {
    const key = `AGENT_BROWSER_DEV_${name}_PORT`;
    const value = env[key];
    const port = Number(value);
    if (!/^\d+$/.test(value ?? '') || !Number.isSafeInteger(port) || port < 1 || port > 65535 || reserved.has(port) || selected.has(port)) {
      throw new Error(`${key} must pin a unique port outside production and the default development lane`);
    }
    selected.add(port);
  }
}
