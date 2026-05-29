#!/usr/bin/env node

import { spawnSync } from 'node:child_process';

const userA = process.env.AGENT_BROWSER_RDP_ROUTE_A_USERNAME || 'agent-browser-rdp-a';
const userB = process.env.AGENT_BROWSER_RDP_ROUTE_B_USERNAME || 'agent-browser-rdp-b';
const existingUser = process.env.AGENT_BROWSER_RDP_EXISTING_USERNAME ||
  process.env.XRDP_AGENT_BROWSER_USERNAME ||
  'agent-browser-rdp';
const shellOutput = process.argv.includes('--shell');

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

function inspectUser(rows, user) {
  const candidates = rows
    .filter((row) => row.user === user)
    .filter((row) => /^(Xorg|Xvnc|Xvfb)$/i.test(row.command) || /\b(Xorg|Xvnc|Xvfb)\b/i.test(row.args))
    .map((row) => ({
      pid: row.pid,
      command: row.command,
      displayName: displayFromArgs(row.args),
      args: row.args,
    }))
    .filter((row) => row.displayName);
  return {
    user,
    displayName: candidates[0]?.displayName || null,
    candidates,
  };
}

const rows = processRows();
const routeA = inspectUser(rows, userA);
const routeB = inspectUser(rows, userB);
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
    A: effectiveRouteA,
    B: effectiveRouteB,
  },
  routeSpecificUsers: {
    A: routeA,
    B: routeB,
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
