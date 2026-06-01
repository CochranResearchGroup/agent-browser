#!/usr/bin/env node

import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';

const args = process.argv.slice(2);
const baseArgIndex = args.findIndex((arg) => arg === '--base');
const json = args.includes('--json');
const base = baseArgIndex >= 0 ? args[baseArgIndex + 1] : defaultBase();

if (baseArgIndex >= 0 && !base) {
  console.error('Missing value for --base');
  process.exit(2);
}

const files = changedFiles(base);
const recommendations = selectRecommendations(files, base);

if (json) {
  console.log(JSON.stringify({ base, files, recommendations }, null, 2));
} else {
  printText({ base, files, recommendations });
}

function defaultBase() {
  return process.env.VALIDATION_BASE || 'HEAD';
}

function changedFiles(ref) {
  const output = git(['diff', '--name-only', `${ref}...HEAD`]).trim();
  const committed = output ? output.split('\n') : [];
  const worktree = git(['diff', '--name-only']).trim();
  const staged = git(['diff', '--cached', '--name-only']).trim();
  const untracked = git(['ls-files', '--others', '--exclude-standard']).trim();
  return [...new Set([
    ...committed,
    ...(worktree ? worktree.split('\n') : []),
    ...(staged ? staged.split('\n') : []),
    ...(untracked ? untracked.split('\n') : []),
  ])].filter(Boolean).sort();
}

function git(args) {
  try {
    return execFileSync('git', args, { encoding: 'utf8' });
  } catch (error) {
    const stderr = error?.stderr ? String(error.stderr) : '';
    const message = stderr.trim() || error?.message || String(error);
    console.error(`git ${args.join(' ')} failed: ${message}`);
    process.exit(2);
  }
}

function selectRecommendations(files, base) {
  const checks = new Map();
  const add = (command, reason) => {
    if (!checks.has(command)) {
      checks.set(command, new Set());
    }
    checks.get(command).add(reason);
  };

  if (files.length === 0) {
    add('git diff --check', 'baseline hygiene check');
    return mapChecks(checks);
  }

  add('git diff --check', 'patch whitespace and conflict-marker hygiene');

  if (files.includes('package.json') && packageJsonFieldsChanged(base, dependencyMetadataFields())) {
    add('pnpm install --lockfile-only', 'package dependency metadata changed; verify lockfile stays current');
  }

  if ((files.includes('package.json') && packageJsonFieldsChanged(base, ['version'])) || files.includes('cli/Cargo.toml')) {
    add('pnpm version:sync', 'version metadata changed');
  }

  if (files.some((file) => file.startsWith('cli/src/') || file === 'cli/Cargo.toml' || file === 'cli/Cargo.lock')) {
    add('cargo fmt --manifest-path cli/Cargo.toml -- --check', 'Rust source or manifest changed');
    add('cargo clippy --manifest-path cli/Cargo.toml -- -D warnings', 'Rust Quality CI gate');
  }

  if (files.some(isCdpTabStreamingSurface)) {
    add(
      'pnpm test:service-cdp-tab-streaming-live',
      'CDP tab streaming route, stream, or dashboard surface changed',
    );
  }

  const focusedRustTests = focusedRustTestCommands(files);
  if (focusedRustTests.length > 0) {
    for (const { command, reason } of focusedRustTests) {
      add(command, reason);
    }
  } else if (files.some((file) => file.startsWith('cli/src/'))) {
    add(
      'cargo test --manifest-path cli/Cargo.toml <focused-filter> -- --test-threads=1',
      'Rust source changed; replace <focused-filter> with the touched module or contract test',
    );
  }

  if (files.some(isServiceContractSurface)) {
    add('pnpm test:service-api-mcp-parity', 'service API/MCP or service-request action surface changed');
    add('pnpm test:service-client-contract', 'generated service client contracts may drift');
    add('pnpm test:service-client-types', 'service client type coverage may drift');
  }

  if (files.some(isBrowserCapabilityRegistryDraftSurface)) {
    add(
      'pnpm test:browser-capability-registry-draft',
      'browser capability registry draft schema, sample, or validator changed',
    );
  }

  if (files.some(isServiceClientSurface)) {
    add('pnpm test:service-client', 'service client package, examples, or generated helpers changed');
  }

  if (files.some((file) => file.startsWith('docs/src/app/') || file.startsWith('docs/src/components/') || file === 'docs/package.json')) {
    add('pnpm --dir docs build', 'docs site changed');
  }

  if (files.some((file) => file.startsWith('packages/dashboard/') || file.startsWith('docs/src/components/dashboard/'))) {
    if (files.some((file) =>
      file.includes('view-stream') ||
      file === 'packages/dashboard/src/components/workspace-remote-viewport.tsx' ||
      file === 'packages/dashboard/src/components/service-panel.tsx'
    )) {
      add('pnpm test:dashboard-view-streams', 'dashboard view stream provider handling changed');
    }
    if (files.some(isDashboardBrowserRowActionSurface)) {
      add(
        'pnpm test:dashboard-browser-row-actions-render',
        'dashboard browser row action rendered titles changed',
      );
    }
    if (files.some((file) => file === 'packages/dashboard/src/components/service-panel.tsx' || file === 'packages/dashboard/src/app/globals.css')) {
      add('pnpm test:dashboard-browser-table', 'dashboard browser table filtering or visibility changed');
    }
    if (files.some((file) =>
      file === 'packages/dashboard/src/lib/service-workspaces.ts' ||
      file === 'packages/dashboard/src/lib/selected-workspace-context.ts' ||
      file === 'packages/dashboard/src/lib/selected-workspace-chat-packet.ts' ||
      file === 'scripts/test-dashboard-workspace-nodes.js' ||
      file === 'scripts/test-dashboard-selected-workspace-context.js' ||
      file === 'scripts/test-dashboard-selected-workspace-chat-packet.js'
    )) {
      add('pnpm test:dashboard-workspace-nodes', 'dashboard workspace navigator model changed');
      add('pnpm test:dashboard-selected-workspace-context', 'selected workspace context matching changed');
      add('pnpm test:dashboard-selected-workspace-chat-packet', 'selected workspace contextual chat packet changed');
    }
    if (files.some((file) =>
      file === 'packages/dashboard/src/components/chat-panel.tsx' ||
      file === 'packages/dashboard/src/lib/dashboard-api.ts' ||
      file === 'packages/dashboard/src/lib/selected-workspace-chat-packet.ts' ||
      file === 'scripts/test-dashboard-contextual-chat.js'
    )) {
      add('pnpm test:dashboard-contextual-chat', 'dashboard contextual Chat Codex app-server surface changed');
    }
    if (files.some((file) =>
      file === 'packages/dashboard/src/lib/launcher-eligibility.ts' ||
      file === 'packages/dashboard/src/components/workspace-navigator.tsx' ||
      file === 'scripts/test-dashboard-launcher-eligibility.js'
    )) {
      add('pnpm test:dashboard-launcher-eligibility', 'dashboard launcher eligibility preview changed');
    }
    if (files.some((file) =>
      file === 'packages/dashboard/src/components/workspace-navigator.tsx' ||
      file === 'packages/dashboard/src/lib/service-workspaces.ts' ||
      file === 'packages/dashboard/src/lib/selected-workspace-context.ts' ||
      file === 'packages/dashboard/src/hooks/use-selected-workspace-context.ts' ||
      file === 'packages/dashboard/src/lib/workspace-url-selection.ts' ||
      file === 'packages/dashboard/src/components/workspace-selection-panel.tsx' ||
      file === 'packages/dashboard/src/components/workspace-remote-viewport.tsx' ||
      file === 'packages/dashboard/src/app/page.tsx' ||
      file === 'packages/dashboard/src/app/globals.css' ||
      file === 'scripts/test-dashboard-workspace-navigator.js'
    )) {
      add('pnpm test:dashboard-workspace-navigator', 'dashboard workspace navigator UI changed');
    }
    if (files.some((file) => file === 'packages/dashboard/src/components/service-panel.tsx' || file === 'packages/dashboard/src/app/page.tsx')) {
      add('pnpm test:dashboard-inspector-actions', 'dashboard service inspector action wiring changed');
    }
    add('pnpm build:dashboard', 'dashboard package or dashboard UI changed');
    add(
      'pnpm publish:local-dashboard -- --expect-marker <changed-ui-marker>',
      'operator-visible dashboard QA must rebuild the embedded binary, restart the user dashboard service, and prove port 4848 serves the change',
    );
  }

  if (files.some((file) => file === 'skills/agent-browser/SKILL.md')) {
    add('diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md', 'repo and installed agent-browser skill must stay synced');
  }

  if (files.some((file) => file.startsWith('scripts/') || file === '.github/workflows/ci.yml')) {
    add('node scripts/dev/select-validation.js --base HEAD --json', 'validation selector or CI scripts changed');
  }

  return mapChecks(checks);
}

function dependencyMetadataFields() {
  return [
    'dependencies',
    'devDependencies',
    'optionalDependencies',
    'peerDependencies',
    'packageManager',
    'pnpm',
  ];
}

function packageJsonFieldsChanged(base, fields) {
  const current = readJsonFile('package.json');
  const previous = readJsonFromGit(base, 'package.json');
  return fields.some((field) => JSON.stringify(current?.[field]) !== JSON.stringify(previous?.[field]));
}

function readJsonFile(path) {
  try {
    return JSON.parse(readFileSync(path, 'utf8'));
  } catch (error) {
    const message = error?.message || String(error);
    console.error(`Reading ${path} failed: ${message}`);
    process.exit(2);
  }
}

function readJsonFromGit(ref, path) {
  try {
    return JSON.parse(execFileSync('git', ['show', `${ref}:${path}`], { encoding: 'utf8' }));
  } catch (error) {
    const stderr = error?.stderr ? String(error.stderr) : '';
    const message = stderr.trim() || error?.message || String(error);
    console.error(`Reading ${path} from ${ref} failed: ${message}`);
    process.exit(2);
  }
}

function isServiceContractSurface(file) {
  return (
    file.startsWith('docs/dev/contracts/') ||
    file === 'cli/src/native/service_contracts.rs' ||
    file === 'cli/src/native/service_request.rs' ||
    file === 'cli/src/native/service_access.rs' ||
    file === 'cli/src/native/service_model.rs' ||
    file === 'cli/src/native/stream/http.rs' ||
    file.startsWith('cli/src/native/mcp') ||
    file.startsWith('scripts/generate-service-') ||
    file === 'scripts/check-service-api-mcp-parity.js'
  );
}

function isServiceClientSurface(file) {
  return (
    file.startsWith('packages/client/') ||
    file.startsWith('examples/service-client/') ||
    file.startsWith('scripts/test-service-') ||
    file === 'scripts/generate-service-request-client.js' ||
    file === 'scripts/generate-service-observability-client.js'
  );
}

function isBrowserCapabilityRegistryDraftSurface(file) {
  return (
    file === 'docs/dev/contracts/service-browser-capability-registry.v1.schema.json' ||
    file === 'docs/dev/contracts/examples/browser-capability-registry.sample.json' ||
    file === 'scripts/smoke-browser-capability-registry-draft.js'
  );
}

function isDashboardBrowserRowActionSurface(file) {
  return (
    file === 'packages/dashboard/src/components/service-panel.tsx' ||
    file === 'packages/dashboard/src/lib/service-browser-row-actions.ts' ||
    file === 'scripts/test-dashboard-browser-row-actions-render.js'
  );
}

function isCdpTabStreamingSurface(file) {
  return (
    file === 'cli/src/native/stream/mod.rs' ||
    file === 'cli/src/native/stream/cdp_loop.rs' ||
    file === 'cli/src/native/stream/websocket.rs' ||
    file === 'cli/src/native/actions.rs' ||
    file === 'packages/dashboard/src/lib/service-view-streams.ts' ||
    file === 'packages/dashboard/src/components/service-panel.tsx' ||
    file === 'scripts/smoke-service-cdp-tab-streaming-live.js'
  );
}

function focusedRustTestCommands(files) {
  const checks = [];
  const add = (command, reason) => {
    if (!checks.some((check) => check.command === command)) {
      checks.push({ command, reason });
    }
  };

  if (files.includes('cli/src/native/service_model.rs')) {
    add(
      'cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1',
      'service model wire-shape and contract fixtures changed',
    );
  }

  if (files.includes('cli/src/native/service_access.rs')) {
    add(
      'cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1',
      'access-plan decision model changed',
    );
  }

  if (files.includes('cli/src/native/service_health.rs')) {
    add(
      'cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1',
      'browser health, recovery, or launch event model changed',
    );
  }

  if (files.includes('cli/src/native/stream/mod.rs') || files.includes('cli/src/native/stream/cdp_loop.rs')) {
    add(
      'cargo test --manifest-path cli/Cargo.toml set_cdp_session_id -- --nocapture',
      'stream server target switching or frame cache behavior changed',
    );
  }

  if (files.includes('cli/src/native/actions.rs')) {
    add(
      'cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture',
      'service-owned CDP stream derivation changed',
    );
  }

  if (files.includes('cli/src/native/service_contracts.rs')) {
    add(
      'cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1',
      'service contract metadata changed',
    );
  }

  if (files.includes('cli/src/native/service_config.rs')) {
    add(
      'cargo test --manifest-path cli/Cargo.toml service_config -- --test-threads=1',
      'service config mutation model changed',
    );
  }

  if (files.includes('cli/src/native/service_monitors.rs')) {
    add(
      'cargo test --manifest-path cli/Cargo.toml service_monitors -- --test-threads=1',
      'service monitor state or run-due logic changed',
    );
  }

  return checks;
}

function mapChecks(checks) {
  return [...checks.entries()].map(([command, reasons]) => ({
    command,
    reasons: [...reasons].sort(),
  }));
}

function printText({ base, files, recommendations }) {
  console.log(`Validation base: ${base}`);
  console.log(`Changed files: ${files.length}`);
  for (const file of files) {
    console.log(`  ${file}`);
  }
  console.log('');
  console.log('Recommended checks:');
  for (const { command, reasons } of recommendations) {
    console.log(`  ${command}`);
    for (const reason of reasons) {
      console.log(`    reason: ${reason}`);
    }
  }
}
