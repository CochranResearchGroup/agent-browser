/** Public SiteGuide verification; every browser, task and process uses an owned disposable namespace. */
import assert from 'node:assert/strict';
import {createPlaybookClient} from '../packages/client/src/playbooks.js';
import {createServer} from 'node:http';
import {execFileSync} from 'node:child_process';
import {existsSync,mkdirSync,readFileSync,readlinkSync,readdirSync,realpathSync,writeFileSync} from 'node:fs';
import {isAbsolute,join} from 'node:path';
import {createMcpStdioClient,createSmokeContext,rootDir} from './smoke-utils.js';
import {retrievalPage,runRetrievalFixture} from './siteguide-retrieval-fixture.js';

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
const qbo=process.env.AGENT_BROWSER_SMOKE_SITE==='qbo';
const guideId=qbo?'qbo-fixture':'bill-fixture';
const accountRef=qbo?'intuit-fixture-user':'demo-account';
const organizationRef=qbo?'realm-fixture-42':'demo-company';
const appId=qbo?'qbo-company':'application';
const companyAttribute=qbo?'data-realm-id':'data-company';
const appHtml=qbo?`<main id="qbo-company" data-account="${accountRef}" data-realm-id="${organizationRef}"><span id="qbo-user" data-user="${accountRef}">${accountRef}@example.invalid</span></main>`:
 `<main id="application" data-account="${accountRef}" data-company="${organizationRef}">${accountRef}@example.invalid</main>`;
const controls=qbo?{identifier:'input[name=userid]',password:'input[name=password]',submit:'#intuit-signin',accountScope:'[data-auth-account]',identifierSubmit:'#intuit-next'}:
 {identifier:'#username',password:'#password',submit:'#submit',accountScope:'[data-auth-account]'};
const server=createServer((req,res)=>{res.setHeader('content-type','text/html');res.end(retrievalPage(req.url)||appHtml);});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));const origin=`http://127.0.0.1:${server.address().port}`;
writeFileSync(join(directory,guideId+'.json'),JSON.stringify({schemaVersion:'agent-browser.siteguide.v1',guideId,name:qbo?'Synthetic QBO':'Synthetic BILL',allowedOrigins:[origin],
 authenticatedSelector:'#'+appId,account:{selector:qbo?'#qbo-user':'#application',attribute:qbo?'data-user':'data-account'},
 organization:{selector:'#'+appId,attribute:companyAttribute},controls,
 states:[{state:'password_required',selector:controls.password},{state:'identifier_required',selector:controls.identifier},{state:'mfa_required',selector:'input[autocomplete=one-time-code]'}]}));
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
 const alice=await call('browser_open',{serviceName:'siteguide-verification-fixture',agentName:'synthetic-software-client',taskName:'Verify intended synthetic account and company',profile:'siteguide-fixture',profileMode:'persistent',taskKey:'alice',url:origin});tasks.push(alice.taskId);
 const guide=await call('browser_command',{taskId:alice.taskId,action:'playbook',params:{operation:'inspect_guide',guideId}});assert.equal(guide.guideId,guideId);
 const bob=await call('browser_open',{serviceName:'siteguide-verification-fixture',agentName:'synthetic-peer',taskName:'Preserve unrelated synthetic peer work',profile:'siteguide-fixture',profileMode:'existing',taskKey:'bob',url:'data:text/html,<p>Peer work preserved</p>'});tasks.push(bob.taskId);
 const verify=params=>call('browser_command',{taskId:alice.taskId,action:'playbook',params:{operation:'verify_login',guideId,accountRef,organizationRef,...params}});
 const software=createPlaybookClient((name,args)=>client.send('tools/call',{name,arguments:args}));
 const completed=await software.verifyLogin({taskId:alice.taskId,guideId,accountRef,organizationRef});assert.equal(completed.status,'completed');assert.equal(completed.taskId,alice.taskId);assert.equal(completed.effectState,'no_effect');
 const replay=await verify();assert.equal(replay.taskId,alice.taskId);assert.equal(replay.tabId,completed.tabId);
 if(process.env.AGENT_BROWSER_SMOKE_RETRIEVAL==='1')await runRetrievalFixture({call,software,taskId:alice.taskId,directory,origin});
 if(process.env.AGENT_BROWSER_SMOKE_GUIDE_AUTHORING==='1') {
  const {runAuthoringFixture}=await import('./siteguide-authoring-fixture.js');
  await runAuthoringFixture({call,software,taskId:alice.taskId,guide,context,directory,appHtml});
 }
 if(process.env.AGENT_BROWSER_SMOKE_POPUP_ATTRIBUTION==='1') {
  const opened=await call('browser_command',{taskId:alice.taskId,action:'evaluate',params:{script:`window.open(${JSON.stringify(origin+'/password-manager-destination')},'fixture-login-destination')!==null`}});
  assert.equal(opened.result,true,'Synthetic popup was created');
  await call('browser_tab',{taskId:bob.taskId,operation:'select',tabId:bob.tabId});
  const binding=JSON.parse(execFileSync('/usr/bin/python3',['-c',`
import json,sqlite3,sys
connection=sqlite3.connect('file:'+sys.argv[1]+'?mode=ro',uri=True)
state=json.loads(connection.execute("SELECT json FROM state_documents WHERE kind='browser_session_state'").fetchone()[0])
tab=state['tabs'][sys.argv[2]]
print(json.dumps({'endpoint':state['browsers'][tab['browserId']]['cdpEndpoint'],'targetId':tab['targetId']}))
`,join(context.agentHome,'service','runtime.sqlite3'),alice.tabId],{encoding:'utf8'}));
  const endpoint=new URL(binding.endpoint);endpoint.protocol='http:';endpoint.pathname='/json/list';
  const target=(await (await fetch(endpoint)).json()).find(page=>page.id===binding.targetId);
  assert(target,'Exact original task target remains observable');
  const visibility=()=>JSON.parse(execFileSync('node',[join(rootDir,'scripts/lib/p237_cdp_value.cjs'),target.webSocketDebuggerUrl,'document.visibilityState'],{encoding:'utf8'}));
  assert.equal(visibility(),'hidden','Peer selection must hide the original task before observation');
  const tabs=await call('browser_tab',{taskId:alice.taskId,operation:'list'});
  assert.equal(tabs.tabs.length,2,'Password-manager destination must be attributed to the original task');
  assert.equal(visibility(),'hidden','Popup inventory must preserve the physically selected peer');
 }
 let negative=false;
 if(process.env.AGENT_BROWSER_SMOKE_AUTH_HELPER_SOURCE) {
  const {runAuthFixture}=await import('./siteguide-auth-fixture.js');
  negative=(await runAuthFixture({context,software,taskId:alice.taskId,origin,guideId,accountRef,organizationRef,appHtml,qbo}))?.negative===true;
 }
 if(!negative){
 await call('browser_command',{taskId:alice.taskId,action:'evaluate',params:{script:`document.querySelector(${JSON.stringify('#'+appId)}).setAttribute(${JSON.stringify(companyAttribute)},"wrong-company")`}});
 const wrong=await verify();assert.equal(wrong.status,'failed');assert.equal(wrong.reasonCode,'organization_mismatch');
 await call('browser_command',{taskId:alice.taskId,action:'evaluate',params:{script:'const select=document.querySelectorAll.bind(document); document.querySelectorAll=(...args)=>{const end=performance.now()+25;while(performance.now()<end){};return select(...args)}'}});
 const stale=await verify({maxObservationAgeMs:1});assert.equal(stale.status,'adaptation_required');assert.equal(stale.reasonCode,'observation_stale');

 await call('browser_command',{taskId:alice.taskId,action:'evaluate',params:{script:`document.body.innerHTML=${JSON.stringify(qbo?'<input name=password type=password aria-label=Password>':'<input id=password type=password aria-label=Password>')}`}});
 const login=await verify();assert.equal(login.status,'waiting');assert.equal(login.reasonCode,'password_required');
 assert(JSON.stringify(await call('browser_snapshot',{taskId:bob.taskId})).includes('Peer work preserved'));
 console.log('Public existing-session verification, wrong company, login-required and peer-preserving replay passed');
 }
} finally {
 try{for(const taskId of tasks.reverse())await call('browser_close',{taskId});}finally{await cleanup();}
}
