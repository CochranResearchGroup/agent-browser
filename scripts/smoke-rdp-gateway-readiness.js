#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { request } from 'node:http';
import { request as httpsRequest } from 'node:https';
import { connect } from 'node:net';

const requireHtml5Client = process.argv.includes('--require-html5-client');
loadAgentBrowserEnv();
const viewUrl = process.env.AGENT_BROWSER_REMOTE_VIEW_URL || '';

function commandExists(command) {
  const result = spawnSync('sh', ['-lc', `command -v ${command}`], {
    encoding: 'utf8',
    stdio: 'pipe',
  });
  return result.status === 0 ? result.stdout.trim() : null;
}

function loadAgentBrowserEnv() {
  const agentHome = process.env.AGENT_BROWSER_HOME || join(process.env.HOME || '', '.agent-browser');
  const envPath = join(agentHome, '.env');
  if (!existsSync(envPath)) return;

  const text = readFileSync(envPath, 'utf8');
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const separatorIndex = trimmed.indexOf('=');
    if (separatorIndex <= 0) continue;
    const key = trimmed.slice(0, separatorIndex).trim();
    const value = trimmed.slice(separatorIndex + 1).trim();
    if (!process.env[key]) {
      process.env[key] = value;
    }
  }
}

function tcpCheck(host, port, timeoutMs = 1500) {
  return new Promise((resolve) => {
    const socket = connect({ host, port });
    const done = (ok, error = null) => {
      socket.destroy();
      resolve({ ok, host, port, error });
    };
    socket.setTimeout(timeoutMs);
    socket.once('connect', () => done(true));
    socket.once('timeout', () => done(false, 'timeout'));
    socket.once('error', (error) => done(false, error.message));
  });
}

function httpCheck(url, timeoutMs = 3000) {
  return new Promise((resolve) => {
    let parsed;
    try {
      parsed = new URL(url);
    } catch (error) {
      resolve({ ok: false, url, error: error.message });
      return;
    }

    const client = parsed.protocol === 'https:' ? httpsRequest : request;
    const req = client(
      parsed,
      {
        method: 'GET',
        timeout: timeoutMs,
      },
      (res) => {
        res.resume();
        resolve({
          ok: (res.statusCode ?? 0) >= 200 && (res.statusCode ?? 0) < 500,
          url,
          statusCode: res.statusCode,
        });
      },
    );
    req.once('timeout', () => {
      req.destroy();
      resolve({ ok: false, url, error: 'timeout' });
    });
    req.once('error', (error) => resolve({ ok: false, url, error: error.message }));
    req.end();
  });
}

const commands = {
  guacd: commandExists('guacd'),
  xrdp: commandExists('xrdp'),
  xfreerdp: commandExists('xfreerdp'),
};
const guacdTcp = await tcpCheck('127.0.0.1', 4822);
const xrdpTcp = await tcpCheck('127.0.0.1', 3389);
const html5Client = viewUrl ? await httpCheck(viewUrl) : { ok: false, url: null, error: 'AGENT_BROWSER_REMOTE_VIEW_URL is unset' };
const backendReady = Boolean(commands.guacd && commands.xrdp && commands.xfreerdp && guacdTcp.ok && xrdpTcp.ok);
const ready = backendReady && (!requireHtml5Client || html5Client.ok);

const result = {
  success: ready,
  backendReady,
  html5ClientReady: html5Client.ok,
  commands,
  tcp: {
    guacd: guacdTcp,
    xrdp: xrdpTcp,
  },
  html5Client,
  viewStream: {
    provider: 'rdp_gateway',
    url: viewUrl || null,
  },
  nextStep: html5Client.ok
    ? 'Set AGENT_BROWSER_REMOTE_VIEW_PROVIDER=rdp_gateway for remote_headed launches and use this URL as AGENT_BROWSER_REMOTE_VIEW_URL.'
    : 'Install or start an HTML5 RDP gateway frontend, then set AGENT_BROWSER_REMOTE_VIEW_URL to its browser URL.',
};

console.log(JSON.stringify(result, null, 2));

if (!ready) {
  process.exitCode = 1;
}
