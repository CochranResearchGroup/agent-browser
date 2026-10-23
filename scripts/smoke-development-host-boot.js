#!/usr/bin/env node
// Exercise generated ingress during the supervisor's pre-publication boot window.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
import {mkdtemp, mkdir, readFile, readdir, realpath, stat, writeFile, rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join, resolve} from 'node:path';
import {createServer} from 'node:net';
import {developmentRuntimeDescriptor, renderDevelopmentUnits} from './lib/development-runtime.js';
import {cleanupExactOwnedProcesses, readFreshCleanupIdentity} from './hosted-fixture-cleanup.mjs';

const binary=await realpath(resolve(process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD||'cli/target/ci/agent-browser'));
const root=await mkdtemp(join(tmpdir(),'ab-dev-boot-'));
const socketDir=join(root,'socket'),home=join(root,'home'),supervisor=join(root,'supervisor');
const known=new Map(),children=[];
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
async function identity(pid){const path=`/proc/${pid}`;const raw=await readFile(path+'/stat','utf8');const parts=raw.slice(raw.lastIndexOf(')')+2).split(/\s+/);return {pid,uid:(await stat(path)).uid,start:parts[19]};}
const fresh=pid=>readFreshCleanupIdentity(pid,{identity,executable:pid=>realpath(`/proc/${pid}/exe`)});
async function census(){const owned=[],unresolved=[];for(const name of await readdir('/proc')){if(!/^\d+$/.test(name))continue;const pid=Number(name);try{if((await stat(`/proc/${pid}`)).uid!==process.getuid())continue;const env=await readFile(`/proc/${pid}/environ`);if(!env.toString().split('\0').includes(`AGENT_BROWSER_SOCKET_DIR=${socketDir}`))continue;const row=await fresh(pid);if(row){assert.equal(row.executable,binary);owned.push(row);known.set(`${pid}:${row.start}`,row);}}catch(error){if(!['ENOENT','ESRCH','EACCES','EPERM'].includes(error.code))unresolved.push({pid,code:error.code||error.message});}}return {owned,unresolved};}
async function port(){const server=createServer();await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));const value=server.address().port;await new Promise(resolve=>server.close(resolve));return value;}
async function waitFor(test,label){const until=Date.now()+10000;do{if(await test())return;await sleep(25);}while(Date.now()<until);throw Error(`Timed out: ${label}`);}
function environment(source){const result={...process.env,AGENT_BROWSER_HOME:join(home,'.agent-browser'),AGENT_BROWSER_SESSION_SUPERVISOR_ROOT:supervisor,AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS:'0'};for(const line of source.split('\n'))if(line.startsWith('Environment=')){let value=line.slice(12);if(value.startsWith('"'))value=JSON.parse(value);const split=value.indexOf('=');result[value.slice(0,split)]=value.slice(split+1);}return result;}
function start(args,env){const child=spawn(binary,args,{env,stdio:['ignore','pipe','pipe']});let output='';child.stdout.on('data',data=>{output+=data;});child.stderr.on('data',data=>{output+=data;});children.push(child);return {child,output:()=>output};}
let failure,cleanup;
try{
 await mkdir(join(supervisor,'manifests'),{recursive:true});await mkdir(home,{recursive:true});
 const [dashboardPort,backendPort,lanePort]=await Promise.all([port(),port(),port()]);
 const descriptor=developmentRuntimeDescriptor({...process.env,AGENT_BROWSER_DEV_USER_HOME:root,AGENT_BROWSER_DEV_HOME:home,AGENT_BROWSER_DEV_RUNTIME_DIR:socketDir,AGENT_BROWSER_DEV_BROWSER_EXECUTABLE:binary,AGENT_BROWSER_DEV_DASHBOARD_PORT:String(dashboardPort),AGENT_BROWSER_DEV_BACKEND_PORT:String(backendPort),AGENT_BROWSER_DEV_LANE_PORT:String(lanePort)});
 // Pin the fixture lane explicitly; the descriptor's default is never borrowed.
 descriptor.laneStreamPort=lanePort;
 const units=renderDevelopmentUnits(descriptor,binary,{});
 const digest=createHash('sha256').update(await readFile(binary)).digest('hex');
 await writeFile(join(supervisor,'manifests','boot-fixture.json'),JSON.stringify({schemaVersion:'agent-browser.session-supervisor.v1',session:'boot-fixture',executablePath:binary,executableSha256:digest,streamPort:lanePort,runtimeProfile:'boot-fixture',provenance:{packageVersion:'fixture',installedAt:new Date().toISOString(),installedBy:'isolated boot regression'}}),{mode:0o600});
 const ingress=start([],environment(units[descriptor.unitNames.dashboard]));
 await waitFor(async()=>{assert.equal(ingress.child.exitCode,null,ingress.output());try{const response=await fetch(`http://127.0.0.1:${dashboardPort}/api/runtime/manifest`,{signal:AbortSignal.timeout(250)});await response.body?.cancel();return true;}catch{return false;}},'ingress listener');
 // With the host deliberately not started yet, ingress must remain a client.
 const until=Date.now()+1500;
 do{const rows=(await census()).owned;assert.equal(rows.filter(row=>row.pid!==ingress.child.pid).length,0,'generated ingress spawned a competing runtime host before supervised startup');await sleep(25);}while(Date.now()<until);
 const host=start(['session','supervisor','run-host'],environment(units[descriptor.unitNames.runtimeHost]));
 await waitFor(async()=>{assert.equal(host.child.exitCode,null,host.output());try{return Number((await readFile(join(socketDir,'runtime-host.pid'),'utf8')).trim())===host.child.pid;}catch{return false;}},'supervised host publication');
 await waitFor(async()=>{try{const response=await fetch(`http://127.0.0.1:${lanePort}/api/sessions`);await response.body?.cancel();return response.ok;}catch{return false;}},'supervised lane readiness');
 await sleep(500);assert.equal(host.child.exitCode,null);assert.equal(Number((await readFile(join(socketDir,'runtime-host.pid'),'utf8')).trim()),host.child.pid);await census();
 console.log(JSON.stringify({passed:true,ingressDidNotSpawnHost:true,hostPid:host.child.pid,binarySha256:digest,browserLaunches:0}));
}catch(error){failure=error;}finally{
 cleanup=await cleanupExactOwnedProcesses({census,readIdentity:fresh,signal:(pid,signal)=>process.kill(pid,signal),known,sleep,allowSigkill:false,graceMs:3000,pollMs:25});
 assert.equal(cleanup.qualified,true,'exact fixture cleanup must qualify');
 await rm(root,{recursive:true});
 console.log(JSON.stringify({fixtureRoot:root,cleanupQualified:cleanup.qualified,ownedProcesses:known.size,processes:[...known.values()]}));
}
if(failure)throw failure;
