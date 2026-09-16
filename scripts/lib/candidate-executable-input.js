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
export const CANDIDATE_MANIFEST_SCHEMA_VERSION = 'agent-browser.candidate-manifest.v1';
export const BUILD_SUPPORT_MANIFEST_SCHEMA_VERSION =
  'agent-browser.candidate-build-support.v1';

const SHA256 = /^[a-f0-9]{64}$/u;

function fail(code, message) {
  const error = new Error(`${code}:${message}`);
  error.code = code;
  throw error;
}

function sha256File(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function sha256Bytes(value) {
  return createHash('sha256').update(value).digest('hex');
}

function digestJson(value) {
  return sha256Bytes(JSON.stringify(value));
}

function compareCanonical(left, right) {
  if (left < right) return -1;
  if (left > right) return 1;
  return 0;
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
    .sort((left, right) => compareCanonical(left.name, right.name))
    .flatMap((entry) => filesUnder(resolve(path, entry.name)));
}

function pathWithin(root, path) {
  return path === root || path.startsWith(`${root}${sep}`);
}

function repositoryPath(repoRoot, path, sourceControlRoots = {}) {
  const absolute = realpathSync(isAbsolute(path) ? path : resolve(repoRoot, path));
  const repositoryRoot = realpathSync(repoRoot);
  if (pathWithin(repositoryRoot, absolute)) {
    return {
      absolute,
      relative: relative(repositoryRoot, absolute).split(sep).join('/'),
    };
  }

  // Linked worktrees keep the branch ref and packed refs in the shared Git
  // directory. Cargo records those files because build.rs embeds source
  // provenance. Provenance is already sealed independently by the source
  // commit, tree state, and binary digest, so it must not turn a docs-only
  // merge into an executable-input change.
  for (const configuredRoot of [
    sourceControlRoots.worktree,
    sourceControlRoots.common,
  ]) {
    if (!configuredRoot) continue;
    const sourceControlRoot = realpathSync(configuredRoot);
    if (pathWithin(sourceControlRoot, absolute)) {
      return {
        absolute,
        relative: null,
      };
    }
  }

  fail('candidate_input_outside_repository', absolute);
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
  sourceControlRoots = {},
}) {
  assertContext(context);
  const root = realpathSync(repoRoot);
  const paths = new Set(requiredPaths);
  for (const depInfoPath of depInfoPaths) {
    const depInfo = repositoryPath(root, depInfoPath, sourceControlRoots);
    for (const path of parseCargoDepInfo(readFileSync(depInfo.absolute, 'utf8'))) {
      paths.add(path);
    }
  }
  for (const recursiveRoot of recursiveRoots) {
    const directory = repositoryPath(root, recursiveRoot, sourceControlRoots);
    for (const path of filesUnder(directory.absolute)) paths.add(path);
  }

  const inputsByPath = new Map();
  for (const path of paths) {
    const repositoryInput = repositoryPath(root, path, sourceControlRoots);
    for (const inputPath of filesUnder(repositoryInput.absolute)) {
      const input = repositoryPath(root, inputPath, sourceControlRoots);
      if (!input.relative) continue;
      inputsByPath.set(input.relative, {
        path: input.relative,
        sha256: sha256File(input.absolute),
        category: category(input.relative),
      });
    }
  }
  const inputs = [...inputsByPath.values()]
    .sort((left, right) => compareCanonical(left.path, right.path));
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
          compareCanonical(left, right)),
      ),
    },
    inputs,
  };
}

export function digestExecutableInputClosure(closure) {
  if (closure?.schemaVersion !== EXECUTABLE_INPUT_CLOSURE_SCHEMA_VERSION) {
    fail('candidate_input_closure_schema_invalid', closure?.schemaVersion ?? 'missing');
  }
  return digestJson(closure);
}

export function createBuildSupportManifest(closure) {
  const executableInputSha256 = digestExecutableInputClosure(closure);
  const dashboardInputs = closure.inputs
    .filter((input) => input.category === 'embedded_dashboard');
  const embeddedAssetDigests = Object.fromEntries(
    closure.inputs
      .filter((input) => input.category === 'embedded_asset')
      .map((input) => [input.path, input.sha256]),
  );
  return {
    schemaVersion: BUILD_SUPPORT_MANIFEST_SCHEMA_VERSION,
    executableInputSha256,
    embeddedDashboardSha256: digestJson(dashboardInputs),
    embeddedAssetDigests,
    reviewedEnvironmentInputSha256: digestJson(
      closure.context.reviewedEnvironmentInputs,
    ),
    resolvedBuildProfileSha256: digestJson(closure.context.resolvedBuildProfile),
  };
}

export function encodeBuildSupportManifest(manifest) {
  return Buffer.from(`${JSON.stringify(manifest, null, 2)}\n`);
}

export function createCandidateManifest({
  closure,
  source,
  artifactClass,
  binarySha256,
  supportManifestSha256,
  createdAt,
  validationReceipts = [],
}) {
  for (const [name, digest] of [
    ['source.tree', source?.tree],
    ['binarySha256', binarySha256],
    ['supportManifestSha256', supportManifestSha256],
  ]) {
    if (!SHA256.test(digest ?? '')) fail('candidate_manifest_digest_invalid', name);
  }
  if (!/^[a-f0-9]{40}$/u.test(source?.commit ?? '')) {
    fail('candidate_manifest_commit_invalid', source?.commit ?? 'missing');
  }
  if (!['clean', 'dirty', 'unknown'].includes(source?.state)) {
    fail('candidate_manifest_tree_state_invalid', source?.state ?? 'missing');
  }
  if (!['fast_iteration', 'production_shaped'].includes(artifactClass)) {
    fail('candidate_manifest_artifact_class_invalid', artifactClass ?? 'missing');
  }
  if (typeof createdAt !== 'string' || !createdAt.includes('T') || !createdAt.endsWith('Z')) {
    fail('candidate_manifest_creation_time_invalid', createdAt ?? 'missing');
  }
  if (!validationReceipts.every((receipt) => typeof receipt === 'string' && receipt.trim())) {
    fail('candidate_manifest_receipt_invalid', 'validationReceipts');
  }

  const support = createBuildSupportManifest(closure);
  if (
    artifactClass === 'production_shaped'
    && (
      !closure.inputs.some((input) => input.category === 'embedded_dashboard')
      || Object.keys(support.embeddedAssetDigests).length === 0
    )
  ) {
    fail('candidate_build_support_incomplete', support.executableInputSha256);
  }
  const candidateId = `candidate-${support.executableInputSha256.slice(0, 16)}-${binarySha256.slice(0, 16)}`;
  return {
    schemaVersion: CANDIDATE_MANIFEST_SCHEMA_VERSION,
    candidateId,
    source,
    executableInputSha256: support.executableInputSha256,
    target: closure.context.target,
    toolchain: closure.context.toolchain,
    cargoProfile: closure.context.cargoProfile,
    features: closure.context.features,
    reviewedEnvironmentInputSha256: support.reviewedEnvironmentInputSha256,
    embeddedDashboardSha256: support.embeddedDashboardSha256,
    embeddedAssetDigests: support.embeddedAssetDigests,
    binarySha256,
    supportManifestSha256,
    createdAt,
    artifactClass,
    resolvedBuildProfile: closure.context.resolvedBuildProfile,
    resolvedBuildProfileSha256: support.resolvedBuildProfileSha256,
    validationReceipts,
  };
}
