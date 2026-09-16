import { createHash } from 'node:crypto';
import {
  lstatSync,
  readFileSync,
  realpathSync,
  readdirSync,
} from 'node:fs';
import { isAbsolute, relative, resolve, sep } from 'node:path';

/**
 * Provider-free collector for the immutable executable-input closure. Process
 * execution and candidate builds remain in a later orchestration adapter.
 */
export const EXECUTABLE_INPUT_CLOSURE_SCHEMA_VERSION =
  'agent-browser.executable-input-closure.v1';

const SHA256 = /^[a-f0-9]{64}$/u;

function fail(code, message) {
  const error = new Error(`${code}:${message}`);
  error.code = code;
  throw error;
}

function sha256File(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function splitMakeWords(value) {
  const words = [];
  let current = '';
  let escaped = false;
  for (const character of value) {
    if (escaped) {
      current += character;
      escaped = false;
    } else if (character === '\\') {
      escaped = true;
    } else if (/\s/u.test(character)) {
      if (current) words.push(current);
      current = '';
    } else {
      current += character;
    }
  }
  if (escaped) current += '\\';
  if (current) words.push(current);
  return words;
}

export function parseCargoDepInfo(body) {
  const logical = body.replace(/\\\r?\n/gu, ' ');
  const dependencies = [];
  for (const line of logical.split(/\r?\n/u)) {
    const separator = line.indexOf(': ');
    if (separator < 0) continue;
    dependencies.push(...splitMakeWords(line.slice(separator + 2)));
  }
  return [...new Set(dependencies)];
}

function filesUnder(path) {
  const stat = lstatSync(path);
  if (!stat.isDirectory()) return [path];
  return readdirSync(path, { withFileTypes: true })
    .sort((left, right) => left.name.localeCompare(right.name))
    .flatMap((entry) => filesUnder(resolve(path, entry.name)));
}

function repositoryPath(repoRoot, path) {
  const absolute = realpathSync(isAbsolute(path) ? path : resolve(repoRoot, path));
  const normalizedRoot = `${realpathSync(repoRoot)}${sep}`;
  if (absolute !== realpathSync(repoRoot) && !absolute.startsWith(normalizedRoot)) {
    fail('candidate_input_outside_repository', absolute);
  }
  return {
    absolute,
    relative: relative(realpathSync(repoRoot), absolute).split(sep).join('/'),
  };
}

function category(path) {
  if (path.endsWith('.rs') && path.split('/').at(-1) === 'build.rs') return 'build_script';
  if (path.endsWith('.rs')) return 'rust_source';
  if (path.endsWith('Cargo.toml')) return 'cargo_manifest';
  if (path === 'Cargo.lock') return 'cargo_lock';
  if (path.startsWith('packages/dashboard/out/')) return 'embedded_dashboard';
  if (path === 'package.json' || path.endsWith('/package.json')) return 'package_version';
  if (path.startsWith('.cargo/') || path.startsWith('rust-toolchain')) {
    return 'toolchain_configuration';
  }
  return 'embedded_asset';
}

function assertContext(context) {
  for (const field of ['target', 'toolchain', 'cargoProfile']) {
    if (typeof context[field] !== 'string' || !context[field].trim()) {
      fail('candidate_input_context_missing', field);
    }
  }
  const profile = context.resolvedBuildProfile;
  if (
    !profile
    || typeof profile.optLevel !== 'string'
    || typeof profile.lto !== 'string'
    || !Number.isInteger(profile.codegenUnits)
    || profile.codegenUnits < 1
    || typeof profile.strip !== 'boolean'
  ) {
    fail('candidate_input_profile_invalid', 'resolvedBuildProfile');
  }
  for (const [name, digest] of Object.entries(context.reviewedEnvironmentInputs ?? {})) {
    if (!name.trim() || !SHA256.test(digest)) {
      fail('candidate_input_environment_digest_invalid', name);
    }
  }
}

export function collectExecutableInputClosure({
  repoRoot,
  depInfoPaths,
  requiredPaths = [],
  recursiveRoots = [],
  context,
  productionShaped = false,
}) {
  assertContext(context);
  const root = realpathSync(repoRoot);
  const paths = new Set(requiredPaths);
  for (const depInfoPath of depInfoPaths) {
    const depInfo = repositoryPath(root, depInfoPath);
    for (const path of parseCargoDepInfo(readFileSync(depInfo.absolute, 'utf8'))) {
      paths.add(path);
    }
  }
  for (const recursiveRoot of recursiveRoots) {
    const directory = repositoryPath(root, recursiveRoot);
    for (const path of filesUnder(directory.absolute)) paths.add(path);
  }

  const inputsByPath = new Map();
  for (const path of paths) {
    const repositoryInput = repositoryPath(root, path);
    if (!repositoryInput.relative) continue;
    inputsByPath.set(repositoryInput.relative, {
      path: repositoryInput.relative,
      sha256: sha256File(repositoryInput.absolute),
      category: category(repositoryInput.relative),
    });
  }
  const inputs = [...inputsByPath.values()]
    .sort((left, right) => left.path.localeCompare(right.path));
  if (inputs.length === 0) fail('candidate_input_closure_empty', root);

  if (productionShaped) {
    const dashboard = inputs.filter((input) => input.category === 'embedded_dashboard');
    const index = dashboard.find((input) => input.path === 'packages/dashboard/out/index.html');
    if (!index) fail('candidate_dashboard_incomplete', 'index.html missing');
    const indexPath = resolve(root, index.path);
    if (readFileSync(indexPath, 'utf8').includes('Dashboard not built. Run:')) {
      fail('candidate_dashboard_placeholder', index.path);
    }
    if (!inputs.some((input) => input.category === 'embedded_asset')) {
      fail('candidate_embedded_assets_missing', root);
    }
  }

  return {
    schemaVersion: EXECUTABLE_INPUT_CLOSURE_SCHEMA_VERSION,
    context: {
      target: context.target,
      toolchain: context.toolchain,
      cargoProfile: context.cargoProfile,
      resolvedBuildProfile: context.resolvedBuildProfile,
      features: [...new Set(context.features ?? [])].sort(),
      reviewedEnvironmentInputs: Object.fromEntries(
        Object.entries(context.reviewedEnvironmentInputs ?? {}).sort(([left], [right]) =>
          left.localeCompare(right)),
      ),
    },
    inputs,
  };
}
