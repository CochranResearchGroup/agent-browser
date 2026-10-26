#!/usr/bin/env node
/** Real public MCP task recovery on a disposable synthetic HTTPS application. */
import assert from 'node:assert/strict';
import { createHash, randomUUID } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { createServer as httpsServer, get } from 'node:https';
import { get as getHttp } from 'node:http';
import { createServer as netServer, connect } from 'node:net';
import { readFileSync, writeFileSync, mkdirSync, existsSync, readdirSync, renameSync, unlinkSync, statSync, chmodSync, createReadStream } from 'node:fs';
import { join, isAbsolute } from 'node:path';
import { createSmokeContext, createMcpStdioClient, runCli } from './smoke-utils.js';

const candidate=process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD;
const browser=process.env.AGENT_BROWSER_SMOKE_BROWSER_EXECUTABLE;
assert(candidate && isAbsolute(candidate),'Pin the source candidate executable');
assert(browser && isAbsolute(browser),'Pin the reviewed Linux Chrome executable');
async function sha256(path) {const hash=createHash('sha256');for await(const chunk of createReadStream(path)) hash.update(chunk);return hash.digest('hex');}
const candidateSha256=await sha256(candidate);
const browserSha256=await sha256(browser);
assert.equal(browserSha256,process.env.AGENT_BROWSER_SMOKE_BROWSER_SHA256,'Reviewed browser digest must match');
const context=createSmokeContext({prefix:'ab-recovery468-',sessionPrefix:'recovery'});
const env=Object.fromEntries(Object.entries(context.env).filter(([key])=>
 !key.startsWith('AGENT_BROWSER_') && !key.startsWith('XDG_') && !/^(HTTP|HTTPS|ALL|NO)_PROXY$/i.test(key)));
const config=join(context.tempHome,'fixture-config.json'), envFile=join(context.tempHome,'fixture.env');
writeFileSync(config,'{}');writeFileSync(envFile,'');
Object.assign(env,{
 AGENT_BROWSER_HOME:context.agentHome,AGENT_BROWSER_SOCKET_DIR:context.socketDir,
 AGENT_BROWSER_RUNTIME_HOST:'1',AGENT_BROWSER_EXECUTABLE_PATH:browser,
 AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD:candidate,AGENT_BROWSER_CONFIG:config,
 AGENT_BROWSER_ENV_FILE:envFile,AGENT_BROWSER_IGNORE_HTTPS_ERRORS:'1',
 AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS:'0',
 XDG_RUNTIME_DIR:join(context.tempHome,'runtime'),XDG_CONFIG_HOME:join(context.tempHome,'config'),
 XDG_DATA_HOME:join(context.tempHome,'data'),
});
mkdirSync(env.XDG_RUNTIME_DIR,{mode:0o700});context.env=env;
const certPath=join(context.tempHome,'cert.pem'),keyPath=join(context.tempHome,'key.pem');
execFileSync('openssl',['req','-x509','-newkey','rsa:2048','-nodes','-keyout',keyPath,'-out',certPath,
 '-days','1','-subj','/CN=localhost','-addext','subjectAltName=IP:127.0.0.1,DNS:localhost'],{stdio:'ignore'});
chmodSync(keyPath,0o600);
const cert=readFileSync(certPath), counts={}, requestLog=[];
const application=httpsServer({key:readFileSync(keyPath),cert},(req,res)=>{
 const url=new URL(req.url,'https://localhost');requestLog.push({path:url.pathname,actor:url.searchParams.get('actor')});
 if(url.pathname==='/event') {
  const key=`${url.searchParams.get('actor')}:${url.searchParams.get('kind')}`;
  counts[key]=(counts[key]??0)+1;res.end('ok');return;
 }
 if(url.pathname==='/counts') {res.setHeader('content-type','application/json');res.end(JSON.stringify(counts));return;}
 res.setHeader('content-type','text/html');
 res.end(`<!doctype html><title>Owned recovery fixture</title><h1>Report task</h1>
 <label>Draft <input id="draft" aria-label="Draft"></label><button id="action">Mark</button>
 <button id="submit">Submit</button><p id="result"></p><script>
 const actor=new URL(location.href).searchParams.get('actor');
 const event=kind=>fetch('/event?actor='+encodeURIComponent(actor)+'&kind='+kind);
 document.querySelector('#draft').addEventListener('input',event_=>{if(event_.target.value!=='')event('fill');});
 document.querySelector('#action').onclick=()=>{event('action');document.querySelector('#result').textContent='Marked';};
 document.querySelector('#submit').onclick=()=>{event('submit');document.querySelector('#result').textContent='Submitted';};
 </script>`);
});
await new Promise(resolve=>application.listen(0,'127.0.0.1',resolve));
const origin=`https://127.0.0.1:${application.address().port}`;
const readCounts=()=>new Promise((resolve,reject)=>{
 get(origin+'/counts',{ca:cert},res=>{let body='';res.on('data',chunk=>body+=chunk);res.on('end',()=>resolve(JSON.parse(body)));}).on('error',reject);
});
const pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
function ownedProcesses() {
 return readdirSync('/proc').filter(value=>/^\d+$/.test(value) && Number(value)!==process.pid).filter(pid=>{
  try{return readFileSync(`/proc/${pid}/environ`).toString().split('\0').includes(`AGENT_BROWSER_HOME=${context.agentHome}`);}catch{return false;}
 }).map(Number);
}
const receipt={profileMode:'existing_owned_fixture_browser',tls:'self_signed_exception_confined_to_fixture_process',schemaVersion:'agent-browser.task-recovery-acceptance.v1',synthetic:true,candidateSha256,browserSha256,checks:[],beforeOwnedProcesses:ownedProcesses()};
let client,proxy;
const tasks=[];
const browserRoots=[], profileDirectories=[];
async function prepareExistingProfile(name) {
 const created=JSON.parse((await runCli(context,['--json','runtime','create',name])).stdout);
 const profile=created.userDataDir ?? created.data?.userDataDir;
 assert(typeof profile==='string' && profile.startsWith(context.tempHome+'/'),'Runtime must return a profile inside this exact disposable home');
 mkdirSync(profile,{recursive:true});profileDirectories.push(profile);
 const child=spawn(browser,['--headless=new','--no-sandbox','--disable-dev-shm-usage',
  '--disable-background-networking','--password-store=basic','--no-first-run','--no-default-browser-check',
  '--remote-debugging-port=0',`--user-data-dir=${profile}`,'--ignore-certificate-errors','about:blank'],
  {env,stdio:'ignore'});
 browserRoots.push(child);
 const deadline=Date.now()+10000;
 while(!existsSync(join(profile,'DevToolsActivePort')) && child.exitCode===null && Date.now()<deadline)await pause(25);
 assert(existsSync(join(profile,'DevToolsActivePort')),'Exact fixture browser must be ready before existing-profile acquisition');
}
async function newClient() {
 const next=createMcpStdioClient({context,args:['mcp','serve'],onFatal:()=>{}});
 await next.send('initialize',{protocolVersion:'2025-06-18',capabilities:{},clientInfo:{name:'task-recovery-fixture',version:'1'}});
 next.notify('notifications/initialized');return next;
}
async function raw(name,args) {
 let timer;
 try {return await Promise.race([client.send('tools/call',{name,arguments:args}),new Promise((_,reject)=>{timer=setTimeout(()=>reject(new Error('bounded fixture MCP deadline')),70000);})]);}
 finally {clearTimeout(timer);}
}
async function call(name,args) {
 const result=await raw(name,args);
 const payload=JSON.parse(result.content.find(block=>block.type==='text').text);
 assert.equal(payload.success,true,JSON.stringify(payload));return payload.data;
}
function exactOwnedPid(pid) {assert(ownedProcesses().includes(pid),`PID${pid} must belong to this exact fixture home`);return pid;}
function runtimePid() {return Number(readFileSync(join(context.socketDir,'runtime-host.pid'),'utf8').trim());}
async function installReplyLossProxy() {
 const canonical=join(context.socketDir,'runtime-host.sock'),original=canonical+'.origin';
 assert(existsSync(canonical),'Owned runtime socket must exist');
 const originalInode=statSync(canonical).ino;
 renameSync(canonical,original);
 const sockets=new Set();let armed=null,requests=0,drops=0,lastDroppedSuccess=null;
 const server=netServer(socket=>{
  sockets.add(socket);socket.on('close',()=>sockets.delete(socket));socket.on('error',()=>{});
  const upstream=connect(original);sockets.add(upstream);
  upstream.on('close',()=>sockets.delete(upstream));
  upstream.on('error',()=>socket.destroy());
  let request='',response='',parsed=false,drop=false;
  socket.on('data',chunk=>{
   if(!parsed) {
    request+=chunk.toString();
    if(request.length>1048576){socket.destroy();upstream.destroy();return;}
    if(request.includes('\n')) {
     parsed=true;const command=JSON.parse(request.slice(0,request.indexOf('\n')));
     requests++;
     if(armed===command.action){drop=true;armed=null;}
    }
   }
   upstream.write(chunk);
  });
  upstream.on('data',chunk=>{
   if(drop) {
    response+=chunk.toString();
    if(response.length>8388608){socket.destroy();upstream.destroy();return;}
    if(response.includes('\n')) {lastDroppedSuccess=JSON.parse(response.slice(0,response.indexOf('\n'))).success===true;drops++;socket.destroy();upstream.destroy();}
   } else socket.write(chunk);
  });
  upstream.on('end',()=>socket.end());socket.on('end',()=>upstream.end());
 });
 await new Promise(resolve=>server.listen(canonical,resolve));
 const proxyInode=statSync(canonical).ino;
 return {
  arm(action){assert.equal(armed,null);armed=action;},
  get requests(){return requests;},get drops(){return drops;},get lastDroppedSuccess(){return lastDroppedSuccess;},
  async restore() {
   for(const socket of sockets)socket.destroy();
   await new Promise(resolve=>server.close(resolve));
   if(existsSync(canonical)){assert.equal(statSync(canonical).ino,proxyInode);unlinkSync(canonical);}
   assert.equal(statSync(original).ino,originalInode);renameSync(original,canonical);
  },
 };
}
async function expectOneLostEffect(task,{action,name,args,kind,requestId}) {
 proxy.arm(action);
 let response,failed=false;
 try {response=await raw(name,{taskId:task.taskId,requestId,...args});failed=response.isError===true;}
 catch {failed=true;}
 const deadline=Date.now()+3000;
 let observed;
 do {observed=await readCounts();if((observed[`alice:${kind}`]??0)>=1)break;await pause(25);}while(Date.now()<deadline);
 assert.equal(observed[`alice:${kind}`],1,`Lost ${kind} reply must not repeat the application's effect`);
 assert.equal(failed,true,'Lost reply must remain an uncertain original operation');
 assert.equal(proxy.lastDroppedSuccess,true,'Drop must occur after a successful owner response, not before physical effect');
 const record=await call('browser_command',{taskId:task.taskId,action:'task_operation',params:{requestId}});
 assert.equal(record.operation.requestId,requestId);assert.equal(record.operation.state,'uncertain');
 assert.equal(record.operation.retryAllowed,false);
 const sent=proxy.requests;
 if(kind==='fill') {
  let conflict=false;
  try {const result=await raw(name,{taskId:task.taskId,requestId,...args,value:'changed request must not execute'});conflict=result.isError===true;}catch(error){conflict=String(error).includes('request_conflict');}
  assert(conflict,'Changed arguments with original request ID must conflict');
  assert.equal(proxy.requests,sent,'Conflicting request must not start transport');
 }
 try {await raw(name,{taskId:task.taskId,requestId,...args});}catch{}
 assert.equal(proxy.requests,sent,'Original request replay must return before transport');
 assert.equal((await readCounts())[`alice:${kind}`],1);
 receipt.checks.push({kind,requestId,count:1,originalState:record.operation.state});
}
try {
 receipt.phase='prepare_existing_profile';await prepareExistingProfile('recovery-fixture');
 receipt.httpsReady=typeof await readCounts()==='object';
 client=await newClient();
 const common={profile:'recovery-fixture',profileMode:'existing',serviceName:'RecoveryFixture',agentName:'synthetic-agent',taskName:'Preserve original report task'};
 receipt.phase='open_alice';console.log('Recovery fixture: open Alice');
 const alice=await call('browser_open',{...common,taskKey:'alice',url:origin+'/?actor=alice'});tasks.push(alice);
 receipt.phase='open_bob';console.log('Recovery fixture: open Bob');
 const bob=await call('browser_open',{...common,taskKey:'bob',url:origin+'/?actor=bob'});tasks.push(bob);
 assert.equal(bob.browserId,alice.browserId);assert.notEqual(bob.tabId,alice.tabId);
 receipt.phase='peer_draft';
 await call('browser_fill',{taskId:bob.taskId,selector:'#draft',value:'peer unsaved draft',requestId:'peer-draft'});
 receipt.phase='screenshot';
 const screenshot=join(context.tempHome,'report.png');
 await call('browser_screenshot',{taskId:alice.taskId,path:screenshot,requestId:'report-capture'});
 assert(readFileSync(screenshot).subarray(0,8).equals(Buffer.from([137,80,78,71,13,10,26,10])));
 const screenshotSha256=await sha256(screenshot);
 receipt.phase='inject_reply_loss';console.log('Recovery fixture: inject owned reply loss');
 proxy=await installReplyLossProxy();
 const lostCapturePath=join(context.tempHome,'lost-capture.png');
 proxy.arm('screenshot');
 let captureUncertain=false;
 try {captureUncertain=(await raw('browser_screenshot',{taskId:alice.taskId,path:lostCapturePath,requestId:'lost-capture'})).isError===true;}catch{captureUncertain=true;}
 assert(captureUncertain,'Lost capture reply must remain uncertain');
 assert.equal(proxy.lastDroppedSuccess,true);
 assert(readFileSync(lostCapturePath).subarray(0,8).equals(Buffer.from([137,80,78,71,13,10,26,10])));
 const lostCaptureSha256=await sha256(lostCapturePath), lostCaptureMtime=statSync(lostCapturePath).mtimeMs;
 const captureRecord=await call('browser_command',{taskId:alice.taskId,action:'task_operation',params:{requestId:'lost-capture'}});
 assert.equal(captureRecord.operation.state,'uncertain');
 assert.equal(captureRecord.operation.evidenceReferences.artifactId,undefined,'Lost acknowledgement does not fabricate successful artifact metadata');
 const captureRequests=proxy.requests;
 try {await raw('browser_screenshot',{taskId:alice.taskId,path:lostCapturePath,requestId:'lost-capture'});}catch{}
 assert.equal(proxy.requests,captureRequests,'Lost screenshot reply must never recapture');
 assert.equal(await sha256(lostCapturePath),lostCaptureSha256);
 assert.equal(statSync(lostCapturePath).mtimeMs,lostCaptureMtime);
 receipt.checks.push({kind:'screenshot',originalState:'uncertain',fileProduced:true,recapture:false});
 await expectOneLostEffect(alice,{action:'fill',name:'browser_fill',args:{selector:'#draft',value:'retained alice draft'},kind:'fill',requestId:'lost-fill'});
 await expectOneLostEffect(alice,{action:'click',name:'browser_click',args:{selector:'#action'},kind:'action',requestId:'lost-action'});
 await expectOneLostEffect(alice,{action:'click',name:'browser_click',args:{selector:'#submit'},kind:'submit',requestId:'lost-submit'});
 const beforeRestart=await call('browser_command',{taskId:alice.taskId,action:'task_operation',params:{requestId:'report-capture'}});
 assert.equal(beforeRestart.operation.evidenceReferences.artifactId,'report-capture');
 assert.equal(proxy.drops,4);receipt.droppedSuccessfulReplies=proxy.drops;
 await proxy.restore();proxy=null;
 receipt.phase='owner_restart';
 client.close();client=null;
 const stopped=exactOwnedPid(runtimePid());process.kill(stopped,'SIGKILL');
 for(let i=0;i<100 && ownedProcesses().includes(stopped);i++)await pause(25);
 assert(!ownedProcesses().includes(stopped),'Exact old host must be absent');
 client=await newClient();
 const original=await call('browser_command',{taskId:alice.taskId,action:'task_operation',params:{requestId:'lost-submit'}});
 assert.equal(original.operation.state,'uncertain');
 assert(!ownedProcesses().includes(stopped));
 const restartedReplay=await raw('browser_click',{taskId:alice.taskId,selector:'#submit',requestId:'lost-submit'});
 assert.equal(restartedReplay.isError,true,'Original uncertain request must remain uncertain after restart');
 assert.equal(runtimePid(),stopped,'Original effect readback must not create a replacement runtime host');
 assert.equal((await readCounts())['alice:submit'],1);
 const continued=await call('browser_continue',{taskId:alice.taskId});
 assert.equal(continued.taskId,alice.taskId);assert.equal(continued.browserId,alice.browserId);assert.equal(continued.tabId,alice.tabId);
 assert.notEqual(runtimePid(),stopped);
 const aliceSnapshot=await call('browser_snapshot',{taskId:alice.taskId});
 const bobSnapshot=await call('browser_snapshot',{taskId:bob.taskId});
 assert(JSON.stringify(aliceSnapshot).includes('retained alice draft'));
 assert(JSON.stringify(bobSnapshot).includes('peer unsaved draft'));
 assert.equal(await sha256(screenshot),screenshotSha256);
 const briefing=await call('browser_command',{taskId:alice.taskId,action:'task_briefing'});
 assert(briefing.artifacts.references.some(ref=>ref.source==='task_operation' && ref.operationId==='report-capture'));
 assert(briefing.uncertainOperations.some(ref=>ref.operationId==='lost-submit'));
 assert(briefing.uncertainOperations.some(ref=>ref.operationId==='lost-capture'));
 receipt.checks.push({kind:'owner_restart',sameTask:true,sameTarget:true,peerDraftPreserved:true,artifactPreserved:true});
 receipt.phase='physical_loss';
 await prepareExistingProfile('lost-process-fixture');
 const lost=await call('browser_open',{...common,profile:'lost-process-fixture',taskKey:'lost',url:origin+'/?actor=lost'});tasks.push(lost);
 await call('browser_fill',{taskId:lost.taskId,selector:'#draft',value:'transient scratch',requestId:'lost-process-draft'});
 const lostOperation=await call('browser_command',{taskId:lost.taskId,action:'task_operation',params:{requestId:'lost-process-draft'}});
 const lostPid=exactOwnedPid(lostOperation.operation.processPid);process.kill(lostPid,'SIGTERM');
 for(let i=0;i<100 && ownedProcesses().includes(lostPid);i++)await pause(25);
 assert(!ownedProcesses().includes(lostPid));
 const afterLoss=await call('browser_continue',{taskId:lost.taskId});
 assert.equal(afterLoss.status,'intervention_required');assert.equal(afterLoss.transientPageState,'unavailable_not_reconstructed');
 assert(!ownedProcesses().includes(lostPid));
 assert(JSON.stringify(await call('browser_snapshot',{taskId:bob.taskId})).includes('peer unsaved draft'));
 assert.equal((await readCounts())['alice:submit'],1);
 receipt.checks.push({kind:'physical_loss',intervention:true,navigationReplay:false,peerDraftPreserved:true});
 receipt.phase='complete';receipt.status='pass';
} catch(error) {
 receipt.status='failed';receipt.error=error.message;
 receipt.applicationCounts={...counts};receipt.httpRequests=requestLog.slice(-20);
 receipt.targets=[];
 for(const profile of profileDirectories) {
  try {
   const port=Number(readFileSync(join(profile,'DevToolsActivePort'),'utf8').split('\n')[0]);
   const targets=await new Promise((resolve,reject)=>{const req=getHttp(`http://127.0.0.1:${port}/json/list`,res=>{let body='';res.on('data',chunk=>body+=chunk);res.on('end',()=>resolve(JSON.parse(body)));});req.setTimeout(1000,()=>req.destroy(new Error('diagnostic deadline')));req.on('error',reject);});
   receipt.targets.push(...targets.map(target=>({id:target.id,type:target.type,url:target.url.startsWith(origin)||target.url.startsWith('about:blank')?target.url:'outside_fixture_omitted'})));
  }catch{}
 }
 throw error;
}
finally {
 if(proxy){await proxy.restore();proxy=null;}
 if(client) {
  for(const task of tasks.reverse()) {
   try {await Promise.race([call('browser_close',{taskId:task.taskId}),pause(3000).then(()=>{throw new Error('cleanup deadline');})]);}catch{}
  }
  client.close();
 }
 for(const child of browserRoots) {if(child.exitCode===null)child.kill('SIGTERM');}
 for(const pid of ownedProcesses()) {try {if(ownedProcesses().includes(pid))process.kill(pid,'SIGTERM');}catch{}}
 for(let i=0;i<100 && ownedProcesses().length;i++)await pause(50);
 receipt.afterOwnedProcesses=ownedProcesses();
 await new Promise(resolve=>application.close(resolve));
 assert.equal(await sha256(candidate),candidateSha256,'Candidate bytes must remain frozen during acceptance');
 const receiptDirectory='/tmp/agent-browser-spec466-implementation';mkdirSync(receiptDirectory,{recursive:true,mode:0o700});
 const receiptPath=join(receiptDirectory,`recovery468-${randomUUID()}.json`);
 writeFileSync(receiptPath,JSON.stringify(receipt,null,2)+'\n',{mode:0o600,flag:'wx'});
 console.log(JSON.stringify({status:receipt.status,receiptPath,ownedResidue:receipt.afterOwnedProcesses.length,checks:receipt.checks}));
 assert.equal(receipt.afterOwnedProcesses.length,0,`Retained cleanup obligation at ${context.tempHome}`);
 context.cleanupTempHome();
}
