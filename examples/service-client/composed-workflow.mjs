#!/usr/bin/env node
// @ts-check

import {
  attachServiceTabCdp,
  captureServiceNetwork,
  getServiceTabDiagnostics,
  probeServiceTab,
  requestServiceCdpDetach,
  requestServiceTabFromAccessPlan,
  requireServiceTabHandle,
  runServiceUiAction,
  transferServiceFiles,
} from '@agent-browser/client/service-request';
import { getServiceAccessPlan } from '@agent-browser/client/service-observability';

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
 *   accountId?: string;
 *   tabParams?: Record<string, unknown>;
 *   timeoutMs?: number;
 *   maxReturnBytes?: number;
 *   maxTextBytes?: number;
 *   maxBodyBytes?: number;
 *   uploadFile?: string;
 *   uploadAllowedPath?: string;
 *   downloadDir?: string;
 *   downloadAllowedDir?: string;
 *   includeDiagnosticsScreenshot?: boolean;
 *   fetch?: typeof globalThis.fetch;
 *   dryRun?: boolean;
 * }} ComposedWorkflowOptions
 */

/**
 * @param {ComposedWorkflowOptions} options
 */
export function buildComposedWorkflowPlan({
  url = DEFAULT_URL,
  serviceName = 'ExampleComposedWorkflow',
  agentName = 'example-composed-agent',
  taskName = 'brokerFirstComposedWorkflow',
  loginId = 'example',
  targetServiceId = loginId,
  accountId = 'example-account',
  tabParams = {},
  timeoutMs = 3000,
  maxReturnBytes = 512,
  maxTextBytes = 256,
  maxBodyBytes = 128,
  uploadFile = '/tmp/agent-browser-upload.txt',
  uploadAllowedPath = '/tmp',
  downloadDir = '/tmp/agent-browser-downloads',
  downloadAllowedDir = '/tmp',
  includeDiagnosticsScreenshot = false,
} = {}) {
  const caller = { serviceName, agentName, taskName, loginId, targetServiceId, accountId };
  return {
    ...caller,
    url,
    tabParams,
    sequence: [
      'read no-launch access plan',
      'request service-owned tab from access plan',
      'attach through policy-gated descriptor',
      'run provider-neutral probe recipe',
      'run provider-neutral UI action recipe',
      'capture capped network evidence',
      'transfer service-owned upload and download files',
      'collect compact diagnostics',
      'detach without closing the browser process',
    ],
    probe: {
      timeoutMs,
      maxReturnBytes,
      recipe: {
        recipeId: 'generic-composed-probe',
        expectedIdentity: accountId,
        detectors: [
          { id: 'page', type: 'url_title' },
          { id: 'account-text', type: 'selector_text', selector: '[data-account]', maxTextBytes },
          { id: 'identity-object', type: 'evaluate', expression: 'window.__composedIdentity' },
        ],
      },
    },
    uiAction: {
      timeoutMs,
      maxTextBytes,
      recipe: {
        recipeId: 'generic-composed-ui',
        maxActions: 4,
        steps: [
          { id: 'find-main', type: 'find', selector: 'main', maxCandidates: 1 },
          { id: 'fill-query', type: 'fill', selector: '#query', value: 'service text' },
          { id: 'click-apply', type: 'click', selector: '#apply' },
          { id: 'wait-applied', type: 'wait', text: 'Applied service text' },
        ],
      },
    },
    networkCapture: {
      timeoutMs,
      maxBodyBytes,
      recipe: {
        recipeId: 'generic-composed-network',
        urlPatterns: ['/api/data'],
        methods: ['GET'],
        status: '2xx',
        maxEvents: 1,
        captureBodies: true,
        maxBodyBytes,
        trigger: { type: 'reload' },
      },
    },
    fileTransfer: {
      timeoutMs,
      recipe: {
        recipeId: 'generic-composed-files',
        upload: {
          labelText: 'Upload report',
          files: [uploadFile],
          allowedPaths: [uploadAllowedPath],
          maxFiles: 1,
          verifySelectedNames: true,
        },
        download: {
          selector: '#download',
          directory: downloadDir,
          allowedDirectories: [downloadAllowedDir],
          expectedFileName: 'composed-download.txt',
          maxBytes: 1024,
        },
      },
    },
    diagnostics: {
      includeScreenshot: includeDiagnosticsScreenshot,
      maxConsoleEntries: 10,
      maxErrorEntries: 10,
      maxRequestEntries: 10,
    },
  };
}

/**
 * @param {ComposedWorkflowOptions} options
 */
export async function runComposedWorkflow({
  baseUrl,
  url = DEFAULT_URL,
  serviceName = 'ExampleComposedWorkflow',
  agentName = 'example-composed-agent',
  taskName = 'brokerFirstComposedWorkflow',
  loginId = 'example',
  targetServiceId = loginId,
  accountId = 'example-account',
  tabParams = {},
  timeoutMs = 3000,
  maxReturnBytes = 512,
  maxTextBytes = 256,
  maxBodyBytes = 128,
  uploadFile = '/tmp/agent-browser-upload.txt',
  uploadAllowedPath = '/tmp',
  downloadDir = '/tmp/agent-browser-downloads',
  downloadAllowedDir = '/tmp',
  includeDiagnosticsScreenshot = false,
  fetch = globalThis.fetch,
  dryRun = false,
} = {}) {
  const plan = buildComposedWorkflowPlan({
    url,
    serviceName,
    agentName,
    taskName,
    loginId,
    targetServiceId,
    accountId,
    tabParams,
    timeoutMs,
    maxReturnBytes,
    maxTextBytes,
    maxBodyBytes,
    uploadFile,
    uploadAllowedPath,
    downloadDir,
    downloadAllowedDir,
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

  let serviceTabHandle = null;
  let detach = null;
  let tab = null;
  let attach = null;
  let probe = null;
  let uiAction = null;
  let networkCapture = null;
  let fileTransfer = null;
  let diagnostics = null;
  const caller = { serviceName, agentName, taskName, loginId, targetServiceId, accountId };
  const accessPlan = await requireServiceSuccess(
    getServiceAccessPlan({
      baseUrl,
      fetch,
      serviceName,
      agentName,
      taskName,
      loginId,
      targetServiceId,
      accountId,
      url,
    }),
    'access plan',
  );

  try {
    tab = await requireServiceSuccess(
      requestServiceTabFromAccessPlan({
        baseUrl,
        fetch,
        accessPlan,
        ...caller,
        url,
        params: tabParams,
        jobTimeoutMs: 30000,
      }),
      'service tab',
    );
    serviceTabHandle = requireServiceTabHandle(tab);
    const cdpAttachmentAllowed =
      accessPlan?.decision?.serviceRequest?.cdpAttachmentAllowed === true ||
      accessPlan?.decision?.launchPosture?.cdpAttachmentAllowed === true ||
      accessPlan?.decision?.cdpAttachmentAllowed === true;

    attach = await requireServiceSuccess(
      attachServiceTabCdp({
        baseUrl,
        fetch,
        serviceName,
        agentName,
        taskName,
        serviceTabHandle,
        cdpAttachmentAllowed,
      }),
      'cdp attach',
    );
    probe = await requireServiceSuccess(
      probeServiceTab({
        baseUrl,
        fetch,
        ...caller,
        serviceTabHandle,
        timeoutMs,
        maxReturnBytes,
        probe: plan.probe.recipe,
      }),
      'probe',
    );
    uiAction = await requireServiceSuccess(
      runServiceUiAction({
        baseUrl,
        fetch,
        serviceName,
        agentName,
        taskName,
        serviceTabHandle,
        timeoutMs,
        maxTextBytes,
        uiAction: plan.uiAction.recipe,
      }),
      'ui action',
    );
    networkCapture = await requireServiceSuccess(
      captureServiceNetwork({
        baseUrl,
        fetch,
        serviceName,
        agentName,
        taskName,
        serviceTabHandle,
        timeoutMs,
        maxBodyBytes,
        networkCapture: plan.networkCapture.recipe,
      }),
      'network capture',
    );
    fileTransfer = await requireServiceSuccess(
      transferServiceFiles({
        baseUrl,
        fetch,
        serviceName,
        agentName,
        taskName,
        serviceTabHandle,
        timeoutMs,
        fileTransfer: plan.fileTransfer.recipe,
      }),
      'file transfer',
    );
    diagnostics = await requireServiceSuccess(
      getServiceTabDiagnostics({
        baseUrl,
        fetch,
        serviceName,
        agentName,
        taskName,
        serviceTabHandle,
        ...plan.diagnostics,
      }),
      'diagnostics',
    );
  } finally {
    if (serviceTabHandle) {
      detach = await requireServiceSuccess(
        requestServiceCdpDetach({
          baseUrl,
          fetch,
          serviceName,
          agentName,
          taskName,
          serviceTabHandle,
        }),
        'cdp detach',
      );
    }
  }

  return {
    dryRun: false,
    plan,
    accessPlan,
    tab,
    serviceTabHandle,
    attach,
    probe,
    uiAction,
    networkCapture,
    fileTransfer,
    diagnostics,
    detach,
  };
}

/**
 * @template T
 * @param {Promise<T>} promise
 * @param {string} label
 * @returns {Promise<T extends { data?: infer D } ? D : T>}
 */
async function requireServiceSuccess(promise, label) {
  const response = await promise;
  if (response && typeof response === 'object') {
    const record = /** @type {Record<string, unknown>} */ (response);
    if (record.success === false) {
      throw new Error(`${label} failed: ${record.error ?? JSON.stringify(record)}`);
    }
    if (record.success === true && 'data' in record) {
      return /** @type {any} */ (record.data);
    }
  }
  return /** @type {any} */ (response);
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
    else if (arg === '--account-id') options.accountId = next();
    else if (arg === '--timeout-ms') options.timeoutMs = Number(next());
    else if (arg === '--max-return-bytes') options.maxReturnBytes = Number(next());
    else if (arg === '--max-text-bytes') options.maxTextBytes = Number(next());
    else if (arg === '--max-body-bytes') options.maxBodyBytes = Number(next());
    else if (arg === '--upload-file') options.uploadFile = next();
    else if (arg === '--upload-allowed-path') options.uploadAllowedPath = next();
    else if (arg === '--download-dir') options.downloadDir = next();
    else if (arg === '--download-allowed-dir') options.downloadAllowedDir = next();
    else if (arg === '--headless') options.tabParams = { ...(options.tabParams ?? {}), headless: true };
    else if (arg === '--wait-until') options.tabParams = { ...(options.tabParams ?? {}), waitUntil: next() };
    else if (arg === '--include-diagnostics-screenshot') options.includeDiagnosticsScreenshot = true;
  }
  return options;
}

if (import.meta.url === `file://${nodeProcess.argv[1]}`) {
  runComposedWorkflow(parseArgs(nodeProcess.argv.slice(2)))
    .then((result) => {
      console.log(JSON.stringify(result, null, 2));
    })
    .catch((error) => {
      console.error(error instanceof Error ? error.message : String(error));
      nodeProcess.exit(1);
    });
}
