import assert from 'node:assert/strict';
import {mkdirSync,readFileSync,existsSync,readdirSync,readlinkSync,realpathSync} from 'node:fs';
import {join} from 'node:path';
import {spawn} from 'node:child_process';
import {createMcpStdioClient,createSmokeContext,runCli} from './smoke-utils.js';
const binary=process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD,chrome=process.env.AGENT_BROWSER_SMOKE_BROWSER_EXECUTABLE;
assert(binary&&chrome);
const delay=ms=>new Promise(r=>setTimeout(r,ms));
async function cleanupFixture(fixture) {
 const {context,client}=fixture;
 client.close(); await delay(150);
 const pidFile=join(context.socketDir,'runtime-host.pid');
 if(existsSync(pidFile)) {
  const pid=Number(readFileSync(pidFile));
  try {
   const environment=readFileSync(`/proc/${pid}/environ`,'utf8').split('\0');
   assert(environment.includes(`HOME=${context.tempHome}`)&&environment.includes(`AGENT_BROWSER_HOME=${context.agentHome}`));
   assert.equal(readlinkSync(`/proc/${pid}/exe`),realpathSync(binary));
   process.kill(pid,'SIGTERM');
  } catch(error) { if(error.code!=='ENOENT')throw error; }
 }
 function residue() {
  return readdirSync('/proc').filter(name=>/^\d+$/.test(name)).filter(name=>{
   try { const environment=readFileSync(`/proc/${name}/environ`,'utf8').split('\0');
    return environment.includes(`HOME=${context.tempHome}`)&&environment.includes(`AGENT_BROWSER_HOME=${context.agentHome}`);
   }catch{return false;}
  });
 }
 const deadline=Date.now()+10000;
 while(residue().length&&Date.now()<deadline)await delay(50);
 assert.equal(residue().length,0,'Zero exact fixture-owned process residue');
 console.log('Fixture census: zero owned process residue');
 context.cleanupTempHome();
}

async function fixture(pressure=false){
 const context=createSmokeContext({prefix:'ab-p240-',sessionPrefix:'resource'});
 context.env=Object.fromEntries(Object.entries(context.env).filter(([k])=>!k.startsWith('AGENT_BROWSER_')&&!k.startsWith('XDG_')));
 Object.assign(context.env,{AGENT_BROWSER_HOME:context.agentHome,AGENT_BROWSER_SOCKET_DIR:context.socketDir,AGENT_BROWSER_RUNTIME_HOST:'1',AGENT_BROWSER_EXECUTABLE_PATH:chrome,AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD:binary,AGENT_BROWSER_TERMINAL_HISTORY_LIMIT:'3',XDG_RUNTIME_DIR:join(context.tempHome,'runtime'),XDG_CONFIG_HOME:join(context.tempHome,'config'),XDG_DATA_HOME:join(context.tempHome,'data')});
 if(pressure)Object.assign(context.env,{AGENT_BROWSER_HOST_MEMORY_RESERVE_BYTES:'18446744073709551615',AGENT_BROWSER_LAUNCH_PRESSURE_WAIT_MS:'100'});
 mkdirSync(context.env.XDG_RUNTIME_DIR,{mode:0o700});
 const client=createMcpStdioClient({context:context,args:['mcp','serve'],onFatal:()=>{}});
 await client.send('initialize',{protocolVersion:'2025-06-18',capabilities:{},clientInfo:{name:'resource-acceptance',version:'1'}});client.notify('notifications/initialized');
 const call=async(name,args)=>{let r=await client.send('tools/call',{name,arguments:args}),pressureFixture=JSON.parse(r.content.find(x=>x.type==='text').text);assert.equal(pressureFixture.success,true,JSON.stringify(pressureFixture));return pressureFixture.data};
 return {context,client,call};
}
const archiveFixture=await fixture();let tasks=[];
try{
 for(let i=0;i<5;i++){let t=await archiveFixture.call('browser_open',{profile:'archive-fixture',profileMode:'persistent',taskKey:'completed-'+i,url:'about:blank'});await archiveFixture.call('browser_close',{taskId:t.taskId});}
 let alice=await archiveFixture.call('browser_open',{profile:'archive-fixture',profileMode:'persistent',taskKey:'alice-live',url:'data:text/html,<input id=draft aria-label=Draft>'});tasks.push(alice.taskId);
 let bob=await archiveFixture.call('browser_open',{profile:'archive-fixture',profileMode:'existing',taskKey:'bob-live',url:'about:blank'});tasks.push(bob.taskId);assert.equal(alice.browserId,bob.browserId);
 await archiveFixture.call('browser_fill',{taskId:alice.taskId,selector:'#draft',value:'Preserved active edit'});
 const result=await runCli(archiveFixture.context,['--json','runtime','history','sessions','1']);const page=JSON.parse(result.stdout).data;assert.equal(page.activeLimit,3);assert.equal(page.records.length,1);assert(page.nextCursor);
 const next=JSON.parse((await runCli(archiveFixture.context,['--json','runtime','history','sessions','1',String(page.nextCursor)])).stdout).data;assert.equal(next.records.length,1);assert(next.records[0].sequence<page.records[0].sequence);
 assert(JSON.stringify(await archiveFixture.call('browser_snapshot',{taskId:alice.taskId})).includes('Preserved active edit'));
 await archiveFixture.call('browser_continue',{taskId:alice.taskId});await archiveFixture.call('browser_close',{taskId:bob.taskId});tasks=tasks.filter(x=>x!==bob.taskId);
 assert(JSON.stringify(await archiveFixture.call('browser_snapshot',{taskId:alice.taskId})).includes('Preserved active edit'));
 console.log('Public archive pagination, live edit/continuation and peer-preserving close passed');
}finally{for(const taskId of tasks)await archiveFixture.call('browser_close',{taskId});await cleanupFixture(archiveFixture);}
const pressureFixture=await fixture(true);let external,taskId;
try{
 await assert.rejects(pressureFixture.call('browser_open',{profileMode:'disposable',taskKey:'pressure-new',url:'about:blank'}),/browser_launch_resource_pressure/);
 const created=JSON.parse((await runCli(pressureFixture.context,['--json','runtime','create','pressure-borrowed','--browser-family','chromium'])).stdout);
 external=spawn(chrome,['--headless=new','--no-sandbox','--remote-debugging-port=0',`--user-data-dir=${created.userDataDir}`,'about:blank'],{env:pressureFixture.context.env,stdio:'ignore'});
 const portFile=join(created.userDataDir,'DevToolsActivePort');let deadline=Date.now()+10000;while(!existsSync(portFile)&&Date.now()<deadline)await delay(50);assert(existsSync(portFile));
 let borrowed=await pressureFixture.call('browser_open',{profile:'pressure-borrowed',profileMode:'existing',taskKey:'borrowed',url:'about:blank'});taskId=borrowed.taskId;
 await pressureFixture.call('browser_continue',{taskId});await pressureFixture.call('browser_close',{taskId});taskId=null;assert.equal(external.exitCode,null,'Borrowed browser survives scoped close');assert.equal(borrowed.profileId,'pressure-borrowed');
 console.log('New-process pressure defers while existing borrowed browser remains usable');
}finally{if(taskId)await pressureFixture.call('browser_close',{taskId});if(external&&external.exitCode===null){external.kill('SIGTERM');await new Promise(r=>external.once('exit',r));}await cleanupFixture(pressureFixture);}
