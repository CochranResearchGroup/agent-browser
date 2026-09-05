#!/usr/bin/env node

import {
  assertProductionUnchanged,
  assertDefaultDevelopmentUnchanged,
  defaultDevelopmentSnapshot,
  developmentCandidateBinary,
  developmentRuntimeStatus,
  doctorDevelopmentRuntime,
  garbageCollectDevelopmentRuntime,
  installDevelopmentRuntime,
  productionSnapshot,
} from './lib/development-runtime.js';
import {
  developmentAgentSkillStatus,
  developmentPresentationProviderDescriptor,
  synchronizeDevelopmentAgentSkill,
} from './lib/development-presentation-provider.js';
import {
  applyDevelopmentPresentationProvider,
  developmentPresentationProviderDeploymentPlan,
  stageDevelopmentPresentationProviderBundle,
} from './lib/development-presentation-provider-deployment.js';
import {
  createDevelopmentPresentationProviderSystemEffects,
  createDevelopmentPresentationLifecycleSystemEffects,
  developmentPresentationProviderSystemPreflight,
} from './lib/development-presentation-provider-system-effects.js';
import {
  scaleInDevelopmentPresentation,
  scaleOutDevelopmentPresentation,
} from './lib/development-presentation-lifecycle.js';

const args = process.argv.slice(2).filter((arg) => arg !== '--');
const command = args.shift() || 'status';
const json = removeFlag(args, '--json');

try {
  let result;
  if (command === 'install') {
    const binary = takeOption(args, '--binary') || developmentCandidateBinary();
    const activate = !removeFlag(args, '--no-activate');
    rejectArgs(args);
    result = installDevelopmentRuntime({ binary, activate });
  } else if (command === 'status') {
    rejectArgs(args);
    result = developmentRuntimeStatus();
  } else if (command === 'doctor') {
    rejectArgs(args);
    result = doctorDevelopmentRuntime();
  } else if (command === 'gc') {
    const retain = Number(takeOption(args, '--retain') || 2);
    if (!Number.isInteger(retain) || retain < 1) throw new Error('--retain must be a positive integer');
    rejectArgs(args);
    result = garbageCollectDevelopmentRuntime({ retain });
  } else if (command === 'skill-sync') {
    rejectArgs(args);
    result = synchronizeDevelopmentAgentSkill();
  } else if (command === 'skill-status') {
    rejectArgs(args);
    result = developmentAgentSkillStatus();
  } else if (command === 'provider-plan') {
    rejectArgs(args);
    result = developmentPresentationProviderDeploymentPlan(
      developmentPresentationProviderDescriptor(),
    );
  } else if (command === 'provider-stage') {
    rejectArgs(args);
    result = stageDevelopmentPresentationProviderBundle();
  } else if (command === 'provider-status') {
    rejectArgs(args);
    result = developmentRuntimeStatus().presentationProvider;
  } else if (command === 'provider-preflight') {
    rejectArgs(args);
    result = developmentPresentationProviderSystemPreflight();
  } else if (command === 'provider-apply') {
    const apply = removeFlag(args, '--apply');
    const deferIngress = removeFlag(args, '--defer-ingress');
    rejectArgs(args);
    if (!apply) throw new Error('provider-apply requires --apply');
    if (!deferIngress) {
      throw new Error('provider-apply currently requires --defer-ingress');
    }
    const providerStatus = developmentRuntimeStatus().presentationProvider;
    const preflight = providerStatus.ready
      ? { success: true, checks: [] }
      : developmentPresentationProviderSystemPreflight();
    if (!preflight.success) {
      const failed = preflight.checks.filter((item) => !item.ok).map((item) => item.name);
      throw new Error(`Development provider preflight failed: ${failed.join(', ')}`);
    }
    result = applyDevelopmentPresentationProvider({
      authorizeEffects: true,
      deferIngress: true,
      effects: createDevelopmentPresentationProviderSystemEffects({
        productionSnapshot,
        assertProductionUnchanged,
        defaultDevelopmentSnapshot,
        assertDefaultDevelopmentUnchanged,
      }),
    });
  } else if (command === 'provider-scale-out' || command === 'provider-scale-in') {
    const apply = removeFlag(args, '--apply');
    rejectArgs(args);
    if (!apply) throw new Error(`${command} requires --apply`);
    const providerStatus = developmentRuntimeStatus().presentationProvider;
    if (providerStatus.ready !== true) {
      throw new Error(`${command} requires a ready development presentation provider`);
    }
    const effects = createDevelopmentPresentationLifecycleSystemEffects({
      productionSnapshot,
      assertProductionUnchanged,
      defaultDevelopmentSnapshot,
      assertDefaultDevelopmentUnchanged,
    });
    result = command === 'provider-scale-out'
      ? scaleOutDevelopmentPresentation({ effects })
      : scaleInDevelopmentPresentation({ effects });
  } else if (command === 'help' || command === '--help' || command === '-h') {
    printHelp();
    process.exit(0);
  } else {
    throw new Error(`Unknown command: ${command}`);
  }
  if (json) console.log(JSON.stringify(result, null, 2));
  else printSummary(command, result);
  if (command === 'doctor' && result.success === false) process.exitCode = 1;
  if (command === 'status' && result.ready !== true) process.exitCode = 1;
  if ((command === 'provider-scale-out' || command === 'provider-scale-in') &&
      result.success === false) process.exitCode = 1;
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}

function removeFlag(values, flag) {
  const index = values.indexOf(flag);
  if (index < 0) return false;
  values.splice(index, 1);
  return true;
}

function takeOption(values, option) {
  const index = values.indexOf(option);
  if (index < 0) return null;
  if (!values[index + 1]) throw new Error(`${option} requires a value`);
  return values.splice(index, 2)[1];
}

function rejectArgs(values) {
  if (values.length) throw new Error(`Unknown arguments: ${values.join(' ')}`);
}

function printSummary(command, result) {
  if (command === 'install') {
    console.log(`Installed development generation ${result.generation.generationId}`);
    console.log(`Dashboard: http://127.0.0.1:${result.descriptor.dashboardPort}`);
    console.log(`Production unchanged: ${result.production.unchanged}`);
  } else if (command === 'gc') {
    console.log(`Removed ${result.removed.length} unreferenced development generation(s)`);
  } else if (command === 'skill-sync' || command === 'skill-status') {
    console.log(`Development skill: ${result.state || (result.success ? 'current' : 'unavailable')}`);
    console.log(`Target: ${result.target}`);
  } else if (command === 'provider-plan') {
    console.log(`Development provider effects authorized: ${result.authorizesEffects}`);
    console.log(`Explicit apply required: ${result.requiresExplicitApply}`);
    console.log(`Planned steps: ${result.steps.length}`);
  } else if (command === 'provider-stage') {
    console.log(`Staged development provider bundle: ${result.root}`);
    console.log(`Provider effects authorized: ${result.authorizesProviderEffects}`);
  } else if (command === 'provider-status') {
    console.log(`Development provider: ${result.state}`);
    console.log(`Ready: ${result.ready}`);
    console.log(`Blocking: ${result.blocking}`);
  } else if (command === 'provider-preflight') {
    console.log(`Development provider preflight: ${result.success}`);
    console.log(`Provider effects authorized: ${result.authorizesEffects}`);
    for (const item of result.checks) {
      console.log(`${item.ok ? 'PASS' : 'FAIL'} ${item.name}: ${item.observed}`);
    }
  } else if (command === 'provider-apply') {
    console.log(`Development provider: ${result.state}`);
    console.log(`Provider ready: ${result.providerReady}`);
    console.log(`Ingress published: ${result.ingressPublished}`);
    console.log(`Production unchanged: ${result.productionUnchanged}`);
    console.log(`Receipt: ${result.receipt}`);
  } else if (command === 'provider-scale-out' || command === 'provider-scale-in') {
    console.log(`Development presentation lifecycle: ${result.state}`);
    console.log(`Route: ${result.routeId || 'none'}`);
    console.log(`Slots: ${result.beforeSlots} -> ${result.afterSlots}`);
    console.log(`Production unchanged: ${result.productionUnchanged}`);
    console.log(`Receipt: ${result.receipt}`);
  } else {
    const status = result.status || result;
    console.log(`Development runtime ready: ${status.ready}`);
    console.log(`Selected generation: ${status.selectedGeneration || 'none'}`);
    console.log(`Dashboard: http://127.0.0.1:${status.descriptor.dashboardPort}`);
    console.log(`Presentation provider: ${status.presentationProvider.state}`);
    if (result.checks) {
      for (const item of result.checks) console.log(`${item.ok ? 'PASS' : 'FAIL'} ${item.name}: ${item.observed}`);
    }
  }
}

function printHelp() {
  console.log(`Usage: node scripts/development-runtime.js <command> [options]

Commands:
  install [--binary <path>] [--no-activate]  Stage and activate an isolated development generation
  status                                    Read development runtime identity and health
  doctor                                    Validate units, executable, and manifest identity
  gc [--retain <count>]                     Remove unselected, non-running old generations
  skill-sync                                Publish the repository skill into the development pseudo-home
  skill-status                              Compare the development skill with its repository source
  provider-plan                             Print the ordered provider effect boundary without executing it
  provider-stage                            Stage secret-free development provider assets only
  provider-status                           Read development provider configuration and readiness
  provider-preflight                        Validate fresh provider admission without effects
  provider-apply --apply --defer-ingress   Provision the isolated provider and stop before ingress
  provider-scale-out --apply               Activate exactly one admitted elastic presentation slot
  provider-scale-in --apply                Reclaim exactly one unreferenced elastic presentation slot

All commands accept --json.
Optional AGENT_BROWSER_DEV_NAMESPACE selects a separate development lane.
It requires explicit DASHBOARD, BACKEND, SHADOW, LANE_STREAM, GUACAMOLE,
GUACD, and POSTGRES ports using AGENT_BROWSER_DEV_<NAME>_PORT variables.
Keep the same namespace and port environment for every operation.`);
}
