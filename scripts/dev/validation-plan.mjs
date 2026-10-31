#!/usr/bin/env node
import {createValidationPlan,parseOptions,writeNewJson} from '../lib/local-validation.mjs';
const help='Usage: node scripts/dev/validation-plan.mjs --base <ref> [--head <ref>] [--timeout-ms <positive-ms>] --output <new-plan.json>\nGenerate complete selected local checks and source identities. Output must be outside the source tree. No checks execute.';
try{
 if(process.argv.slice(2).join(' ')==='--help')console.log(help);
 else{const options=parseOptions(process.argv.slice(2),['--base','--head','--output','--timeout-ms'],['--base','--output']);const plan=await createValidationPlan({base:options['--base'],head:options['--head'],timeoutMs:options['--timeout-ms']===undefined?undefined:Number(options['--timeout-ms'])});writeNewJson(options['--output'],plan);console.log(JSON.stringify({schemaVersion:plan.schemaVersion,source:plan.source,coverage:plan.coverage,checks:plan.checks.map(check=>({id:check.id,command:check.command,args:check.args}))}));if(!plan.coverage.complete)process.exitCode=1;}
}catch(error){console.error(JSON.stringify({code:error.code??'validation_plan_unavailable',...(error.problems?{problems:error.problems}:{})}));process.exitCode=1;}
