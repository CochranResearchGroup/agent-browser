#!/usr/bin/env node
/** Explicit opt-in actual Native/Bots/Agent Browser integration qualification.
 * Required absolute paths: HOSTED_SMOKE_AB_BINARY, HOSTED_SMOKE_CHROME,
 * HOSTED_SMOKE_BOTS_ROOT, HOSTED_SMOKE_EVIDENCE_DIR. Required reviewed values:
 * HOSTED_SMOKE_INSTALLATION_ID, HOSTED_SMOKE_APP_ORIGIN, HOSTED_SMOKE_NATIVE_ORIGIN.
 * Optional HOSTED_SMOKE_AUTH_STATE is a private read-only Playwright storage state;
 * HOSTED_SMOKE_BROWSER_MANIFEST optionally pins the reviewed browser manifest.
 * HOSTED_SMOKE_NATIVE_SOCKET pins the already installed provider MCP socket.
 * This script creates one disposable task/assignment. It never installs, signs in,
 * retries effects, borrows a baseline assignment or removes retained evidence.
 */
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {createHash,randomUUID} from 'node:crypto';
import {once} from 'node:events';
import {createServer} from 'node:http';
import {mkdir,mkdtemp,readFile,readdir,realpath,stat,writeFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {isAbsolute,join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createRequire} from 'node:module';
import {createMcpStdioClient,runCli} from './smoke-utils.js';

const required=name=>{const value=process.env[name];assert(value,`Missing ${name}`);return value;};
async function absolute(name){const value=required(name);assert(isAbsolute(value),`${name} must be absolute`);return realpath(value);}
function origin(name){const value=required(name),url=new URL(value);assert(url.protocol==='https:'&&url.origin===value,`${name} must be canonical HTTPS`);return value;}
const binary=await absolute('HOSTED_SMOKE_AB_BINARY'),chrome=await absolute('HOSTED_SMOKE_CHROME');
const botsRoot=await absolute('HOSTED_SMOKE_BOTS_ROOT'),appOrigin=origin('HOSTED_SMOKE_APP_ORIGIN'),nativeOrigin=origin('HOSTED_SMOKE_NATIVE_ORIGIN');
const installationId=required('HOSTED_SMOKE_INSTALLATION_ID');
const evidence=required('HOSTED_SMOKE_EVIDENCE_DIR');assert(isAbsolute(evidence));
await mkdir(evidence,{mode:0o700}); // Existing evidence paths fail closed; never overwrite another run.
assert.equal((await stat(evidence)).mode&0o077,0,'Evidence directory must be private');
const save=(name,value)=>writeFile(join(evidence,name),JSON.stringify(value,null,2)+'\n',{mode:0o600});
await save('cleanup.json',{state:'preparing',effectsDispatched:false,retainedEvidence:evidence});
const root=await mkdtemp(join(evidence,'fixture-')),socketDir=await mkdtemp(join(tmpdir(),'abh-'));
const agentHome=join(root,'.agent-browser');
const env=Object.fromEntries(Object.entries(process.env).filter(([key])=>!key.startsWith('AGENT_BROWSER_')&&!key.startsWith('XDG_')&&!key.startsWith('HOSTED_SMOKE_')));
Object.assign(env,{HOME:root,AGENT_BROWSER_HOME:agentHome,AGENT_BROWSER_SOCKET_DIR:socketDir,AGENT_BROWSER_RUNTIME_HOST:'1',AGENT_BROWSER_EXECUTABLE_PATH:chrome,AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD:binary,AGENT_BROWSER_REMOTE_VIEW_APPLICATION:'codex-bots',AGENT_BROWSER_REMOTE_VIEW_PUBLIC_ORIGIN:nativeOrigin,AGENT_BROWSER_REMOTE_VIEW_POOL:'main',AGENT_BROWSER_REMOTE_VIEW_DESKTOP_COUNT:'1',AGENT_BROWSER_HOSTS_FILE:join(root,'hosts.json'),XDG_RUNTIME_DIR:join(root,'runtime'),XDG_CONFIG_HOME:join(root,'config'),XDG_CACHE_HOME:join(root,'cache'),XDG_DATA_HOME:join(root,'data')});
for(const path of [agentHome,socketDir,env.XDG_RUNTIME_DIR,env.XDG_CONFIG_HOME,env.XDG_CACHE_HOME,env.XDG_DATA_HOME])await mkdir(path,{recursive:true,mode:0o700});
if(process.env.HOSTED_SMOKE_BROWSER_MANIFEST)env.AGENT_BROWSER_STEALTHCDP_CHROMIUM_MANIFEST_PATH=await absolute('HOSTED_SMOKE_BROWSER_MANIFEST');
await writeFile(env.AGENT_BROWSER_HOSTS_FILE,JSON.stringify({version:1,hosts:{bots:{origin:appOrigin,providerApplication:'codex-bots'}}}),{mode:0o600});
const context={env,agentHome,socketDir,tempHome:root,session:'hosted-'+randomUUID()};
const {createBotService}=await import(pathToFileURL(join(botsRoot,'src/service.mjs')));
const {connectRemoteView}=await import(pathToFileURL(join(botsRoot,'src/remote-view.mjs')));
const require=createRequire(join(botsRoot,'package.json')),{chromium}=require('playwright-core');
let remote,baseline,broker,service,client,viewer,watcher,task,taskClosed=false,taskCloseDispatched=false,ambiguous=false,passed=false;
const acquired=[],cleanup={errors:[],retainedEvidence:evidence,processesBefore:[],processesAfter:[],released:[]};
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
 return {pid:Number(pid),start:fields[19],parentPid:Number(fields[1]),uid:(await stat(`/proc/${pid}`)).uid};
}
async function foreignDeniedAncestry(identity){
 const chain=[],seen=new Set();let current=identity;
 while(current&&chain.length<64){
  if(seen.has(current.pid)||current.pid===1||harnessAncestors.has(current.pid))return null;
  seen.add(current.pid);chain.push(current);
  if(observedOwnedProcesses.get(current.pid)?.start===current.start)return null;
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
const brokerOperations=[],correlations=new Map();
async function startScopedBroker(){
 const assignmentOperations=new Set(['observe_assignment','launch_environment','windows','activate','events','observe_idle','read_idle_retirement','register_native_handoff_observation']);
 const server=createServer(async(req,res)=>{
  const reply=(status,value)=>{res.writeHead(status,{'content-type':'application/json'});res.end(JSON.stringify(value));};
  const denied=async()=>{brokerOperations.push({operation,denied:true});await save('broker-operations.json',brokerOperations);reply(400,{schema_version:1,code:'fixture_target_denied',message:'Operation outside exact fixture assignment',retryable:false});};
  let operation='invalid';
  try{
   if(req.method!=='POST'||req.url!=='/v1/consumer')return denied();
   let raw='';for await(const chunk of req){raw+=chunk;if(Buffer.byteLength(raw)>65536)return denied();}
   const envelope=JSON.parse(raw),request=envelope.request;operation=request?.operation;
   if(envelope.application!=='codex-bots'||Object.keys(envelope).some(k=>!['application','request'].includes(k))||!request)return denied();
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
   let upstream;try{upstream=await fetch('http://127.0.0.1:19096/v1/consumer',{method:'POST',headers:{'content-type':'application/json'},body:raw,redirect:'error',signal:AbortSignal.timeout(120000)});}catch(error){ambiguous=true;throw error;}
   const value=await upstream.json();
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
async function census({captureDeniedBaseline=false}={}){
 const rows=[];
 for(const pid of await readdir('/proc')){
  if(!/^\d+$/.test(pid)||Number(pid)===process.pid)continue;
  let identity,owned=false,uid;
  try{const path=`/proc/${pid}`;uid=(await stat(path)).uid;
   identity=await processIdentity(pid);assert(/^\d+$/.test(identity.start));
   if(identity.uid!==process.getuid()){
    if(captureDeniedBaseline&&identity.pid!==1&&!harnessAncestors.has(identity.pid))baselineReadableProcesses.set(identity.pid,identity);
    continue;
   }
   owned=(await readFile(path+'/environ')).toString().split('\0').includes('AGENT_BROWSER_HOME='+agentHome);
   if(!owned){if(captureDeniedBaseline&&identity.pid!==1&&!harnessAncestors.has(identity.pid))baselineReadableProcesses.set(identity.pid,identity);continue;}
   observedOwnedProcesses.set(identity.pid,identity);
   rows.push({...identity,executable:await realpath(path+'/exe')});
  }catch(error){
   if(['ENOENT','ESRCH'].includes(error.code))continue;
   // A positively different UID is outside this fixture's process custody.
   // If its identity is unreadable, omit it as an ancestry anchor.
   if(uid!==undefined&&uid!==process.getuid()&&['EACCES','EPERM'].includes(error.code))continue;
   if(['EACCES','EPERM'].includes(error.code)&&identity&&!owned){
    if(captureDeniedBaseline){baselineDeniedProcesses.set(identity.pid,identity);continue;}
    const prior=baselineDeniedProcesses.get(identity.pid);
    if(prior&&prior.start===identity.start&&prior.parentPid===identity.parentPid&&prior.uid===identity.uid)continue;
    const chain=await foreignDeniedAncestry(identity).catch(()=>null);
    if(chain){cleanup.foreignDeniedAncestry??=[];cleanup.foreignDeniedAncestry.push(chain);continue;}
   }
   error.processIdentity=identity;throw error;
  }
 }
 return rows;
}

async function exactOwnedIdentity(row){
 const path=`/proc/${row.pid}`;
 try{
  if((await stat(path)).uid!==process.getuid())return null;
  const identity=await processIdentity(row.pid);
  if(identity.start!==row.start)return null;
  if(!(await readFile(path+'/environ')).toString().split('\0').includes('AGENT_BROWSER_HOME='+agentHome))return null;
  if(await realpath(path+'/exe')!==row.executable)return null;
  const confirmed=await processIdentity(row.pid);
  return confirmed.start===row.start?confirmed:null;
 }catch(error){if(['ENOENT','ESRCH'].includes(error.code))return null;throw error;}
}
async function settledCensus(){
 const deadline=Date.now()+10000;
 while(true){
  try{return await census();}
  catch(error){if(!['EACCES','EPERM','ENOENT','ESRCH'].includes(error.code)||Date.now()>=deadline)throw error;
   cleanup.censusSettling??=[];cleanup.censusSettling.push(safeError(error));
   await new Promise(resolve=>setTimeout(resolve,250));
  }
 }
}

async function send(method,params){
 let timer;
 try{return await Promise.race([client.send(method,params),new Promise((_,reject)=>{timer=setTimeout(()=>{ambiguous=true;client.rejectPending('Unknown MCP outcome; no replay');reject(Error('Unknown MCP outcome'));},120000);})]);}
 finally{clearTimeout(timer);}
}
async function workflow(name,args){
 let result;try{result=await send('tools/call',{name,arguments:args});}catch(error){ambiguous=true;throw error;}
 const value=result.structuredContent??JSON.parse(result.content.find(c=>c.type==='text').text);
 if(result.isError||value.success===false)throw Object.assign(Error('Agent Browser rejected workflow'),{code:value.error?.code||'workflow_rejected'});
 const data=value.data??value;if(name==='browser_open'){
  task=data;await save('browser-open-identity.json',Object.fromEntries(['taskId','browserId','profileId','tabId','host'].filter(k=>k in data).map(k=>[k,data[k]])));
 }return data;
}
async function readTask(id){const response=await runCli(context,['--json','mcp','host-task',id]);const value=JSON.parse(response.stdout);assert.equal(value.success,true);return value.data;}
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

try{
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
  try{result=await fetch(nativeOrigin+'/',{method:'GET',headers:cookies?{cookie:cookies}:{},redirect:'manual',signal:AbortSignal.timeout(10000)});}
  catch{throw Object.assign(Error('Native authentication preflight requires operator readback'),{code:'fixture_auth_preflight_required'});}
  await result.body?.cancel();
  let accepted=result.status>=200&&result.status<300;
  if(result.status>=300&&result.status<400){const location=result.headers.get('location');if(location){try{accepted=new URL(location,nativeOrigin).origin===nativeOrigin;}catch{accepted=false;}}}
  await save('auth-preflight.json',{method:'GET',status:result.status,accepted,matchedCookieCount:cookies?cookies.split('; ').length:0});
  if(!accepted)throw Object.assign(Error('Refresh private Native QA authentication before acquiring a fixture assignment'),{code:'fixture_auth_preflight_required'});
 }
 step='connect-native';remote=await connectRemoteView({expectedInstallationId:installationId,embedOrigin:appOrigin,publicOrigin:nativeOrigin,...(process.env.HOSTED_SMOKE_NATIVE_SOCKET?{socketPath:process.env.HOSTED_SMOKE_NATIVE_SOCKET}:{})});
 baseline=await remote.inventory();await save('baseline.json',baseline);
 broker=await startScopedBroker();env.AGENT_BROWSER_REMOTE_VIEW_ORIGIN=`http://127.0.0.1:${broker.address().port}`;
 await save('inputs.json',{binary,binarySha256:createHash('sha256').update(await readFile(binary)).digest('hex'),chrome,chromeSha256:createHash('sha256').update(await readFile(chrome)).digest('hex'),botsRoot,installationId,appOrigin,nativeOrigin,agentHome,socketDir});
 const desktops=new Proxy(remote,{get(target,key){if(key!=='acquire')return typeof target[key]==='function'?target[key].bind(target):target[key];return async args=>{let binding;try{binding=await target.acquire(args);}catch(error){ambiguous=true;throw error;}assert(!baseline.assignments.some(a=>a.assignmentId===binding.assignmentId),'Provider returned a baseline peer assignment');acquired.push(binding);await save('acquired.json',acquired);Object.assign(binding,await target.observe(binding));await save('acquired.json',acquired);return binding;};}});
 step='initialize-ab-mcp';client=createMcpStdioClient({context,args:['mcp','serve'],onFatal:()=>{ambiguous=true;}});
 await send('initialize',{protocolVersion:'2025-06-18',capabilities:{},clientInfo:{name:'hosted-native-smoke',version:'1'}});client.notify('notifications/initialized');
 const bot={id:'fixture-'+randomUUID(),name:'Synthetic hosted browser',threadId:'synthetic-'+randomUUID()};
 const native={status:async()=>({connected:true,activity:'ready',turnId:null,requests:[]}),historyPage:async()=>({messages:[],nextCursor:null})};
 step='start-bots-fixture';service=await createBotService({bots:[bot],native,stateDir:join(root,'bots-state'),desktops,publicOrigin:appOrigin,hostedAgentBrowser:{hostId:'bots',publicOrigin:appOrigin,workflow,readTask}});
 const html='<!doctype html><title>Hosted integration synthetic marker</title><body style="margin:0;background:#165dcc"><div style="width:90vw;height:85vh;background:#ff00aa;color:white;font:48px sans-serif">HOSTED SYNTHETIC MARKER</div></body>';
 // One explicit POST. Any uncertain response is retained and never replayed.
 step='hosted-launch';let response;try{response=await fetch(service.url+`/api/bots/${bot.id}/hosted-browser`,{method:'POST',headers:{origin:service.url,'content-type':'application/json'},body:JSON.stringify({sessionId:bot.threadId,requestId:randomUUID(),taskKey:'hosted-smoke-'+randomUUID(),taskName:'Render synthetic hosted browser in owning app',profileMode:'disposable',url:'data:text/html,'+encodeURIComponent(html)}),signal:AbortSignal.timeout(120000)});}catch(error){ambiguous=true;throw error;}
 const opened=await response.json();await save('launch.json',{status:response.status,opened});if(response.status!==201)throw Object.assign(Error(opened.message||'Hosted launch rejected'),{code:opened.error||'fixture_launch_rejected'});task=opened;
 assert.equal(acquired.length,1);assert.equal(opened.host.desktop.desktopId,acquired[0].desktopId);
 step='metadata-initial';const watchReader=startWatch(task.taskId);
 const snapshot=await readTask(task.taskId),event=await watchReader.next();assert.equal(event.event,'snapshot');assert.deepEqual(event.data,snapshot);assert.equal(snapshot.taskId,task.taskId);await save('metadata.json',{snapshot,event});
 const handoff=new URL(opened.handoffUrl);assert.equal(handoff.origin,appOrigin);assert.match(handoff.pathname,/^\/agent-browser\/handoff\/[a-f0-9-]{36}$/i);
 step='native-viewer';viewer=await chromium.launch({executablePath:chrome,headless:true,env:{...env,AGENT_BROWSER_HOME:agentHome}});
 const browserContext=await viewer.newContext({...(authState?{storageState:authState}:{}),viewport:{width:1440,height:1000}});
 const page=await browserContext.newPage();let frames=0;page.on('websocket',ws=>{if(new URL(ws.url()).origin===nativeOrigin.replace('https:','wss:'))ws.on('framereceived',()=>frames++);});
 // Serve only the reviewed app origin through this local fixture. Native viewer
 // requests retain their real origin/authentication; no fake viewer is mounted.
 await page.route(appOrigin+'/**',async route=>{const request=route.request(),url=new URL(request.url());const local=service.url+url.pathname+url.search;const headers={...request.headers(),host:new URL(service.url).host};if(headers.origin===appOrigin)headers.origin=service.url;const result=await route.fetch({url:local,headers});await route.fulfill({response:result});});
 await page.goto(handoff.href,{waitUntil:'domcontentloaded',timeout:30000});
 let rendered=false,canvas=null;const deadline=Date.now()+45000;
 while(Date.now()<deadline&&!rendered){
  for(const frame of page.frames().filter(f=>f.url().startsWith(nativeOrigin+'/'))){
   canvas=await frame.evaluate(()=>{for(const c of document.querySelectorAll('canvas')){if(c.width<100||c.height<100)continue;try{const ctx=c.getContext('2d');if(!ctx)continue;let markers=0;for(let x=0.1;x<0.95;x+=0.1)for(let y=0.1;y<0.9;y+=0.1){const p=ctx.getImageData(Math.floor(c.width*x),Math.floor(c.height*y),1,1).data;if(p[0]>200&&p[1]<90&&p[2]>100)markers++;}if(markers>=8)return {width:c.width,height:c.height,markers};}catch{}}return null;});
   if(canvas&&frames>0){rendered=true;break;}
  }
  if(!rendered)await new Promise(resolve=>setTimeout(resolve,250));
 }
 await page.screenshot({path:join(evidence,'hosted-native.png')});await import('node:fs/promises').then(fs=>fs.chmod(join(evidence,'hosted-native.png'),0o600));
 await save('render.json',{rendered,canvas,receivedWebSocketFrames:frames});assert(rendered,'Actual Native canvas must render the synthetic marker with WebSocket frames');
 // Produce a genuine metadata transition after the visual proof. Closing the
 // exact fixture task is dispatched once, including when its outcome is unknown.
 step='close-exact-task';taskCloseDispatched=true;await workflow('browser_close',{taskId:task.taskId});taskClosed=true;
 step='metadata-changed';const changed=await watchReader.next();
 assert.equal(changed.event,'changed');assert.equal(changed.data.taskId,task.taskId);assert.equal(changed.data.taskState,'ended');
 assert.equal(changed.cursor.watchId,event.cursor.watchId);assert.equal(changed.cursor.sequence,2);
 const ended=await readTask(task.taskId);assert.equal(ended.taskState,'ended');assert.deepEqual(changed.data,ended);
 await save('metadata.json',{snapshot,event,changed,ended});
 assert(!brokerOperations.some(entry=>['acquire','release','release_idle'].includes(entry.operation)), 'Hosted runtime must never request app pool growth or retirement');
 passed=true;
}catch(error){await save('failure.json',{...safeError(error),ambiguous});process.exitCode=1;}
finally{
 async function clean(name,action){try{await action();}catch(error){cleanup.errors.push({...safeError(error),step:name});process.exitCode=1;}}
 await clean('close-viewer',async()=>{await viewer?.close();});
 await clean('stop-watch',async()=>{if(watcher&&watcher.exitCode===null&&watcher.signalCode===null){const exit=once(watcher,'exit');watcher.kill('SIGTERM');await exit;}});
 if(task?.taskId&&!taskCloseDispatched)await clean('close-task',async()=>{taskCloseDispatched=true;await workflow('browser_close',{taskId:task.taskId});taskClosed=true;});
 await clean('close-service',async()=>{await service?.close();});
 await clean('close-mcp',async()=>{await client?.close();});
 await clean('owned-process-cleanup',async()=>{cleanup.processesBefore=await settledCensus();for(const row of cleanup.processesBefore){const current=await exactOwnedIdentity(row);if(current)process.kill(row.pid,'SIGTERM');}await new Promise(resolve=>setTimeout(resolve,1000));cleanup.processesAfter=await settledCensus();assert.equal(cleanup.processesAfter.length,0,'Owned fixture processes remain');cleanup.processesClear=true;});
 if(remote&&baseline)await clean('assignment-cleanup',async()=>{
  const afterStop=await remote.inventory();
  for(const binding of acquired){assert(!baseline.assignments.some(a=>a.assignmentId===binding.assignmentId));assert.equal(cleanup.processesClear,true,'Fresh process cleanup proof is required before release');if(afterStop.assignments.some(a=>a.assignmentId===binding.assignmentId&&a.state==='active')){const current=await remote.observe(binding);assert.equal(current.generation,binding.generation);await remote.release(current);cleanup.released.push({assignmentId:binding.assignmentId,generation:binding.generation});}}
  const final=await remote.inventory();await save('final-inventory.json',final);
  const ids=new Set(baseline.assignments.map(a=>a.assignmentId));assert.deepEqual(peerIdentity(final).filter(a=>ids.has(a.assignmentId)),peerIdentity(baseline),'Baseline peers changed');
  cleanup.unresolvedNewAssignments=final.assignments.filter(a=>!ids.has(a.assignmentId)&&a.state==='active').map(a=>({assignmentId:a.assignmentId,generation:a.generation}));assert.equal(cleanup.unresolvedNewAssignments.length,0,'New assignments require explicit reconciliation');
 });
 await clean('close-scoped-broker',async()=>{if(broker){broker.closeAllConnections();await new Promise(resolve=>broker.close(resolve));}});
 remote?.close();cleanup.ambiguous=ambiguous;cleanup.taskClosed=taskClosed;cleanup.taskCloseDispatched=taskCloseDispatched;cleanup.passed=passed;await save('cleanup.json',cleanup);
 if(ambiguous||cleanup.errors.length||!passed)process.exitCode=1;
 console.log(JSON.stringify({evidence,passed,cleanupQualified:!process.exitCode,ambiguous}));
}
