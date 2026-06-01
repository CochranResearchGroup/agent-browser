#!/usr/bin/env node

import { spawn } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { homedir } from 'node:os';

const args = process.argv.slice(2);
const options = {
  agentBrowserBin: process.env.AGENT_BROWSER_BIN || 'agent-browser',
  dashboardUrl: process.env.AGENT_BROWSER_DASHBOARD_URL || 'http://127.0.0.1:4848/',
  expectMarkers: [],
  json: false,
  keepBrowser: false,
  browserProfile: '',
  skipBrowser: false,
  skipChat: false,
  session: `local-dashboard-runtime-smoke-${process.pid}`,
  workspaceSession: '',
};

for (let index = 0; index < args.length; index += 1) {
  const arg = args[index];
  if (arg === '--') {
    continue;
  } else if (arg === '--agent-browser-bin') {
    options.agentBrowserBin = requiredValue(args, ++index, arg);
  } else if (arg === '--dashboard-url') {
    options.dashboardUrl = requiredValue(args, ++index, arg);
  } else if (arg === '--expect-marker') {
    options.expectMarkers.push(requiredValue(args, ++index, arg));
  } else if (arg === '--json') {
    options.json = true;
  } else if (arg === '--keep-browser') {
    options.keepBrowser = true;
  } else if (arg === '--browser-profile') {
    options.browserProfile = requiredValue(args, ++index, arg);
  } else if (arg === '--session') {
    options.session = requiredValue(args, ++index, arg);
  } else if (arg === '--skip-browser') {
    options.skipBrowser = true;
  } else if (arg === '--skip-chat') {
    options.skipChat = true;
  } else if (arg === '--workspace-session') {
    options.workspaceSession = requiredValue(args, ++index, arg);
  } else if (arg === '--help' || arg === '-h') {
    printHelp();
    process.exit(0);
  } else {
    fail(`Unknown argument: ${arg}`);
  }
}

const report = {
  dashboardUrl: options.dashboardUrl,
  http: null,
  markers: [],
  browser: null,
};

try {
  await run();
  if (options.json) {
    console.log(JSON.stringify({ success: true, ...report }, null, 2));
  } else {
    console.log(`Local dashboard runtime smoke passed: ${options.dashboardUrl}`);
  }
} catch (error) {
  if (options.json) {
    console.log(JSON.stringify({
      success: false,
      error: error instanceof Error ? error.message : String(error),
      ...report,
    }, null, 2));
  } else {
    console.error(error instanceof Error ? error.message : String(error));
  }
  process.exit(1);
}

async function run() {
  const dashboardUrl = new URL(options.dashboardUrl);
  const html = await getText(dashboardUrl);
  report.http = {
    htmlBytes: Buffer.byteLength(html),
    title: html.match(/<title>([^<]+)<\/title>/i)?.[1] ?? null,
  };
  if (!html.includes('Agent Browser') && !html.includes('__next')) {
    throw new Error(`Dashboard HTML at ${dashboardUrl.href} did not look like the Agent Browser dashboard.`);
  }

  const chunks = [...new Set([...html.matchAll(/(?:\/_next\/)?static\/[^"']+\.js/g)].map((match) => match[0]))];
  report.http.chunkCount = chunks.length;
  report.http.chunks = chunks.slice(0, 20);

  const chunkTexts = [];
  for (const chunk of chunks) {
    const chunkUrl = chunk.startsWith('/')
      ? new URL(chunk, dashboardUrl.origin)
      : new URL(`/_next/${chunk}`, dashboardUrl.origin);
    try {
      chunkTexts.push(await getText(chunkUrl));
    } catch (error) {
      throw new Error(`Failed to read dashboard chunk ${chunkUrl.href}: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

  for (const marker of options.expectMarkers) {
    const foundInHtml = html.includes(marker);
    const foundInChunk = chunkTexts.some((text) => text.includes(marker));
    report.markers.push({ marker, foundInHtml, foundInChunk });
    if (!foundInHtml && !foundInChunk) {
      throw new Error(`Expected dashboard marker was not served from ${dashboardUrl.origin}: ${marker}`);
    }
  }

  if (!options.skipBrowser) {
    report.browser = await runBrowserSmoke(dashboardUrl);
  }
}

async function runBrowserSmoke(baseUrl) {
  const smokeUrl = new URL(baseUrl.href);
  if (options.workspaceSession) {
    smokeUrl.searchParams.set('view', 'workspace:control');
    smokeUrl.searchParams.set('workspace', `daemon-session:${options.workspaceSession}`);
    smokeUrl.searchParams.set('session', options.workspaceSession);
    smokeUrl.searchParams.set('tab', '0');
  }

  try {
    await runAgent([...baseAgentArgs(), 'open', smokeUrl.href], { timeoutMs: 90000 });
    await runAgent(['--json', '--session', options.session, 'wait', '1000'], { timeoutMs: 30000 });
    const first = await evalAgent(`
JSON.stringify({
  needsLogin: Boolean(document.querySelector('input[type="password"]')),
  url: location.href,
  title: document.title
})
`);
    let firstState = parseEvalJson(first, 'initial dashboard browser state');
    if (firstState.needsLogin) {
      const credentials = dashboardCredentials();
      await evalAgent(`
(async () => {
  await fetch('/api/dashboard-auth/login', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    credentials: 'same-origin',
    body: JSON.stringify(${JSON.stringify(credentials)})
  });
  location.reload();
  return JSON.stringify({ loginSubmitted: true });
})()
`);
      await runAgent(['--json', '--session', options.session, 'wait', '1500'], { timeoutMs: 30000 });
      firstState = parseEvalJson(await evalAgent(`
JSON.stringify({
  needsLogin: Boolean(document.querySelector('input[type="password"]')),
  url: location.href,
  title: document.title
})
`), 'post-login dashboard browser state');
      if (firstState.needsLogin) {
        throw new Error('Dashboard browser smoke could not authenticate with the user-scoped dashboard auth file.');
      }
    }

    const finalState = parseEvalJson(await evalAgent(`
JSON.stringify({
  url: location.href,
  hasAgentBrowserChrome: document.body.innerText.includes('Agent Browser'),
  hasWorkspacePane: document.body.innerText.includes('Workspaces'),
  hasWorkspaceTab: Array.from(document.querySelectorAll('[role=tab],button')).some((element) => element.textContent?.trim() === 'Workspace'),
  rightPaneText: document.querySelector('.dashboard-pane-right')?.innerText.slice(0, 500) || '',
  viewport: Boolean(document.querySelector('.workspace-remote-viewport')),
  frameSrc: document.querySelector('.workspace-remote-viewport-frame')?.getAttribute('src') || null,
  hasCdpCanvas: Boolean(document.querySelector('.workspace-cdp-stream-canvas')),
  cdpProvider: document.querySelector('.workspace-cdp-stream')?.getAttribute('data-provider') || null,
  readinessStatus: document.querySelector('.workspace-remote-viewport')?.getAttribute('data-readiness-status') || null
})
`), 'final dashboard browser state');

    if (!finalState.hasAgentBrowserChrome || !finalState.hasWorkspacePane) {
      throw new Error(`Dashboard browser smoke did not see the expected app chrome: ${JSON.stringify(finalState)}`);
    }
    if (options.workspaceSession && (!finalState.viewport || (!finalState.frameSrc && !finalState.hasCdpCanvas))) {
      throw new Error(`Workspace route did not render an embedded viewport: ${JSON.stringify(finalState)}`);
    }
    if (options.workspaceSession) {
      const workspaceState = parseEvalJson(await evalAgent(`
JSON.stringify({
  hasWorkspaceDetail: Boolean(document.querySelector('[data-selected-workspace-context="ready"]')),
  selectedWorkspaceId: document.querySelector('[data-selected-workspace-context="ready"]')?.getAttribute('data-selected-workspace-id') || null,
  selectedWorkspaceState: document.querySelector('[data-selected-workspace-context="ready"]')?.getAttribute('data-selected-workspace-state') || null,
  hasPidIndicator: /\\bPID\\b/.test(document.querySelector('.dashboard-pane-right')?.innerText || ''),
  hasMemoryIndicator: /\\bRSS\\b|\\bMemory\\b/.test(document.querySelector('.dashboard-pane-right')?.innerText || ''),
  hasCpuIndicator: /\\bCPU\\b/.test(document.querySelector('.dashboard-pane-right')?.innerText || ''),
  hasCdpFact: /\\bCDP\\b/.test(document.querySelector('.dashboard-pane-right')?.innerText || ''),
  hasStreamFact: /\\bStream\\b/.test(document.querySelector('.dashboard-pane-right')?.innerText || ''),
  hasCopyDiagnostics: (document.querySelector('.dashboard-pane-right')?.innerText || '').includes('Copy diagnostics'),
  rightPaneText: document.querySelector('.dashboard-pane-right')?.innerText.slice(0, 800) || ''
})
`), 'workspace tab detail state');
      if (!workspaceState.hasWorkspaceDetail || !workspaceState.hasPidIndicator || !workspaceState.hasMemoryIndicator || !workspaceState.hasCpuIndicator || !workspaceState.hasCdpFact || !workspaceState.hasStreamFact || !workspaceState.hasCopyDiagnostics) {
        throw new Error(`Workspace tab did not expose dense selected-workspace detail: ${JSON.stringify(workspaceState)}`);
      }
      finalState.workspaceState = workspaceState;
    }
    let chatState = null;
    if (options.workspaceSession && !options.skipChat) {
      chatState = parseEvalJson(await evalAgent(`
(async () => {
const chatButton = Array.from(document.querySelectorAll('[role=tab],button'))
  .find((element) => element.textContent?.trim() === 'Chat');
for (const type of ['pointerdown', 'mousedown', 'pointerup', 'mouseup', 'click']) {
  chatButton?.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true, view: window }));
}
await new Promise((resolve) => setTimeout(resolve, 500));
const text = document.querySelector('.dashboard-pane-right')?.innerText || document.body.innerText;
const inspectButton = Array.from(document.querySelectorAll('button'))
  .find((element) => element.textContent?.trim() === 'Inspect viewport readiness');
for (const type of ['pointerdown', 'mousedown', 'pointerup', 'mouseup', 'click']) {
  inspectButton?.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true, view: window }));
}
return JSON.stringify({
  hasCodexProvider: text.includes('Codex app server'),
  hasReadOnlyProvider: text.includes('read-only'),
  hasContextualChatMarker: Boolean(document.querySelector('[data-codex-app-server-contextual-chat="ready"]')),
  hasModelSelectorText: /model|OpenAI|AI Gateway/i.test(text),
  clickedInspect: Boolean(inspectButton),
  rightPaneText: text.slice(0, 500)
});
})()
`), 'workspace Chat contextual state');
      if (!chatState.clickedInspect) {
        throw new Error(`Workspace Chat did not expose the viewport inspection action: ${JSON.stringify(chatState)}`);
      }
      for (let attempt = 0; attempt < 75; attempt += 1) {
        const pollState = parseEvalJson(await evalAgent(`
(() => {
const inspectedText = document.querySelector('.dashboard-pane-right')?.innerText || document.body.innerText;
return JSON.stringify({
  hasStructuredObservation: /observation/i.test(inspectedText) && inspectedText.includes('codex-app-server'),
  hasStructuredFailure: /inspection failure/i.test(inspectedText) && inspectedText.includes('codex-app-server'),
  hasEventLog: /event log/i.test(inspectedText),
  hasThreadOrTurn: /\\b(thread|turn)\\s+[a-zA-Z0-9_-]{4,}/.test(inspectedText),
  rightPaneText: inspectedText.slice(0, 500)
});
})()
`), 'workspace Chat inspection poll state');
        chatState = { ...chatState, ...pollState };
        if ((pollState.hasStructuredObservation || pollState.hasStructuredFailure) && pollState.hasEventLog) {
          break;
        }
        await new Promise((resolve) => setTimeout(resolve, 1000));
      }
      if (!chatState.hasCodexProvider || !chatState.hasContextualChatMarker) {
        throw new Error(`Workspace Chat did not expose the Codex app-server context surface: ${JSON.stringify(chatState)}`);
      }
      if (chatState.hasModelSelectorText) {
        throw new Error(`Workspace Chat exposed a model/provider selector label: ${JSON.stringify(chatState)}`);
      }
      if (!chatState.hasStructuredObservation) {
        throw new Error(`Workspace Chat did not render a structured Codex inspection observation: ${JSON.stringify(chatState)}`);
      }
      if (!chatState.hasEventLog || !chatState.hasThreadOrTurn) {
        throw new Error(`Workspace Chat did not render app-server ledger metadata: ${JSON.stringify(chatState)}`);
      }
    }
    return {
      session: options.session,
      smokeUrl: smokeUrl.href,
      ...finalState,
      chatState,
    };
  } finally {
    if (!options.keepBrowser) {
      await runAgent([...baseAgentArgs(), 'close'], { timeoutMs: 30000 }).catch(() => undefined);
    }
  }
}

function baseAgentArgs() {
  const command = ['--json', '--session', options.session];
  if (options.browserProfile) {
    command.push('--profile', options.browserProfile);
  }
  return command;
}

async function getText(url) {
  const response = await fetch(url, { redirect: 'follow' });
  if (!response.ok) {
    throw new Error(`HTTP ${response.status} from ${url.href}`);
  }
  return response.text();
}

function dashboardCredentials() {
  const authPath = process.env.AGENT_BROWSER_DASHBOARD_AUTH_ENV ||
    `${homedir()}/.agent-browser/dashboard-auth.env`;
  if (!existsSync(authPath)) {
    throw new Error(`Dashboard auth env file is missing: ${authPath}`);
  }
  const values = parseEnv(readFileSync(authPath, 'utf8'));
  const username = values.AGENT_BROWSER_DASHBOARD_CODEX_USERNAME ||
    values.AGENT_BROWSER_DASHBOARD_ADMIN_USERNAME ||
    'admin';
  const password = values.AGENT_BROWSER_DASHBOARD_CODEX_PASSWORD ||
    values.AGENT_BROWSER_DASHBOARD_ADMIN_PASSWORD;
  if (!password) {
    throw new Error(`Dashboard auth env file does not contain a usable dashboard password: ${authPath}`);
  }
  return { username, password };
}

function parseEnv(text) {
  const values = {};
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const index = trimmed.indexOf('=');
    if (index <= 0) continue;
    const key = trimmed.slice(0, index).trim();
    let value = trimmed.slice(index + 1).trim();
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    values[key] = value.replace(/\\"/g, '"');
  }
  return values;
}

async function evalAgent(script) {
  const result = await runAgent(['--json', '--session', options.session, 'eval', '--stdin'], {
    input: script,
    timeoutMs: 60000,
  });
  const parsed = parseJson(result.stdout, 'agent-browser eval');
  if (!parsed.success) {
    throw new Error(`agent-browser eval failed: ${result.stdout}${result.stderr}`);
  }
  return parsed.data?.result;
}

function parseEvalJson(value, label) {
  if (typeof value !== 'string') {
    throw new Error(`${label} did not return a JSON string: ${JSON.stringify(value)}`);
  }
  return parseJson(value, label);
}

function parseJson(text, label) {
  try {
    return JSON.parse(String(text).trim());
  } catch (error) {
    throw new Error(`Failed to parse ${label} JSON: ${error instanceof Error ? error.message : String(error)}\n${text}`);
  }
}

function runAgent(commandArgs, { input = '', timeoutMs = 60000 } = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(options.agentBrowserBin, commandArgs, {
      cwd: new URL('..', import.meta.url),
      env: process.env,
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    const timeout = setTimeout(() => {
      child.kill('SIGTERM');
      reject(new Error(`agent-browser command timed out: ${commandArgs.join(' ')}`));
    }, timeoutMs);
    let stdout = '';
    let stderr = '';
    child.stdout.setEncoding('utf8');
    child.stderr.setEncoding('utf8');
    child.stdout.on('data', (chunk) => {
      stdout += chunk;
    });
    child.stderr.on('data', (chunk) => {
      stderr += chunk;
    });
    child.on('error', (error) => {
      clearTimeout(timeout);
      reject(error);
    });
    child.on('exit', (code, signal) => {
      clearTimeout(timeout);
      if (code === 0) {
        resolve({ stdout, stderr });
      } else {
        reject(new Error(`agent-browser ${commandArgs.join(' ')} failed with code=${code} signal=${signal}\n${stdout}${stderr}`));
      }
    });
    if (input) child.stdin.end(input);
    else child.stdin.end();
  });
}

function requiredValue(values, index, flag) {
  const value = values[index];
  if (!value) fail(`Missing value for ${flag}`);
  return value;
}

function fail(message) {
  console.error(message);
  process.exit(2);
}

function printHelp() {
  console.log(`Usage: node scripts/smoke-local-dashboard-runtime.js [options]

Options:
  --dashboard-url <url>       Dashboard URL to verify. Default: http://127.0.0.1:4848/
  --expect-marker <text>      Require a served HTML or JS bundle to contain text. Repeatable.
  --agent-browser-bin <path>  agent-browser binary used for browser smoke.
  --browser-profile <path>    Use an isolated runtime profile for the smoke browser.
  --workspace-session <name>  Open a workspace viewport route for a daemon session.
  --skip-browser              Only run HTTP and bundle marker checks.
  --skip-chat                 Do not run contextual Chat checks on workspace route smoke.
  --json                      Print structured JSON.
`);
}
