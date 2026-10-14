/** Public SiteGuide verification; every browser, task and process uses an owned disposable namespace. */
import assert from 'node:assert/strict';
import {createPlaybookClient} from '../packages/client/src/playbooks.js';
import {createServer} from 'node:http';
import {existsSync,mkdirSync,readFileSync,readlinkSync,readdirSync,realpathSync,writeFileSync} from 'node:fs';
import {isAbsolute,join} from 'node:path';
import {createMcpStdioClient,createSmokeContext} from './smoke-utils.js';

const binary=process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD;
const chrome=process.env.AGENT_BROWSER_SMOKE_BROWSER_EXECUTABLE;
assert(binary&&isAbsolute(binary),'Pin an absolute installed/candidate executable');
assert(chrome&&isAbsolute(chrome),'Pin the reviewed browser executable');
const context=createSmokeContext({prefix:'ab-siteguide-',sessionPrefix:'siteguide'});
context.env=Object.fromEntries(Object.entries(context.env).filter(([key])=>!key.startsWith('AGENT_BROWSER_')&&!key.startsWith('XDG_')));
const directory=join(context.agentHome,'siteguides');mkdirSync(directory,{recursive:true});
Object.assign(context.env,{AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD:binary,AGENT_BROWSER_EXECUTABLE_PATH:chrome,
 AGENT_BROWSER_HOME:context.agentHome,AGENT_BROWSER_SOCKET_DIR:context.socketDir,AGENT_BROWSER_RUNTIME_HOST:'1',
 AGENT_BROWSER_SITEGUIDE_DIRECTORY:directory,XDG_RUNTIME_DIR:join(context.tempHome,'runtime'),XDG_CONFIG_HOME:join(context.tempHome,'config'),XDG_DATA_HOME:join(context.tempHome,'data')});
mkdirSync(context.env.XDG_RUNTIME_DIR,{mode:0o700});
const server=createServer((_req,res)=>{res.setHeader('content-type','text/html');res.end('<main id="application" data-account="demo-account" data-company="demo-company"><h1>BILL fixture company</h1></main>');});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));const origin=`http://127.0.0.1:${server.address().port}`;
writeFileSync(join(directory,'bill-fixture.json'),JSON.stringify({schemaVersion:'agent-browser.siteguide.v1',guideId:'bill-fixture',name:'Synthetic BILL',allowedOrigins:[origin],
 authenticatedSelector:'#application',account:{selector:'#application',attribute:'data-account'},organization:{selector:'#application',attribute:'data-company'},
 states:[{state:'password_required',selector:'input[type=password]'}]}));
const client=createMcpStdioClient({context,args:['mcp','serve'],onFatal:()=>{}});
const tasks=[];
async function call(name,args){const result=await client.send('tools/call',{name,arguments:args});const payload=JSON.parse(result.content.find(item=>item.type==='text').text);assert.equal(payload.success,true,JSON.stringify(payload));return payload.data;}
const wait=ms=>new Promise(resolve=>setTimeout(resolve,ms));
async function cleanup(){
 client.close();await wait(150);
 const file=join(context.socketDir,'runtime-host.pid');if(existsSync(file)){
  const pid=Number(readFileSync(file));try{const env=readFileSync(`/proc/${pid}/environ`,'utf8').split('\0');assert(env.includes(`HOME=${context.tempHome}`)&&env.includes(`AGENT_BROWSER_HOME=${context.agentHome}`));assert.equal(readlinkSync(`/proc/${pid}/exe`),realpathSync(binary));process.kill(pid,'SIGTERM');}catch(error){if(error.code!=='ENOENT')throw error;}
 }
 function owned(){return readdirSync('/proc').filter(name=>/^\d+$/.test(name)).filter(name=>{try{const env=readFileSync(`/proc/${name}/environ`,'utf8').split('\0');return env.includes(`HOME=${context.tempHome}`)&&env.includes(`AGENT_BROWSER_HOME=${context.agentHome}`);}catch{return false;}});}
 const deadline=Date.now()+10000;while(owned().length&&Date.now()<deadline)await wait(50);assert.equal(owned().length,0,'Zero owned process residue');
 await new Promise(resolve=>server.close(resolve));context.cleanupTempHome();console.log('Fixture census: zero owned process residue');
}
try {
 await client.send('initialize',{protocolVersion:'2025-06-18',capabilities:{},clientInfo:{name:'siteguide-verification',version:'1'}});client.notify('notifications/initialized');
 const discovery=await call('browser_discover',{category:'playbooks'});assert(discovery.operations.includes('verify_login'));
 console.log('Public SiteGuide routine discovery passed');
 const alice=await call('browser_open',{profile:'siteguide-fixture',profileMode:'persistent',taskKey:'alice',url:origin});tasks.push(alice.taskId);
 const guide=await call('browser_command',{taskId:alice.taskId,action:'playbook',params:{operation:'inspect_guide',guideId:'bill-fixture'}});assert.equal(guide.guideId,'bill-fixture');
 const bob=await call('browser_open',{profile:'siteguide-fixture',profileMode:'existing',taskKey:'bob',url:'data:text/html,<p>Peer work preserved</p>'});tasks.push(bob.taskId);
 const verify=params=>call('browser_command',{taskId:alice.taskId,action:'playbook',params:{operation:'verify_login',guideId:'bill-fixture',accountRef:'demo-account',organizationRef:'demo-company',...params}});
 const software=createPlaybookClient((name,args)=>client.send('tools/call',{name,arguments:args}));
 const completed=await software.verifyLogin({taskId:alice.taskId,guideId:'bill-fixture',accountRef:'demo-account',organizationRef:'demo-company'});assert.equal(completed.status,'completed');assert.equal(completed.taskId,alice.taskId);assert.equal(completed.effectState,'no_effect');
 const replay=await verify();assert.equal(replay.taskId,alice.taskId);assert.equal(replay.tabId,completed.tabId);
 if(process.env.AGENT_BROWSER_SMOKE_POPUP_ATTRIBUTION==='1') {
  const opened=await call('browser_command',{taskId:alice.taskId,action:'evaluate',params:{script:`window.open(${JSON.stringify(origin+'/password-manager-destination')},'fixture-login-destination')!==null`}});
  assert.equal(opened.result,true,'Synthetic popup was created');
  const tabs=await call('browser_tab',{taskId:alice.taskId,operation:'list'});
  assert.equal(tabs.tabs.length,2,'Password-manager destination must be attributed to the original task');
 }
 if(process.env.AGENT_BROWSER_SMOKE_AUTH_HELPER_SOURCE) {
  const {runAuthFixture}=await import('./siteguide-auth-fixture.js');
  await runAuthFixture({context,software,taskId:alice.taskId,origin});
 }
 await call('browser_command',{taskId:alice.taskId,action:'evaluate',params:{script:'document.querySelector("#application").setAttribute("data-company","wrong-company")'}});
 const wrong=await verify();assert.equal(wrong.status,'failed');assert.equal(wrong.reasonCode,'organization_mismatch');
 await call('browser_command',{taskId:alice.taskId,action:'evaluate',params:{script:'const select=document.querySelectorAll.bind(document); document.querySelectorAll=(...args)=>{const end=performance.now()+25;while(performance.now()<end){};return select(...args)}'}});
 const stale=await verify({maxObservationAgeMs:1});assert.equal(stale.status,'adaptation_required');assert.equal(stale.reasonCode,'observation_stale');

 await call('browser_command',{taskId:alice.taskId,action:'evaluate',params:{script:'document.body.innerHTML="<input type=password aria-label=Password>"'}});
 const login=await verify();assert.equal(login.status,'waiting');assert.equal(login.reasonCode,'password_required');
 assert(JSON.stringify(await call('browser_snapshot',{taskId:bob.taskId})).includes('Peer work preserved'));
 console.log('Public existing-session verification, wrong company, login-required and peer-preserving replay passed');
} finally {
 try{for(const taskId of tasks.reverse())await call('browser_close',{taskId});}finally{await cleanup();}
}
