export const VALIDATION_SELECTION_SCHEMA_VERSION = 'agent-browser.validation-selection.v2';
export const VALIDATION_SELECTION_VERSION = VALIDATION_SELECTION_SCHEMA_VERSION;

const JOB_KEYS = Object.freeze(['docs', 'versionSync', 'rustQuality', 'rust', 'dashboard', 'serviceClient', 'repositoryTooling', 'workstation']);

/** Classifies changed paths without reading Git, the filesystem, or process environment. */
export function classifyValidationSelection(files) {
  const changedFiles = normalizeFiles(files);
  if (changedFiles.length === 0) return result({ changedFiles, tier: 'none', reasons: ['no changed files'] });
  if (changedFiles.some(isMaterialDependencyOrToolchain)) return broadResult(changedFiles, [], ['material dependency or toolchain change fails safe to broad presubmit']);
  if (changedFiles.some(isClassifierOrWorkflow)) return broadResult(changedFiles, [], ['classifier or workflow change fails safe to broad presubmit']);

  const perFile = changedFiles.map((file) => ({ file, surfaces: surfacesFor(file) }));
  const unknownFiles = perFile.filter(({ surfaces }) => surfaces.length === 0).map(({ file }) => file);
  const matchedSurfaces = [...new Set(perFile.flatMap(({ surfaces }) => surfaces))].sort();
  if (unknownFiles.length > 0) return broadResult(changedFiles, unknownFiles, ['unknown changed surface fails safe to broad presubmit'], matchedSurfaces);

  const jobs = emptyJobs();
  for (const surface of matchedSurfaces) enableSurfaceJobs(jobs, surface);
  const tier = matchedSurfaces.length === 1 && matchedSurfaces[0] !== 'rust' ? matchedSurfaces[0] : 'focused';
  return result({
    changedFiles,
    tier,
    matchedSurfaces,
    jobs,
    rustCompartments: rustCompartmentsFor(changedFiles),
    serviceSmokes: changedFiles.some(isServiceSmokeSurface),
    reasons: matchedSurfaces.map((surface) => `${surface} surface selected`),
  });
}

export const selectValidation = classifyValidationSelection;

function broadResult(changedFiles, unknownFiles, reasons, matchedSurfaces = []) {
  return result({ changedFiles, tier: 'broad', matchedSurfaces, unknownFiles, jobs: allPresubmitJobs(), rustCompartments: allRustCompartments(), serviceSmokes: true, reasons });
}

function result({ changedFiles, tier, matchedSurfaces = [], unknownFiles = [], jobs = emptyJobs(), rustCompartments = [], serviceSmokes = false, reasons }) {
  const normalizedJobs = { ...emptyJobs(), ...jobs, versionSync: true };
  return Object.freeze({
    schemaVersion: VALIDATION_SELECTION_SCHEMA_VERSION,
    changedFiles: Object.freeze([...changedFiles]),
    tier,
    matchedSurfaces: Object.freeze([...matchedSurfaces].sort()),
    unknownFiles: Object.freeze([...unknownFiles].sort()),
    jobs: Object.freeze(normalizedJobs),
    rustCompartments: Object.freeze([...rustCompartments].sort()),
    serviceSmokes,
    exclusions: Object.freeze(JOB_KEYS.filter((key) => !normalizedJobs[key])),
    reasons: Object.freeze([...reasons]),
  });
}

function emptyJobs() { return Object.fromEntries(JOB_KEYS.map((key) => [key, false])); }
function allPresubmitJobs() { return { ...emptyJobs(), docs: true, versionSync: true, rustQuality: true, rust: true, dashboard: true, serviceClient: true, repositoryTooling: true, workstation: true }; }
function enableSurfaceJobs(jobs, surface) {
  if (surface === 'docs') jobs.docs = true;
  if (surface === 'dashboard') jobs.dashboard = true;
  if (surface === 'service-client') jobs.serviceClient = true;
  if (surface === 'repository-tooling') jobs.repositoryTooling = true;
  if (surface === 'workstation') jobs.workstation = true;
  if (surface === 'rust') { jobs.rustQuality = true; jobs.rust = true; }
}

function normalizeFiles(files) {
  if (!Array.isArray(files) || files.some((file) => typeof file !== 'string' || !file.trim())) return ['__unknown_invalid_changed_surface__'];
  return [...new Set(files.map((file) => file.trim()))].sort();
}

function surfacesFor(file) {
  const surfaces = [];
  if (isDocsOrGovernance(file)) surfaces.push('docs');
  if (file.startsWith('packages/dashboard/')) surfaces.push('dashboard');
  if (isServiceClient(file)) surfaces.push('service-client');
  if (isRepositoryTooling(file)) surfaces.push('repository-tooling');
  if (isWorkstationOrRelease(file)) surfaces.push('workstation');
  if (isRustSource(file)) surfaces.push('rust');
  return surfaces;
}

function isDocsOrGovernance(file) {
  return (
    file.startsWith('docs/') ||
    file === 'AGENTS.md' ||
    file === 'README.md' ||
    file === 'ROADMAP.md' ||
    /^RUNBOOK(?:-.+)?\.md$/.test(file) ||
    file.startsWith('skills/')
  ) && !file.startsWith('docs/dev/contracts/');
}
function isServiceClient(file) {
  return file.startsWith('packages/client/') || file.startsWith('examples/service-client/') || file.startsWith('scripts/generate-service-') || file.startsWith('scripts/test-service-') || file.startsWith('docs/dev/contracts/') || file === 'cli/src/native/service_contracts.rs' || file === 'cli/src/native/service_request.rs' || file === 'cli/src/native/service_model.rs' || file.startsWith('cli/src/native/mcp');
}
function isRepositoryTooling(file) {
  return file === 'scripts/candidate-build.js' || file.startsWith('scripts/lib/candidate-build-') || file.startsWith('scripts/test-candidate-build-') || file === 'scripts/dev/worktree-closeout.js' || file.startsWith('scripts/lib/worktree-closeout') || file === 'scripts/test-worktree-closeout.js';
}
function isWorkstationOrRelease(file) {
  return file.startsWith('cli/assets/workstation/') || file.startsWith('scripts/release/') || file.startsWith('scripts/vm/') || file.startsWith('scripts/test-workstation-') || file.startsWith('scripts/smoke-install-workstation-') || file === 'cli/src/install.rs' || file === 'cli/src/workstation_install.rs' || file.startsWith('cli/src/workstation_install/') || file === 'scripts/test-fresh-workstation-vm-harness.js' || file === 'scripts/test-guacamole-postgres-durability.js' || file === 'scripts/test-rdp-guac-postgres-hardening.js' || file === 'scripts/test-rdp-guac-route-specific-user-sync.js' || file === 'CHANGELOG.md' || file === '.github/workflows/release.yml';
}
function isRustSource(file) { return file.startsWith('cli/src/') || file.startsWith('cli/tests/') || file.startsWith('crates/'); }
function isServiceSmokeSurface(file) {
  return file.startsWith('cli/src/native/service_') || file.startsWith('cli/src/native/mcp') || file === 'cli/src/native/stream/http.rs' || file === 'cli/src/mcp.rs';
}
function isClassifierOrWorkflow(file) { return file === 'package.json' || file === 'scripts/lib/validation-selection.js' || file === 'scripts/test-validation-selection.js' || file === 'scripts/dev/select-validation.js' || file.startsWith('.github/workflows/'); }
function isMaterialDependencyOrToolchain(file) {
  return new Set(['pnpm-lock.yaml', 'package-lock.json', 'yarn.lock', 'Cargo.lock', 'Cargo.toml', 'cli/Cargo.toml', 'rust-toolchain', 'rust-toolchain.toml']).has(file) || /^crates\/[^/]+\/Cargo\.toml$/.test(file) || file.startsWith('.cargo/') || file.startsWith('.github/actions/');
}

function rustCompartmentsFor(files) {
  const compartments = new Set();
  for (const file of files) {
    if (file.startsWith('crates/agent-browser-candidate/')) compartments.add('candidate');
    else if (file.startsWith('crates/agent-browser-cdp/')) compartments.add('transport');
    else if (file.startsWith('crates/agent-browser-challenge-control/')) compartments.add('challenge-control');
    else if (file.startsWith('crates/agent-browser-desktop-services/')) compartments.add('desktop-services');
    else if (file.startsWith('crates/agent-browser-lease-authority/')) compartments.add('lease-authority');
    else if (file.startsWith('crates/')) return allRustCompartments();
    if (file.startsWith('cli/src/candidate')) compartments.add('candidate');
    if (file.includes('challenge_control')) compartments.add('challenge-control');
    if (file === 'cli/src/workstation_install.rs' || file.startsWith('cli/src/workstation_install/')) compartments.add('cli-workstation');
    else if (file.startsWith('cli/src/native/')) compartments.add('cli-native');
    else if (file.startsWith('cli/tests/')) compartments.add('cli-integration');
    else if (file.startsWith('cli/src/')) compartments.add('cli-core');
  }
  return [...compartments];
}

function allRustCompartments() { return ['candidate', 'challenge-control', 'cli-core', 'cli-integration', 'cli-native', 'cli-workstation', 'desktop-services', 'lease-authority', 'transport']; }
