import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,writeFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {qualifyHostedBrowserPolicy} from './smoke-hosted-browser-policy.mjs';

async function fixture({unknownOverride=false}={}){
 const artifactRoot=await mkdtemp(join(tmpdir(),'ab-policy-oracle-'));
 const task={taskId:'owner',browserId:'shared',tabId:'owner-logical'},peerTask={taskId:'peer',browserId:'shared',tabId:'peer-logical'};
 const calls=[],receipts=[];let revision=0,denied=false,value='original draft',grant;
 const reject=code=>{throw Object.assign(Error(code),{code});};
 const workflow=async(name,args)=>{
  calls.push({name,args:structuredClone(args)});
  if(name==='browser_policy'){
   if(args.operation==='status')return {revision};
   assert.equal(args.expectedRevision,revision);
   if(args.operation==='set_task'){denied=true;revision++;return {revision};}
   if(args.operation==='override'){
    if(unknownOverride)throw Object.assign(Error('Unknown override delivery; no replay'),{code:'transport_unknown'});
    grant={requestId:args.requestId,token:'scoped-token'};revision++;return {revision,overrideId:grant.token};
   }
   if(args.operation==='clear_task'){denied=false;revision++;return {revision};}
  }
  if(name==='browser_command'&&args.action==='evaluate')return {result:{marker:true,value,url:'https://example.test/synthetic'}};
  if(name==='browser_fill'||(name==='browser_command'&&args.action==='fill')){
   if(args.taskId===task.taskId&&denied&&!(grant&&args.requestId===grant.requestId&&args.policyOverrideId===grant.token))return reject('browser_policy_action_denied');
   value=args.value??args.params.value;return {};
  }
  if(name==='browser_artifact'){
   if(args.name.includes('..'))return reject('browser_artifact_name_invalid');
   const path=join(artifactRoot,args.name);await writeFile(path,Buffer.from([137,80,78,71,13,10,26,10,1]));
   return {path,handle:args.handle,name:args.name,artifactId:args.requestId,publication:'not_requested'};
  }
  if(name==='browser_close')return reject('browser_lifecycle_borrowed_process_preserved');
  throw Error('Unexpected fixture dispatch '+name);
 };
 return {artifactRoot,task,peerTask,workflow,calls,receipts,save:async(_,receipt)=>receipts.push(structuredClone(receipt)),cleanup:()=>rm(artifactRoot,{recursive:true,force:true})};
}

test('policy acceptance oracle recognizes the full known-good public workflow sequence',async()=>{
 const context=await fixture();try{
  const receipt=await qualifyHostedBrowserPolicy(context);
  assert.equal(receipt.passed,true);assert.equal(receipt.checks.length,10);
  assert.equal(receipt.publication,'not_requested');
  assert.equal(context.calls.filter(call=>call.name==='browser_close').length,1);
  assert.equal(context.calls.filter(call=>call.name==='browser_artifact').length,2);
  assert.equal(context.receipts.at(-1).passed,true);
 }finally{await context.cleanup();}
});

test('unknown override delivery stops the oracle without replay, artifacts or a successful receipt',async()=>{
 const context=await fixture({unknownOverride:true});try{
  await assert.rejects(qualifyHostedBrowserPolicy(context),error=>error.code==='transport_unknown');
  const overrides=context.calls.filter(call=>call.name==='browser_policy'&&call.args.operation==='override');assert.equal(overrides.length,1);
  assert(!context.calls.some(call=>call.name==='browser_artifact'||call.name==='browser_close'));
  assert(!context.calls.some(call=>call.name==='browser_command'&&call.args.action==='fill'));
  assert(!context.calls.some(call=>call.name==='browser_policy'&&call.args.operation==='clear_task'));
  assert(!context.receipts.some(receipt=>receipt.passed));
 }finally{await context.cleanup();}
});
