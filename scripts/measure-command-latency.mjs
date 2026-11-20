#!/usr/bin/env node
/** Paired synthetic reads through addressed workflow MCP and exact-target CDP.
 * A fresh CLI MCP facade is a separate diagnostic on that same managed task.
 * No installed
 * runtime, private profile or external site is used. Trusted elapsed intervals
 * come from StableTime service observations; guest clocks are not substituted.
 */
import assert from 'node:assert/strict';
import {createHash, randomUUID} from 'node:crypto';
import {readFileSync, readdirSync, realpathSync, existsSync, writeFileSync, unlinkSync, mkdirSync} from 'node:fs';
import {join, isAbsolute} from 'node:path';
import {createSmokeContext, createMcpStdioClient, runCli, parseJsonOutput} from './smoke-utils.js';

const binary=process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD;
const browser=process.env.AGENT_BROWSER_SMOKE_BROWSER_EXECUTABLE;
const clockConfig=process.env.AGENT_BROWSER_LATENCY_STABLE_TIME_CLIENT;
for(const [name,value] of Object.entries({binary,browser,clockConfig}))assert(value&&isAbsolute(value)&&existsSync(value),`Pin an existing absolute ${name} path`);
const samples=Number(process.env.AGENT_BROWSER_LATENCY_SAMPLES??3);
assert(Number.isSafeInteger(samples)&&samples>=3&&samples<=20,'Use 3 to 20 paired samples');
const context=createSmokeContext({prefix:'ab-command-latency-',sessionPrefix:'latency'});
const environment=Object.fromEntries(Object.entries(context.env).filter(([key])=>!key.startsWith('AGENT_BROWSER_')&&!key.startsWith('XDG_')));
Object.assign(environment,{
 AGENT_BROWSER_HOME:context.agentHome,AGENT_BROWSER_SOCKET_DIR:context.socketDir,
 AGENT_BROWSER_RUNTIME_HOST:'1',AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD:binary,
 AGENT_BROWSER_EXECUTABLE_PATH:browser,AGENT_BROWSER_ARGS:'--no-sandbox,--disable-background-networking',
 AGENT_BROWSER_IDLE_TIMEOUT_MS:'0',AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS:'0',
 AGENT_BROWSER_LATENCY_STABLE_TIME_CLIENT:clockConfig,
 XDG_RUNTIME_DIR:join(context.tempHome,'runtime'),XDG_CONFIG_HOME:join(context.tempHome,'config'),XDG_DATA_HOME:join(context.tempHome,'data'),
});
for(const key of ['AGENT_BROWSER_LATENCY_PHASE','AGENT_BROWSER_LATENCY_STABLE_TIME_BINARY'])if(process.env[key])environment[key]=process.env[key];
mkdirSync(environment.XDG_RUNTIME_DIR,{mode:0o700});context.env=environment;
const profileName='latency-synthetic';
const ownedProcesses=()=>readdirSync('/proc').filter(pid=>/^\d+$/.test(pid)&&Number(pid)!==process.pid).flatMap(pid=>{
 try{
  const command=readFileSync(`/proc/${pid}/cmdline`).toString();
  const environment=readFileSync(`/proc/${pid}/environ`).toString().split('\0');
  if(!command.includes(context.tempHome)&&!environment.includes(`AGENT_BROWSER_SOCKET_DIR=${context.socketDir}`))return [];
  return [{pid:Number(pid),startToken:readFileSync(`/proc/${pid}/stat`,'utf8').split(')').at(-1).trim().split(/\s+/)[19],executable:realpathSync(`/proc/${pid}/exe`)}];
 }catch(error){if(['ENOENT','ESRCH','EACCES'].includes(error.code))return [];throw error;}
});
const cli=async args=>parseJsonOutput((await runCli(context,['--json',...args])).stdout,'latency command');
const createWorkflowClient=()=>createMcpStdioClient({context,args:['mcp','serve'],onFatal:()=>{}});
const workflow=createWorkflowClient();
async function initialize(client){
 await client.send('initialize',{protocolVersion:'2025-06-18',capabilities:{},clientInfo:{name:'latency-acceptance',version:'1'}});
 client.notify('notifications/initialized');
}
async function call(client,name,args){
 const result=await client.send('tools/call',{name,arguments:args});
 assert.equal(result.isError,false,'Workflow MCP returned an error');
 const envelope=JSON.parse(result.content.find(block=>block.type==='text').text);
 assert.equal(envelope.success,true,'Workflow MCP operation failed');return envelope.data;
}
const clockContext={env:{...process.env,AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD:process.env.AGENT_BROWSER_LATENCY_STABLE_TIME_BINARY??'stable-time'}};
const clock=createMcpStdioClient({context:clockContext,args:['mcp',clockConfig],onFatal:()=>{}});
let socket,hostPid,task;
const receipt={schemaVersion:'agent-browser.command-latency.v1',binarySha256:createHash('sha256').update(readFileSync(binary)).digest('hex'),browser:realpathSync(browser),samples:[],selectedPhase:process.env.AGENT_BROWSER_LATENCY_PHASE??null,phaseMeasurements:[],clock:'StableTime service UTC intervals at measurement instant',limitation:'Service-to-service brackets include clock delivery/request overhead. Actual operation elapsed time has only an upper bound. Persistent addressed workflow MCP is the primary comparison; fresh CLI MCP facade startup is a separate diagnostic. Cold managed open is separate. This fresh synthetic state does not reproduce historical retained-state size or private account load.'};
// Keep operation provenance: warm addressed page tools send directly and do
// not run ensure_daemon/readiness. Cold open and continue do run that phase.
function collectPhases(value,operation){
 for(const sample of value.latencyPhases??[])if(sample.phase===receipt.selectedPhase)
  receipt.phaseMeasurements.push({...sample,operation});
}
const timestamp=value=>{
 const match=/^(.*\.)(\d{1,9})Z$/.exec(value);assert(match,'RFC3339 nanosecond UTC required');
 return BigInt(Date.parse(match[1]+'000Z'))*1_000_000n+BigInt(match[2].padEnd(9,'0'));
};
async function time(){
 const reply=await clock.send('tools/call',{name:'get_time',arguments:{}});
 assert(!reply.isError,'StableTime client failure');
 const result=JSON.parse(reply.content.find(block=>block.type==='text').text);
 assert(['qualified_live','qualified_holdover'].includes(result.status)&&result.bound==='finite','Reliable StableTime unavailable');
 return result;
}
async function measure(operation,run,verify=()=>{}){
 const before=await time();const value=await run();const after=await time();
 assert.equal(before.epoch,after.epoch,'StableTime epoch changed; discard sample');
 const lower=timestamp(after.estimate.earliest_utc)-timestamp(before.estimate.latest_utc);
 const upper=timestamp(after.estimate.latest_utc)-timestamp(before.estimate.earliest_utc);
 assert(upper>=0n,'StableTime intervals moved backwards; discard sample');
 const sample={operation,lowerMs:Number(lower>0n?lower:0n)/1e6,upperMs:Number(upper)/1e6,intervalAppliesTo:'service-observation span including delivery',operationElapsedIntervalMs:[0,Number(upper)/1e6],before:before.estimate,after:after.estimate};
 receipt.samples.push(sample);verify(value);return value;
}
function summarize(operation){
 const rows=receipt.samples.filter(row=>row.operation===operation);
 const quantile=(key,p)=>[...rows].sort((a,b)=>a[key]-b[key])[Math.ceil(rows.length*p)-1]?.[key]??null;
 return {n:rows.length,medianServiceBracketIntervalMs:[quantile('lowerMs',.5),quantile('upperMs',.5)],p95ServiceBracketIntervalMs:[quantile('lowerMs',.95),quantile('upperMs',.95)],medianOperationElapsedIntervalMs:[0,quantile('upperMs',.5)],p95OperationElapsedIntervalMs:[0,quantile('upperMs',.95)]};
}
try{
 await clock.send('initialize',{protocolVersion:'2024-11-05',capabilities:{},clientInfo:{name:'ab-latency',version:'1'}});
 await initialize(workflow);
 const created=await cli(['runtime','create',profileName,'--browser-family','chrome']);
 assert.equal(created.created,true);assert(isAbsolute(created.userDataDir));
 assert(realpathSync(created.userDataDir).startsWith(realpathSync(context.tempHome)+'/'),'Synthetic profile escaped its owned namespace');
 const url='data:text/html,<title>Latency sample</title><main>synthetic latency sample</main>';
 task=await measure('cold_workflow_open',()=>call(workflow,'browser_open',{
  profile:profileName,profileMode:'existing',taskKey:'latency-synthetic',mode:'automation',
  serviceName:'agent-browser-latency',agentName:'latency-acceptance',taskName:'Read synthetic page for paired command measurement',url,
 }));
 assert.equal(typeof task.taskId,'string');assert.equal(task.profileId,profileName);assert.equal(task.url,url);
 collectPhases(task,'cold_workflow_open');
 const briefing=await call(workflow,'browser_command',{taskId:task.taskId,action:'task_briefing'});
 assert.equal(briefing.browserId,task.browserId);assert.equal(briefing.tabId,task.tabId);
 hostPid=Number(readFileSync(join(context.socketDir,'runtime-host.pid'),'utf8').trim());
 const [port]=readFileSync(join(created.userDataDir,'DevToolsActivePort'),'utf8').trim().split('\n');
 const pages=await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
 const page=pages.find(page=>page.id===briefing.targetId&&page.type==='page'&&page.url===url);assert(page,'Exact managed synthetic target missing');
 const browserIdentity=()=>({executable:realpathSync(`/proc/${briefing.processPid}/exe`),startToken:readFileSync(`/proc/${briefing.processPid}/stat`,'utf8').split(')').at(-1).trim().split(/\s+/)[19]});
 const originalIdentity=browserIdentity();assert.equal(originalIdentity.executable,realpathSync(browser));
 if(receipt.selectedPhase==='readiness'){
  receipt.phaseCoverage={warmAddressedEvaluate:{status:'not_run',reason:'task_operations::send uses existing transport directly; no readiness phase runs on this path'},coldWorkflowOpen:{status:'collected_when_returned'},warmWorkflowContinue:{status:'diagnostic_requested'}};
  const continued=await measure('warm_workflow_readiness_probe',()=>call(workflow,'browser_continue',{taskId:task.taskId}));
  for(const [key,value] of Object.entries({taskId:task.taskId,browserId:task.browserId,tabId:task.tabId,targetId:briefing.targetId}))assert.equal(continued[key],value,'Readiness diagnostic changed retained task identity');
  assert.equal(continued.url,url);assert.equal(continued.urlObserved,true);assert.deepEqual(browserIdentity(),originalIdentity);
  collectPhases(continued,'warm_workflow_readiness_probe');
 }

 assert(readFileSync(`/proc/${briefing.processPid}/cmdline`).toString().includes(created.userDataDir),'Current synthetic browser does not own the selected profile');
 receipt.target={taskId:task.taskId,browserId:task.browserId,tabId:task.tabId,targetId:page.id,processPid:briefing.processPid,...originalIdentity};
 socket=new WebSocket(page.webSocketDebuggerUrl);
 await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
 let nextId=0;const pending=new Map();
 socket.addEventListener('message',event=>{const value=JSON.parse(event.data);const callback=pending.get(value.id);if(callback){pending.delete(value.id);value.error?callback.reject(new Error('CDP evaluation failed')):callback.resolve(value.result);}});
 const expression='document.body.firstElementChild.textContent';
 const direct=(script=expression)=>new Promise((resolve,reject)=>{const id=++nextId;pending.set(id,{resolve,reject});socket.send(JSON.stringify({id,method:'Runtime.evaluate',params:{expression:script,returnByValue:true,awaitPromise:true,timeout:24750}}));});
 const expected='synthetic latency sample';
 // Public page arguments contain only script. The owned browser's default
 // 25,000ms caller budget maps to the same 24,750ms renderer budget above.
 const evaluate=()=>({taskId:task.taskId,requestId:`latency-${randomUUID()}`,action:'evaluate',params:{script:expression}});
 const verify=(value,operation='workflow_mcp')=>{assert.equal(value.result,expected);assert.deepEqual(browserIdentity(),originalIdentity);collectPhases(value,operation);return value;};
 // Diagnostics are response-only. A changed sample must not change request identity.
 const replayRequest=evaluate();
 const first=await call(workflow,'browser_command',replayRequest);
 assert.equal(first.dispatchAttempted,true);
 const replay=await call(workflow,'browser_command',replayRequest);
 assert.equal(replay.replayed,true);
 assert.equal(replay.dispatchAttempted,false);
 receipt.sameRequestReplay={replayed:true,dispatchAttempted:false,selectedPhase:receipt.selectedPhase};
 const agent=()=>call(workflow,'browser_command',evaluate());
 const freshCli=async()=>{const client=createWorkflowClient();try{await initialize(client);return await call(client,'browser_command',evaluate());}finally{client.close();}};
 const cdp=()=>direct();
 const verifyCdp=(value,expectedValue=expected)=>{assert(!value.exceptionDetails);assert.equal(value.result.value,expectedValue);assert.deepEqual(browserIdentity(),originalIdentity);};
 const continueTask=()=>call(workflow,'browser_continue',{taskId:task.taskId});
 const verifyContinuation=value=>{
  for(const [key,expectedValue] of Object.entries({taskId:task.taskId,browserId:task.browserId,tabId:task.tabId,targetId:briefing.targetId}))assert.equal(value[key],expectedValue,'Continuation changed retained task identity');
  assert.equal(value.url,url);assert.equal(value.urlObserved,true);assert.deepEqual(browserIdentity(),originalIdentity);collectPhases(value,'workflow_mcp_continue');
 };
 const directUrl=()=>direct('location.href');
 verify(await agent());verifyCdp(await cdp());
 verifyContinuation(await continueTask());verifyCdp(await directUrl(),url);
 const startupLock=join(context.socketDir,'runtime-host.startup.lock');
 async function heldStartupLockProbe(run,verifyResult){
  const token=`latency-probe-${randomUUID()}`;
  writeFileSync(startupLock,token,{flag:'wx',mode:0o600});
  let completed=false,completedBeforeStartupLockRelease=false;
  const outcome=run().then(value=>{completed=true;return {value};},error=>({error}));
  // Incidental OS coordination only, not trusted elapsed-time evidence.
  try{await new Promise(resolve=>setTimeout(resolve,1000));completedBeforeStartupLockRelease=completed;}
  finally{assert.equal(readFileSync(startupLock,'utf8'),token);unlinkSync(startupLock);}
  const result=await outcome;if(result.error)throw result.error;verifyResult(result.value);
  return {completedBeforeStartupLockRelease,observation:'OS coordination check; no latency bound or delivery guarantee'};
 }
 receipt.heldStartupLockEvaluateProbe={...await heldStartupLockProbe(agent,verify),callPath:'existing task_operations transport; does not call ensure_daemon',causalForWarmStartupLockOptimization:false};
 // Retain the old field for receipt consumers, explicitly scoped to evaluate.
 receipt.warmReuseCompletedBeforeStartupLockRelease=receipt.heldStartupLockEvaluateProbe.completedBeforeStartupLockRelease;
 receipt.heldStartupLockContinuationProbe={...await heldStartupLockProbe(continueTask,verifyContinuation),callPath:'workflow::continue_task -> ensure_transport -> ensure_daemon_inner',causalForWarmStartupLockOptimization:true};
 for(let i=0;i<samples;i++){
  const order=i%2?[['workflow_mcp',agent,verify],['direct_cdp',cdp,verifyCdp]]:[['direct_cdp',cdp,verifyCdp],['workflow_mcp',agent,verify]];
  for(const [name,run,check] of order)await measure(name,run,check);
  await measure('fresh_cli_mcp_facade',freshCli,value=>verify(value,'fresh_cli_mcp_facade'));
  const continuationOrder=i%2?[['workflow_mcp_continue',continueTask,verifyContinuation],['direct_cdp_url',directUrl,value=>verifyCdp(value,url)]]:[['direct_cdp_url',directUrl,value=>verifyCdp(value,url)],['workflow_mcp_continue',continueTask,verifyContinuation]];
  for(const [name,run,check] of continuationOrder)await measure(name,run,check);
 }
 receipt.pairedSemantics={evaluate:{expression,returnByValue:true,awaitPromise:true,rendererTimeoutMs:24750},continuation:{expression:'location.href',returnByValue:true,awaitPromise:true,rendererTimeoutMs:24750,retainedTarget:page.id},verification:'Output and browser process identity verified after the trailing service observation on both paired paths; continuation also verifies retained task bindings.'};
 receipt.summary=Object.fromEntries(['cold_workflow_open','direct_cdp','workflow_mcp','direct_cdp_url','workflow_mcp_continue','fresh_cli_mcp_facade','warm_workflow_readiness_probe'].map(name=>[name,summarize(name)]));
 if(receipt.selectedPhase)assert(receipt.phaseMeasurements.some(sample=>sample.status==='measured'),'Selected phase produced no qualified measurement; inspect response wiring and StableTime availability');
 receipt.measurementCompleted=true;
}catch(error){receipt.measurementCompleted=false;receipt.error=error.message;process.exitCode=1;}
finally{
 socket?.close();
 if(task)try{await call(workflow,'browser_close',{taskId:task.taskId});}catch{receipt.cleanupFailed=true;process.exitCode=1;}
 workflow.close();
 clock.close();
 // Only this fixture's exact host may be stopped. Keep residue available for
 // diagnosis if it cannot be proven to have this private namespace.
 if(!hostPid&&existsSync(join(context.socketDir,'runtime-host.pid')))hostPid=Number(readFileSync(join(context.socketDir,'runtime-host.pid'),'utf8').trim());
 if(hostPid&&existsSync(`/proc/${hostPid}/environ`)){
  const own=readFileSync(`/proc/${hostPid}/environ`).toString().split('\0').includes(`AGENT_BROWSER_SOCKET_DIR=${context.socketDir}`);
  if(own)try{process.kill(hostPid,'SIGTERM');}catch(error){if(error.code!=='ESRCH')throw error;}
 }
 let residue=ownedProcesses();
 for(let attempt=0;residue.length&&attempt<200;attempt++){await new Promise(resolve=>setTimeout(resolve,25));residue=ownedProcesses();}
 receipt.cleanup={remainingOwnedProcesses:residue};
 if(residue.length){receipt.cleanupFailed=true;receipt.fixtureRoot=context.tempHome;process.exitCode=1;}
 else context.cleanupTempHome();
 console.log(JSON.stringify(receipt,null,2));
}
