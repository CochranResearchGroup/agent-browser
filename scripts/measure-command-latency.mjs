#!/usr/bin/env node
/** Paired synthetic reads through the CLI and exact-target CDP. No installed
 * runtime, private profile or external site is used. Trusted elapsed intervals
 * come from StableTime service observations; guest clocks are not substituted.
 */
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFileSync, readdirSync, realpathSync, existsSync, writeFileSync, unlinkSync} from 'node:fs';
import {join, isAbsolute} from 'node:path';
import {createSmokeContext, createMcpStdioClient, runCli, parseJsonOutput} from './smoke-utils.js';

const binary=process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD;
const browser=process.env.AGENT_BROWSER_SMOKE_BROWSER_EXECUTABLE;
const clockConfig=process.env.AGENT_BROWSER_LATENCY_STABLE_TIME_CLIENT;
for(const [name,value] of Object.entries({binary,browser,clockConfig}))assert(value&&isAbsolute(value)&&existsSync(value),`Pin an existing absolute ${name} path`);
const samples=Number(process.env.AGENT_BROWSER_LATENCY_SAMPLES??5);
assert(Number.isSafeInteger(samples)&&samples>=3&&samples<=20,'Use 3 to 20 paired samples');
const context=createSmokeContext({prefix:'ab-command-latency-',sessionPrefix:'latency'});
context.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD=binary;
context.env.AGENT_BROWSER_EXECUTABLE_PATH=browser;
context.env.AGENT_BROWSER_ARGS='--no-sandbox,--disable-background-networking';
context.env.AGENT_BROWSER_IDLE_TIMEOUT_MS='0';
const profile=join(context.tempHome,'profile');
const ownedProcesses=()=>readdirSync('/proc').filter(pid=>/^\d+$/.test(pid)&&Number(pid)!==process.pid).flatMap(pid=>{
 try{
  const command=readFileSync(`/proc/${pid}/cmdline`).toString();
  const environment=readFileSync(`/proc/${pid}/environ`).toString().split('\0');
  if(!command.includes(context.tempHome)&&!environment.includes(`AGENT_BROWSER_SOCKET_DIR=${context.socketDir}`))return [];
  return [{pid:Number(pid),startToken:readFileSync(`/proc/${pid}/stat`,'utf8').split(')').at(-1).trim().split(/\s+/)[19],executable:realpathSync(`/proc/${pid}/exe`)}];
 }catch(error){if(['ENOENT','ESRCH','EACCES'].includes(error.code))return [];throw error;}
});
const cli=async args=>parseJsonOutput((await runCli(context,['--json','--session',context.session,'--profile',profile,...args])).stdout,'latency command');
const clockContext={env:{...process.env,AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD:process.env.AGENT_BROWSER_LATENCY_STABLE_TIME_BINARY??'stable-time'}};
const clock=createMcpStdioClient({context:clockContext,args:['mcp',clockConfig],onFatal:()=>{}});
let socket,hostPid,opened=false;
const receipt={schemaVersion:'agent-browser.command-latency.v1',binarySha256:createHash('sha256').update(readFileSync(binary)).digest('hex'),browser:realpathSync(browser),samples:[],selectedPhase:process.env.AGENT_BROWSER_LATENCY_PHASE??null,phaseMeasurements:[],clock:'StableTime service UTC intervals at measurement instant',limitation:'Service-to-service brackets include clock delivery/request overhead. Actual operation elapsed time has only an upper bound. CLI process startup is included. Cold launch is separate. No installed-runtime or MCP-path speedup is implied.'};
const timestamp=value=>{
 const match=/^(.*\.)(\d{1,9})Z$/.exec(value);assert(match,'RFC3339 nanosecond UTC required');
 return BigInt(Date.parse(match[1]+'000Z'))*1_000_000n+BigInt(match[2].padEnd(9,'0'));
};
async function time(){
 const reply=await clock.send('tools/call',{name:'get_time',arguments:{}});
 assert(!reply.isError,'StableTime client failure');
 const result=JSON.parse(reply.content.find(block=>block.type==='text').text);
 assert(result.status==='qualified_live'&&result.bound==='finite','Reliable StableTime unavailable');
 return result;
}
async function measure(operation,run){
 const before=await time();const value=await run();const after=await time();
 assert.equal(before.epoch,after.epoch,'StableTime epoch changed; discard sample');
 const lower=timestamp(after.estimate.earliest_utc)-timestamp(before.estimate.latest_utc);
 const upper=timestamp(after.estimate.latest_utc)-timestamp(before.estimate.earliest_utc);
 assert(upper>=0n,'StableTime intervals moved backwards; discard sample');
 const sample={operation,lowerMs:Number(lower>0n?lower:0n)/1e6,upperMs:Number(upper)/1e6,intervalAppliesTo:'service-observation span including delivery',operationElapsedIntervalMs:[0,Number(upper)/1e6],before:before.estimate,after:after.estimate};
 receipt.samples.push(sample);return value;
}
function summarize(operation){
 const rows=receipt.samples.filter(row=>row.operation===operation);
 const quantile=(key,p)=>[...rows].sort((a,b)=>a[key]-b[key])[Math.ceil(rows.length*p)-1]?.[key]??null;
 return {n:rows.length,medianServiceBracketIntervalMs:[quantile('lowerMs',.5),quantile('upperMs',.5)],p95ServiceBracketIntervalMs:[quantile('lowerMs',.95),quantile('upperMs',.95)],medianOperationElapsedIntervalMs:[0,quantile('upperMs',.5)],p95OperationElapsedIntervalMs:[0,quantile('upperMs',.95)]};
}
try{
 await clock.send('initialize',{protocolVersion:'2024-11-05',capabilities:{},clientInfo:{name:'ab-latency',version:'1'}});
 const url='data:text/html,<title>Latency sample</title><main>synthetic latency sample</main>';
 assert.equal((await measure('cold_launch',()=>cli(['open',url]))).success,true);opened=true;
 hostPid=Number(readFileSync(join(context.socketDir,'runtime-host.pid'),'utf8').trim());
 const [port]=readFileSync(join(profile,'DevToolsActivePort'),'utf8').trim().split('\n');
 const pages=await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
 const page=pages.find(page=>page.type==='page'&&page.url===url);assert(page,'Exact synthetic target missing');
 socket=new WebSocket(page.webSocketDebuggerUrl);
 await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
 let nextId=0;const pending=new Map();
 socket.addEventListener('message',event=>{const value=JSON.parse(event.data);const callback=pending.get(value.id);if(callback){pending.delete(value.id);value.error?callback.reject(new Error('CDP evaluation failed')):callback.resolve(value.result);}});
 const expression='document.body.firstElementChild.textContent';
 const direct=()=>new Promise((resolve,reject)=>{const id=++nextId;pending.set(id,{resolve,reject});socket.send(JSON.stringify({id,method:'Runtime.evaluate',params:{expression,returnByValue:true,awaitPromise:true,timeout:29750}}));});
 const expected='synthetic latency sample';
 const agent=async()=>{const value=await cli(['eval',expression]);assert.equal(value.success,true);assert.equal(value.data.result,expected);for(const sample of value.data.latencyPhases??[])if(sample.phase===receipt.selectedPhase)receipt.phaseMeasurements.push(sample);return value;};
 const cdp=async()=>{const value=await direct();assert(!value.exceptionDetails);assert.equal(value.result.value,expected);return value;};
 await agent();await cdp();
 const startupLock=join(context.socketDir,'runtime-host.startup.lock');
 writeFileSync(startupLock,'latency-probe-owned-startup',{flag:'wx',mode:0o600});
 let completed=false;
 const contendedRead=agent().then(value=>{completed=true;return value;});
 // Observe completion before releasing an unrelated startup admission. This
 // bounded coordination check is not an elapsed-time performance assertion.
 await new Promise(resolve=>setTimeout(resolve,1000));
 receipt.warmReuseCompletedBeforeStartupLockRelease=completed;
 assert.equal(readFileSync(startupLock,'utf8'),'latency-probe-owned-startup');
 unlinkSync(startupLock);
 await contendedRead;
 for(let i=0;i<samples;i++){
  const order=i%2?[['agent_browser',agent],['direct_cdp',cdp]]:[['direct_cdp',cdp],['agent_browser',agent]];
  for(const [name,run] of order)await measure(name,run);
 }
 const profiled=await measure('profiled_single_batch',()=>cli(['batch','--dependent',`eval ${expression}`]));
 assert.equal(profiled.success,true);assert.equal(profiled.data.results[0].result.result,expected);
 receipt.phaseDiagnostics={
  clock:'Existing daemon guest-clock diagnostics; not qualified elapsed-time acceptance',
  daemonTimings:profiled.data.results[0].daemonTimings,
  unmeasuredPhases:['routing','state reads','readiness','CDP versus other action execution','persistence'],
 };
 receipt.summary=Object.fromEntries(['cold_launch','direct_cdp','agent_browser'].map(name=>[name,summarize(name)]));
 if(receipt.selectedPhase)assert(receipt.phaseMeasurements.some(sample=>sample.status==='measured'),'Selected phase produced no qualified measurement; inspect response wiring and StableTime availability');
 receipt.measurementCompleted=true;
}catch(error){receipt.measurementCompleted=false;receipt.error=error.message;process.exitCode=1;}
finally{
 socket?.close();
 if(opened)try{await cli(['close']);}catch{receipt.cleanupFailed=true;process.exitCode=1;}
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
