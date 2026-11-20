#!/usr/bin/env node
/** Public retained-task acceptance using the existing disposable smoke harness. */
import assert from 'node:assert/strict';
import {createHash,randomUUID} from 'node:crypto';
import {existsSync, mkdirSync, readFileSync, readdirSync} from 'node:fs';
import {isAbsolute, join} from 'node:path';
import {createSmokeContext, createMcpStdioClient, runCli} from './smoke-utils.js';

const binary=process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD;
const browser=process.env.AGENT_BROWSER_SMOKE_BROWSER_EXECUTABLE;
for(const path of [binary,browser])assert(path&&isAbsolute(path)&&existsSync(path),'Pin reviewed absolute executables');
const context=createSmokeContext({prefix:'ab-continuation-',sessionPrefix:'continuation'});
context.env=Object.fromEntries(Object.entries(context.env).filter(([key])=>!key.startsWith('AGENT_BROWSER_')&&!key.startsWith('XDG_')));
Object.assign(context.env,{AGENT_BROWSER_HOME:context.agentHome,AGENT_BROWSER_SOCKET_DIR:context.socketDir,
  AGENT_BROWSER_RUNTIME_HOST:'1',AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD:binary,AGENT_BROWSER_EXECUTABLE_PATH:browser,
  AGENT_BROWSER_ARGS:'--no-sandbox,--disable-background-networking',AGENT_BROWSER_IDLE_TIMEOUT_MS:'0',
  AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS:'0',XDG_RUNTIME_DIR:join(context.tempHome,'runtime'),
  XDG_CONFIG_HOME:join(context.tempHome,'config'),XDG_DATA_HOME:join(context.tempHome,'data')});
mkdirSync(context.env.XDG_RUNTIME_DIR,{mode:0o700});
let client,task;
const receipt={sourceBinarySha256:createHash('sha256').update(readFileSync(binary)).digest('hex'),passed:false};
const timer=setTimeout(()=>client?.rejectPending('Continuation acceptance deadline exceeded'),300000);
const start=async()=>{
  client=createMcpStdioClient({context,args:['mcp','serve'],onFatal:()=>{}});
  await client.send('initialize',{protocolVersion:'2025-06-18',capabilities:{},clientInfo:{name:'continuation-acceptance',version:'1'}});
  client.notify('notifications/initialized');
};
async function envelope(name,args){
  try {
    const result=await client.send('tools/call',{name,arguments:args});
    return JSON.parse(result.content.find(block=>block.type==='text').text);
  }catch(error){
    const encoded=error.message.slice(error.message.indexOf('{'));
    const rpc=JSON.parse(encoded);
    return {success:false,error:rpc.data?.message,failure:rpc.data?.failure};
  }
}
async function call(name,args){const reply=await envelope(name,args);assert.equal(reply.success,true,JSON.stringify(reply));return reply.data;}
const advanced=(action,params={},requestId)=>({taskId:task.taskId,action,params,...(requestId?{requestId}:{})});
const capacity=policy=>call('browser_command',advanced('task_operation',{operation:'capacity',...(policy?{policy}:{})}));
const evaluate=(script,id)=>call('browser_command',advanced('evaluate',{script},id));
const owned=()=>readdirSync('/proc').filter(pid=>/^\d+$/.test(pid)&&Number(pid)!==process.pid).flatMap(pid=>{
  try{return readFileSync(`/proc/${pid}/environ`).toString().split('\0').includes(`AGENT_BROWSER_SOCKET_DIR=${context.socketDir}`)?[Number(pid)]:[];}
  catch{return [];}
});
try {
  await start();
  const absent=`session:${'opaque'.repeat(60)}`;
  const refusal=await envelope('browser_fill',{taskId:absent,requestId:'not-dispatched',selector:'#field',value:'PRIVATE-MUST-NOT-LOG'});
  assert.equal(refusal.success,false);assert(refusal.failure.occurrenceId);assert(refusal.failure.executableNextAction);
  const log=await call('browser_command',{taskId:absent,action:'failure_log',params:{occurrenceId:refusal.failure.occurrenceId}});
  assert.equal(log.records.length,1);assert.equal(log.records[0].occurrenceId,refusal.failure.occurrenceId);
  assert(!JSON.stringify(log).includes('PRIVATE-MUST-NOT-LOG'));
  receipt.refusalCorrelation=true;
  const created=await runCli(context,['--json','runtime','create','continuation-synthetic','--browser-family','chrome']);
  assert.equal(JSON.parse(created.stdout).created,true);
  task=await call('browser_open',{profile:'continuation-synthetic',profileMode:'existing',mode:'automation',taskKey:'continuation',
    serviceName:'agent-browser-acceptance',agentName:'continuation-acceptance',taskName:'Continue synthetic task beyond retained capacity',
    url:'data:text/html,<title>Continuation</title><main>synthetic</main>'});
  const identity=[task.taskId,task.browserId,task.tabId];
  await capacity({initialCapacity:2,extensionIncrement:1,maximumCapacity:2});
  const increment='globalThis.continuationCount=(globalThis.continuationCount??0)+1';
  for(let i=0;i<260;i++){
    await evaluate(increment,`step-${i}`);
    receipt.completedOperations=i+1;
  }
  let usage=await capacity();
  assert(usage.archivedCount>=256,JSON.stringify(usage));
  client.close();await start();
  const original=await call('browser_command',advanced('task_operation',{requestId:'step-0'}));
  assert.equal(original.operation.state,'owner_acknowledged');
  const replay=await evaluate(increment,'step-0');
  assert.equal(replay.replayed,true);assert.equal(replay.dispatchAttempted,false);
  const conflict=await envelope('browser_command',advanced('evaluate',{script:'globalThis.continuationCount+=100'},'step-0'));
  assert.equal(conflict.success,false);assert.match(conflict.error,/request_conflict/);
  const counted=await evaluate('globalThis.continuationCount','verify-count');
  assert.equal(counted.result,260);
  receipt.operations=260;receipt.archivedOriginalLookup=true;receipt.duplicateProtection=true;
  for(const id of ['unknown-1','unknown-2']) {
    const failed=await envelope('browser_command',advanced('evaluate',{script:'(()=>{throw new Error("synthetic failed action")})()'},id));
    assert.equal(failed.success,false);
  }
  const full=await envelope('browser_command',advanced('evaluate',{script:'globalThis.continuationCount'},'after-ceiling'));
  assert.equal(full.success,false);assert.match(full.error,/capacity_reached/);
  const retained=await call('browser_command',advanced('task_operation',{requestId:'unknown-1'}));
  assert.equal(retained.operation.state,'uncertain');
  const occurrenceId=full.failure.occurrenceId;
  assert(occurrenceId,'capacity refusal must retain its journal occurrence');
  const recover=params=>call('browser_command',advanced('failure_recovery',params));
  const inspection=await recover({operation:'inspect',occurrenceId});
  assert.equal(inspection.failure.code,'browser_task_operation_capacity_reached');
  await recover({operation:'configure',scope:'task',actor:'synthetic-acceptance',reason:'continue the same bounded synthetic work',policy:{maxAttempts:4,maxNoProgress:3}});
  const attemptId=randomUUID();
  const remedy={tool:'browser_command',arguments:advanced('task_operation',{operation:'capacity',scope:'task',policy:{initialCapacity:2,extensionIncrement:1,maximumCapacity:3}})};
  const repair={operation:'run',occurrenceId,attemptId,remedy};
  const recovered=await recover(repair);
  assert.equal(recovered.status,'ready_for_explicit_resume',JSON.stringify(recovered));
  assert.equal(recovered.originalOperationReplayed,false);
  const recoveryReplay=await recover(repair);
  assert.equal(recoveryReplay.replayedRecoveryResult,true);
  assert.equal(recoveryReplay.attemptId,attemptId);
  const changedRepair=await envelope('browser_command',advanced('failure_recovery',{...repair,remedy:{...remedy,arguments:advanced('task_operation',{operation:'capacity',scope:'task',policy:{initialCapacity:2,extensionIncrement:1,maximumCapacity:4}})}}));
  assert.equal(changedRepair.success,false);assert.match(changedRepair.error,/attempt_conflict/);
  receipt.boundedRecovery={attemptId,occurrenceId,replayedWithoutRemedy:true,changedIntentRefused:true};
  const continued=await evaluate('globalThis.continuationCount','after-ceiling');
  assert.equal(continued.result,260);
  const report=await recover({operation:'record_resume',occurrenceId,attemptId,requestId:'after-ceiling',actor:'synthetic-acceptance'});
  assert.equal(report.ownerAcknowledgement,'verified_from_original_journal');
  assert.equal(report.taskCompletion,'unverified');
  const duplicateReport=await recover({operation:'record_resume',occurrenceId,attemptId,requestId:'after-ceiling',actor:'synthetic-acceptance'});
  assert.equal(duplicateReport.replayedResumeReport,true);
  receipt.boundedRecovery.resumeAcknowledgementVerified=true;
  const current=await call('browser_continue',{taskId:task.taskId});
  assert.deepEqual([current.taskId,current.browserId,current.tabId],identity);
  usage=await capacity();receipt.finalCapacity=usage;receipt.ceilingRecovery=true;receipt.passed=true;
}catch(error){
  receipt.error=error.message;process.exitCode=1;
  if(task)for(const action of ['mode','task_briefing']) {
    try{receipt[action]=await envelope('browser_command',advanced(action));}catch{}
  }
}
finally {
  clearTimeout(timer);
  if(task)try{await call('browser_close',{taskId:task.taskId});}catch(error){receipt.cleanupError=error.message;process.exitCode=1;}
  client?.close();
  const pidPath=join(context.socketDir,'runtime-host.pid');
  if(existsSync(pidPath)){
    const pid=Number(readFileSync(pidPath,'utf8').trim());
    if(owned().includes(pid))try{process.kill(pid,'SIGTERM');}catch(error){if(error.code!=='ESRCH')throw error;}
  }
  let residue=owned();
  for(let i=0;residue.length&&i<200;i++){await new Promise(resolve=>setTimeout(resolve,25));residue=owned();}
  receipt.remainingOwnedProcesses=residue;
  if(residue.length||!receipt.passed){receipt.retainedFixture=context.tempHome;if(residue.length)process.exitCode=1;}else context.cleanupTempHome();
  console.log(JSON.stringify(receipt,null,2));
}
