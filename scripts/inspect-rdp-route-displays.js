#!/usr/bin/env node

import { spawnSync } from 'node:child_process';

const userA = process.env.AGENT_BROWSER_RDP_ROUTE_A_USERNAME || 'agent-browser-rdp-a';
const userB = process.env.AGENT_BROWSER_RDP_ROUTE_B_USERNAME || 'agent-browser-rdp-b';
const existingUser = process.env.AGENT_BROWSER_RDP_EXISTING_USERNAME ||
  process.env.XRDP_AGENT_BROWSER_USERNAME ||
  'agent-browser-rdp';
const preferredRouteADisplay = process.env.AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME || null;
const preferredRouteBDisplay = process.env.AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME || null;
const shellOutput = process.argv.includes('--shell');
const includeWindows = process.argv.includes('--windows') || process.argv.includes('--display-content');

function commandResult(command, args) {
  return spawnSync(command, args, {
    encoding: 'utf8',
    stdio: 'pipe',
  });
}

function shellQuote(value) {
  return `'${String(value).replace(/'/g, "'\\''")}'`;
}

function processRows() {
  const result = commandResult('ps', ['-eo', 'user:64=,pid=,comm=,args=']);
  if (result.status !== 0) {
    throw new Error((result.stderr || result.stdout || 'ps failed').trim());
  }
  return result.stdout
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      const parts = line.split(/\s+/);
      return {
        user: parts[0],
        pid: parts[1],
        command: parts[2],
        args: parts.slice(3).join(' '),
      };
    });
}

function displayFromArgs(args) {
  const match = args.match(/(?:^|\s)(:\d+(?:\.\d+)?)(?=\s|$)/);
  return match?.[1] || null;
}

function inspectUser(rows, user, preferredDisplayName = null) {
  const candidates = rows
    .filter((row) => row.user === user)
    .filter((row) => /^(Xorg|Xvnc|Xvfb)$/i.test(row.command) || /\b(Xorg|Xvnc|Xvfb)\b/i.test(row.args))
    .map((row) => ({
      pid: row.pid,
      command: row.command,
      displayName: displayFromArgs(row.args),
      args: row.args,
    }))
    .filter((row) => row.displayName)
    .sort((left, right) => {
      if (preferredDisplayName) {
        if (left.displayName === preferredDisplayName && right.displayName !== preferredDisplayName) {
          return -1;
        }
        if (right.displayName === preferredDisplayName && left.displayName !== preferredDisplayName) {
          return 1;
        }
      }
      return Number(right.pid) - Number(left.pid);
    });
  return {
    user,
    displayName: candidates[0]?.displayName || null,
    candidates,
  };
}

function inspectDisplayContent(displayName) {
  if (!displayName) {
    return {
      state: 'display_missing',
      windows: [],
      error: 'display name is missing',
    };
  }
  const result = commandResult('xwininfo', ['-display', displayName, '-root', '-tree']);
  if (result.status !== 0) {
    return {
      state: 'probe_failed',
      windows: [],
      error: (result.stderr || result.stdout || 'xwininfo failed').trim(),
    };
  }
  const windows = result.stdout
    .split(/\r?\n/)
    .map((line) => {
      const match = line.match(/^\s*(0x[0-9a-f]+)\s+"([^"]*)"/i);
      if (!match) return null;
      return {
        id: match[1],
        title: match[2],
        raw: line.trim(),
      };
    })
    .filter(Boolean);
  const rawText = windows.map((window) => `${window.title}\n${window.raw}`.toLowerCase()).join('\n');
  const browserVisible = /\b(chromium|google chrome|chrome browser|firefox|agent browser)\b/i.test(rawText);
  const terminalVisible = /\b(xterm|terminal|shell)\b/i.test(rawText);
  return {
    state: browserVisible ? 'browser_window_visible' : terminalVisible ? 'terminal_only' : windows.length ? 'non_browser_windows' : 'empty_display',
    windows,
  };
}

function attachDisplayContent(route) {
  if (!includeWindows) return route;
  return {
    ...route,
    displayContent: inspectDisplayContent(route.displayName),
  };
}

const rows = processRows();
const routeA = inspectUser(rows, userA, preferredRouteADisplay);
const routeB = inspectUser(rows, userB, preferredRouteBDisplay);
const existingUserRoutes = rows
  .filter((row) => row.user === existingUser)
  .filter((row) => /^(Xorg|Xvnc|Xvfb)$/i.test(row.command) || /\b(Xorg|Xvnc|Xvfb)\b/i.test(row.args))
  .map((row) => ({
    pid: row.pid,
    command: row.command,
    displayName: displayFromArgs(row.args),
    args: row.args,
  }))
  .filter((row) => row.displayName);
const existingDisplays = [...new Set(existingUserRoutes.map((row) => row.displayName))];
const effectiveRouteA = routeA.displayName ? routeA : {
  user: existingUser,
  displayName: existingDisplays[0] || null,
  candidates: existingUserRoutes,
};
const effectiveRouteB = routeB.displayName ? routeB : {
  user: existingUser,
  displayName: existingDisplays[1] || null,
  candidates: existingUserRoutes,
};
const displays = [effectiveRouteA.displayName, effectiveRouteB.displayName].filter(Boolean);
const success = Boolean(
  effectiveRouteA.displayName &&
    effectiveRouteB.displayName &&
    effectiveRouteA.displayName !== effectiveRouteB.displayName,
);

const result = {
  success,
  status: success ? 'ready' : 'blocked',
  routes: {
    A: attachDisplayContent(effectiveRouteA),
    B: attachDisplayContent(effectiveRouteB),
  },
  routeSpecificUsers: {
    A: attachDisplayContent(routeA),
    B: attachDisplayContent(routeB),
  },
  existingUserRoutes,
  env: success
    ? {
        AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME: effectiveRouteA.displayName,
        AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME: effectiveRouteB.displayName,
      }
    : {},
  nextStep: success
    ? 'Export these display-name variables with the route pool JSON before running pnpm test:rdp-guac-many-to-many-live.'
    : 'Open both RDP route sessions, then rerun this display inspection helper.',
};

if (displays.length > 1 && new Set(displays).size !== displays.length) {
  result.nextStep = 'The route users appear to share one display. P03 requires distinct route displays for this host-XRDP topology.';
}

if (!success && existingUserRoutes.length === 1 && !routeA.displayName && !routeB.displayName) {
  result.nextStep = 'The existing agent-browser-rdp user has one active display only. If both Guacamole route clients are already open, use a route-specific user or XRDP policy isolation fallback before rerunning this helper.';
}

if (!success && existingUserRoutes.length === 1 && (effectiveRouteA.displayName || effectiveRouteB.displayName)) {
  result.nextStep = 'Only one existing-user RDP display is active. If both Guacamole route clients are already open, the host is collapsing existing-user routes onto one desktop; use a route-specific user or XRDP policy isolation fallback.';
}

if (shellOutput && success) {
  console.log(`export AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=${shellQuote(effectiveRouteA.displayName)}`);
  console.log(`export AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=${shellQuote(effectiveRouteB.displayName)}`);
} else {
  console.log(JSON.stringify(result, null, 2));
}

if (!success) {
  process.exitCode = 1;
}
