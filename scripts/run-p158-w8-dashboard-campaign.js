#!/usr/bin/env node

import { spawn } from 'node:child_process';
import { mkdir, open, readFile, rename, writeFile } from 'node:fs/promises';
import { dirname, isAbsolute, join, resolve } from 'node:path';

import {
  aggregateP158DashboardCampaignReceipts,
  buildP158DashboardCampaignPlan,
  executeP158DashboardCampaignAction,
  prepareP158DashboardCampaign,
  resolveP158DashboardActionResume,
  sha256Bytes,
  validateP158ExternalPlaywrightRunner,
} from './lib/p158-w8-dashboard-campaign.js';

const argv = process.argv.slice(2).filter((argument) => argument !== '--');
const command = argv.shift();

function option(name, required = true) {
  const index = argv.indexOf(name);
  if (index === -1) {
    if (required) throw new Error(`${name} is required`);
    return null;
  }
  const value = argv[index + 1];
  if (!value || value.startsWith('--')) throw new Error(`${name} requires a value`);
  argv.splice(index, 2);
  return value;
}

function flag(name) {
  const index = argv.indexOf(name);
  if (index === -1) return false;
  argv.splice(index, 1);
  return true;
}

async function jsonFile(path, label) {
  if (!isAbsolute(path)) throw new Error(`${label} path must be absolute`);
  return JSON.parse(await readFile(path, 'utf8'));
}

async function writeNewJson(path, value) {
  if (!isAbsolute(path)) throw new Error('Output path must be absolute');
  await mkdir(dirname(path), { recursive: true, mode: 0o700 });
  const handle = await open(path, 'wx', 0o600);
  try {
    await handle.writeFile(`${JSON.stringify(value, null, 2)}\n`);
  } finally {
    await handle.close();
  }
}

function parseJson(text, label) {
  try {
    return JSON.parse(text.trim());
  } catch (error) {
    throw new Error(`${label} did not return JSON: ${error.message}`);
  }
}

function runProcess(executable, args, { env, input = '' } = {}) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(executable, args, {
      env,
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => { stdout += chunk; });
    child.stderr.on('data', (chunk) => { stderr += chunk; });
    child.on('error', reject);
    child.on('exit', (code, signal) => {
      if (code === 0) resolvePromise({ stdout, stderr });
      else reject(new Error(`${executable} failed with code=${code} signal=${signal}: ${stderr}`));
    });
    child.stdin.end(input);
  });
}

async function assertExecutableDigest(candidate) {
  const bytes = await readFile(candidate.executablePath);
  if (sha256Bytes(bytes) !== candidate.executableSha256) {
    throw new Error('Frozen candidate executable digest changed');
  }
}

function isolatedEnvironment(root) {
  return {
    PATH: process.env.PATH ?? '/usr/bin:/bin',
    LANG: process.env.LANG ?? 'C.UTF-8',
    NO_COLOR: '1',
    ...root.environment,
  };
}

async function validateInstalledState(request) {
  await assertExecutableDigest(request.candidate);
  await mkdir(dirname(request.validationInputPath), { recursive: true, mode: 0o700 });
  const handle = await open(request.validationInputPath, 'wx', 0o600);
  try {
    await handle.writeFile(request.stateBytes);
  } finally {
    await handle.close();
  }
  const result = await runProcess(request.candidate.executablePath, [
    'service', 'state', 'validate', '--path', request.validationInputPath, '--json',
  ], { env: {
    PATH: process.env.PATH ?? '/usr/bin:/bin',
    LANG: process.env.LANG ?? 'C.UTF-8',
    NO_COLOR: '1',
    ...request.environment,
  } });
  return parseJson(result.stdout, 'installed Service State validator');
}

async function selectorReceipt(selector, request) {
  if (!isAbsolute(selector?.executablePath ?? '') || !/^[a-f0-9]{64}$/.test(selector?.executableSha256 ?? '')) {
    throw new Error('Execution input requires an absolute reviewed development ingress selector and SHA-256');
  }
  const bytes = await readFile(selector.executablePath);
  if (sha256Bytes(bytes) !== selector.executableSha256) throw new Error('Development ingress selector digest changed');
  const result = await runProcess(selector.executablePath, [], {
    env: { PATH: process.env.PATH ?? '/usr/bin:/bin', LANG: process.env.LANG ?? 'C.UTF-8' },
    input: `${JSON.stringify({
      schemaVersion: 'agent-browser.p158-development-ingress-selection-request.v1',
      ...request,
    })}\n`,
  });
  return parseJson(result.stdout, 'development ingress selector');
}

function createLiveEffects(executionInput) {
  return {
    startExact: async (root) => {
      await assertExecutableDigest(root.candidate);
      await Promise.all(Object.entries(root.environment)
        .filter(([key]) => key === 'HOME' || key.startsWith('XDG_') || key === 'AGENT_BROWSER_SOCKET_DIR')
        .map(([, path]) => mkdir(path, { recursive: true, mode: 0o700 })));
      const result = await runProcess(root.candidate.executablePath, [
        '--json', 'dashboard', 'start', '--port', String(root.ports.dashboard),
      ], { env: isolatedEnvironment(root) });
      const parsed = parseJson(result.stdout, 'dashboard start');
      const pid = parsed?.data?.pid;
      if (parsed?.success !== true || !Number.isInteger(pid)) throw new Error('Dashboard start omitted its exact PID');
      return {
        state: 'ready',
        pid,
        candidateSha256: root.candidate.executableSha256,
        statePath: root.target.statePath,
      };
    },
    selectExternalIngress: (request) => selectorReceipt(executionInput.ingressSelector, request),
    openExternalPage: async ({ publicUrl }) => {
      const endpoint = process.env.AGENT_BROWSER_P158_EXTERNAL_PLAYWRIGHT_WS;
      if (!endpoint) throw new Error('AGENT_BROWSER_P158_EXTERNAL_PLAYWRIGHT_WS is required for off-host capture');
      validateP158ExternalPlaywrightRunner({
        endpoint,
        attestation: executionInput.externalRunnerAttestation,
      });
      const { chromium } = await import('playwright');
      const browser = await chromium.connect(endpoint);
      const context = await browser.newContext();
      const page = await context.newPage();
      await page.goto(publicUrl, { waitUntil: 'networkidle' });
      const username = process.env.AGENT_BROWSER_P158_DASHBOARD_USERNAME;
      const password = process.env.AGENT_BROWSER_P158_DASHBOARD_PASSWORD;
      if (username || password) {
        if (!username || !password) throw new Error('Both external dashboard credential variables are required');
        const authenticated = await page.evaluate(async ({ login, secret }) => {
          const response = await fetch('/api/dashboard-auth/login', {
            method: 'POST',
            credentials: 'same-origin',
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify({ username: login, password: secret }),
          });
          return response.ok;
        }, { login: username, secret: password });
        if (!authenticated) throw new Error('External dashboard authentication failed');
        await page.reload({ waitUntil: 'networkidle' });
      }
      return {
        page,
        close: async () => {
          await context.close();
          await browser.close();
        },
      };
    },
    produceChurn: async ({ page, root, churnPlan }) => {
      let state = JSON.parse(await readFile(root.target.statePath, 'utf8'));
      const browserId = Object.keys(state.browsers).sort()[0];
      if (!browserId) throw new Error('D09 active churn requires at least one exact browser row');
      const transitions = [];
      for (const operation of churnPlan.operations) {
        const health = operation.ordinal % 2 === 0 ? 'not_started' : 'degraded';
        state = structuredClone(state);
        state.stateRevision += 1;
        state.browsers[browserId].health = health;
        const bytes = `${JSON.stringify(state)}\n`;
        const validationInputPath = `${root.target.disposableRoot}/churn/state-${String(operation.ordinal).padStart(4, '0')}.json`;
        const parserReceipt = await validateInstalledState({
          candidate: root.candidate,
          environment: root.environment,
          validationInputPath,
          stateBytes: bytes,
        });
        if (parserReceipt.accepted !== true || parserReceipt.classification !== 'accepted' ||
            parserReceipt.parserIdentitySha256 !== root.candidate.executableSha256 ||
            parserReceipt.stateSha256 !== sha256Bytes(bytes)) {
          throw new Error(`D09 transition parser rejected ${operation.correlationId}`);
        }
        const nextPath = `${root.target.statePath}.p158-${String(operation.ordinal).padStart(4, '0')}`;
        await writeFile(nextPath, bytes, { flag: 'wx', mode: 0o600 });
        await rename(nextPath, root.target.statePath);
        const observed = await page.evaluate(async ({ id, expectedHealth }) => {
          const response = await fetch('/api/service/browsers?limit=500', {
            credentials: 'same-origin', cache: 'no-store',
          });
          const body = await response.json();
          const browsers = body?.data?.browsers ?? body?.data ?? [];
          return response.ok && body.success === true &&
            browsers.find((browser) => browser.id === id)?.health === expectedHealth;
        }, { id: browserId, expectedHealth: health });
        if (!observed) throw new Error(`D09 live Service state did not reach ${operation.correlationId}`);
        transitions.push({ correlationId: operation.correlationId, health, stateSha256: parserReceipt.stateSha256 });
      }
      return {
        churnPlanSha256: churnPlan.churnPlanSha256,
        completedOperationCount: transitions.length,
        transitionsSha256: sha256Bytes(`${JSON.stringify(transitions)}\n`),
        retryAttempted: false,
      };
    },
    stopExact: async ({ expectedPid, environment }) => {
      const root = { environment };
      const pidPath = join(environment.AGENT_BROWSER_SOCKET_DIR, 'dashboard.pid');
      const selectedPid = Number((await readFile(pidPath, 'utf8')).trim());
      if (selectedPid !== expectedPid) throw new Error('Dashboard PID selector does not match the started instance');
      const result = await runProcess(executionInput.campaignPlan.candidate.executablePath,
        ['--json', 'dashboard', 'stop'],
        { env: isolatedEnvironment(root) });
      const parsed = parseJson(result.stdout, 'dashboard stop');
      if (parsed?.success !== true || parsed?.data?.stopped !== true) throw new Error('Dashboard stop did not stop the selected root');
      return { state: 'stopped', pid: expectedPid };
    },
  };
}

async function main() {
  const inputPath = option('--input');
  const outputPath = option('--output');
  const apply = flag('--apply');
  if (argv.length > 0) throw new Error(`Unexpected arguments: ${argv.join(' ')}`);
  const input = await jsonFile(resolve(inputPath), 'Input');
  let output;
  if (command === 'plan') {
    if (apply) throw new Error('plan does not accept --apply');
    output = buildP158DashboardCampaignPlan(input);
  } else if (command === 'prepare') {
    output = await prepareP158DashboardCampaign({ ...input, apply, validateState: validateInstalledState });
  } else if (command === 'execute') {
    if (!apply) throw new Error('execute requires --apply');
    const effects = createLiveEffects(input);
    const receipts = [];
    for (const root of input.campaignPlan.roots) {
      const receiptPath = join(dirname(resolve(outputPath)), 'action-receipts', `${sha256Bytes(root.actionId).slice(0, 24)}.json`);
      const claimPath = join(dirname(resolve(outputPath)), 'action-claims', `${sha256Bytes(root.actionId).slice(0, 24)}.json`);
      let existingReceipt = null;
      let existingClaim = null;
      try {
        existingReceipt = JSON.parse(await readFile(receiptPath, 'utf8'));
      } catch (error) {
        if (error.code !== 'ENOENT') throw error;
      }
      try {
        existingClaim = JSON.parse(await readFile(claimPath, 'utf8'));
      } catch (error) {
        if (error.code !== 'ENOENT') throw error;
      }
      const resume = resolveP158DashboardActionResume({
        campaignPlan: input.campaignPlan,
        actionId: root.actionId,
        claim: existingClaim,
        receipt: existingReceipt,
      });
      if (resume.disposition === 'reuse_terminal') {
        receipts.push(resume.receipt);
        continue;
      }
      await writeNewJson(claimPath, {
        schemaVersion: 'agent-browser.p158-dashboard-action-claim.v1',
        actionId: root.actionId,
        campaignPlanSha256: input.campaignPlan.campaignPlanSha256,
        effectState: 'claimed_uncertain_until_terminal_receipt',
      });
      const receipt = await executeP158DashboardCampaignAction({
        campaignPlan: input.campaignPlan,
        preparation: input.preparation,
        freezeState: input.freezeState,
        actionId: root.actionId,
        effects,
      });
      receipts.push(receipt);
      await writeNewJson(receiptPath, receipt);
    }
    output = {
      receipts,
      aggregate: aggregateP158DashboardCampaignReceipts({ campaignPlan: input.campaignPlan, receipts }),
    };
  } else {
    throw new Error('Command must be plan, prepare, or execute');
  }
  await writeNewJson(resolve(outputPath), output);
  process.stdout.write(`${JSON.stringify({ success: true, outputPath: resolve(outputPath) })}\n`);
}

main().catch((error) => {
  process.stderr.write(`${JSON.stringify({ success: false, code: error.code ?? 'runner_failed', error: error.message })}\n`);
  process.exitCode = 1;
});
