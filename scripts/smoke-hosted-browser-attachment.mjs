#!/usr/bin/env node
/** Explicit opt-in app-owned attachment qualification through public MCP and Native.
 * Required absolute paths: HOSTED_SMOKE_AB_BINARY, HOSTED_SMOKE_CHROME,
 * HOSTED_SMOKE_BOTS_ROOT, HOSTED_SMOKE_EVIDENCE_DIR. Required reviewed values:
 * HOSTED_SMOKE_INSTALLATION_ID, HOSTED_SMOKE_APP_ORIGIN, HOSTED_SMOKE_NATIVE_ORIGIN.
 * Optional HOSTED_SMOKE_AUTH_STATE is a private read-only Playwright storage state;
 * HOSTED_SMOKE_BROWSER_MANIFEST optionally pins the reviewed browser manifest.
 * HOSTED_SMOKE_NATIVE_SOCKET pins the already installed provider MCP socket.
 * HOSTED_SMOKE_CONTINUITY_REQUIRED=1 qualifies reconnect, scoped rules and local artifacts.
 * HOSTED_SMOKE_LOSS_REQUIRED=1 adds one separately created managed browser loss/reopen trial.
 * This script creates two borrowed tasks, one app-owned browser and one disposable assignment, plus the optional managed trial. It never installs, signs in,
 * retries effects, borrows a baseline assignment or removes retained evidence.
 */
import assert from 'node:assert/strict';
import {execFileSync,spawn} from 'node:child_process';
import {createHash,randomUUID} from 'node:crypto';
import {once} from 'node:events';
import {performance} from 'node:perf_hooks';
import {createServer} from 'node:http';
import {createServer as createHttpsServer} from 'node:https';
import {createPublicKey} from 'node:crypto';
import {qualifyNativeChallengeAssistance,probeCurrentNativeObservationAge} from './qualify-native-challenge-assistance.mjs';
import {qualifyNativeNoCdpChallenge} from './qualify-native-no-cdp-challenge.mjs';
import {createNativeCheckboxOracle,qualifyNativeCheckbox} from './qualify-native-checkbox.mjs';
import {mkdir,mkdtemp,readFile,readdir,realpath,stat,writeFile} from 'node:fs/promises';
import {createReadStream} from 'node:fs';
import {tmpdir} from 'node:os';
import {isAbsolute,join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createRequire} from 'node:module';
import {createMcpStdioClient,runCli,httpJsonResult} from './smoke-utils.js';
import {appProcessOwned,privateProfileReferenced,retainedProcessOwned,retainViewerProcess,viewerCleanupCensus} from './hosted-process-ownership.mjs';
import {cleanupExactOwnedProcesses,readFreshCleanupIdentity} from './hosted-fixture-cleanup.mjs';
import {createColdFirstPixelClock,summarizeColdFirstPixels} from './hosted-performance.mjs';
import {qualifyHostedBrowserControl} from './smoke-hosted-browser-control.mjs';
import {qualifyHostedBrowserReconnect} from './smoke-hosted-browser-reconnect.mjs';
import {qualifyHostedBrowserPolicy} from './smoke-hosted-browser-policy.mjs';
import {qualifyHostedBrowserLoss} from './smoke-hosted-browser-loss.mjs';
import {qualifyHostedBrowserRestart} from './smoke-hosted-browser-restart.mjs';

let coldPixels;
const stageTimings=[];
async function timedStage(stage,operation){const start=performance.now();let outcome='failed_or_unknown';try{const result=await operation();outcome='response';return result;}finally{stageTimings.push({stage,startedMs:start,durationMs:performance.now()-start,outcome});}}
async function boundedTeardown(stage,operation){
 let timer;try{return await Promise.race([operation(),new Promise((_,reject)=>{timer=setTimeout(()=>reject(Object.assign(Error('Teardown deadline: '+stage),{code:'fixture_teardown_deadline'})),30000);})]);}
 finally{clearTimeout(timer);}
}
const required=name=>{const value=process.env[name];assert(value,`Missing ${name}`);return value;};
async function absolute(name){const value=required(name);assert(isAbsolute(value),`${name} must be absolute`);return realpath(value);}
function origin(name){const value=required(name),url=new URL(value);assert(url.protocol==='https:'&&url.origin===value,`${name} must be canonical HTTPS`);return value;}
const binary=await absolute('HOSTED_SMOKE_AB_BINARY'),chrome=await absolute('HOSTED_SMOKE_CHROME');
const botsRoot=await absolute('HOSTED_SMOKE_BOTS_ROOT'),appOrigin=origin('HOSTED_SMOKE_APP_ORIGIN'),nativeOrigin=origin('HOSTED_SMOKE_NATIVE_ORIGIN');
const installationId=required('HOSTED_SMOKE_INSTALLATION_ID');
const application=process.env.HOSTED_SMOKE_NATIVE_APPLICATION??'codex-bots';assert(/^[a-z0-9_-]{1,64}$/.test(application));
const controlOrigin=process.env.HOSTED_SMOKE_NATIVE_CONTROL_ORIGIN??'http://127.0.0.1:19096';
assert.equal(new URL(controlOrigin).origin,controlOrigin);assert.equal(new URL(controlOrigin).protocol,'http:');assert.equal(new URL(controlOrigin).hostname,'127.0.0.1');
let controlAuthorization;
if(process.env.HOSTED_SMOKE_NATIVE_CONTROL_TOKEN_FILE){
 const file=await absolute('HOSTED_SMOKE_NATIVE_CONTROL_TOKEN_FILE'),info=await stat(file);assert.equal(info.uid,process.getuid());assert.equal(info.mode&0o077,0);
 const token=(await readFile(file,'utf8')).trim();assert(token&&!/[\r\n]/.test(token));controlAuthorization='Bearer '+token;
}
let workloadPlan,inputDiagnostic=false;
if(process.env.HOSTED_SMOKE_WORKLOAD_PLAN){
 const file=await absolute('HOSTED_SMOKE_WORKLOAD_PLAN'),info=await stat(file);assert.equal(info.uid,process.getuid());assert.equal(info.mode&0o077,0);
 workloadPlan=JSON.parse(await readFile(file,'utf8'));assert.equal(workloadPlan.version,1);
 inputDiagnostic=workloadPlan.mode==='native_input_diagnostic';assert(inputDiagnostic||workloadPlan.mode===undefined);
 assert.equal(workloadPlan.coldContexts,inputDiagnostic?1:5);
 assert.equal(workloadPlan.publicControlPairs,inputDiagnostic?0:10);assert.equal(workloadPlan.nativeControlPairs,inputDiagnostic?0:3);assert.equal(workloadPlan.warmTargetMs,250);
 assert.equal(workloadPlan.functionalUiPairs,1);assert.equal(workloadPlan.nativeTextEdits,1);assert.equal(workloadPlan.deadlineSeconds,inputDiagnostic?300:1200);
 assert(/^[a-f0-9]{64}$/.test(workloadPlan.pairDigest));assert.equal(typeof workloadPlan.runId,'string');
 assert.equal(workloadPlan.sourcePair.agentBinarySha256,createHash('sha256').update(await readFile(binary)).digest('hex'));
 assert.equal(workloadPlan.pairDigest,createHash('sha256').update(JSON.stringify(workloadPlan.sourcePair)).digest('hex'));
}
let fixtureNativeConnectOrigin;
if(process.env.HOSTED_SMOKE_FIXTURE_NATIVE_CONNECT_ORIGIN){
 assert(workloadPlan&&application==='fixture');
 assert.equal(new URL(nativeOrigin).hostname,'native.p279.example.invalid');
 assert.equal(new URL(appOrigin).hostname,'app.p279.example.invalid');
 fixtureNativeConnectOrigin=process.env.HOSTED_SMOKE_FIXTURE_NATIVE_CONNECT_ORIGIN;
 const connect=new URL(fixtureNativeConnectOrigin);
 assert.equal(connect.origin,fixtureNativeConnectOrigin);assert.equal(connect.protocol,'https:');assert.equal(connect.hostname,'127.0.0.1');
 assert.equal(connect.port,new URL(nativeOrigin).port);
}
let restartBridge,restartAuthorization;
if(process.env.HOSTED_SMOKE_RESTART_BRIDGE){
 assert(workloadPlan&&application==='fixture');assert(fixtureNativeConnectOrigin||new URL(nativeOrigin).hostname==='127.0.0.1');
 restartBridge=process.env.HOSTED_SMOKE_RESTART_BRIDGE;const address=new URL(restartBridge);
 assert.equal(address.origin,restartBridge);assert.equal(address.protocol,'http:');assert.equal(address.hostname,'127.0.0.1');
 const file=await absolute('HOSTED_SMOKE_RESTART_TOKEN_FILE'),info=await stat(file);assert.equal(info.uid,process.getuid());assert.equal(info.mode&0o077,0);
 restartAuthorization='Bearer '+(await readFile(file,'utf8')).trim();assert(!/[\r\n]/.test(restartAuthorization));
}
const botsSourceCommit=execFileSync('git',['-C',botsRoot,'rev-parse','HEAD'],{encoding:'utf8'}).trim();
if(workloadPlan)assert.equal(workloadPlan.sourcePair.consumerSourceCommit,botsSourceCommit);
assert.equal(execFileSync('git',['-C',botsRoot,'status','--porcelain','--untracked-files=no'],{encoding:'utf8'}).trim(),'','Commit the reviewed Bots source before actual acceptance');
const evidence=required('HOSTED_SMOKE_EVIDENCE_DIR');assert(isAbsolute(evidence));
await mkdir(evidence,{mode:0o700}); // Existing evidence paths fail closed; never overwrite another run.
assert.equal((await stat(evidence)).mode&0o077,0,'Evidence directory must be private');
const save=(name,value)=>writeFile(join(evidence,name),JSON.stringify(value,null,2)+'\n',{mode:0o600});
await save('cleanup.json',{state:'preparing',effectsDispatched:false,retainedEvidence:evidence});
if(workloadPlan)await save('workload-plan.json',workloadPlan);
const root=await mkdtemp(join(evidence,'fixture-')),socketDir=await mkdtemp(join(tmpdir(),'abh-'));
const agentHome=join(root,'.agent-browser');
const env=Object.fromEntries(Object.entries(process.env).filter(([key])=>!key.startsWith('AGENT_BROWSER_')&&!key.startsWith('XDG_')&&!key.startsWith('HOSTED_SMOKE_')));
Object.assign(env,{HOME:root,AGENT_BROWSER_HOME:agentHome,AGENT_BROWSER_SOCKET_DIR:socketDir,AGENT_BROWSER_RUNTIME_HOST:'1',AGENT_BROWSER_EXECUTABLE_PATH:chrome,AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD:binary,AGENT_BROWSER_REMOTE_VIEW_APPLICATION:application,AGENT_BROWSER_REMOTE_VIEW_PUBLIC_ORIGIN:nativeOrigin,AGENT_BROWSER_REMOTE_VIEW_POOL:'main',AGENT_BROWSER_REMOTE_VIEW_DESKTOP_COUNT:'1',AGENT_BROWSER_HOSTS_FILE:join(root,'hosts.json'),XDG_RUNTIME_DIR:join(root,'runtime'),XDG_CONFIG_HOME:join(root,'config'),XDG_CACHE_HOME:join(root,'cache'),XDG_DATA_HOME:join(root,'data')});
if(process.env.HOSTED_SMOKE_NATIVE_CONTROL_TOKEN_FILE)env.AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE=await absolute('HOSTED_SMOKE_NATIVE_CONTROL_TOKEN_FILE');
for(const path of [agentHome,socketDir,env.XDG_RUNTIME_DIR,env.XDG_CONFIG_HOME,env.XDG_CACHE_HOME,env.XDG_DATA_HOME])await mkdir(path,{recursive:true,mode:0o700});
if(process.env.HOSTED_SMOKE_BROWSER_MANIFEST)env.AGENT_BROWSER_STEALTHCDP_CHROMIUM_MANIFEST_PATH=await absolute('HOSTED_SMOKE_BROWSER_MANIFEST');
await writeFile(env.AGENT_BROWSER_HOSTS_FILE,JSON.stringify({version:1,hosts:{bots:{origin:appOrigin,providerApplication:application}}}),{mode:0o600});
const context={env,agentHome,socketDir,tempHome:root,session:'hosted-'+randomUUID()};
const {createBotService}=await import(pathToFileURL(join(botsRoot,'src/service.mjs')));
const {connectRemoteView}=await import(pathToFileURL(join(botsRoot,'src/remote-view.mjs')));
const require=createRequire(join(botsRoot,'package.json')),{chromium}=require('playwright-core');
let remote,baseline,broker,service,client,viewer,viewerServer,watcher,task,appBrowser,fixtureSite,appObserver,uiHandoffId,uiTakeoverDispatched=false,uiReturnDispatched=false,uiControlSettled=true,taskClosed=false,taskCloseDispatched=false,ambiguous=false,passed=false;
const borrowedTasks=[],closedBorrowedTasks=new Set(),managedLossTasks=[],closedManagedLossTasks=new Set();
let performanceTargetsMet=true;
const acquired=[],cleanup={errors:[],retainedEvidence:evidence,processesBefore:[],processesAfter:[],released:[]};
let appRootProof=null;const appOwnedProcesses=new Map();
let viewerRootProof=null;const viewerOwnedProcesses=new Map();
let step='preflight';
const errorCode=error=>error?.code||error?.name||'Error';
function safeError(error){
 const message=String(error?.message??'');
 return {code:errorCode(error),step,...(error?.syscall?{syscall:error.syscall}:{}),
  ...(typeof error?.path==='string'&&/^\/proc\/\d+(?:\/[a-z]+)?$/.test(error.path)?{path:error.path}:{}),
  messageCodes:[...new Set(message.match(/\b(?:browser|remote_view|hosted|fixture)_[a-z0-9_]+\b/g)??[])],
  ...(error?.operator?{assertionOperator:error.operator,actualType:typeof error.actual,expectedType:typeof error.expected,
   ...(typeof error.actual==='number'?{actual:error.actual}:{}),...(typeof error.expected==='number'?{expected:error.expected}:{})}:{}),
  ...(error?.processIdentity?{processIdentity:error.processIdentity}:{})};
}
const baselineDeniedProcesses=new Map(),baselineReadableProcesses=new Map(),observedOwnedProcesses=new Map(),harnessAncestors=new Set([process.pid]);
async function processIdentity(pid){
 const fields=(await readFile(`/proc/${pid}/stat`,'utf8')).split(')').at(-1).trim().split(/\s+/);
 return {pid:Number(pid),start:fields[19],parentPid:Number(fields[1]),processGroup:Number(fields[2]),session:Number(fields[3]),uid:(await stat(`/proc/${pid}`)).uid};
}
async function appDescendantAncestry(identity){
 if(!appRootProof)return false;
 const chain=[],seen=new Set();let current=identity;
 while(current&&chain.length<64){
  if(seen.has(current.pid)||harnessAncestors.has(current.pid)||current.uid!==appRootProof.uid)return false;
  seen.add(current.pid);chain.push(current);
  if(appProcessOwned(current,appRootProof,appOwnedProcesses)){
   for(const link of chain){const fresh=await processIdentity(link.pid);if(fresh.start!==link.start||fresh.parentPid!==link.parentPid||fresh.uid!==link.uid)return false;}
   return true;
  }
  if(current.parentPid<=1)return false;
  const parent=await processIdentity(current.parentPid);if(BigInt(parent.start)>BigInt(current.start))return false;
  current=parent;
 }
 return false;
}

async function foreignDeniedAncestry(identity){
 const chain=[],seen=new Set();let current=identity;
 while(current&&chain.length<64){
  if(seen.has(current.pid)||current.pid===1||harnessAncestors.has(current.pid))return null;
  seen.add(current.pid);chain.push(current);
  if(observedOwnedProcesses.get(current.pid)?.start===current.start||viewerOwnedProcesses.get(current.pid)?.start===current.start)return null;
  const baseline=baselineReadableProcesses.get(current.pid);
  if(baseline&&baseline.start===current.start&&baseline.parentPid===current.parentPid&&baseline.uid===current.uid){
   // Re-read every link to reject PID reuse or a reparent during qualification.
   for(const link of chain){const fresh=await processIdentity(link.pid);if(fresh.start!==link.start||fresh.parentPid!==link.parentPid||fresh.uid!==link.uid)return null;}
   return chain;
  }
  if(current.parentPid<=1)return null;
  const parent=await processIdentity(current.parentPid);
  if(Number(parent.start)>Number(current.start))return null;
  current=parent;
 }
 return null;
}
const peerIdentity=inventory=>inventory.assignments.map(a=>Object.fromEntries(['assignmentId','desktopId','generation','viewingGeneration','state'].filter(k=>k in a).map(k=>[k,a[k]]))).sort((a,b)=>a.assignmentId.localeCompare(b.assignmentId));
// Protocol: RemoteViewApplicationEnvelope {application,request}, POST
// /v1/consumer. Published request fields are snake_case; response inventory is
// camelCase (remote_view_application.rs and *_records.rs). This broker never
// forwards acquisition, release, retirement or route/grant operations.
const brokerOperations=[],correlations=new Map(),nativeTargets=new Map();
async function startScopedBroker(){
 const assignmentOperations=new Set(['observe_assignment','launch_environment','windows','activate','events','observe_idle','read_idle_retirement','register_native_handoff_observation']);
 const server=createServer(async(req,res)=>{
  const reply=(status,value)=>{res.writeHead(status,{'content-type':'application/json'});res.end(JSON.stringify(value));};
  const denied=async()=>{brokerOperations.push({operation,denied:true});await save('broker-operations.json',brokerOperations);reply(400,{schema_version:1,code:'fixture_target_denied',message:'Operation outside exact fixture assignment',retryable:false});};
  let operation='invalid';
  try{
   if(req.method==='POST'&&['/v1/agent-desktop/observe','/v1/agent-desktop/ocr','/v1/agent-desktop/capture_region','/v1/agent-desktop/control','/v1/agent-desktop/input'].includes(req.url)){
    let raw='';for await(const chunk of req){raw+=chunk;if(Buffer.byteLength(raw)>65536)return denied();}
    const body=JSON.parse(raw),allowed=acquired.some(binding=>{const target=nativeTargets.get(binding.assignmentId);return target&&body.desktopId===target.viewingDesktopId&&body.expectedGeneration===target.viewingGeneration;});
    if(!allowed||process.env.HOSTED_SMOKE_CHALLENGE_469!=='1'||req.url.endsWith('/control')&&body.operation!=='status')return denied();
    if(req.url.endsWith('/input')){
      if(process.env.HOSTED_SMOKE_NATIVE_CHECKBOX_OBSERVE_ONLY==='1')return denied();
      const motion=process.env.HOSTED_SMOKE_OBSERVATION_AGE_PROBE==='1'&&Array.isArray(body.actions)&&body.actions.length>0&&body.actions.length<=16&&
        body.actions.every(action=>action.kind==='move'&&Object.keys(action).sort().join(',')==='kind,x,y');
      const action=body.actions?.[0],visual=body.expectedVisual;
      const checkbox=process.env.HOSTED_SMOKE_NATIVE_CHECKBOX==='1'&&Array.isArray(body.actions)&&body.actions.length===1&&
        action.kind==='click'&&action.button===1&&action.count===1&&Object.keys(action).sort().join(',')==='button,count,kind,x,y'&&
        Number.isSafeInteger(body.controlEpoch)&&body.controlEpoch>=0&&/^[a-f0-9]{64}$/.test(visual?.sha256??'')&&visual.region;
      if(!motion&&!checkbox)return denied();
    }
    const upstream=await fetch(controlOrigin+req.url,{method:'POST',headers:{'content-type':'application/json',...(controlAuthorization?{authorization:controlAuthorization}:{})},body:raw,redirect:'error',signal:AbortSignal.timeout(15000)});
    return reply(upstream.status,await upstream.json());
   }
   if(req.method!=='POST'||req.url!=='/v1/consumer')return denied();
   let raw='';for await(const chunk of req){raw+=chunk;if(Buffer.byteLength(raw)>65536)return denied();}
   const envelope=JSON.parse(raw),request=envelope.request;operation=request?.operation;
   if(envelope.application!==application||Object.keys(envelope).some(k=>!['application','request'].includes(k))||!request)return denied();
   const owned=new Map(acquired.map(a=>[a.assignmentId,a]));
   if(operation==='inventory'){
    if(Object.keys(request).length!==1)return denied();
   }else if(assignmentOperations.has(operation)){
    const binding=owned.get(request.assignment_id);
    if(!binding||request.expected_generation!==binding.generation||baseline.assignments.some(a=>a.assignmentId===request.assignment_id))return denied();
    if('expected_viewing_generation' in request&&request.expected_viewing_generation!==binding.viewingGeneration)return denied();
   }else if(operation==='observe_native_handoff_use'){
    if(correlations.get(request.correlation_id)!==request.target_digest)return denied();
   }else return denied();
   // One forwarding attempt; no automatic retries and no redirects.
   let upstream;try{upstream=await fetch(controlOrigin+'/v1/consumer',{method:'POST',headers:{'content-type':'application/json',...(controlAuthorization?{authorization:controlAuthorization}:{})},body:raw,redirect:'error',signal:AbortSignal.timeout(120000)});}catch(error){ambiguous=true;throw error;}
   const value=await upstream.json();
   if(upstream.ok&&operation==='observe_assignment'){
    const binding=owned.get(request.assignment_id),target=value.target;
    assert(target&&target.assignmentId===binding.assignmentId&&target.desktopId===binding.desktopId&&target.lifecycleGeneration===binding.generation&&target.viewingGeneration===binding.viewingGeneration);
    nativeTargets.set(binding.assignmentId,target);
   }
   if(upstream.ok&&operation==='inventory'){
    assert(Array.isArray(value.assignments)&&Array.isArray(value.desktops)&&Array.isArray(value.pools));
    value.assignments=value.assignments.filter(a=>owned.has(a.assignmentId)&&a.generation===owned.get(a.assignmentId).generation);
    const ids=new Set(acquired.map(a=>a.desktopId));
    value.desktops=value.desktops.filter(d=>ids.has(d.desktopId));
    value.pools=value.pools.map(pool=>({...pool,desktopMembers:pool.desktopMembers.filter(id=>ids.has(id))}));
    await save('broker-inventory-shape.json',{topLevelKeys:Object.keys(value),policyKeys:Object.keys(value.policy??{}),poolPolicyKeys:Object.fromEntries(Object.entries(value.policy?.pools??{}).map(([key,policy])=>[key,Object.keys(policy)])),assignmentCount:value.assignments.length,desktopCount:value.desktops.length,poolCount:value.pools.length});
   }
   if(upstream.ok&&operation==='register_native_handoff_observation'){
    assert(typeof value.correlationId==='string'&&value.targetDigest===request.target_digest);
    correlations.set(value.correlationId,request.target_digest);
   }
   brokerOperations.push({operation,status:upstream.status});await save('broker-operations.json',brokerOperations);
   reply(upstream.status,value);
  }catch(error){brokerOperations.push({operation,...safeError(error),ambiguous});await save('broker-operations.json',brokerOperations);reply(502,{schema_version:1,code:'fixture_broker_failed',message:'Fixture broker failed; inspect private evidence without replay',retryable:false});}
 });
 await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(0,'127.0.0.1',resolve);});
 return server;
}
async function census({captureDeniedBaseline=false,collectUnresolved=false}={}){
 const rows=[],unresolved=[],foreignDenied=[];
 for(const pid of await readdir('/proc')){
  if(!/^\d+$/.test(pid)||Number(pid)===process.pid)continue;
  let identity,owned=false,viewerOwned=false,uid;
  try{const path=`/proc/${pid}`;uid=(await stat(path)).uid;
   identity=await processIdentity(pid);assert(/^\d+$/.test(identity.start));
   viewerOwned=await retainViewerProcess(identity,{root:viewerRootProof,observed:viewerOwnedProcesses,readRoot:cleanupProcessIdentity,readKernel:processIdentity});
   owned=viewerOwned;
   if(identity.uid!==process.getuid()){
    if(captureDeniedBaseline&&identity.pid!==1&&!harnessAncestors.has(identity.pid))baselineReadableProcesses.set(identity.pid,identity);
    continue;
   }
   const retained=observedOwnedProcesses.get(identity.pid);
   if(retained&&retained.start===identity.start)owned=retainedProcessOwned({...identity,executable:await realpath(path+'/exe')},observedOwnedProcesses);
   if(!owned){
    owned=appProcessOwned(identity,appRootProof,appOwnedProcesses)||await appDescendantAncestry(identity);
    if(owned){const executable=await realpath(path+'/exe');appOwnedProcesses.set(identity.pid,{...identity,executable});}
    else owned=(await readFile(path+'/environ')).toString().split('\0').includes('AGENT_BROWSER_HOME='+agentHome);
   }
   if(!owned){if(captureDeniedBaseline&&identity.pid!==1&&!harnessAncestors.has(identity.pid))baselineReadableProcesses.set(identity.pid,identity);continue;}
   const full={...identity,executable:await realpath(path+'/exe')};
   if(observedOwnedProcesses.get(identity.pid)?.start===full.start)retainedProcessOwned(full,observedOwnedProcesses);
   observedOwnedProcesses.set(identity.pid,full);
   rows.push(full);
  }catch(error){
   if(['ENOENT','ESRCH'].includes(error.code)){
    // An exe link may disappear while its kernel process is still live.
    // Only a fresh kernel absence permits dropping owned custody from census.
    if(!owned&&!viewerOwnedProcesses.has(Number(pid)))continue;
    try{await processIdentity(pid);}catch(kernelError){if(['ENOENT','ESRCH'].includes(kernelError.code))continue;error=kernelError;}
    if(['ENOENT','ESRCH'].includes(error.code))error=Object.assign(Error('fixture_process_executable_unknown'),{code:'fixture_process_executable_unknown'});
   }
   if(viewerOwned&&['EACCES','EPERM','fixture_process_executable_unknown'].includes(error.code)){
    const fresh=await processIdentity(pid).catch(kernelError=>{if(['ENOENT','ESRCH'].includes(kernelError.code))return null;throw kernelError;});
    if(fresh===null)continue;
    if(fresh.uid!==identity.uid||fresh.start!==identity.start)throw Error('fixture_viewer_identity_changed');
    // Active custody is visible even when sandboxed /proc files are restricted.
    // Cleanup cannot signal this row or call it absent while its kernel lives.
    const restricted={...fresh,kernelOnly:true,code:'fixture_viewer_executable_unavailable'};
    if(collectUnresolved)unresolved.push(restricted);else rows.push(restricted);
    continue;
   }
   // A positively different UID is outside this fixture's process custody.
   // If its identity is unreadable, omit it as an ancestry anchor.
   if(uid!==undefined&&uid!==process.getuid()&&['EACCES','EPERM'].includes(error.code))continue;
   if(['EACCES','EPERM'].includes(error.code)&&identity&&!owned){
    if(captureDeniedBaseline){baselineDeniedProcesses.set(identity.pid,identity);continue;}
    const prior=baselineDeniedProcesses.get(identity.pid);
    if(prior&&prior.start===identity.start&&prior.parentPid===identity.parentPid&&prior.uid===identity.uid)continue;
    const chain=await foreignDeniedAncestry(identity).catch(()=>null);
    if(chain){cleanup.foreignDeniedAncestry??=[];cleanup.foreignDeniedAncestry.push(chain);foreignDenied.push(chain);continue;}
   }
   error.processIdentity=identity;if(collectUnresolved){unresolved.push(safeError(error));continue;}throw error;
  }
 }
 if(collectUnresolved){
  // A /proc directory listing alone is not the absence fence for retained
  // sandbox descendants. Independently observe every kernel-only generation.
  const kernelOnly=new Map([...viewerOwnedProcesses].filter(([pid])=>!observedOwnedProcesses.has(pid)));
  const restricted=await viewerCleanupCensus(kernelOnly,cleanupProcessIdentity);
  rows.push(...restricted.owned);unresolved.push(...restricted.unresolved);
 }
 if(viewerRootProof)await save('viewer-process-observed.json',[...viewerOwnedProcesses.values()]);
 return collectUnresolved?{owned:rows,unresolved,foreignDenied}:rows;
}

async function cleanupProcessIdentity(pid){return readFreshCleanupIdentity(pid,{identity:processIdentity,executable:pid=>realpath(`/proc/${pid}/exe`)});}

async function appAbsenceFence(){
 if(!appRootProof)return;
 await save('app-process-observed.json',[...appOwnedProcesses.values()]);
 // Kernel absence, not merely a missing env marker or a recycled PID.
 for(const row of appOwnedProcesses.values()){
  try{await processIdentity(row.pid);throw Error('fixture_owned_process_not_absent');}
  catch(error){if(!['ENOENT','ESRCH'].includes(error.code))throw error;}
 }
 const references=[];
 for(const name of await readdir('/proc')){
  if(!/^\d+$/.test(name)||Number(name)===process.pid)continue;
  try{const identity=await processIdentity(name);if(identity.uid!==process.getuid())continue;
   const cmdline=await readFile(`/proc/${name}/cmdline`,'utf8');
   if(privateProfileReferenced(cmdline,appRootProof.profilePath))references.push(identity);
  }catch(error){if(!['ENOENT','ESRCH'].includes(error.code))throw error;}
 }
 cleanup.appProfileReferences=references;await save('app-process-absence.json',{root:appRootProof,observed:[...appOwnedProcesses.values()],profileReferences:references,absent:references.length===0});
 assert.equal(references.length,0,'Unqualified private profile process reference blocks release');
}

async function send(method,params){
 let timer;
 try{return await Promise.race([client.send(method,params),new Promise((_,reject)=>{timer=setTimeout(()=>{ambiguous=true;client.rejectPending('Unknown MCP outcome; no replay');reject(Error('Unknown MCP outcome'));},120000);})]);}
 finally{clearTimeout(timer);}
}
async function workflow(name,args){
 let result;try{result=await timedStage('consumer_workflow.'+name+(args.operation?'.'+args.operation:''),()=>send('tools/call',{name,arguments:args}));}catch(error){ambiguous=true;throw error;}
 const value=result.structuredContent??JSON.parse(result.content.find(c=>c.type==='text').text);
 if(result.isError||value.success===false)throw Object.assign(Error('Agent Browser rejected workflow'),{code:value.error?.code||'workflow_rejected'});
 const data=value.data??value;if(name==='browser_open'){
  task=data;await save('browser-open-identity.json',Object.fromEntries(['taskId','browserId','profileId','tabId','host'].filter(k=>k in data).map(k=>[k,data[k]])));
 }return data;
}
async function readTask(id){return timedStage('cli.task_metadata',async()=>{const response=await runCli(context,['--json','mcp','host-task',id]);const value=JSON.parse(response.stdout);assert.equal(value.success,true);return value.data;});}
async function publicWorkflow(name,args){return timedStage('persistent_mcp.'+name+(args.operation?'.'+args.operation:''),async()=>{const reply=await send('tools/call',{name,arguments:args});const value=reply.structuredContent??JSON.parse(reply.content.find(c=>c.type==='text').text);if(reply.isError||value.success===false)throw Object.assign(Error(value.error?.message??JSON.stringify(value.error??value)),{code:value.error?.code??'workflow_rejected'});return value.data??value;});}
function startWatch(id){
 watcher=spawn(binary,['--json','mcp','watch-host-task',id],{env,stdio:['ignore','pipe','pipe']});watcher.stderr.resume();
 const queue=[];let buffer='',waiting=null,failure=null;
 const fail=error=>{failure=error;if(waiting){const pending=waiting;waiting=null;clearTimeout(pending.timer);pending.reject(error);}};
 watcher.on('error',fail);watcher.once('exit',()=>fail(Error('Host metadata stream ended')));
 watcher.stdout.on('data',chunk=>{
  buffer+=chunk.toString();if(Buffer.byteLength(buffer)>65536){fail(Error('Host metadata stream exceeded compact frame bound'));watcher.kill('SIGTERM');return;}
  while(buffer.includes('\n')){
   const end=buffer.indexOf('\n'),line=buffer.slice(0,end);buffer=buffer.slice(end+1);
   try{const event=JSON.parse(line);if(waiting){const pending=waiting;waiting=null;clearTimeout(pending.timer);pending.resolve(event);}else{queue.push(event);if(queue.length>16){fail(Error('Host metadata stream exceeded bounded event queue'));watcher.kill('SIGTERM');return;}}}
   catch(error){fail(error);watcher.kill('SIGTERM');return;}
  }
 });
 return {next(){
  if(queue.length)return Promise.resolve(queue.shift());if(failure)return Promise.reject(failure);assert.equal(waiting,null);
  return new Promise((resolve,reject)=>{const timer=setTimeout(()=>{waiting=null;failure=Error('Host metadata stream event timeout');watcher.kill('SIGTERM');reject(failure);},15000);waiting={resolve,reject,timer};});
 }};
}

try{workload:{
 step='baseline-process-census';
 for(let current=await processIdentity(process.pid),depth=0;current.parentPid>1&&depth<64;depth++){harnessAncestors.add(current.parentPid);current=await processIdentity(current.parentPid);}
 await census({captureDeniedBaseline:true});await save('baseline-denied-processes.json',[...baselineDeniedProcesses.values()]);await save('baseline-readable-processes.json',[...baselineReadableProcesses.values()]);
 step='private-auth-state';
 const authState=process.env.HOSTED_SMOKE_AUTH_STATE;
 if(authState){
  assert(isAbsolute(authState));assert.equal((await stat(authState)).mode&0o077,0,'Auth state must be private');
  step='native-auth-preflight';
  const storage=JSON.parse(await readFile(authState,'utf8')),nativeUrl=new URL(nativeOrigin),now=Date.now()/1000;
  const cookies=(storage.cookies??[]).filter(cookie=>{
   const domain=String(cookie.domain??'').toLowerCase(),host=nativeUrl.hostname.toLowerCase();
   const domainMatch=domain.startsWith('.')?(host===domain.slice(1)||host.endsWith(domain)):host===domain;
   return domainMatch&&cookie.path==='/'&&(cookie.expires===-1||cookie.expires>now)&&typeof cookie.name==='string'&&typeof cookie.value==='string'&&!/[\r\n;]/.test(cookie.name+cookie.value);
  }).map(cookie=>cookie.name+'='+cookie.value).join('; ');
  let result;
  try{result=await fetch((fixtureNativeConnectOrigin??nativeOrigin)+'/',{method:'GET',headers:cookies?{cookie:cookies}:{},redirect:'manual',signal:AbortSignal.timeout(10000)});}
  catch{throw Object.assign(Error('Native authentication preflight requires operator readback'),{code:'fixture_auth_preflight_required'});}
  await result.body?.cancel();
  let accepted=result.status>=200&&result.status<300;
  if(result.status>=300&&result.status<400){const location=result.headers.get('location');if(location){try{accepted=new URL(location,nativeOrigin).origin===nativeOrigin;}catch{accepted=false;}}}
  await save('auth-preflight.json',{method:'GET',status:result.status,accepted,matchedCookieCount:cookies?cookies.split('; ').length:0});
  if(!accepted)throw Object.assign(Error('Refresh private Native QA authentication before acquiring a fixture assignment'),{code:'fixture_auth_preflight_required'});
 }
 step='connect-native';remote=await connectRemoteView({expectedInstallationId:installationId,application,embedOrigin:appOrigin,publicOrigin:nativeOrigin,...(process.env.HOSTED_SMOKE_NATIVE_SOCKET?{socketPath:process.env.HOSTED_SMOKE_NATIVE_SOCKET}:{})});
 for(const name of ['observe','controlStatus','takeControl','releaseControl','acquireSelectiveControl','replaceSelectiveControl','releaseSelectiveControl'])if(typeof remote[name]==='function'){const original=remote[name].bind(remote);remote[name]=(...args)=>timedStage('native.'+name,()=>original(...args));}
 baseline=await remote.inventory();await save('baseline.json',baseline);
 broker=await startScopedBroker();env.AGENT_BROWSER_REMOTE_VIEW_ORIGIN=`http://127.0.0.1:${broker.address().port}`;
 await save('inputs.json',{binary,binarySha256:createHash('sha256').update(await readFile(binary)).digest('hex'),chrome,chromeSha256:createHash('sha256').update(await readFile(chrome)).digest('hex'),botsRoot,botsSourceCommit,installationId,appOrigin,nativeOrigin,agentHome,socketDir});
 const desktops=new Proxy(remote,{get(target,key){if(key!=='acquire')return typeof target[key]==='function'?target[key].bind(target):target[key];return async args=>{let binding;try{binding=await target.acquire(args);}catch(error){ambiguous=true;throw error;}assert(!baseline.assignments.some(a=>a.assignmentId===binding.assignmentId),'Provider returned a baseline peer assignment');acquired.push(binding);await save('acquired.json',acquired);Object.assign(binding,await target.observe(binding));await save('acquired.json',acquired);return binding;};}});
 const bot={id:'fixture-'+randomUUID(),name:'Synthetic attachment owner',threadId:'synthetic-'+randomUUID()},second={id:'shared-'+randomUUID(),name:'Synthetic shared participant',threadId:'shared-session-'+randomUUID()};
 step='acquire-app-desktop';const binding=await desktops.acquire({botId:bot.id,idempotencyKey:'attachment-'+randomUUID()});
 assert.equal(acquired.length,1);
 step='app-launch-environment';const launch=await remote.environment(binding);
 assert.equal(launch.target.desktopId,binding.desktopId);assert.equal(launch.target.lifecycleGeneration,binding.generation);assert.equal(launch.target.viewingGeneration,binding.viewingGeneration);
 // The existing manual-login X11 focus adapter authenticates from its owner
 // environment. This fixture has one exact AB desktop; copy only its Native
 // supplied cookie-file locator before starting the private owner, never a
 // production display or a global X11 access change.
 if(process.env.HOSTED_SMOKE_CHALLENGE_469==='1'){
  assert(typeof launch.environment.XAUTHORITY==='string'&&isAbsolute(launch.environment.XAUTHORITY));
  env.XAUTHORITY=launch.environment.XAUTHORITY;
 }
 step='initialize-ab-mcp';client=createMcpStdioClient({context,args:['mcp','serve'],onFatal:()=>{ambiguous=true;}});
 await send('initialize',{protocolVersion:'2025-06-18',capabilities:{},clientInfo:{name:'hosted-native-smoke',version:'1'}});client.notify('notifications/initialized');
 step='register-disposable-named-profile';const createdProfile=JSON.parse((await runCli(context,['--json','runtime','create','borrowed-profile','--browser-family','chromium'])).stdout);assert.equal(createdProfile.created,true);const profilePath=await realpath(createdProfile.userDataDir);assert(profilePath.startsWith(root+'/'),'Named fixture profile must remain inside its private home');
 const challenge469=process.env.HOSTED_SMOKE_CHALLENGE_469==='1';
 const nativeCheckbox=process.env.HOSTED_SMOKE_NATIVE_CHECKBOX==='1',checkboxOracle=createNativeCheckboxOracle();
 const challengeState={completionRequests:0,protectedRequests:0};
 const nativeChallengeState={completionRequests:0,protectedRequests:0,pageRequests:0};
 let challengeTls,challengeSpki;
 if(challenge469){
  assert(inputDiagnostic&&fixtureNativeConnectOrigin,'Challenge fixture must remain the bounded isolated Native lane');
  challengeTls={cert:await readFile(await absolute('HOSTED_SMOKE_CHALLENGE_SITE_CERT')),key:await readFile(await absolute('HOSTED_SMOKE_CHALLENGE_SITE_KEY'))};
  challengeSpki=createHash('sha256').update(createPublicKey(challengeTls.cert).export({type:'spki',format:'der'})).digest('base64');
 }
 const html='<!doctype html><title>App-owned attachment synthetic marker</title><body style="margin:0;background:#165dcc"><div style="height:75vh;background:#ff00aa;color:white;font:48px sans-serif">ATTACHMENT SYNTHETIC MARKER</div><input id="draft"><input id="control"><span id="peer">PREEXISTING PEER</span></body>';
 const fixtureHandler=(req,res)=>{
  if(nativeCheckbox&&checkboxOracle.handle(req,res))return;
  if(challenge469&&req.url==='/native-no-cdp'){
   nativeChallengeState.pageRequests++;
   res.writeHead(200,{'content-type':'text/html'});res.end('<!doctype html><title>Owned Native challenge</title><body style="background:white;color:black;padding:20px;font:24px sans-serif"><div style="position:absolute;right:10px;bottom:10px;background:#00ff00;width:80px;height:40px"></div><h1 style="font-size:26px">Select all images containing buses</h1><p>Owned unsupported image puzzle</p><div style="height:80px;background:#ddd">Synthetic image grid</div><button style="font:22px sans-serif" onclick="completeChallenge()">Complete verification</button><script>async function completeChallenge(){const result=await fetch("/native/challenge/complete",{method:"POST"});if(!result.ok)return;const report=await fetch("/native/protected-report");if(report.ok)document.body.innerHTML="<h1>Report ready</h1>";}</script>');return;
  }
  if(challenge469&&req.url==='/native/challenge/complete'&&req.method==='POST'){
   nativeChallengeState.completionRequests++;res.writeHead(nativeChallengeState.completionRequests===1?200:409,{'content-type':'application/json'});res.end(JSON.stringify(nativeChallengeState));return;
  }
  if(challenge469&&req.url==='/native/challenge/evidence'){res.writeHead(200,{'content-type':'application/json'});res.end(JSON.stringify(nativeChallengeState));return;}
  if(challenge469&&req.url==='/native/protected-report'){
   nativeChallengeState.protectedRequests++;res.writeHead(nativeChallengeState.completionRequests===1?200:403,{'content-type':'application/json'});res.end(JSON.stringify({report:nativeChallengeState.completionRequests===1?'ready':'denied'}));return;
  }
  if(challenge469&&req.url==='/challenge/complete'&&req.method==='POST'){
   challengeState.completionRequests++;res.writeHead(challengeState.completionRequests===1?200:409,{'content-type':'application/json'});res.end(JSON.stringify(challengeState));return;
  }
  if(challenge469&&req.url==='/challenge/evidence'){res.writeHead(200,{'content-type':'application/json'});res.end(JSON.stringify(challengeState));return;}
  if(challenge469&&req.url==='/protected-report'){
   challengeState.protectedRequests++;res.writeHead(challengeState.completionRequests===1?200:403,{'content-type':'application/json'});res.end(JSON.stringify({report:challengeState.completionRequests===1?'ready':'denied'}));return;
  }
  res.writeHead(200,{'content-type':'text/html'});res.end(html);
 };
 fixtureSite=challenge469?createHttpsServer(challengeTls,fixtureHandler):createServer(fixtureHandler);await new Promise(resolve=>fixtureSite.listen(0,'127.0.0.1',resolve));
 const site=(challenge469?'https://127.0.0.1:':'http://127.0.0.1:')+fixtureSite.address().port;
 step='app-owned-browser-launch';appBrowser=spawn(chrome,[...(challengeSpki?['--ignore-certificate-errors-spki-list='+challengeSpki]:[]),`--user-data-dir=${profilePath}`,'--no-first-run','--no-default-browser-check','--password-store=basic','--remote-debugging-address=127.0.0.1','--remote-debugging-port=0',site+'/primary'],{env:{...env,...launch.environment,AGENT_BROWSER_HOME:agentHome},detached:true,stdio:'ignore'});
 appBrowser.on('error',()=>{ambiguous=true;});
 await once(appBrowser,'spawn');
 appRootProof={...await processIdentity(appBrowser.pid),executable:await realpath(`/proc/${appBrowser.pid}/exe`),profilePath};
 appOwnedProcesses.set(appRootProof.pid,appRootProof);await save('app-process-root-proof.json',appRootProof);
 assert.equal(appRootProof.executable,chrome,'App root must be the reviewed Chrome executable');
 assert.equal(appRootProof.executable,chrome,'App process must execute the pinned browser');
 assert.equal(appRootProof.session,appRootProof.pid);assert.equal(appRootProof.processGroup,appRootProof.pid);assert.equal(appRootProof.uid,process.getuid());
 assert(privateProfileReferenced(await readFile(`/proc/${appBrowser.pid}/cmdline`,'utf8'),profilePath));
 await save('native-environment-override-keys.json',Object.keys(launch.environment).sort());
 await census();await save('app-process-observed.json',[...appOwnedProcesses.values()]);
 const deadline=Date.now()+30000;let active;
 while(Date.now()<deadline){try{active=(await readFile(join(profilePath,'DevToolsActivePort'),'utf8')).trim().split('\n');if(active.length===2)break;}catch(error){if(error.code!=='ENOENT')throw error;}if(appBrowser.exitCode!==null)throw Error('App browser exited before CDP readiness');await new Promise(resolve=>setTimeout(resolve,100));}
 assert(active?.length===2,'App browser CDP endpoint must be observed');
 const endpoint='ws://127.0.0.1:'+active[0]+active[1];
 appObserver=await chromium.connectOverCDP(endpoint);const browserContext=appObserver.contexts()[0],primary=browserContext.pages()[0];await primary.waitForLoadState('domcontentloaded');
 const draft='unfinished-'+randomUUID(),storage='stored-'+randomUUID(),cookie='cookie-'+randomUUID();
 await primary.evaluate(({draft,storage,cookie})=>{document.querySelector('#draft').value=draft;localStorage.setItem('attachment-synthetic',storage);document.cookie='attachment-synthetic='+cookie+'; Path=/; SameSite=Lax';},{draft,storage,cookie});
 const peer=await browserContext.newPage();await peer.goto(site+'/peer',{waitUntil:'domcontentloaded'});await peer.evaluate(()=>{document.querySelector('#draft').value='peer unfinished';});
 await primary.bringToFront();
 const targetFor=async page=>{const cdp=await browserContext.newCDPSession(page);try{return (await cdp.send('Target.getTargetInfo')).targetInfo.targetId;}finally{await cdp.detach();}};
 const primaryTarget=await targetFor(primary),peerTarget=await targetFor(peer),initialTargets=(await Promise.all(browserContext.pages().map(targetFor))).sort();
 const nativeBinding=await remote.bindingForBrowser({pid:appBrowser.pid},binding);assert.deepEqual(Object.fromEntries(['assignmentId','desktopId','generation','viewingGeneration'].map(k=>[k,nativeBinding[k]])),Object.fromEntries(['assignmentId','desktopId','generation','viewingGeneration'].map(k=>[k,binding[k]])));
 const original={...await processIdentity(appBrowser.pid),executablePath:await realpath(`/proc/${appBrowser.pid}/exe`),profilePath:await realpath(profilePath)};
 const boot=(await readFile('/proc/sys/kernel/random/boot_id','utf8')).trim();
 const processProof={pid:original.pid,startToken:`linux:${boot}:${original.start}`,executablePath:original.executablePath,browserFamily:original.executablePath.toLowerCase().includes('chromium')?'chromium':'chrome'};
 const sharedWith=[{hostId:'bots',participantId:'bot:'+second.id,sessionId:second.threadId}];
 const attachment={version:1,process:processProof,profilePath:original.profilePath,cdpEndpoint:endpoint,targetId:primaryTarget,sharedWith};
 async function continuity(label){
  const current=await processIdentity(original.pid);assert.equal(current.start,original.start);assert.equal(await realpath(`/proc/${original.pid}/exe`),original.executablePath);
  assert.deepEqual((await Promise.all(browserContext.pages().map(targetFor))).sort(),initialTargets);
  assert.deepEqual(await primary.evaluate(()=>({draft:document.querySelector('#draft').value,storage:localStorage.getItem('attachment-synthetic')})),{draft,storage});
  assert.equal((await browserContext.cookies(site)).find(value=>value.name==='attachment-synthetic')?.value,cookie,'The app cookie must survive attachment, sharing and control');
  assert.equal(await peer.locator('#draft').inputValue(),'peer unfinished');assert.equal(await targetFor(peer),peerTarget);
  const observed=await remote.bindingForBrowser({pid:original.pid},binding);assert.equal(observed.assignmentId,binding.assignmentId);assert.equal(observed.generation,binding.generation);
  await save('continuity-'+label+'.json',{original,processPreserved:true,preexistingTargetsPreserved:true,syntheticStorageAndDraftPreserved:true,syntheticCookiePreserved:true,peerPreserved:true,nativeAssignmentPreserved:true});
 }
 // Seed only the fixture's existing application-owned binding. Attach mode
 // must never acquire a replacement assignment or accept process/CDP from HTTP.
 const botsStateDir=join(root,'bots-state');await mkdir(botsStateDir,{mode:0o700});
 const ownedFor=owner=>({profile:{profileId:'borrowed-profile',profilePath:original.profilePath,botId:owner.id},browser:{pid:original.pid,startTimeTicks:original.start,executable:original.executablePath}});
 await writeFile(join(botsStateDir,'state.json'),JSON.stringify({bots:[bot,second],submissions:{},sessionRequests:{},desktops:Object.fromEntries([bot,second].map(owner=>[owner.id,{binding,ownedProfiles:{'stealthcdp-chromium':ownedFor(owner)}}]))}),{mode:0o600});
 const native={status:async()=>({connected:true,activity:'ready',turnId:null,requests:[]}),historyPage:async()=>({messages:[],nextCursor:null})};
 const observedAttachments=[];
 step='create-actual-bots-consumer';
 service=await createBotService({bots:[bot,second],native,stateDir:botsStateDir,desktops,publicOrigin:appOrigin,hostedAgentBrowser:{hostId:'bots',publicOrigin:appOrigin,controlEnabled:true,workflow,readTask:async id=>(await publicWorkflow('browser_reconnect',{taskId:id,operation:'status'})).host,
  observeAttachment:async input=>{
   const owner=[bot,second].find(owner=>'bot:'+owner.id===input.participantId&&owner.threadId===input.sessionId);
   assert(owner,'Trusted attachment observation must address a prepared fixture participant');
   assert.equal(input.profile,'borrowed-profile');assert.equal(input.profileMode,'existing');
   assert.deepEqual(input.appBrowser,ownedFor(owner));
   assert.deepEqual(input.desktop,Object.fromEntries(['assignmentId','desktopId','generation','viewingGeneration'].map(key=>[key,binding[key]])));
   const fresh=await processIdentity(original.pid);assert.equal(fresh.start,original.start);assert.equal(await realpath(`/proc/${original.pid}/exe`),original.executablePath);assert.equal(await realpath(profilePath),original.profilePath);
   assert.equal(await targetFor(primary),primaryTarget);
   const current=await remote.bindingForBrowser({pid:original.pid},binding);assert.equal(current.assignmentId,binding.assignmentId);assert.equal(current.generation,binding.generation);
   observedAttachments.push({participantId:input.participantId,sessionId:input.sessionId,processPreserved:true,desktopPreserved:true});await save('trusted-attachment-observations.json',observedAttachments);
   return structuredClone(attachment);
  }}});
 async function attachInBots(owner,taskKey,taskName){
  const input={mode:'attach',sessionId:owner.threadId,requestId:randomUUID(),taskKey,taskName,profileMode:'existing',profile:'borrowed-profile'};
  let response;try{response=await fetch(service.url+`/api/bots/${owner.id}/hosted-browser`,{method:'POST',headers:{origin:service.url,'content-type':'application/json'},body:JSON.stringify(input),signal:AbortSignal.timeout(120000)});}catch(error){ambiguous=true;throw error;}
  let opened;try{opened=await response.json();}catch(error){ambiguous=true;throw error;}
  await save('bots-attach-'+owner.id+'.json',{status:response.status,input,opened});
  if(response.status!==201)throw Object.assign(Error(opened.message||'Bots attach refused'),{code:opened.error||'fixture_bots_attach_rejected'});
  const handoff=new URL(opened.handoffUrl);assert.equal(handoff.origin,appOrigin);assert.match(handoff.pathname,/^\/agent-browser\/handoff\/[a-f0-9-]{36}$/i);assert.equal(handoff.search,'');assert.equal(handoff.hash,'');
  return opened;
 }
 const primaryTaskKey='attachment-primary-'+randomUUID();
 step='attach-through-bots-http';const opened=await attachInBots(bot,primaryTaskKey,'Use app-owned synthetic browser');task=opened;borrowedTasks.push(opened);
 assert.equal(typeof opened.tabId,'string');assert.notEqual(opened.tabId,primaryTarget);assert.equal(opened.host.hostId,'bots');assert.equal(opened.host.participantId,'bot:'+bot.id);await continuity('first-attach');
 step='public-page-action';const snapshotBefore=await workflow('browser_snapshot',{taskId:opened.taskId});assert(JSON.stringify(snapshotBefore).includes('ATTACHMENT SYNTHETIC MARKER'));
 if(process.env.HOSTED_SMOKE_MODE_PREFLIGHT==='1'){
  step='public-page-mode-preflight';
  async function processShape(){
   const observed=await processIdentity(original.pid),executable=await realpath(`/proc/${original.pid}/exe`),argv=await readFile(`/proc/${original.pid}/cmdline`);
   const digest=createHash('sha256');for await(const chunk of createReadStream(`/proc/${original.pid}/exe`))digest.update(chunk);
   const args=argv.toString('utf8').split('\0').filter(Boolean),split=args.flatMap(arg=>arg.split(/[ \t\n\r\f\v]+/).filter(Boolean));
   const debug=values=>values.some(arg=>['--remote-debugging-port','--remote-debugging-pipe'].some(flag=>arg===flag||arg.startsWith(flag+'=')));
   const rootType=values=>values.some(arg=>arg==='--type'||arg.startsWith('--type='));
   const current=await processIdentity(original.pid);
   return {pid:observed.pid,start:observed.start,executable,executableSha256:digest.digest('hex'),
    stableDuringRead:current.start===observed.start&&current.pid===observed.pid,
    matchesOriginal:observed.start===original.start&&executable===original.executablePath,
    completeArgvPresent:argv.length>0&&argv.at(-1)===0,argvBytes:argv.length,argvSha256:createHash('sha256').update(argv).digest('hex'),
    nulArgc:args.length,anyEmbeddedAsciiWhitespace:args.some(arg=>/[ \t\n\r\f\v]/.test(arg)),
    debugWholeToken:debug(args),debugAsciiWhitespaceSplit:debug(split),rootTypeWholeToken:rootType(args),rootTypeAsciiWhitespaceSplit:rootType(split)};
  }
  const before=await processShape();
  const mode=await publicWorkflow('browser_command',{taskId:opened.taskId,action:'mode'});
  await save('pre-fill-browser-mode.json',mode);
  const after=await processShape();
  await save('pre-fill-owner-process-proof.json',{before,after,
   identityStable:before.pid===after.pid&&before.start===after.start&&before.executable===after.executable&&before.executableSha256===after.executableSha256,
   argvStable:before.argvSha256===after.argvSha256,
   modeJoins:{task:mode.taskId===opened.taskId,browser:mode.browserId===opened.browserId,profile:mode.profileId===opened.profileId,pid:mode.pid===original.pid},
   scope:'Current synthetic app owner parent readback; no raw argv retained or substituted for MCP mode observation'});
  if(mode.mode==='unverified')throw Object.assign(Error('Public browser mode unverified; no fill dispatched'),{code:'fixture_browser_mode_unverified'});
  if(process.env.HOSTED_SMOKE_MODE_DIAGNOSTIC_ONLY==='1')throw Object.assign(Error('Read-only mode diagnostic complete; no fill dispatched'),{code:'fixture_mode_diagnostic_complete'});
 }
 await workflow('browser_fill',{taskId:opened.taskId,selector:'#control',value:'public MCP preserved app browser'});assert.equal(await primary.locator('#control').inputValue(),'public MCP preserved app browser');await continuity('public-action');
 step='shared-attach-through-bots-http';const shared=await attachInBots(second,'attachment-shared-'+randomUUID(),'Explicitly share app-owned synthetic browser');borrowedTasks.push(shared);assert.notEqual(shared.taskId,opened.taskId);assert.notEqual(shared.tabId,opened.tabId);assert.notEqual(shared.tabId,primaryTarget);assert.equal(shared.browserId,opened.browserId);assert.equal(acquired.length,1);await continuity('shared-attach');
 await save('attachment.json',{original,processProof,initialTargets,primaryTarget,peerTarget,tasks:borrowedTasks.map(t=>({taskId:t.taskId,browserId:t.browserId,tabId:t.tabId,profileId:t.profileId})),scope:'Actual Bots attach HTTP and trusted fresh attachment observation, public Agent Browser controls, app-routed durable handoff and actual Native viewer'});
 task=opened;
 const sessionName='workflow-'+createHash('sha256').update(JSON.stringify(['codex-bots','bot:'+bot.id,primaryTaskKey,'borrowed-profile','bots','bot:'+bot.id,bot.threadId])).digest('hex');
 step='enable-isolated-service-http';
 let stream=JSON.parse((await runCli(context,['--json','--session',sessionName,'stream','status'])).stdout);
 assert.equal(stream.success,true);
 if(!stream.data?.enabled)stream=JSON.parse((await runCli(context,['--json','--session',sessionName,'stream','enable'])).stdout);
 assert.equal(stream.success,true);assert(Number.isInteger(stream.data?.port)&&stream.data.port>0);
 const servicePort=stream.data.port;
 step='public-control-qualification';
 await qualifyHostedBrowserControl({send,runCli:args=>runCli(context,args),task:opened,sessionName,readTask,save,syntheticOnly:true,
  serviceRequest:async body=>{const response=await httpJsonResult(servicePort,'POST','/api/service/request',body);assert.equal(response.statusCode,200);assert.equal(response.body?.success,false);return response.body;}});
 await continuity('after-human-control');
 step='native-viewer';viewerServer=await chromium.launchServer({executablePath:chrome,headless:true,
  ...(fixtureNativeConnectOrigin?{args:['--no-proxy-server','--host-resolver-rules=MAP native.p279.example.invalid 127.0.0.1']}:{}),env:{...env,AGENT_BROWSER_HOME:agentHome}});
 const viewerProcess=viewerServer.process();assert(viewerProcess?.pid,'Viewer server must expose its owned process');
 viewerRootProof=await cleanupProcessIdentity(viewerProcess.pid);assert(viewerRootProof,'Viewer root must be live');
 observedOwnedProcesses.set(viewerRootProof.pid,viewerRootProof);
 await save('viewer-process-root-proof.json',viewerRootProof);
 assert.equal(viewerRootProof.uid,process.getuid());assert.equal(viewerRootProof.executable,chrome);
 assert.equal(viewerRootProof.session,viewerRootProof.pid);assert.equal(viewerRootProof.processGroup,viewerRootProof.pid);
 viewerOwnedProcesses.set(viewerRootProof.pid,viewerRootProof);
 viewer=await chromium.connect(viewerServer.wsEndpoint());
 let viewerContext,page,frames=0,canvas=null;
 const coldSamples=[],coldContexts=new Set(),coldCount=workloadPlan?.coldContexts??1;
 const fixtureTls=process.env.HOSTED_SMOKE_FIXTURE_TLS==='1';
 if(fixtureTls){assert(workloadPlan,'Private TLS mode requires the declared isolated workload');assert(fixtureNativeConnectOrigin||new URL(nativeOrigin).hostname==='127.0.0.1');}
 uiHandoffId=new URL(opened.handoffUrl).pathname.split('/').at(-1);
 for(let index=0;index<coldCount;index++){
  if(viewerContext)await viewerContext.close();
  const sample={id:randomUUID(),contextId:randomUUID(),runId:workloadPlan?.runId??'single-cold-sample',pairDigest:workloadPlan?.pairDigest??null,outcome:'failed_or_unknown',contextCreated:false};
  let sampleFrames=0;const transportDiagnostics=[];
  const recordTransport=(url,detail)=>{const address=new URL(url);if(address.origin===nativeOrigin&&transportDiagnostics.length<64)transportDiagnostics.push({pathKind:address.pathname.split('/')[1]||'root',...detail});};
 coldPixels=null;
  try{
   viewerContext=await viewer.newContext({...(authState?{storageState:authState}:{}),viewport:{width:1440,height:1000},ignoreHTTPSErrors:fixtureTls});
   if(fixtureNativeConnectOrigin)await viewerContext.grantPermissions(['local-network-access'],{origin:appOrigin});
   assert(!coldContexts.has(viewerContext));coldContexts.add(viewerContext);assert.equal(viewer.contexts().length,1,'Cold contexts must not overlap');sample.contextCreated=true;
   page=await viewerContext.newPage();page.on('websocket',ws=>{if(new URL(ws.url()).origin===nativeOrigin.replace('https:','wss:'))ws.on('framereceived',()=>sampleFrames++);});
   page.on('requestfailed',request=>recordTransport(request.url(),{event:'request_failed',resourceType:request.resourceType(),code:request.failure()?.errorText}));
   page.on('response',response=>{if(response.status()>=400)recordTransport(response.url(),{event:'http_refusal',status:response.status()});});
   // Only the app origin reaches the local actual consumer. Native frames and
   // WebSocket traffic still reach the live, independently owned producer.
   await page.route(appOrigin+'/**',async route=>{const request=route.request(),url=new URL(request.url()),headers={...request.headers(),host:new URL(service.url).host};if(headers.origin===appOrigin)headers.origin=service.url;const response=await route.fetch({url:service.url+url.pathname+url.search,headers});await route.fulfill({response});});
   const chatReady=page.waitForResponse(response=>new URL(response.url()).pathname===`/api/bots/${bot.id}/chat`,{timeout:15000});
   const statusReady=page.waitForResponse(response=>new URL(response.url()).pathname===`/api/bots/${bot.id}/status`,{timeout:15000});
   coldPixels=createColdFirstPixelClock();
   const [chatResponse,statusResponse]=await Promise.all([chatReady,statusReady,page.goto(opened.handoffUrl,{waitUntil:'domcontentloaded',timeout:30000})]);
   assert.equal(chatResponse.status(),200);assert.equal(statusResponse.status(),200);
   const chatState=await chatResponse.json(),peerStatus=await statusResponse.json();assert.equal(chatState.threadId,bot.threadId);assert(Array.isArray(chatState.messages));assert.equal(peerStatus.connected,true);
   await save('synthetic-chat-readiness.json',{chatStatus:200,statusStatus:200,threadMatched:true,connected:true});
   await page.locator('#bot-name').filter({hasText:bot.name}).waitFor({timeout:15000});
   await census();await save('app-process-observed.json',[...appOwnedProcesses.values()]);
   assert.equal(await page.locator('#error').isVisible(),false,'Synthetic peer chat must not be degraded');
   let rendered=false;canvas=null;const renderDeadline=Date.now()+45000;
   while(Date.now()<renderDeadline&&!rendered){
    for(const frame of page.frames().filter(f=>f.url().startsWith(nativeOrigin+'/'))){
   canvas=await frame.evaluate(()=>{for(const c of document.querySelectorAll('canvas')){if(c.width<100||c.height<100)continue;try{const ctx=c.getContext('2d');if(!ctx)continue;let markers=0;for(let x=0.1;x<0.95;x+=0.1)for(let y=0.1;y<0.9;y+=0.1){const p=ctx.getImageData(Math.floor(c.width*x),Math.floor(c.height*y),1,1).data;if(p[0]>200&&p[1]<90&&p[2]>100)markers++;}if(markers>=8)return {width:c.width,height:c.height,markers};}catch{}}return null;});
     if(canvas&&coldPixels.observe({canvas,receivedWebSocketFrames:sampleFrames})){rendered=true;break;}
    }
    if(!rendered)await new Promise(resolve=>setTimeout(resolve,250));
   }
   frames=sampleFrames;await save('render.json',{rendered,canvas,receivedWebSocketFrames:frames});
   assert(rendered,'Actual Native canvas must render the synthetic marker with WebSocket frames');
   sample.outcome='validated_response';
  }catch(error){sample.failureCode=errorCode(error);throw error;}
  finally{
   sample.pixels=coldPixels?.receipt()??null;sample.transportDiagnostics=transportDiagnostics;coldSamples.push(sample);
   await save(`cold-context-${index+1}.json`,sample);
   if(workloadPlan&&!inputDiagnostic)await save('cold-first-pixels-distribution.json',summarizeColdFirstPixels(coldSamples,{runId:workloadPlan.runId,pairDigest:workloadPlan.pairDigest,expectedSamples:workloadPlan.coldContexts}));
  }
 }
 await page.screenshot({path:join(evidence,'hosted-native.png')});await import('node:fs/promises').then(fs=>fs.chmod(join(evidence,'hosted-native.png'),0o600));
 await save('cold-first-pixels.json',coldPixels.receipt());
 // Qualify the app's real Native toolbar callback against both owners. Viewing
 // alone is still agent-owned; no Agent Browser dashboard or extra assignment.
 async function consumerControl(){const response=await fetch(service.url+`/api/agent-browser/handoffs/${uiHandoffId}/control`,{signal:AbortSignal.timeout(15000)});assert.equal(response.status,200);return response.json();}
 const controlReceipt=value=>({control:Object.fromEntries(['owner','epoch','pauseReads','pendingOutcomeCount'].filter(key=>key in value.control).map(key=>[key,value.control[key]])),native:Object.fromEntries(['owner','state','epoch','hostedOperationId'].filter(key=>key in value.native).map(key=>[key,value.native[key]]))});
 const beforeUi=await consumerControl();assert.equal(beforeUi.control.owner,'agent');assert.equal(beforeUi.native.owner,'agent');
 if(nativeCheckbox){
  step='native-checkbox-assistance';
  await qualifyNativeCheckbox({workflow:publicWorkflow,task:opened,nextTask:async label=>{
    const current=await attachInBots(bot,'checkbox470-'+label+'-'+randomUUID(),'Owned Native checkbox '+label);borrowedTasks.push(current);return current;
  },primary,site,profilePath,configFile:join(agentHome,'config.json'),faultFile:join(evidence,'native-checkbox-fault.json'),oracle:checkboxOracle,save,continuity,observeOnly:process.env.HOSTED_SMOKE_NATIVE_CHECKBOX_OBSERVE_ONLY==='1'});
  await save('native-input-diagnostic.json',process.env.HOSTED_SMOKE_NATIVE_CHECKBOX_OBSERVE_ONLY==='1'
    ?{passed:true,fullMatrix:false,diagnosticKind:'read_only_native_checkbox_capture',actualNativeInput:false,challengeInputDispatched:false,scope:'Owned synthetic Native capture/rendered-state diagnosis only; no checkbox acceptance'}
    :{passed:true,fullMatrix:false,scope:'Owned Native checkbox/passive assistance, owner policy and independent HTTPS oracle'});break workload;
 }
 if(process.env.HOSTED_SMOKE_OBSERVATION_AGE_PROBE==='1'){
  step='current-native-observation-age';await probeCurrentNativeObservationAge({workflow:publicWorkflow,task:opened,consumerControl,save});
  await continuity('after-native-motion-age-probe');
  await save('native-input-diagnostic.json',{passed:true,fullMatrix:false,scope:'Current Native motion-only observation-age characterization; no click or operator takeover'});break workload;
 }
 if(workloadPlan){
  const desktop=await remote.observation(binding),windows=desktop.windows.windows.filter(window=>window.pid===original.pid);
  assert.equal(windows.length,1,'The owned browser requires one positively observed Native window');
  const window=windows[0];assert.equal(window.processIdentity.start_token,original.start);assert.equal(window.processIdentity.executable,original.executablePath);
  const activated=await remote.activateWindow(binding,{windowId:window.id,expectedProcessIdentity:window.processIdentity});
  await save('native-human-window-focus.json',{windowId:window.id,processId:original.pid,activated});
 }
 await page.getByText('Browser control options',{exact:true}).click();await page.locator('#hosted-pause-reads').check();
 step='native-toolbar-takeover';uiTakeoverDispatched=true;uiControlSettled=false;
 await page.frameLocator('#desktop-frame').getByRole('button',{name:'Take control',exact:true}).click({timeout:15000});
 await page.frameLocator('#desktop-frame').getByRole('button',{name:'Give control back',exact:true}).waitFor({timeout:30000});
 const human=await consumerControl();assert.equal(human.control.owner,'human');assert.equal(human.control.pauseReads,true);assert.equal(human.control.epoch,beforeUi.control.epoch+1);assert.equal(human.native.owner,'operator');assert.equal(human.native.state,'ready');uiControlSettled=true;
 assert.equal((await readTask(opened.taskId)).controllerState.owner,'human');
 await save('bots-native-control.json',{before:controlReceipt(beforeUi),human:controlReceipt(human),passed:false});
 if(workloadPlan){
  step='native-human-text-edit';
  const edit='native-human-'+randomUUID();
  await save('native-human-edit-intent.json',{syntheticOnly:true,state:'prepared',dispatchCount:0,expectedValue:edit,through:'Native canvas keyboard'});
  const nativeFrame=page.frames().find(frame=>frame.url().startsWith(nativeOrigin+'/'));
  assert(nativeFrame,'The actual Native frame must remain attached');
  const readiness=async phase=>save('native-human-readiness-'+phase+'.json',await nativeFrame.evaluate(()=>({capability:document.body.dataset.capability,connection:document.querySelector('#connection-status')?.dataset.state,connectionMessage:document.querySelector('#connection-status')?.textContent,input:document.querySelector('#input-status')?.dataset.state,activeId:document.activeElement.id})));
  await readiness('before');
  try{
   step='native-human-viewer-connected';await nativeFrame.waitForFunction(()=>document.querySelector('#connection-status')?.dataset.state==='connected',null,{timeout:15000});
   step='native-human-input-unlocked';await nativeFrame.waitForFunction(()=>['active','released'].includes(document.querySelector('#input-status')?.dataset.state),null,{timeout:15000});
  }finally{await readiness('after');}
  step='native-human-text-edit';
  const inputSurface=nativeFrame.locator('#desktop-surface'),surface=await inputSurface.boundingBox();
  assert(surface&&surface.width>100&&surface.height>100);
  step='native-human-surface-focus';
  await inputSurface.click({position:{x:surface.width/2,y:surface.height/2},timeout:15000});
  await inputSurface.focus();
  assert.equal(await nativeFrame.evaluate(()=>document.activeElement.id),'desktop-surface');
  const focusState=async phase=>save('native-human-focus-'+phase+'.json',{viewer:await nativeFrame.evaluate(()=>({activeId:document.activeElement.id,connection:document.querySelector('#connection-status')?.dataset.state,input:document.querySelector('#input-status')?.dataset.state})),app:await primary.evaluate(()=>({activeId:document.activeElement.id,draft:document.querySelector('#draft').value,control:document.querySelector('#control').value}))});
  await focusState('before-tab');step='native-human-first-tab';await page.keyboard.press('Tab');
  try{await primary.waitForFunction(()=>document.activeElement.id==='draft',null,{timeout:5000});}finally{await focusState('first-tab');}
  step='native-human-second-tab';await page.keyboard.press('Tab');
  try{await primary.waitForFunction(()=>document.activeElement.id==='control',null,{timeout:5000});}finally{await focusState('second-tab');}
  step='native-human-text-edit';
  await page.keyboard.press('Control+A');
  // Dispatch exactly once; independent CDP reads may observe completion, never
  // repeat keyboard input after an uncertain response.
  await save('native-human-edit-intent.json',{syntheticOnly:true,state:'dispatching',dispatchCount:1,expectedValue:edit,through:'Native canvas keyboard'});
  await page.keyboard.type(edit);
  await primary.waitForFunction(expected=>document.querySelector('#control').value===expected,edit,{timeout:15000});
  assert.equal(await primary.locator('#control').inputValue(),edit);await continuity('after-native-human-edit');
  await save('native-human-edit.json',{syntheticOnly:true,dispatchCount:1,expectedValue:edit,observedValue:await primary.locator('#control').inputValue(),passed:true});
 }
 step='native-toolbar-return';uiReturnDispatched=true;uiControlSettled=false;
 await page.frameLocator('#desktop-frame').getByRole('button',{name:'Give control back',exact:true}).click({timeout:15000});
 await page.frameLocator('#desktop-frame').getByRole('button',{name:'Take control',exact:true}).waitFor({timeout:30000});
 const returned=await consumerControl();assert.equal(returned.control.owner,'agent');assert.equal(returned.control.epoch,human.control.epoch+1);assert.equal(returned.native.owner,'agent');assert.equal(returned.native.state,'ready');uiControlSettled=true;
 assert.equal((await readTask(opened.taskId)).controllerState.owner,'agent');await continuity('after-native-toolbar-control');
 await save('bots-native-control.json',{before:controlReceipt(beforeUi),human:controlReceipt(human),returned:controlReceipt(returned),passed:true});
 if(challenge469){
  await qualifyNativeChallengeAssistance({workflow:publicWorkflow,task:opened,page,primary,nativeOrigin,site,save,consumerControl});
  await continuity('after-challenge469');
  uiTakeoverDispatched=true;uiReturnDispatched=false;
  await qualifyNativeNoCdpChallenge({workflow:publicWorkflow,runCli,context,chrome,service,bot,page,nativeOrigin,site,save,
   selectHandoff:manual=>{managedLossTasks.push(manual);uiHandoffId=new URL(manual.handoffUrl).pathname.split('/').at(-1);return consumerControl;},
   onClosed:id=>closedManagedLossTasks.add(id),
   observeOwned:async(pid,profile)=>{const identity=await cleanupProcessIdentity(pid);assert(identity&&identity.uid===process.getuid()&&identity.executable===chrome);assert(privateProfileReferenced(await readFile(`/proc/${pid}/cmdline`,'utf8'),profile));observedOwnedProcesses.set(pid,identity);await census();return identity;}});
  uiReturnDispatched=true;
  await continuity('after-no-cdp-challenge469');
 }
 if(inputDiagnostic){await save('native-input-diagnostic.json',{passed:true,fullMatrix:false,scope:'One Native text edit and return with independent app observation'});break workload;}
 // Start after all control transitions so the remaining event is scoped detach.
 if(process.env.HOSTED_SMOKE_CONTINUITY_REQUIRED==='1'){
  step='explicit-host-reconnect';const reconnection=await qualifyHostedBrowserReconnect({workflow:publicWorkflow,task:opened,save,syntheticOnly:true,requirePerformanceTargets:false,nativeControl:{status:consumerControl,change:async body=>{
   uiControlSettled=false;
   try{
    const response=await fetch(service.url+`/api/agent-browser/handoffs/${uiHandoffId}/control`,{method:'POST',headers:{origin:service.url,'content-type':'application/json'},body:JSON.stringify(body),signal:AbortSignal.timeout(120000)});
    assert.equal(response.status,200);const value=await response.json();
    assert.equal(value.control.owner,body.operation==='takeover'?'human':'agent');assert.equal(value.native.owner,body.operation==='takeover'?'operator':'agent');
    uiControlSettled=true;return value;
   }catch(error){ambiguous=true;throw error;}
  }}});performanceTargetsMet=reconnection.performance.allMeasuredTargetsMet;
  await continuity('after-host-reconnect');
  const artifactRoot=join(root,'authorized-artifacts');await mkdir(artifactRoot,{mode:0o700});
  step='task-policy-and-artifacts';await qualifyHostedBrowserPolicy({workflow:publicWorkflow,task:opened,peerTask:shared,artifactRoot,save});
  assert.equal((await publicWorkflow('browser_control',{taskId:opened.taskId,operation:'status'})).control.pendingOutcomeCount,0,'Known refusals must not manufacture uncertain website effects');
  await continuity('after-host-policy');
 }
 step='metadata-initial';const watchReader=startWatch(task.taskId);
 const snapshot=await readTask(task.taskId),event=await watchReader.next();assert.equal(event.event,'snapshot');assert.deepEqual(event.data,snapshot);assert.equal(snapshot.taskId,task.taskId);await save('metadata.json',{snapshot,event});

 // Produce a genuine metadata transition after the visual proof. Closing the
 // exact fixture task is dispatched once, including when its outcome is unknown.
 step='close-exact-task';taskCloseDispatched=true;closedBorrowedTasks.add(task.taskId);await workflow('browser_close',{taskId:task.taskId});taskClosed=true;await continuity('first-detach');const sharedSnapshot=await workflow('browser_snapshot',{taskId:shared.taskId});assert(JSON.stringify(sharedSnapshot).includes('ATTACHMENT SYNTHETIC MARKER'));closedBorrowedTasks.add(shared.taskId);await workflow('browser_close',{taskId:shared.taskId});await continuity('all-detached');
 step='metadata-changed';const changed=await watchReader.next();
 assert.equal(changed.event,'changed');assert.equal(changed.data.taskId,task.taskId);assert.equal(changed.data.taskState,'ended');
 assert.equal(changed.cursor.watchId,event.cursor.watchId);assert.equal(changed.cursor.sequence,2);
 const ended=await readTask(task.taskId);assert.equal(ended.taskState,'ended');assert.deepEqual(changed.data,ended);
 await save('metadata.json',{snapshot,event,changed,ended});
 if(process.env.HOSTED_SMOKE_LOSS_REQUIRED==='1'){
  step='create-fresh-managed-loss-task';
  const profileName='managed-loss-'+randomUUID();
  const created=JSON.parse((await runCli(context,['--json','runtime','create',profileName,'--browser-family','chromium'])).stdout);assert.equal(created.created,true);
  const lossProfile=await realpath(created.userDataDir);assert(lossProfile.startsWith(root+'/'));
  const managed=await publicWorkflow('browser_open',{taskKey:'managed-loss-'+randomUUID(),serviceName:'synthetic-hosted-acceptance',agentName:'loss-fixture',taskName:'Qualify exact owned browser loss and explicit reopen',profile:profileName,profileMode:'persistent',url:site+'/primary',hostBinding:{version:1,hostId:'bots',participantId:'managed-loss-'+randomUUID(),sessionId:'managed-loss-'+randomUUID(),placement:{mode:'required',desktop:{desktopId:binding.desktopId,generation:binding.generation}}}});
  managedLossTasks.push(managed);assert.equal(managed.profileId,profileName);assert.notEqual(managed.browserId,opened.browserId);
  const record=()=>JSON.parse(execFileSync('python3',['-c',"import json,sqlite3,sys;from pathlib import Path;c=sqlite3.connect(Path(sys.argv[1]).as_uri()+'?mode=ro',uri=True);s=json.loads(c.execute('select json from state_documents where kind=?',('browser_session_state',)).fetchone()[0]);t=s['sessions'][sys.argv[2]];assert t['browserId']==sys.argv[3];print(json.dumps(s['browsers'][sys.argv[3]]))",join(agentHome,'service','runtime.sqlite3'),managed.taskId,managed.browserId],{env,encoding:'utf8',timeout:10000}));
  const observeProcess=async({expected})=>{
   if(expected){try{await processIdentity(expected.pid);return {...expected,alive:true,pidAbsent:false};}catch(error){if(!['ENOENT','ESRCH'].includes(error.code))throw error;return {...expected,alive:false,pidAbsent:true};}}
   const browser=record(),kernel=await cleanupProcessIdentity(browser.pid);assert(kernel&&kernel.uid===process.getuid());assert.equal(browser.profileId,profileName);assert.equal(kernel.executable,chrome);
   const commandline=await readFile(`/proc/${browser.pid}/cmdline`,'utf8');assert(privateProfileReferenced(commandline,lossProfile));
   const proof={pid:browser.pid,startToken:`linux:${boot}:${kernel.start}`,uid:kernel.uid,executablePath:kernel.executable,profilePath:lossProfile,freshOwned:true,custody:'managed',alive:true};
   observedOwnedProcesses.set(kernel.pid,kernel);await census();return proof;
  };
  step='actual-managed-browser-loss';
  await qualifyHostedBrowserLoss({workflow:publicWorkflow,task:managed,desktop:{desktopId:binding.desktopId,generation:binding.generation},syntheticOnly:true,save,observeProcess,
   terminateOwnedBrowser:async proof=>{
    const fresh=await observeProcess({expected:null});assert.deepEqual(fresh, {...proof,freshOwned:true,custody:'managed',alive:true});
    await save('managed-loss-signal-intent.json',{...proof,signal:'SIGKILL',syntheticOnly:true});
    const confirmed=await cleanupProcessIdentity(proof.pid);assert(confirmed);assert.equal(`linux:${boot}:${confirmed.start}`,proof.startToken);assert.equal(confirmed.uid,proof.uid);assert.equal(confirmed.executable,proof.executablePath);
    process.kill(proof.pid,'SIGKILL');
    const deadline=Date.now()+10000;let absent=false;while(Date.now()<deadline){try{await processIdentity(proof.pid);}catch(error){if(!['ENOENT','ESRCH'].includes(error.code))throw error;absent=true;break;}await new Promise(resolve=>setTimeout(resolve,100));}
    assert.equal(absent,true,'A zombie or reused PID is not qualified process absence');
    return {pid:proof.pid,startToken:proof.startToken,originalGenerationAbsent:true};
   },
   observeDesktop:async()=>{const current=await remote.observe(binding);assert.equal(current.generation,binding.generation);return {desktopId:current.desktopId,generation:current.generation,state:current.state==='ready'?'active':current.state};},
   observePeers:async()=>{await continuity('during-managed-loss');const inventory=await remote.inventory(),ids=new Set(baseline.assignments.map(a=>a.assignmentId));return peerIdentity(inventory).filter(a=>ids.has(a.assignmentId));},
  });
  closedManagedLossTasks.add(managed.taskId);await publicWorkflow('browser_close',{taskId:managed.taskId});await continuity('after-managed-loss');
 }
 if(restartBridge){
  // The app-owned attachment has completed its scoped lifecycle. Retire it
  // through its own exact custody before testing an independently managed task.
  // Native's second desktop/app remains protected by the outer creating fixture.
  step='retire-attachment-before-restart';
  await boundedTeardown('pre-restart-viewer',()=>viewer.close());viewer=null;
  await boundedTeardown('pre-restart-viewer-server',()=>viewerServer.close());viewerServer=null;
  await boundedTeardown('pre-restart-consumer',()=>service.close());service=null;
  await boundedTeardown('pre-restart-observer',()=>appObserver.close());appObserver=null;
  const retired={};
  const retirement=await cleanupExactOwnedProcesses({known:appOwnedProcesses,
   census:async()=>{const sample=await census({collectUnresolved:true});return {...sample,owned:sample.owned.filter(row=>appOwnedProcesses.get(row.pid)?.start===row.start)};},
   readIdentity:cleanupProcessIdentity,signal:(pid,selected)=>process.kill(pid,selected),sleep:ms=>new Promise(resolve=>setTimeout(resolve,ms)),
   allowSigkill:true,graceMs:3000,killMs:3000,pollMs:100,receipt:retired,record:()=>save('pre-restart-app-cleanup.json',retired)});
  assert.equal(retirement.qualified,true);appBrowser=null;
  const profileName='managed-restart-'+randomUUID();
  const created=JSON.parse((await runCli(context,['--json','runtime','create',profileName,'--browser-family','chromium'])).stdout);assert.equal(created.created,true);
  const restartProfile=await realpath(created.userDataDir);assert(restartProfile.startsWith(root+'/'));
  const managed=await publicWorkflow('browser_open',{taskKey:'managed-restart-'+randomUUID(),serviceName:'synthetic-hosted-acceptance',agentName:'restart-fixture',taskName:'Explicit same-task reopen after isolated Native restart',profile:profileName,profileMode:'persistent',url:site+'/primary',hostBinding:{version:1,hostId:'bots',participantId:'managed-restart-'+randomUUID(),sessionId:'managed-restart-'+randomUUID(),placement:{mode:'required',desktop:{desktopId:binding.desktopId,generation:binding.generation}}}});
  managedLossTasks.push(managed);
  const record=()=>JSON.parse(execFileSync('/usr/bin/python3',['-c',"import json,sqlite3,sys;from pathlib import Path;c=sqlite3.connect(Path(sys.argv[1]).as_uri()+'?mode=ro',uri=True);s=json.loads(c.execute('select json from state_documents where kind=?',('browser_session_state',)).fetchone()[0]);t=s['sessions'][sys.argv[2]];assert t['browserId']==sys.argv[3];print(json.dumps(s['browsers'][sys.argv[3]]))",join(agentHome,'service','runtime.sqlite3'),managed.taskId,managed.browserId],{env,encoding:'utf8',timeout:10000}));
  const observeProcess=async expected=>{
   if(expected){try{await processIdentity(expected.pid);return {...expected,pidAbsent:false};}catch(error){if(!['ENOENT','ESRCH'].includes(error.code))throw error;return {...expected,pidAbsent:true,alive:false};}}
   const browser=record(),proof=await cleanupProcessIdentity(browser.pid);assert(proof&&proof.uid===process.getuid());assert.equal(proof.executable,chrome);assert.equal(browser.profileId,profileName);
   assert(privateProfileReferenced(await readFile(`/proc/${browser.pid}/cmdline`,'utf8'),restartProfile));observedOwnedProcesses.set(proof.pid,proof);await census();
   return {pid:proof.pid,startToken:`linux:${boot}:${proof.start}`,kernelStart:proof.start,uid:proof.uid,executablePath:proof.executable,profilePath:restartProfile,alive:true};
  };
  const bridge=async(operation,extra={})=>{
   const requestId=randomUUID();await save('bridge-intent-'+operation+'.json',{requestId,operation,...extra});
   let response;try{response=await fetch(restartBridge+'/'+operation,{method:'POST',headers:{authorization:restartAuthorization,'content-type':'application/json'},body:JSON.stringify({requestId,...extra}),redirect:'error',signal:AbortSignal.timeout(300000)});}catch(error){ambiguous=true;throw error;}
   try{
    const body=await response.json();assert.equal(response.status,200);assert.equal(body.ok,true);
    if(operation==='restart'){
     const {before,after}=body.value;
     assert.equal(before.assignmentId,binding.assignmentId);assert.equal(after.assignmentId,binding.assignmentId);
     assert.equal(before.desktopId,binding.desktopId);assert.equal(after.desktopId,binding.desktopId);
     assert.equal(before.lifecycleGeneration,binding.generation);assert.equal(after.lifecycleGeneration,binding.generation);
     assert.equal(before.viewingGeneration,binding.viewingGeneration);assert(after.viewingGeneration>before.viewingGeneration);
     // Update only the creating broker's observed Native viewing fence. The
     // task retains its public lifecycle binding until explicit rebind/reopen.
     binding.viewingGeneration=after.viewingGeneration;await save('acquired.json',acquired);
    }
    return body.value;
   }catch(error){if(operation==='restart')ambiguous=true;throw error;}
  };
  step='actual-native-restart-and-explicit-reopen';
  await qualifyHostedBrowserRestart({workflow:publicWorkflow,task:managed,observeProcess,save,syntheticOnly:true,
   observePeers:()=>bridge('peers'),restartNative:proof=>bridge('restart',{proof}),observeReopenedNative:proof=>bridge('reopened',{proof})});
  closedManagedLossTasks.add(managed.taskId);await publicWorkflow('browser_close',{taskId:managed.taskId});
 }
 assert(!brokerOperations.some(entry=>['acquire','release','release_idle'].includes(entry.operation)), 'Hosted runtime must never request app pool growth or retirement');
 await save('qualification-summary.json',{functionalPassed:true,performancePassed:performanceTargetsMet,passed:performanceTargetsMet});
 passed=performanceTargetsMet;if(!passed)process.exitCode=1;
}}catch(error){if(!uiControlSettled)ambiguous=true;await save('failure.json',{...safeError(error),ambiguous});process.exitCode=1;}
finally{
 async function clean(name,action){try{await boundedTeardown(name,action);}catch(error){cleanup.errors.push({...safeError(error),step:name});process.exitCode=1;}}
 const cleanupPhases=[];
 async function cleanupPhase(name,action){
  const phase={name,startedMs:performance.now(),state:'running'};cleanupPhases.push(phase);
  await save('cleanup-phases.json',{clock:'performance.now monotonic milliseconds',phases:cleanupPhases});
  try{const value=await action();phase.state='completed';return value;}
  catch(error){phase.state='failed';phase.code=errorCode(error);throw error;}
  finally{phase.durationMs=performance.now()-phase.startedMs;await save('cleanup-phases.json',{clock:'performance.now monotonic milliseconds',phases:cleanupPhases});}
 }
 await clean('record-stage-timings',()=>save('stage-timings.json',{clock:'performance.now monotonic milliseconds',samples:stageTimings}));
 if(coldPixels)await clean('record-cold-pixel-evidence',()=>save('cold-first-pixels.json',coldPixels.receipt()));
 if(uiHandoffId&&uiTakeoverDispatched&&!uiReturnDispatched)await clean('return-known-native-control',async()=>{
  const status=await fetch(service.url+`/api/agent-browser/handoffs/${uiHandoffId}/control`,{signal:AbortSignal.timeout(15000)});assert.equal(status.status,200);const current=await status.json();
  if(current.control.owner==='human'&&current.native.owner==='operator'&&current.native.state==='ready'){
   uiReturnDispatched=true;const response=await fetch(service.url+`/api/agent-browser/handoffs/${uiHandoffId}/control`,{method:'POST',headers:{origin:service.url,'content-type':'application/json'},body:JSON.stringify({operation:'return',operationId:randomUUID(),expectedEpoch:current.control.epoch,actor:'synthetic-operator',reason:'Explicit synthetic fixture cleanup'}),signal:AbortSignal.timeout(120000)});assert.equal(response.status,200);const returned=await response.json();assert.equal(returned.control.owner,'agent');assert.equal(returned.native.owner,'agent');cleanup.nativeControlReturned=true;
  }else assert.equal(current.control.owner,'agent','Uncertain Native takeover requires explicit readback');
 });
 await clean('close-viewer',async()=>{
  // Drain/ignore only this fixture's routes before closing their contexts;
  // otherwise an in-flight route.fetch can terminate Node during cleanup.
  for(const context of viewer?.contexts()??[])for(const page of context.pages())await page.unrouteAll({behavior:'ignoreErrors'});
  await viewer?.close();
 });
 await clean('close-viewer-server',async()=>{await viewerServer?.close();});
 await clean('stop-watch',async()=>{if(watcher&&watcher.exitCode===null&&watcher.signalCode===null){const exit=once(watcher,'exit');watcher.kill('SIGTERM');await exit;}});
 for(const borrowed of borrowedTasks)if(!closedBorrowedTasks.has(borrowed.taskId))await clean('close-borrowed-task',async()=>{closedBorrowedTasks.add(borrowed.taskId);await workflow('browser_close',{taskId:borrowed.taskId});});
 // App observer transport ends only after detach continuity was checked. The
 // exact owned-process census below remains responsible for app cleanup.
 for(const managed of managedLossTasks)if(!closedManagedLossTasks.has(managed.taskId))await clean('close-managed-loss-task',async()=>{closedManagedLossTasks.add(managed.taskId);await publicWorkflow('browser_close',{taskId:managed.taskId});});
 await clean('close-app-observer',async()=>{await appObserver?.close();});
 await clean('close-synthetic-site',async()=>{if(fixtureSite){fixtureSite.closeAllConnections();await new Promise(resolve=>fixtureSite.close(resolve));}});
 await clean('close-service',async()=>{await service?.close();});
 await clean('close-mcp',async()=>{await client?.close();});
 await clean('owned-process-cleanup',async()=>{
  cleanup.processesBefore=[...observedOwnedProcesses.values()];
  cleanup.processCleanup={};
  const result=await cleanupPhase('exact-owned-process-cleanup',()=>cleanupExactOwnedProcesses({known:observedOwnedProcesses,census:()=>cleanupPhase('owned-process-census',()=>census({collectUnresolved:true})),readIdentity:cleanupProcessIdentity,signal:(pid,selected)=>process.kill(pid,selected),sleep:ms=>new Promise(resolve=>setTimeout(resolve,ms)),allowSigkill:true,graceMs:3000,killMs:3000,pollMs:100,receipt:cleanup.processCleanup,record:async()=>save('cleanup.json',cleanup)}));
  cleanup.processesBefore=result.owned;
  cleanup.processesAfter=result.remaining;
  assert.equal(result.qualified,true,'Exact fixture cleanup remains unresolved; assignment cannot be released');
  cleanup.processesTerminated=true;
 });
 await clean('app-process-absence',async()=>{
  assert.equal(cleanup.errors.length,0,'Unresolved teardown blocks the application absence fence');
  assert.equal(cleanup.processesTerminated,true,'Exact fixture cleanup must complete before the application absence fence');
  await cleanupPhase('app-absence-fence',()=>appAbsenceFence());cleanup.processesClear=true;
 });
 if(remote&&baseline)await clean('assignment-cleanup',async()=>{
  assert.equal(cleanup.errors.length,0,'Unresolved teardown blocks assignment release');
  const afterStop=await remote.inventory();
  for(const binding of acquired){assert(!baseline.assignments.some(a=>a.assignmentId===binding.assignmentId));assert.equal(cleanup.processesClear,true,'Fresh process cleanup proof is required before release');if(afterStop.assignments.some(a=>a.assignmentId===binding.assignmentId&&a.state==='active')){const current=await remote.observe(binding);assert.equal(current.generation,binding.generation);await remote.release(current);cleanup.released.push({assignmentId:binding.assignmentId,generation:binding.generation});}}
  const final=await remote.inventory();await save('final-inventory.json',final);
  const ids=new Set(baseline.assignments.map(a=>a.assignmentId));assert.deepEqual(peerIdentity(final).filter(a=>ids.has(a.assignmentId)),peerIdentity(baseline),'Baseline peers changed');
  cleanup.unresolvedNewAssignments=final.assignments.filter(a=>!ids.has(a.assignmentId)&&a.state==='active').map(a=>({assignmentId:a.assignmentId,generation:a.generation}));assert.equal(cleanup.unresolvedNewAssignments.length,0,'New assignments require explicit reconciliation');
 });
 await clean('close-scoped-broker',async()=>{if(broker){broker.closeAllConnections();await new Promise(resolve=>broker.close(resolve));}});
 remote?.close();cleanup.ambiguous=ambiguous;cleanup.taskClosed=taskClosed;cleanup.taskCloseDispatched=taskCloseDispatched;cleanup.passed=passed;cleanup.cleanupQualified=cleanup.processesClear===true&&cleanup.errors.length===0;await save('cleanup.json',cleanup);
 if(ambiguous||cleanup.errors.length||!passed)process.exitCode=1;
 console.log(JSON.stringify({evidence,passed,cleanupQualified:cleanup.cleanupQualified,ambiguous}));
}
