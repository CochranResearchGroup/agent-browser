/** Optional synthetic-only extension for an already allocated hosted fixture.
 * Imports cause no effects. The caller owns provider/runtime/QA cleanup.
 * send is the existing raw MCP request method (known refusals must not be
 * relabeled as ambiguous transport outcomes). runCli accepts argv and returns
 * {stdout,stderr}; sessionName is the exact retained task lane, never a guess.
 * serviceRequest, when supplied, dispatches one actual isolated HTTP request.
 */
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';

function payload(reply) {
 const value=reply.structuredContent??JSON.parse(reply.content.find(block=>block.type==='text').text);
 if(reply.isError||value.success===false)throw Object.assign(Error('Public workflow rejected: '+codes(JSON.stringify(value)).join(',')),{code:'fixture_workflow_rejected'});
 return value.data??value;
}
function codes(value){return [...new Set(String(value).match(/\bbrowser_control_[a-z0-9_]+\b/g)??[])];}

export async function qualifyHostedBrowserControl({send,runCli,task,sessionName,readTask,save,serviceRequest,syntheticOnly=false}) {
 assert.equal(syntheticOnly,true,'Control acceptance may modify only a prepared synthetic page');
 assert(task?.taskId&&task.browserId&&task.tabId&&sessionName);
 const receipt={version:1,taskId:task.taskId,browserId:task.browserId,tabId:task.tabId,checks:[],standaloneHttp:serviceRequest?'pending':'not_exercised',inFlightRace:'provider_free_concurrent_dispatch_test'};
 const record=async(name,detail={})=>{receipt.checks.push({name,...detail});await save('control-acceptance.json',receipt);};
 const call=async(name,args)=>payload(await send('tools/call',{name,arguments:args}));
 const control=async(operation,extra={})=>(await call('browser_control',{taskId:task.taskId,operation,...extra})).control;
 const evaluate=script=>call('browser_command',{taskId:task.taskId,action:'evaluate',params:{script}});
 async function denied(name,invoke,expected) {
  let response,error;
  try{response=await invoke();}catch(failure){error=failure;}
  const found=codes(error?.message??JSON.stringify(response));
  assert(found.includes(expected),`${name} must reject with ${expected}; received codes=${found.join(',')}`);
  await record(name,{codes:found});
 }
 const marker='hostedControl'+randomUUID().replaceAll('-','');
 let current=await control('status');
 assert.equal(current.owner,'agent');const initialEpoch=current.epoch;
 const identity=snapshot=>[snapshot.taskId,snapshot.browserId,snapshot.profileId,snapshot.tabId];
 const before=await readTask(task.taskId);
 const initial=await evaluate(`globalThis.${marker}=0`);assert.equal(initial.result,0);
 await control('view');
 assert.equal((await control('status')).epoch,initialEpoch);
 const viewed=await evaluate(`++globalThis.${marker}`);assert.equal(viewed.result,1);
 await record('view-only-does-not-pause');
 let returnDispatched=false,human=false;
 try{
  current=await control('takeover',{expectedEpoch:current.epoch,actor:'synthetic-operator',reason:'Qualify explicit human takeover'});human=true;
  assert.equal(current.owner,'human');assert.equal(current.pauseReads,false);await record('explicit-takeover',{epoch:current.epoch});
  await denied('mcp-mutation-blocked',()=>evaluate(`globalThis.${marker}=9001`),'browser_control_human');
  await denied('cli-mutation-blocked',()=>runCli(['--json','--session',sessionName,'eval',`globalThis.${marker}=9002`]),'browser_control_human');
  await call('browser_snapshot',{taskId:task.taskId});await record('default-observation-allowed');
  // Fresh CLI process proves the state is durable and shared with the MCP client.
  const other=JSON.parse((await runCli(['--json','mcp','host-control',task.taskId,'status'])).stdout);
  assert.equal(other.success,true);assert.equal(other.data.control.owner,'human');assert.equal(other.data.control.epoch,current.epoch);
  await record('fresh-cli-process-preserves-takeover');
  if(serviceRequest){
   await denied('http-service-mutation-blocked',()=>serviceRequest({action:'evaluate',sessionName,browserId:task.browserId,serviceName:'agent-browser-fixture',agentName:'hosted-control-acceptance',taskName:'Verify takeover rejects new service effects',params:{script:`globalThis.${marker}=9003`}}),'browser_control_human');
   receipt.standaloneHttp='passed';
  }
  returnDispatched=true;current=await control('return',{expectedEpoch:current.epoch,actor:'synthetic-operator',reason:'Fresh observation before continuing'});human=false;returnDispatched=false;
  assert.equal(current.owner,'agent');assert.deepEqual(identity(await readTask(task.taskId)),identity(before));
  const resumed=await evaluate(`globalThis.${marker}`);assert.equal(resumed.result,1,'Rejected mutations must not alter the synthetic page');
  await record('explicit-return-observes-same-task-and-page',{epoch:current.epoch});
  current=await control('takeover',{expectedEpoch:current.epoch,actor:'synthetic-operator',reason:'Qualify optional page read and capture pause',pauseReads:true});human=true;
  assert.equal(current.pauseReads,true);
  await denied('paused-read-blocked',()=>call('browser_snapshot',{taskId:task.taskId}),'browser_control_observation_paused');
  await denied('paused-capture-blocked',()=>call('browser_screenshot',{taskId:task.taskId}),'browser_control_observation_paused');
  assert.equal((await control('status')).owner,'human');await record('status-remains-available-during-read-pause');
  returnDispatched=true;current=await control('return',{expectedEpoch:current.epoch,actor:'synthetic-operator',reason:'Return after capture pause'});human=false;returnDispatched=false;
  assert.equal(current.owner,'agent');
  const final=await evaluate(`globalThis.${marker}`);assert.equal(final.result,1);
  assert.deepEqual(identity(await readTask(task.taskId)),identity(before));
  receipt.passed=true;await record('final-identity-and-page-continuity',{epoch:current.epoch});return receipt;
 }finally{
  // A known retained takeover may be explicitly returned for fixture cleanup.
  // Never repeat an already dispatched return whose outcome was uncertain.
  if(human&&!returnDispatched){
   returnDispatched=true;
   try{await control('return',{expectedEpoch:current.epoch,actor:'synthetic-operator',reason:'Synthetic acceptance cleanup'});receipt.cleanupReturn='completed';}
   catch(error){receipt.cleanupReturn='requires_readback';receipt.cleanupCodes=codes(error.message);}
   await save('control-acceptance.json',receipt);
  }
 }
}
