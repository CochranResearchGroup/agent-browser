#!/usr/bin/env node
/** Private JSON bridge owned by smoke-hosted-browser-restart-native.py. */
import assert from 'node:assert/strict';
import {readFile,writeFile,mkdir,readdir,stat,realpath} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {join} from 'node:path';
import {createInterface} from 'node:readline';
import {randomUUID} from 'node:crypto';
import {createMcpStdioClient,runCli} from './smoke-utils.js';
import {qualifyHostedBrowserRestart} from './smoke-hosted-browser-restart.mjs';
import {qualifyHostedBrowserReconnect} from './smoke-hosted-browser-reconnect.mjs';
import {cleanupExactOwnedProcesses,readFreshCleanupIdentity} from './hosted-fixture-cleanup.mjs';
const cfg=JSON.parse(await readFile(process.argv[2],'utf8'));
const lines=createInterface({input:process.stdin})[Symbol.asyncIterator]();
const bridge=async(operation,data={})=>{process.stdout.write(JSON.stringify({operation,...data})+'\n');const line=await lines.next();assert(!line.done);const reply=JSON.parse(line.value);assert.equal(reply.ok,true);return reply.value;};
const save=(name,value)=>writeFile(join(cfg.results,name),JSON.stringify(value,null,2)+'\n',{mode:0o600});
const root=join(cfg.results,'agent'),agentHome=join(root,'.agent-browser'),socketDir=join(root,'s');
for(const p of [root,agentHome,socketDir,join(root,'runtime')])await mkdir(p,{recursive:true,mode:0o700});
const env=Object.fromEntries(Object.entries(process.env).filter(([key])=>!key.startsWith('AGENT_BROWSER_')&&!key.startsWith('XDG_')));
Object.assign(env,{HOME:root,AGENT_BROWSER_HOME:agentHome,AGENT_BROWSER_SOCKET_DIR:socketDir,AGENT_BROWSER_RUNTIME_HOST:'1',AGENT_BROWSER_EXECUTABLE_PATH:cfg.chrome,AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD:cfg.binary,AGENT_BROWSER_REMOTE_VIEW_APPLICATION:'fixture',AGENT_BROWSER_REMOTE_VIEW_PUBLIC_ORIGIN:cfg.publicOrigin,AGENT_BROWSER_REMOTE_VIEW_ORIGIN:cfg.controlOrigin,AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE:cfg.tokenFile,AGENT_BROWSER_REMOTE_VIEW_POOL:'main',AGENT_BROWSER_REMOTE_VIEW_DESKTOP_COUNT:'2',AGENT_BROWSER_HOSTS_FILE:join(root,'hosts.json'),XDG_RUNTIME_DIR:join(root,'runtime'),XDG_CONFIG_HOME:join(root,'config'),XDG_CACHE_HOME:join(root,'cache'),XDG_DATA_HOME:join(root,'data')});
await writeFile(env.AGENT_BROWSER_HOSTS_FILE,JSON.stringify({version:1,hosts:{fixture:{origin:'https://synthetic.example.invalid',providerApplication:'fixture'}}}),{mode:0o600});
const context={env,agentHome,socketDir,tempHome:root,session:'restart-'+randomUUID()};
const known=new Map();let client,task,closeDispatched=false,stage='initialize',failure,cleanupFailure;
async function kernelIdentity(pid){
 const raw=await readFile('/proc/'+pid+'/stat','utf8'),fields=raw.slice(raw.lastIndexOf(')')+2).trim().split(/\s+/);
 return {pid,start:fields[19],uid:(await stat('/proc/'+pid)).uid};
}
const readIdentity=pid=>readFreshCleanupIdentity(pid,{identity:kernelIdentity,executable:pid=>realpath('/proc/'+pid+'/exe')});
async function census(){
 for(const name of await readdir('/proc')){
  if(!/^\d+$/.test(name))continue;const pid=Number(name);if(pid===process.pid)continue;
  try{
   if((await stat('/proc/'+pid)).uid!==process.getuid())continue;
   const environment=await readFile('/proc/'+pid+'/environ');
   if(!environment.includes(Buffer.from('AGENT_BROWSER_HOME='+agentHome+'\0')))continue;
   const identity=await readIdentity(pid);if(identity)known.set(pid,identity);
  }catch(error){if(!['ENOENT','ESRCH','EACCES','EPERM'].includes(error.code))throw error;}
 }
 return [...known.values()];
}
const send=async(method,params)=>{
 let timer;try{return await Promise.race([client.send(method,params),new Promise((_,reject)=>{timer=setTimeout(()=>{client.rejectPending('Unknown outcome; no replay');reject(Error('MCP deadline; no replay'));},120000);})]);}finally{clearTimeout(timer);}
};
const workflow=async(name,args)=>{
 const result=await send('tools/call',{name,arguments:args});const value=result.structuredContent??JSON.parse(result.content.find(c=>c.type==='text').text);
 if(result.isError||value.success===false)throw Object.assign(Error(value.error?.message??'Workflow refused'),{code:value.error?.code});
 return value.data??value;
};
const boot=(await readFile('/proc/sys/kernel/random/boot_id','utf8')).trim();
const record=()=>JSON.parse(execFileSync('/usr/bin/python3',['-c',"import json,sqlite3,sys;from pathlib import Path;c=sqlite3.connect(Path(sys.argv[1]).as_uri()+'?mode=ro',uri=True);s=json.loads(c.execute('select json from state_documents where kind=?',('browser_session_state',)).fetchone()[0]);t=s['sessions'][sys.argv[2]];assert t['browserId']==sys.argv[3];print(json.dumps(s['browsers'][sys.argv[3]]))",join(agentHome,'service','runtime.sqlite3'),task.taskId,task.browserId],{encoding:'utf8',timeout:10000}));
let profile;
const observeProcess=async expected=>{
 if(expected){try{await stat('/proc/'+expected.pid);return {...expected,pidAbsent:false};}catch(error){if(!['ENOENT','ESRCH'].includes(error.code))throw error;return {...expected,pidAbsent:true,alive:false};}}
 const browser=record(),proof=await readIdentity(browser.pid);assert(proof);assert.equal(proof.uid,process.getuid());assert.equal(proof.executable,await realpath(cfg.chrome));
 assert((await readFile('/proc/'+browser.pid+'/cmdline','utf8')).includes('--user-data-dir='+profile));
 known.set(proof.pid,proof);await census();
 return {pid:proof.pid,startToken:`linux:${boot}:${proof.start}`,kernelStart:proof.start,uid:proof.uid,executablePath:proof.executable,profilePath:profile,alive:true};
};
try{
 client=createMcpStdioClient({context,args:['mcp','serve'],onFatal:()=>{}});
 await send('initialize',{protocolVersion:'2025-06-18',capabilities:{},clientInfo:{name:'native-restart355',version:'1'}});client.notify('notifications/initialized');
 stage='create-profile';const name='restart-'+randomUUID();const created=JSON.parse((await runCli(context,['--json','runtime','create',name,'--browser-family','chromium'])).stdout);assert.equal(created.created,true);profile=await realpath(created.userDataDir);assert(profile.startsWith(root+'/'));
 stage='open';task=await workflow('browser_open',{taskKey:'restart-'+randomUUID(),serviceName:'synthetic-restart',agentName:'fixture',taskName:'Continue after Native restart without action replay',profile:name,profileMode:'persistent',url:'data:text/html,<title>Restart synthetic marker</title><input id="draft" value="unfinished">',hostBinding:{version:1,hostId:'fixture',participantId:randomUUID(),sessionId:randomUUID(),placement:{mode:'required',desktop:{desktopId:cfg.target.desktopId,generation:cfg.target.viewingGeneration}}}});
 await save('task-before.json',task);
 stage='warm';await qualifyHostedBrowserReconnect({workflow,task,save,syntheticOnly:true,controlPairs:10,requirePerformanceTargets:false});
 stage='restart';await qualifyHostedBrowserRestart({workflow,task,observeProcess,save,syntheticOnly:true,restartNative:proof=>bridge('restart',{proof}),observePeers:()=>bridge('peers')});
 stage='close';closeDispatched=true;await workflow('browser_close',{taskId:task.taskId});task=null;
}catch(error){failure={stage,code:error.code??error.name,message:String(error.message).slice(0,600)};await save('failure.json',failure);}
finally{
 try{
  if(task&&!closeDispatched){closeDispatched=true;await workflow('browser_close',{taskId:task.taskId});}
  await census();await client?.close();
  const receipt={};const result=await cleanupExactOwnedProcesses({known,census,readIdentity,signal:(pid,signal)=>process.kill(pid,signal),sleep:ms=>new Promise(resolve=>setTimeout(resolve,ms)),allowSigkill:true,graceMs:3000,killMs:3000,pollMs:100,receipt,record:()=>save('agent-cleanup.json',receipt)});
  assert.equal(result.qualified,true);await save('agent-cleanup.json',{...receipt,qualified:result.qualified,owned:result.owned,remaining:result.remaining});
 }catch(error){cleanupFailure={code:error.code??error.name,message:String(error.message).slice(0,300)};}
 await save('agent-receipt.json',{verdict:!failure&&!cleanupFailure?'ACCEPT':'INCOMPLETE',failure,cleanupFailure});
 process.stdout.write(JSON.stringify({operation:'complete',passed:!failure&&!cleanupFailure})+'\n');
 process.exitCode=failure||cleanupFailure?1:0;process.stdin.destroy();
}
