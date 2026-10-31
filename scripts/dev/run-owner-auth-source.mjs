#!/usr/bin/env node
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {preflightAuthComposition} from '../lib/auth-composition-preflight.mjs';

const help=`Usage: node scripts/dev/run-owner-auth-source.mjs [--preflight-only]

Validate explicit reviewed Auth sources, then run the five synthetic owner-auth
composition scenarios serially. --preflight-only prints sanitized JSON and runs
no source code. Required environment: AUTH_RECEIPTS_SOURCE,
AUTH_RECEIPTS_SDK_SOURCE, RECEIPTS_SOURCE. No installed source fallback is used.`;
const args=process.argv.slice(2);
try{
 if(args.length===1&&args[0]==='--help')console.log(help);
 else{
  if(args.length>1||args.length===1&&args[0]!=='--preflight-only')throw Object.assign(Error(),{code:'auth_composition_arguments_invalid'});
  const metadata=await preflightAuthComposition({source:process.env.AUTH_RECEIPTS_SOURCE,parentSource:process.env.RECEIPTS_SOURCE,sdkSource:process.env.AUTH_RECEIPTS_SDK_SOURCE});
  console.log(JSON.stringify(metadata));
  if(!args.length){
   for(const name of ['login-playbook','login-recovery','totp-continuation','number-matching','staged-factor']){
    const script=fileURLToPath(new URL(`../test-owner-${name}.js`,import.meta.url));
    const result=spawnSync(process.execPath,[script],{stdio:'inherit',env:process.env});
    if(result.error||result.signal||result.status!==0){process.exitCode=result.status||1;break;}
   }
  }
 }
}catch(error){
 console.error(JSON.stringify({code:error.code??'auth_composition_unavailable',...(error.problems?{problems:error.problems}:{})}));process.exitCode=1;
}
