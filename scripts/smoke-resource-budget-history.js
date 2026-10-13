import assert from 'node:assert/strict';
import {mkdirSync,readFileSync,existsSync,readdirSync,readlinkSync,realpathSync} from 'node:fs';
import {join} from 'node:path';
import {spawn} from 'node:child_process';
import {createMcpStdioClient,createSmokeContext,runCli} from './smoke-utils.js';
const binary=process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD,chrome=process.env.AGENT_BROWSER_SMOKE_BROWSER_EXECUTABLE;
assert(binary&&chrome);
const delay=ms=>new Promise(r=>setTimeout(r,ms));
async function cleanupFixture(fixture) {
 const {c,client}=fixture;
 client.close(); await delay(150);
 const pidFile=join(c.socketDir,'runtime-host.pid');
 if(existsSync(pidFile)) {
  const pid=Number(readFileSync(pidFile));
  try {
   const environment=readFileSync(`/proc/${pid}/environ`,'utf8').split('\0');
   assert(environment.includes(`HOME=${c.tempHome}`)&&environment.includes(`AGENT_BROWSER_HOME=${c.agentHome}`));
   assert.equal(readlinkSync(`/proc/${pid}/exe`),realpathSync(binary));
   process.kill(pid,'SIGTERM');
  } catch(error) { if(error.code!=='ENOENT')throw error; }
 }
 function residue() {
  return readdirSync('/proc').filter(name=>/^\d+$/.test(name)).filter(name=>{
   try { const environment=readFileSync(`/proc/${name}/environ`,'utf8').split('\0');
    return environment.includes(`HOME=${c.tempHome}`)&&environment.includes(`AGENT_BROWSER_HOME=${c.agentHome}`);
   }catch{return false;}
  });
 }
 const deadline=Date.now()+10000;
 while(residue().length&&Date.now()<deadline)await delay(50);
 assert.equal(residue().length,0,'Zero exact fixture-owned process residue');
 console.log('Fixture census: zero owned process residue');
 c.cleanupTempHome();
}

async function fixture(pressure=false){
 const c=createSmokeContext({prefix:'ab-p240-',sessionPrefix:'resource'});
 c.env=Object.fromEntries(Object.entries(c.env).filter(([k])=>!k.startsWith('AGENT_BROWSER_')&&!k.startsWith('XDG_')));
 Object.assign(c.env,{AGENT_BROWSER_HOME:c.agentHome,AGENT_BROWSER_SOCKET_DIR:c.socketDir,AGENT_BROWSER_RUNTIME_HOST:'1',AGENT_BROWSER_EXECUTABLE_PATH:chrome,AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD:binary,AGENT_BROWSER_TERMINAL_HISTORY_LIMIT:'3',XDG_RUNTIME_DIR:join(c.tempHome,'runtime'),XDG_CONFIG_HOME:join(c.tempHome,'config'),XDG_DATA_HOME:join(c.tempHome,'data')});
 if(pressure)Object.assign(c.env,{AGENT_BROWSER_HOST_MEMORY_RESERVE_BYTES:'18446744073709551615',AGENT_BROWSER_LAUNCH_PRESSURE_WAIT_MS:'100'});
 mkdirSync(c.env.XDG_RUNTIME_DIR,{mode:0o700});
 const client=createMcpStdioClient({context:c,args:['mcp','serve'],onFatal:()=>{}});
 await client.send('initialize',{protocolVersion:'2025-06-18',capabilities:{},clientInfo:{name:'resource-acceptance',version:'1'}});client.notify('notifications/initialized');
 const call=async(name,args)=>{let r=await client.send('tools/call',{name,arguments:args}),p=JSON.parse(r.content.find(x=>x.type==='text').text);assert.equal(p.success,true,JSON.stringify(p));return p.data};
 return {c,client,call};
}
const f=await fixture();let tasks=[];
try{
 for(let i=0;i<5;i++){let t=await f.call('browser_open',{profile:'archive-fixture',profileMode:'persistent',taskKey:'completed-'+i,url:'about:blank'});await f.call('browser_close',{taskId:t.taskId});}
 let alice=await f.call('browser_open',{profile:'archive-fixture',profileMode:'persistent',taskKey:'alice-live',url:'data:text/html,<input id=draft aria-label=Draft>'});tasks.push(alice.taskId);
 let bob=await f.call('browser_open',{profile:'archive-fixture',profileMode:'existing',taskKey:'bob-live',url:'about:blank'});tasks.push(bob.taskId);assert.equal(alice.browserId,bob.browserId);
 await f.call('browser_fill',{taskId:alice.taskId,selector:'#draft',value:'Preserved active edit'});
 const result=await runCli(f.c,['--json','runtime','history','sessions','1']);const page=JSON.parse(result.stdout).data;assert.equal(page.activeLimit,3);assert.equal(page.records.length,1);assert(page.nextCursor);
 const next=JSON.parse((await runCli(f.c,['--json','runtime','history','sessions','1',String(page.nextCursor)])).stdout).data;assert.equal(next.records.length,1);assert(next.records[0].sequence<page.records[0].sequence);
 assert(JSON.stringify(await f.call('browser_snapshot',{taskId:alice.taskId})).includes('Preserved active edit'));
 await f.call('browser_continue',{taskId:alice.taskId});await f.call('browser_close',{taskId:bob.taskId});tasks=tasks.filter(x=>x!==bob.taskId);
 assert(JSON.stringify(await f.call('browser_snapshot',{taskId:alice.taskId})).includes('Preserved active edit'));
 console.log('Public archive pagination, live edit/continuation and peer-preserving close passed');
}finally{for(const taskId of tasks)await f.call('browser_close',{taskId});await cleanupFixture(f);}
const p=await fixture(true);let external,taskId;
try{
 await assert.rejects(p.call('browser_open',{profileMode:'disposable',taskKey:'pressure-new',url:'about:blank'}),/browser_launch_resource_pressure/);
 const created=JSON.parse((await runCli(p.c,['--json','runtime','create','pressure-borrowed','--browser-family','chromium'])).stdout);
 external=spawn(chrome,['--headless=new','--no-sandbox','--remote-debugging-port=0',`--user-data-dir=${created.userDataDir}`,'about:blank'],{env:p.c.env,stdio:'ignore'});
 const portFile=join(created.userDataDir,'DevToolsActivePort');let deadline=Date.now()+10000;while(!existsSync(portFile)&&Date.now()<deadline)await delay(50);assert(existsSync(portFile));
 let borrowed=await p.call('browser_open',{profile:'pressure-borrowed',profileMode:'existing',taskKey:'borrowed',url:'about:blank'});taskId=borrowed.taskId;
 await p.call('browser_continue',{taskId});await p.call('browser_close',{taskId});taskId=null;assert.equal(external.exitCode,null,'Borrowed browser survives scoped close');assert.equal(borrowed.profileId,'pressure-borrowed');
 console.log('New-process pressure defers while existing borrowed browser remains usable');
}finally{if(taskId)await p.call('browser_close',{taskId});if(external&&external.exitCode===null){external.kill('SIGTERM');await new Promise(r=>external.once('exit',r));}await cleanupFixture(p);}
