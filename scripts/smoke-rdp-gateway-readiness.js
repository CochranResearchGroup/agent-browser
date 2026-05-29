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
  const result = spawnSync('sh', ['-lc', `command -v ${command} || { for candidate in /usr/sbin/${command} /usr/local/sbin/${command}; do [ -x "$candidate" ] && printf '%s\\n' "$candidate" && exit 0; done; exit 1; }`], {
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
  xrdpSesman: commandExists('xrdp-sesman'),
  xfreerdp: commandExists('xfreerdp'),
};
const guacdTcp = await tcpCheck('127.0.0.1', 4822);
const xrdpTcp = await tcpCheck('127.0.0.1', 3389);
const html5Client = viewUrl ? await httpCheck(viewUrl) : { ok: false, url: null, error: 'AGENT_BROWSER_REMOTE_VIEW_URL is unset' };
const backendReady = Boolean(commands.guacd && commands.xrdp && commands.xrdpSesman && commands.xfreerdp && guacdTcp.ok && xrdpTcp.ok);
const ready = backendReady && (!requireHtml5Client || html5Client.ok);
const readinessComponents = [
  {
    component: 'guacd',
    status: commands.guacd && guacdTcp.ok ? 'ready' : 'failed',
    evidence: commands.guacd ? `guacd command at ${commands.guacd}; tcp 127.0.0.1:4822 ${guacdTcp.ok ? 'reachable' : guacdTcp.error}` : 'guacd command not found',
    nextAction: commands.guacd && guacdTcp.ok ? 'none' : 'start_or_install_guacd',
    recovery: commands.guacd && guacdTcp.ok ? 'guacd is reachable.' : 'Install or start guacd, then rerun the RDP gateway readiness smoke.',
  },
  {
    component: 'xrdp',
    status: commands.xrdp && xrdpTcp.ok ? 'ready' : 'failed',
    evidence: commands.xrdp ? `xrdp command at ${commands.xrdp}; tcp 127.0.0.1:3389 ${xrdpTcp.ok ? 'reachable' : xrdpTcp.error}` : 'xrdp command not found',
    nextAction: commands.xrdp && xrdpTcp.ok ? 'none' : 'start_or_install_xrdp',
    recovery: commands.xrdp && xrdpTcp.ok ? 'xrdp is reachable.' : 'Install or start xrdp, then rerun the RDP gateway readiness smoke.',
  },
  {
    component: 'xrdp_sesman',
    status: commands.xrdpSesman ? 'ready' : 'failed',
    evidence: commands.xrdpSesman ? `xrdp-sesman command at ${commands.xrdpSesman}` : 'xrdp-sesman command not found',
    nextAction: commands.xrdpSesman ? 'none' : 'start_or_install_xrdp_sesman',
    recovery: commands.xrdpSesman ? 'xrdp-sesman is installed.' : 'Install or start xrdp-sesman before validating Guacamole sessions.',
  },
  {
    component: 'backend_tcp',
    status: guacdTcp.ok && xrdpTcp.ok ? 'ready' : 'failed',
    evidence: `guacd tcp=${guacdTcp.ok ? 'ok' : guacdTcp.error}; xrdp tcp=${xrdpTcp.ok ? 'ok' : xrdpTcp.error}`,
    nextAction: guacdTcp.ok && xrdpTcp.ok ? 'none' : 'inspect_backend_tcp',
    recovery: guacdTcp.ok && xrdpTcp.ok ? 'Backend TCP listeners are reachable.' : 'Inspect local firewall, service status, and listener bindings for guacd and xrdp.',
  },
  {
    component: 'guacamole_web_app',
    status: html5Client.ok ? 'ready' : (viewUrl ? 'failed' : 'missing'),
    evidence: viewUrl ? `html5 client ${html5Client.statusCode ?? html5Client.error}` : 'AGENT_BROWSER_REMOTE_VIEW_URL is unset',
    nextAction: html5Client.ok ? 'none' : 'configure_remote_view_url',
    recovery: html5Client.ok ? 'The configured HTML5 RDP gateway route responded.' : 'Install or start the HTML5 RDP gateway frontend, then set AGENT_BROWSER_REMOTE_VIEW_URL.',
  },
  {
    component: 'dashboard_auth',
    status: 'unknown',
    evidence: 'dashboard auth is validated by the browser live harness after isolated AGENT_BROWSER_HOME initialization',
    nextAction: 'run_dashboard_auth_harness',
    recovery: 'Initialize dashboard auth in the same isolated AGENT_BROWSER_HOME used by the live RDP and Guacamole harness.',
  },
  {
    component: 'iframe_embedding',
    status: html5Client.ok ? 'unknown' : 'blocked',
    evidence: html5Client.ok ? 'route responded; browser iframe policy is checked in the dashboard harness' : 'route did not respond successfully',
    nextAction: html5Client.ok ? 'run_dashboard_iframe_harness' : 'configure_remote_view_url',
    recovery: html5Client.ok ? 'Use the dashboard harness to prove iframe embedding and interaction.' : 'Fix the HTML5 gateway route before testing iframe embedding.',
  },
  {
    component: 'public_ingress',
    status: viewUrl && /^https?:\/\//i.test(viewUrl) ? (html5Client.ok ? 'ready' : 'failed') : 'unknown',
    evidence: viewUrl || 'no public or local URL configured',
    nextAction: html5Client.ok ? 'none' : 'inspect_public_ingress',
    recovery: html5Client.ok ? 'Configured ingress responded.' : 'Inspect DNS, tunnel, proxy, and dashboard route configuration for the public Guacamole path.',
  },
];
const readiness = {
  status: ready ? 'ready' : 'blocked',
  components: readinessComponents,
  nextAction: ready
    ? 'none'
    : readinessComponents.find((component) => component.status === 'failed' || component.status === 'blocked' || component.status === 'missing')?.nextAction ?? 'inspect_readiness',
};

const result = {
  success: ready,
  backendReady,
  html5ClientReady: html5Client.ok,
  readiness,
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
