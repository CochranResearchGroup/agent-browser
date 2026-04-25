#!/usr/bin/env node

import { spawn } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const rootDir = new URL('..', import.meta.url).pathname;
const tempHome = mkdtempSync(join(tmpdir(), 'ab-sr-'));
const realHome = process.env.HOME;
const cargoHome = process.env.CARGO_HOME || (realHome ? join(realHome, '.cargo') : undefined);
const rustupHome = process.env.RUSTUP_HOME || (realHome ? join(realHome, '.rustup') : undefined);
const session = `sr-${process.pid}`;
const agentHome = join(tempHome, '.agent-browser');
const socketDir = join(tempHome, 's');
const profileDir = join(tempHome, 'chrome-profile');

mkdirSync(socketDir, { recursive: true });
mkdirSync(profileDir, { recursive: true });

const env = {
  ...process.env,
  HOME: tempHome,
  AGENT_BROWSER_HOME: agentHome,
  AGENT_BROWSER_SOCKET_DIR: socketDir,
  AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS: '0',
  ...(cargoHome ? { CARGO_HOME: cargoHome } : {}),
  ...(rustupHome ? { RUSTUP_HOME: rustupHome } : {}),
};

const timeout = setTimeout(() => {
  fail('Timed out waiting for service reconcile MCP smoke to complete');
}, 90000);

function cargoArgs(args) {
  return ['run', '--quiet', '--manifest-path', 'cli/Cargo.toml', '--', ...args];
}

function runCli(args) {
  return new Promise((resolve, reject) => {
    const proc = spawn('cargo', cargoArgs(args), {
      cwd: rootDir,
      env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const procTimeout = setTimeout(() => {
      proc.kill('SIGTERM');
      reject(new Error(`agent-browser ${args.join(' ')} timed out`));
    }, 60000);
    let out = '';
    let err = '';
    proc.stdout.setEncoding('utf8');
    proc.stderr.setEncoding('utf8');
    proc.stdout.on('data', (chunk) => {
      out += chunk;
    });
    proc.stderr.on('data', (chunk) => {
      err += chunk;
    });
    proc.on('error', (err) => {
      clearTimeout(procTimeout);
      reject(err);
    });
    proc.on('exit', (code, signal) => {
      clearTimeout(procTimeout);
      if (code === 0) {
        resolve({ stdout: out, stderr: err });
      } else {
        reject(
          new Error(
            `agent-browser ${args.join(' ')} failed: code=${code} signal=${signal}\n${out}${err}`,
          ),
        );
      }
    });
  });
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function parseJsonOutput(output, label) {
  const text = output.trim();
  try {
    return JSON.parse(text);
  } catch (err) {
    throw new Error(`Failed to parse ${label} JSON output: ${err.message}\n${output}`);
  }
}

async function cleanup() {
  clearTimeout(timeout);
  try {
    await runCli(['--json', '--session', session, 'close']);
  } catch {
    // The smoke may fail before the daemon starts; cleanup must stay best-effort.
  }
  rmSync(tempHome, { recursive: true, force: true });
}

async function fail(message) {
  await cleanup();
  console.error(message);
  process.exit(1);
}

function readResourceContents(response, label) {
  assert(response.success === true, `${label} read failed: ${JSON.stringify(response)}`);
  const contents = response.data?.contents;
  assert(contents && typeof contents === 'object', `${label} resource missing contents`);
  return contents;
}

try {
  const pageHtml = [
    '<!doctype html>',
    '<html>',
    '<head><title>Service Reconcile MCP Smoke</title></head>',
    '<body><h1 id="ready">Service Reconcile MCP Smoke</h1></body>',
    '</html>',
  ].join('');
  const pageUrl = `data:text/html;charset=utf-8,${encodeURIComponent(pageHtml)}`;

  const openResult = await runCli([
    '--json',
    '--session',
    session,
    '--profile',
    profileDir,
    '--args',
    '--no-sandbox',
    'open',
    pageUrl,
  ]);
  const opened = parseJsonOutput(openResult.stdout, 'open');
  assert(opened.success === true, `Open command failed: ${openResult.stdout}${openResult.stderr}`);

  const reconcileResult = await runCli(['--json', '--session', session, 'service', 'reconcile']);
  const reconciled = parseJsonOutput(reconcileResult.stdout, 'service reconcile');
  assert(reconciled.success === true, `service reconcile failed: ${reconcileResult.stdout}`);
  assert(reconciled.data?.reconciled === true, 'service reconcile did not report reconciled=true');

  const state = reconciled.data?.service_state;
  assert(state && typeof state === 'object', 'service reconcile response missing service_state');
  const stateBrowsers = Object.values(state.browsers || {});
  const stateTabs = Object.values(state.tabs || {});
  const liveBrowser = stateBrowsers.find(
    (browser) => browser.health === 'ready' && typeof browser.cdpEndpoint === 'string',
  );
  assert(
    liveBrowser,
    `service reconcile did not retain a ready browser: ${JSON.stringify(stateBrowsers)}`,
  );
  const liveTab = stateTabs.find(
    (tab) =>
      tab.browserId === liveBrowser.id &&
      tab.lifecycle === 'ready' &&
      tab.title === 'Service Reconcile MCP Smoke' &&
      typeof tab.targetId === 'string',
  );
  assert(
    liveTab,
    `service reconcile did not retain the live smoke tab: ${JSON.stringify(stateTabs)}`,
  );
  assert(
    state.reconciliation?.browserCount >= 1,
    'service reconcile did not update reconciliation browserCount',
  );
  assert(
    state.reconciliation?.lastReconciledAt,
    'service reconcile did not update lastReconciledAt',
  );

  const browsersResourceResult = await runCli([
    '--json',
    'mcp',
    'read',
    'agent-browser://browsers',
  ]);
  const browsersResource = readResourceContents(
    parseJsonOutput(browsersResourceResult.stdout, 'mcp browsers resource'),
    'browsers',
  );
  const resourceBrowser = browsersResource.browsers?.find(
    (browser) =>
      browser.id === liveBrowser.id &&
      browser.health === liveBrowser.health &&
      browser.cdpEndpoint === liveBrowser.cdpEndpoint,
  );
  assert(
    resourceBrowser,
    `MCP browsers resource did not match service reconcile browser ${liveBrowser.id}: ${JSON.stringify(
      browsersResource,
    )}`,
  );

  const tabsResourceResult = await runCli(['--json', 'mcp', 'read', 'agent-browser://tabs']);
  const tabsResource = readResourceContents(
    parseJsonOutput(tabsResourceResult.stdout, 'mcp tabs resource'),
    'tabs',
  );
  const resourceTab = tabsResource.tabs?.find(
    (tab) =>
      tab.id === liveTab.id &&
      tab.browserId === liveBrowser.id &&
      tab.lifecycle === liveTab.lifecycle &&
      tab.title === liveTab.title &&
      tab.targetId === liveTab.targetId,
  );
  assert(
    resourceTab,
    `MCP tabs resource did not match service reconcile tab ${liveTab.id}: ${JSON.stringify(
      tabsResource,
    )}`,
  );

  await cleanup();
  console.log('Service reconcile MCP smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
