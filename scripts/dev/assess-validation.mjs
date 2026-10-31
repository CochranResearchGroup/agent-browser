#!/usr/bin/env node
import {readFileSync} from 'node:fs';
import {assessValidation,parseOptions} from '../lib/local-validation.mjs';
const help='Usage: node scripts/dev/assess-validation.mjs --plan <plan.json> --receipt <receipt.json> [--json]\nRead-only completeness and current-source assessment. Never executes checks or grants merge/runtime authority.';
try{
 const args=process.argv.slice(2);if(args.join(' ')==='--help')console.log(help);
 else{const json=args.includes('--json'),options=parseOptions(args.filter(arg=>arg!=='--json'),['--plan','--receipt'],['--plan','--receipt']);const result=await assessValidation(JSON.parse(readFileSync(options['--plan'],'utf8')),JSON.parse(readFileSync(options['--receipt'],'utf8')));console.log(json?JSON.stringify(result):`Local validation: ${result.ready?'ready':'not ready'}\n${result.problems.join('\n')}`);if(!result.ready)process.exitCode=1;}
}catch(error){console.error(JSON.stringify({code:error.code??'validation_assessment_unavailable'}));process.exitCode=1;}
