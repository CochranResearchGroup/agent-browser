#!/usr/bin/env node

import { resolve } from 'node:path';
import {
  developmentRuntimeStatus,
  doctorDevelopmentRuntime,
  garbageCollectDevelopmentRuntime,
  installDevelopmentRuntime,
} from './lib/development-runtime.js';
import {
  developmentAgentSkillStatus,
  synchronizeDevelopmentAgentSkill,
} from './lib/development-presentation-provider.js';

const args = process.argv.slice(2).filter((arg) => arg !== '--');
const command = args.shift() || 'status';
const json = removeFlag(args, '--json');

try {
  let result;
  if (command === 'install') {
    const binary = takeOption(args, '--binary') || resolve('cli/target/release/agent-browser');
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

All commands accept --json.`);
}
