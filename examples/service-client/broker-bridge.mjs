#!/usr/bin/env node
// @ts-check

import {
  attachServiceTabCdp,
  evaluateServiceTab,
  getServiceTabDiagnostics,
  requestServiceCdpDetach,
  requestServiceTabFromAccessPlan,
  requireServiceTabHandle,
} from '@agent-browser/client/service-request';
import {
  getServiceAccessPlan,
  registerExternalProfile,
} from '@agent-browser/client/service-observability';

const DEFAULT_URL = 'https://example.com/';
const nodeProcess = /** @type {{ argv: string[], env: Record<string, string | undefined>, exit(code?: number): never }} */ (
  /** @type {any} */ (globalThis).process
);

/**
 * @typedef {{
 *   baseUrl?: string;
 *   url?: string;
 *   serviceName?: string;
 *   agentName?: string;
 *   taskName?: string;
 *   loginId?: string;
 *   targetServiceId?: string;
 *   registerExternalProfileId?: string;
 *   externalProfileUserDataDir?: string;
 *   evaluateExpression?: string;
 *   timeoutMs?: number;
 *   maxReturnBytes?: number;
 *   includeDiagnosticsScreenshot?: boolean;
 *   fetch?: typeof globalThis.fetch;
 *   dryRun?: boolean;
 * }} BrokerBridgeOptions
 */

/**
 * @param {BrokerBridgeOptions} options
 */
export function buildBrokerBridgePlan({
  url = DEFAULT_URL,
  serviceName = 'ExampleBridge',
  agentName = 'example-bridge-agent',
  taskName = 'brokerFirstBridge',
  loginId = 'example',
  targetServiceId = loginId,
  registerExternalProfileId,
  externalProfileUserDataDir,
  evaluateExpression = 'document.title',
  timeoutMs = 1000,
  maxReturnBytes = 256,
  includeDiagnosticsScreenshot = false,
} = {}) {
  return {
    serviceName,
    agentName,
    taskName,
    loginId,
    targetServiceId,
    url,
    optionalExternalProfileRegistration:
      registerExternalProfileId && externalProfileUserDataDir
        ? {
            helper: 'registerExternalProfile',
            id: registerExternalProfileId,
            profileOrigin: 'external_byop',
            userDataDir: externalProfileUserDataDir,
          }
        : null,
    bridgeSequence: [
      'read the no-launch access plan',
      'request a service-owned tab from the access plan',
      'extract the lease-backed service tab handle',
      'attach through the policy-gated CDP descriptor',
      'run bounded evaluate against the handle',
      'collect compact diagnostics for the handle',
      'detach without closing the browser process',
    ],
    evaluate: {
      expression: evaluateExpression,
      timeoutMs,
      maxReturnBytes,
    },
    diagnostics: {
      includeScreenshot: includeDiagnosticsScreenshot,
    },
  };
}

/**
 * @param {BrokerBridgeOptions} options
 */
export async function runBrokerBridgeWorkflow({
  baseUrl,
  url = DEFAULT_URL,
  serviceName = 'ExampleBridge',
  agentName = 'example-bridge-agent',
  taskName = 'brokerFirstBridge',
  loginId = 'example',
  targetServiceId = loginId,
  registerExternalProfileId,
  externalProfileUserDataDir,
  evaluateExpression = 'document.title',
  timeoutMs = 1000,
  maxReturnBytes = 256,
  includeDiagnosticsScreenshot = false,
  fetch = globalThis.fetch,
  dryRun = false,
} = {}) {
  const plan = buildBrokerBridgePlan({
    url,
    serviceName,
    agentName,
    taskName,
    loginId,
    targetServiceId,
    registerExternalProfileId,
    externalProfileUserDataDir,
    evaluateExpression,
    timeoutMs,
    maxReturnBytes,
    includeDiagnosticsScreenshot,
  });

  if (dryRun) {
    return {
      dryRun: true,
      plan,
      note: 'Dry run does not contact agent-browser or launch a browser.',
    };
  }
  if (!baseUrl) {
    throw new Error('Missing baseUrl. Pass --base-url http://127.0.0.1:<stream-port>.');
  }

  const externalProfileRegistration =
    registerExternalProfileId && externalProfileUserDataDir
      ? await registerExternalProfile({
          baseUrl,
          fetch,
          id: registerExternalProfileId,
          serviceName,
          agentName,
          targetServiceId,
          loginId,
          userDataDir: externalProfileUserDataDir,
          profileOrigin: 'external_byop',
        })
      : null;

  const accessPlan = await getServiceAccessPlan({
    baseUrl,
    fetch,
    serviceName,
    agentName,
    taskName,
    loginId,
    targetServiceId,
    url,
  });
  const tab = await requestServiceTabFromAccessPlan({
    baseUrl,
    fetch,
    accessPlan,
    serviceName,
    agentName,
    taskName,
    loginId,
    targetServiceId,
    url,
    jobTimeoutMs: 30000,
  });
  const serviceTabHandle = requireServiceTabHandle(tab);
  const cdpAttachmentAllowed = accessPlan?.decision?.cdpAttachmentAllowed === true;
  const attach = await attachServiceTabCdp({
    baseUrl,
    fetch,
    serviceName,
    agentName,
    taskName,
    serviceTabHandle,
    cdpAttachmentAllowed,
  });
  const evaluate = await evaluateServiceTab({
    baseUrl,
    fetch,
    serviceName,
    agentName,
    taskName,
    serviceTabHandle,
    expression: evaluateExpression,
    timeoutMs,
    maxReturnBytes,
  });
  const diagnostics = await getServiceTabDiagnostics({
    baseUrl,
    fetch,
    serviceName,
    agentName,
    taskName,
    serviceTabHandle,
    includeScreenshot: includeDiagnosticsScreenshot,
    maxConsoleEntries: 10,
    maxErrorEntries: 10,
    maxRequestEntries: 10,
  });
  const detach = await requestServiceCdpDetach({
    baseUrl,
    fetch,
    serviceName,
    agentName,
    taskName,
    serviceTabHandle,
  });

  return {
    dryRun: false,
    plan,
    externalProfileRegistration,
    accessPlan,
    tab,
    serviceTabHandle,
    attach,
    evaluate,
    diagnostics,
    detach,
  };
}

function parseArgs(argv) {
  const options = {};
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = () => argv[++index];
    if (arg === '--dry-run') options.dryRun = true;
    else if (arg === '--base-url') options.baseUrl = next();
    else if (arg === '--url') options.url = next();
    else if (arg === '--service-name') options.serviceName = next();
    else if (arg === '--agent-name') options.agentName = next();
    else if (arg === '--task-name') options.taskName = next();
    else if (arg === '--login-id') options.loginId = next();
    else if (arg === '--target-service-id') options.targetServiceId = next();
    else if (arg === '--register-external-profile-id') options.registerExternalProfileId = next();
    else if (arg === '--external-profile-user-data-dir') options.externalProfileUserDataDir = next();
    else if (arg === '--expression') options.evaluateExpression = next();
    else if (arg === '--timeout-ms') options.timeoutMs = Number(next());
    else if (arg === '--max-return-bytes') options.maxReturnBytes = Number(next());
    else if (arg === '--include-diagnostics-screenshot') options.includeDiagnosticsScreenshot = true;
  }
  return options;
}

if (import.meta.url === `file://${nodeProcess.argv[1]}`) {
  runBrokerBridgeWorkflow(parseArgs(nodeProcess.argv.slice(2)))
    .then((result) => {
      console.log(JSON.stringify(result, null, 2));
    })
    .catch((error) => {
      console.error(error instanceof Error ? error.message : String(error));
      nodeProcess.exit(1);
    });
}
