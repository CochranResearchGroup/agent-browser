import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {glob} from 'node:fs/promises';
import {join,resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createOwnerAuthFixture} from './fixtures/owner-auth-fixture.mjs';
import {createOwnerLoginPlaybook} from '../packages/client/src/owner-login-playbook.js';

const key=options=>`owner:${options.taskId}/${options.guideId}`;
async function freshProcess(f){
 const server=createServer(async(req,res)=>{
  const method=req.url==='/verify'?'verifyLogin':req.url==='/validate'?'validatePlan':'prepareLogin';
  res.setHeader('content-type','application/json');res.end(JSON.stringify(await f.browser[method](f.options)));
 });
 await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
 try{
  const files=[];for await(const file of glob('**/external-discovery.json',{cwd:f.root}))files.push(file);assert.equal(files.length,1);
  const helper=pathToFileURL(resolve('packages/client/src/hosted-auth-helper.js')).href;
  const playbook=pathToFileURL(resolve('packages/client/src/owner-login-playbook.js')).href;
  const base=`http://127.0.0.1:${server.address().port}`;
  const code=`import {readFile,writeFile} from 'node:fs/promises';import {join} from 'node:path';import {connectHostedAuthHelper} from ${JSON.stringify(helper)};import {createOwnerLoginPlaybook} from ${JSON.stringify(playbook)};
  const auth=await connectHostedAuthHelper({installationRoot:process.env.FIXTURE_SDK,discoveryFile:process.env.FIXTURE_DISCOVERY});
  const checkpoints={load:async key=>{try{return JSON.parse(await readFile(join(process.env.FIXTURE_STORE,encodeURIComponent(key)+'.json'),'utf8'));}catch(e){if(e.code==='ENOENT')return null;throw e;}},save:async(key,value)=>writeFile(join(process.env.FIXTURE_STORE,encodeURIComponent(key)+'.json'),JSON.stringify(value),{mode:0o600})};
  const browser=Object.fromEntries([['verifyLogin','verify'],['prepareLogin','prepare'],['validatePlan','validate']].map(([method,route])=>[method,async()=>{const r=await fetch(process.env.FIXTURE_BROWSER+'/'+route);return r.json();}]));
  console.log(JSON.stringify(await createOwnerLoginPlaybook({browser,auth,checkpoints}).advance(JSON.parse(process.env.FIXTURE_OPTIONS))));`;
  const child=await promisify(execFile)(process.execPath,['--input-type=module','-e',code],{env:{...process.env,CODEX_THREAD_ID:'synthetic-caller',FIXTURE_SDK:join(f.root,'sdk'),FIXTURE_DISCOVERY:join(f.root,files[0]),FIXTURE_STORE:join(f.root,'checkpoints'),FIXTURE_BROWSER:base,FIXTURE_OPTIONS:JSON.stringify(f.options)},timeout:10000});
  return JSON.parse(child.stdout.trim());
 }finally{await new Promise(resolve=>server.close(resolve));}
}

// A crash after durable intent does not prove whether the next call began.
const intent=await createOwnerAuthFixture();
try{
 const pending=await createOwnerLoginPlaybook(intent).advance(intent.options);intent.respond();
 await intent.auth.helper('receive_credential_approval',{requestId:pending.requestId});
 const checkpoint=await intent.checkpoints.load(key(intent.options));
 assert.equal(checkpoint.issuer,'auth-receipts');assert.equal(checkpoint.callerTaskId,intent.auth.callerTaskId);assert.equal(checkpoint.receiving.generation,'generation-1');assert.equal(checkpoint.preparedPlan.guideDigest,'reviewed-guide');
 await intent.checkpoints.save(key(intent.options),{...checkpoint,advanceIntent:true});
 await intent.restart();
 const resumed=await freshProcess(intent);
 assert.equal(resumed.reasonCode,'auth_outcome_uncertain');assert.equal(resumed.requestId,pending.requestId);
 assert.equal(intent.state.reads,0);assert.equal(intent.state.inputs,0);assert.equal(intent.state.prompts.length,1);
 console.log('Fresh process retains original uncertain dispatch after rotated Auth discovery without inferring no effect');
}finally{await intent.close();}

// A lost response after actual use is reconciled by another OS process.
const lost=await createOwnerAuthFixture();
try{
 const login=createOwnerLoginPlaybook(lost),pending=await login.advance(lost.options);lost.respond();
 const auth={...lost.auth,async advanceLogin(args){await lost.auth.advanceLogin(args);throw Object.assign(Error('Synthetic lost public acknowledgement'),{code:'auth_outcome_uncertain'});}};
 assert.equal((await createOwnerLoginPlaybook({...lost,auth}).advance(lost.options)).effectState,'unknown');
 assert.equal(lost.state.inputs,1);assert.equal(lost.state.reads,1);
 await lost.restart();
 const done=await freshProcess(lost);assert.equal(done.status,'completed');assert.equal(done.requestId,pending.requestId);
 assert.equal(lost.state.inputs,1);assert.equal(lost.state.reads,1);assert.equal(lost.state.prompts.length,1);
 assert.ok(!JSON.stringify(done).includes('SYNTHETIC-PASSWORD-ONLY'));
 console.log('Lost public acknowledgement and Auth restart reconcile the original actual-source request without another use');
}finally{await lost.close();}

const uncertain=await createOwnerAuthFixture({configure:({state,receiver})=>{
 receiver.use=async({authorize})=>{await authorize();state.inputs++;return {released:true};};
 receiver.reconcile=async()=>state.authenticated?{released:true,authenticated:true}:{released:true};
}});
try{
 const login=createOwnerLoginPlaybook(uncertain),pending=await login.advance(uncertain.options);uncertain.respond();
 assert.equal((await login.advance(uncertain.options)).effectState,'unknown');
 await uncertain.restart();
 const held=await freshProcess(uncertain);assert.equal(held.status,'waiting');assert.equal(held.requestId,pending.requestId);assert.equal(uncertain.state.inputs,1);
 uncertain.state.authenticated=true;
 assert.equal((await freshProcess(uncertain)).status,'completed');assert.equal(uncertain.state.inputs,1);assert.equal(uncertain.state.reads,1);
}finally{await uncertain.close();}

const concurrent=await createOwnerAuthFixture();
try{
 const pending=await createOwnerLoginPlaybook(concurrent).advance(concurrent.options);concurrent.respond();
 await concurrent.auth.helper('receive_credential_approval',{requestId:pending.requestId});
 const outcomes=await Promise.all([freshProcess(concurrent),freshProcess(concurrent)]);
 assert.ok(outcomes.some(outcome=>outcome.status==='completed'));assert.ok(outcomes.every(outcome=>outcome.requestId===pending.requestId));
 assert.equal(concurrent.state.reads,1);assert.equal(concurrent.state.inputs,1);assert.equal(concurrent.state.prompts.length,1);
 assert.equal((await freshProcess(concurrent)).status,'completed');assert.equal(concurrent.state.inputs,1);
 console.log('Concurrent fresh orchestrators preserve one original Auth attempt and one private use');
}finally{await concurrent.close();}

const save=await createOwnerAuthFixture();
try{
 const pending=await createOwnerLoginPlaybook(save).advance(save.options);save.respond();await save.auth.helper('receive_credential_approval',{requestId:pending.requestId});
 const checkpoints={load:save.checkpoints.load,save:async()=>{throw Error('Synthetic durable write failure');}};
 await assert.rejects(createOwnerLoginPlaybook({...save,checkpoints}).advance(save.options),/Synthetic durable write failure/);
 assert.equal(save.state.reads,0);assert.equal(save.state.inputs,0);
 assert.equal((await createOwnerLoginPlaybook(save).advance(save.options)).status,'completed');assert.equal(save.state.inputs,1);
}finally{await save.close();}

const refused=await createOwnerAuthFixture();
try{
 const pending=await createOwnerLoginPlaybook(refused).advance(refused.options);refused.respond();await refused.auth.helper('receive_credential_approval',{requestId:pending.requestId});
 const auth={...refused.auth,async advanceLogin(){throw Object.assign(Error('Synthetic pre-input hold'),{code:'auth_capture_held'});}};
 assert.equal((await createOwnerLoginPlaybook({...refused,auth}).advance(refused.options)).reasonCode,'auth_capture_held');
 const current=await refused.auth.helper('inspect_credential_request',{requestId:pending.requestId});assert.equal(current.state,'approved');assert.equal(current.attemptId,undefined);
 const held=await freshProcess(refused);assert.equal(held.reasonCode,'auth_no_effect_proof_unavailable');assert.equal(held.requestId,pending.requestId);
 assert.equal(refused.state.reads,0);assert.equal(refused.state.inputs,0);
}finally{await refused.close();}

const legacy=await createOwnerAuthFixture();
try{
 const original={binding:'legacy-original-binding',requestId:'legacy-request',attempted:true,callerTaskId:'legacy-caller',receivingTarget:{provider:'agent-browser',taskId:'browser-task',profileId:'profile',browserId:'browser',tabId:'tab'}};
 const originalKey=`${legacy.options.taskId}/${legacy.options.guideId}`;await legacy.checkpoints.save(originalKey,original);
 const held=await freshProcess(legacy);assert.equal(held.reasonCode,'legacy_checkpoint_requires_reconciliation');assert.equal(held.effectState,'unknown');
 assert.deepEqual(await legacy.checkpoints.load(originalKey),original);assert.equal(await legacy.checkpoints.load(key(legacy.options)),null);
 assert.equal(legacy.state.prompts.length,0);assert.equal(legacy.state.reads,0);assert.equal(legacy.state.inputs,0);
 console.log('Legacy custody and typed refusals without positive no-effect proof remain held without replacement requests');
}finally{await legacy.close();}

const readback=await createOwnerAuthFixture();
try{
 const result=await createOwnerLoginPlaybook(readback).advance({...readback.options,reconcile:true});
 assert.equal(result.reasonCode,'checkpoint_required_for_reconciliation');assert.equal(readback.state.prompts.length,0);assert.equal(readback.state.reads,0);
 const login=createOwnerLoginPlaybook(readback),pending=await login.advance(readback.options);readback.respond();await readback.auth.helper('receive_credential_approval',{requestId:pending.requestId});
 const observed=await login.advance({...readback.options,reconcile:true});assert.equal(observed.reasonCode,'auth_reconciliation_pending');assert.equal(observed.requestId,pending.requestId);assert.equal(readback.state.inputs,0);assert.equal(readback.state.reads,0);
 assert.equal((await login.advance(readback.options)).status,'completed');assert.equal(readback.state.inputs,1);
}finally{await readback.close();}
