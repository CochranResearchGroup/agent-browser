/** Installed-host consumer acceptance. Only explicit dummy scope; real accounts are preserved. */
import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {readFile,writeFile,rename,mkdir,realpath,readdir,stat} from 'node:fs/promises';
import {join,isAbsolute} from 'node:path';
import {homedir} from 'node:os';
import {pathToFileURL} from 'node:url';
import {randomBytes,createHash} from 'node:crypto';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import {setTimeout as delay} from 'node:timers/promises';
import {createSmokeContext,createMcpStdioClient} from './smoke-utils.js';
import {createPlaybookClient} from '../packages/client/src/playbooks.js';
import {createLoginPlaybook} from '../packages/client/src/login-playbook.js';
import {connectHostedAuthHelper} from '../packages/client/src/hosted-auth-helper.js';
import {browserFixtureEnvironment} from './hosted-auth-fixture-environment.js';

assert(process.argv.includes('--apply-owned-dummy'),'Explicit owned dummy integration flag required');
const nativeStaged=process.argv.includes('--native-staged');
const binary=process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD,chrome=process.env.AGENT_BROWSER_SMOKE_BROWSER_EXECUTABLE;
assert(binary&&chrome&&isAbsolute(binary)&&isAbsolute(chrome)&&process.env.CODEX_THREAD_ID,'Pin browser/runtime and use an actual ordinary Codex turn');
const run=promisify(execFile),hash=value=>createHash('sha256').update(value).digest('hex');
const app=await realpath(join(homedir(),'.local/share/codex-bots/current'));
const {connectRemoteView}=await import(pathToFileURL(join(app,'src/remote-view.mjs')));
const {verifyCandidate}=await import(pathToFileURL(join(app,'scripts/verify-installation-candidate.mjs')));
const context=createSmokeContext({prefix:'p245-hosted-',sessionPrefix:'p245'}),root=context.tempHome;
console.log(JSON.stringify({fixtureRoot:root}));
const receipt={fixtureRoot:root,startedAt:new Date().toISOString(),nativeStaged,scope:'Actual installed existing host plus independent dummy website and consumer playbook; no real source or account effect'};
const save=(name,value)=>writeFile(join(root,name),JSON.stringify(value,null,2)+'\n',{mode:0o600});
const until=async(check,message,ms=180000)=>{const deadline=Date.now()+ms;do{const value=await check();if(value)return value;await delay(200);}while(Date.now()<deadline);throw Error(message);};
const helperUrl='http://127.0.0.1:4781';
async function api(path,input){const response=await fetch(helperUrl+path,input===undefined?{}:{method:'POST',headers:{Origin:helperUrl,'Content-Type':'application/json'},body:JSON.stringify(input)});assert(response.ok,'Installed owner request refused');return response.json();}
const environmentFile=join(homedir(),'.config/codex-bots/runtime.env');
let originalEnvironment,fixtureEnvironment,changed=false,remote,desktop,client,task,peer,caller,requestId,passed=false,website,sourceRelease=false;
let submissions=0,principalReads=0,identifierSubmissions=0,identifiedUsername='';
const password='dummy-'+randomBytes(24).toString('hex'),username='owned-user',passwordHash=hash(password),cookie=randomBytes(24).toString('hex');
const stageCookie=randomBytes(24).toString('hex');
const processes=async()=>{const out=[];for(const pid of await readdir('/proc')){if(!/^\d+$/.test(pid)||Number(pid)===process.pid)continue;try{const env=(await readFile(`/proc/${pid}/environ`,'utf8')).split('\0'),args=await readFile(`/proc/${pid}/cmdline`,'utf8');if(!env.includes(`AGENT_BROWSER_HOME=${context.agentHome}`)&&!args.includes(root))continue;out.push({pid:Number(pid),executable:await realpath(`/proc/${pid}/exe`),startToken:(await readFile(`/proc/${pid}/stat`,'utf8')).split(')').at(-1).trim().split(/\s+/)[19],browser:args.includes('--user-data-dir')&&!args.includes('--type=')});}catch(error){if(!['ENOENT','ESRCH','EACCES'].includes(error.code))throw error;}}return out;};
async function call(name,args){const reply=await client.send('tools/call',{name,arguments:args});const envelope=JSON.parse(reply.content.find(c=>c.type==='text').text);assert.equal(envelope.success,true,'Owned browser request failed');return envelope.data;}
try {
 receipt.producer=await verifyCandidate(app);receipt.consumerHashes=Object.fromEntries(await Promise.all(['packages/client/src/login-playbook.js','packages/client/src/hosted-auth-helper.js','packages/client/src/playbooks.js'].map(async name=>[name,hash(await readFile(new URL('../'+name,import.meta.url)))])));
 receipt.agent={path:await realpath(binary),sha256:hash(await readFile(binary))};receipt.browser={path:await realpath(chrome),sha256:hash(await readFile(chrome))};
 originalEnvironment=await readFile(environmentFile,'utf8');assert(!originalEnvironment.includes('CODEX_AUTH_EXTERNAL_FIXTURE_CONFIG='),'Another fixture owns host configuration');
 const env=Object.fromEntries(originalEnvironment.split('\n').filter(line=>line&&!line.startsWith('#')&&line.includes('=')).map(line=>{const i=line.indexOf('=');return [line.slice(0,i),line.slice(i+1)];}));
 assert(isAbsolute(env.CODEX_BOTS_AUTH_MODULE));const originalModule=await realpath(env.CODEX_BOTS_AUTH_MODULE);receipt.originalEnvironmentSHA256=hash(originalEnvironment);
 receipt.beforeBots=await api('/api/bots');const auth=await api('/api/auth');assert(!auth.requests.some(r=>r.running),'Do not interrupt active private authentication');
 const health=await api('/api/health').catch(()=>null);if(health)assert(!health.capabilities.externalAuthConfigured,'Existing external integration needs separate custody');
 remote=await connectRemoteView({expectedInstallationId:env.CODEX_BOTS_REMOTE_VIEW_INSTALLATION_ID});receipt.remoteIdentity=remote.identity;
 receipt.peerBaseline=(await remote.inventory()).assignments.filter(a=>a.state==='active').map(({assignmentId,desktopId,generation})=>({assignmentId,desktopId,generation}));
 desktop=await remote.acquire({botId:'p245-owned-dummy',idempotencyKey:'p245-'+randomBytes(16).toString('hex')});desktop={...desktop,...await remote.observe(desktop)};
 assert(!receipt.peerBaseline.some(p=>p.assignmentId===desktop.assignmentId||p.desktopId===desktop.desktopId));receipt.desktop=desktop;
 const launch=await remote.environment(desktop);context.env=Object.fromEntries(Object.entries(context.env).filter(([key])=>!key.startsWith('AGENT_BROWSER_')&&!key.startsWith('XDG_')));
 const guides=join(context.agentHome,'siteguides');await mkdir(guides,{recursive:true});
 Object.assign(context.env,launch.environment,{AGENT_BROWSER_HOME:context.agentHome,AGENT_BROWSER_SOCKET_DIR:context.socketDir,AGENT_BROWSER_RUNTIME_HOST:'1',AGENT_BROWSER_SESSION_DISPLAY:launch.environment.DISPLAY,AGENT_BROWSER_EXECUTABLE_PATH:chrome,AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD:binary,AGENT_BROWSER_SITEGUIDE_DIRECTORY:guides});
 context.env=browserFixtureEnvironment(context.env);
 let origin;
 website=createServer(async(req,res)=>{
  res.setHeader('Content-Type','text/html');
  if(nativeStaged&&req.url==='/login'&&req.method==='GET')return res.end('<title>Managed identifier fixture</title><form data-auth-account="dummy" method="post" action="/identify"><input name="username"><button id="identifier-submit">Continue</button></form>');
  if(nativeStaged&&req.url==='/identify'&&req.method==='POST'){
   let body='';for await(const bytes of req){body+=bytes;if(body.length>4096){res.statusCode=413;return res.end('Refused');}}
   identifierSubmissions++;const fields=new URLSearchParams(body);if(fields.get('username')!==username){res.statusCode=403;return res.end('Refused');}
   identifiedUsername=fields.get('username');res.statusCode=303;res.setHeader('Set-Cookie',`stage=${stageCookie}; HttpOnly; SameSite=Strict; Path=/`);res.setHeader('Location','/password');return res.end();
  }
  if(nativeStaged&&req.url==='/password'&&req.method==='GET'&&req.headers.cookie===`stage=${stageCookie}`&&identifiedUsername===username)return res.end('<title>Managed password fixture</title><p id="account">owned-user</p><form data-auth-account="dummy" method="post" action="/login"><input name="password" type="password"><button id="password-submit">Login</button></form>');
  if(req.url==='/login'&&req.method==='GET')return res.end('<title>Managed auth fixture</title><form data-auth-account="dummy" method="post" action="/login"><input name="username"><input name="password" type="password"><button>Login</button></form>');
  if(req.url==='/login'&&req.method==='POST'){
   let body='';for await(const bytes of req){body+=bytes;if(body.length>4096){res.statusCode=413;return res.end('Refused');}}
   const fields=new URLSearchParams(body);submissions++;
   const validAccount=nativeStaged?req.headers.cookie===`stage=${stageCookie}`&&identifiedUsername===username:fields.get('username')===username;
   if(!validAccount||hash(fields.get('password')||'')!==passwordHash){res.statusCode=403;return res.end('Refused');}
   res.statusCode=303;res.setHeader('Set-Cookie',nativeStaged?[`fixture=${cookie}; HttpOnly; SameSite=Strict; Path=/`,'stage=; Max-Age=0; HttpOnly; SameSite=Strict; Path=/']:`fixture=${cookie}; HttpOnly; SameSite=Strict; Path=/`);res.setHeader('Location','/authenticated');return res.end();
  }
  if(req.headers.cookie===`fixture=${cookie}`){
   if(req.url==='/whoami'){principalReads++;res.setHeader('Content-Type','application/json');return res.end(JSON.stringify({origin,username}));}
   if(req.url==='/authenticated')return res.end('<title>Managed authenticated account</title><main id="application" data-account="dummy" data-company="owned-company"><h1 id="authenticated">Managed authentication verified</h1>owned-user</main>');
  }
  res.statusCode=403;res.end('Refused');
 });
 await new Promise(resolve=>website.listen(0,'127.0.0.1',resolve));origin=`http://127.0.0.1:${website.address().port}`;
 if(nativeStaged){
  const post=(path,body,headers={})=>fetch(origin+path,{method:'POST',redirect:'manual',headers:{'Content-Type':'application/x-www-form-urlencoded',...headers},body:new URLSearchParams(body)});
  assert.equal((await post('/login',{password})).status,403);
  assert.equal((await post('/identify',{username:'wrong-user'})).status,403);
  assert.equal((await post('/identify',{username})).status,303);
  assert.equal((await fetch(origin+'/password',{headers:{Cookie:`stage=${stageCookie}`}})).status,200);
  assert.equal((await post('/login',{password:'wrong-password'},{Cookie:`stage=${stageCookie}`})).status,403);
  assert.equal((await post('/login',{password},{Cookie:`stage=${stageCookie}`})).status,303);
  const principal=await fetch(origin+'/whoami',{headers:{Cookie:`fixture=${cookie}`}});assert.equal(principal.status,200);assert.equal((await principal.json()).username,username);
  // The browser starts with a fresh cookie jar and counters after HTTP oracle rehearsal.
  submissions=0;identifierSubmissions=0;principalReads=0;identifiedUsername='';receipt.websiteOraclePreflight=true;
 }
 await writeFile(join(guides,'owned.json'),JSON.stringify({schemaVersion:'agent-browser.siteguide.v1',guideId:'owned',name:'Owned independent dummy',allowedOrigins:[origin],authenticatedSelector:'#application',account:{selector:'#application',attribute:'data-account'},organization:{selector:'#application',attribute:'data-company'},controls:{identifier:'input[name=username]',password:'input[name=password]',submit:nativeStaged?'#password-submit':'button',accountScope:'form[data-auth-account]',...(nativeStaged?{identifierSubmit:'#identifier-submit'}:{})},states:[...(nativeStaged?[{state:'identifier_required',selector:'input[name=username]'}]:[]),{state:'password_required',selector:'input[name=password]'}]}),{mode:0o600});
 client=createMcpStdioClient({context,args:['mcp','serve'],onFatal:()=>{}});await client.send('initialize',{protocolVersion:'2025-06-18',capabilities:{},clientInfo:{name:'p245-hosted-native',version:'1'}});client.notify('notifications/initialized');
 task=await call('browser_open',{profile:'p245-owned',profileMode:'persistent',taskKey:'dummy',url:origin+'/login'});peer=await call('browser_open',{profile:'p245-owned',profileMode:'existing',taskKey:'peer',url:'data:text/html,<p>Peer work preserved</p>'});await call('browser_tab',{taskId:task.taskId,operation:'select',tabId:task.tabId});
 const receivingTarget={provider:'agent-browser',...Object.fromEntries(['taskId','profileId','browserId','tabId'].map(k=>[k,task[k]]))};receipt.receivingTarget=receivingTarget;
 const processIdentity=await until(async()=>{const candidates=(await processes()).filter(p=>p.browser&&p.executable===receipt.browser.path);assert(candidates.length<=1);return candidates[0];},'Exact owned browser process absent');receipt.browserProcess=processIdentity;
 await until(async()=>(await remote.observation(desktop)).windows.windows.some(w=>w.active&&w.processIdentity?.pid===processIdentity.pid),'Owned browser not in foreground');
 let fixtureModule=join(app,'scripts/auth-external-fixture-host.mjs'),nativeModules;
 if(nativeStaged){
  const directory=join(root,'native-modules');await mkdir(directory,{mode:0o700});await writeFile(join(directory,'package.json'),'{"type":"module"}',{mode:0o400});nativeModules={};
  for(const [key,name] of [['receiver','hosted-login-receiver.js'],['browser','native-receiving-browser.js']]){const bytes=await readFile(new URL('../packages/client/src/'+name,import.meta.url)),path=join(directory,name);await writeFile(path,bytes,{mode:0o400});nativeModules[key]={path,sha256:hash(bytes)};}
  const bytes=await readFile(new URL('./hosted-auth-native-fixture-host.mjs',import.meta.url));fixtureModule=join(root,'native-host.mjs');await writeFile(fixtureModule,bytes,{mode:0o400});receipt.nativeHostSHA256=hash(bytes);receipt.nativeModules=nativeModules;
 }
 await save('host-config.json',{version:1,root,origin,username,password,nativeStaged,nativeModules,installedRoot:app,callerTaskId:process.env.CODEX_THREAD_ID,receivingTarget,processIdentity,desktop,installationId:env.CODEX_BOTS_REMOTE_VIEW_INSTALLATION_ID,smokeUtilities:new URL('./smoke-utils.js',import.meta.url).pathname,browserEnv:context.env});
 const wrapper=join(root,'host-owner.mjs');
 await writeFile(wrapper,`import {createAuthConfig as original} from ${JSON.stringify(pathToFileURL(originalModule).href)};\nimport {createAuthConfig as dummy} from ${JSON.stringify(pathToFileURL(fixtureModule).href)};\nexport async function createAuthConfig(context){const base=await original(context),extra=await dummy(context);return {...base,external:extra.external,accounts:[...base.accounts,...extra.accounts.map(a=>({...a,approvers:base.approvers}))],store:{read:(source,options)=>source==='dummy-fixture'?extra.store.read(source,options):base.store.read(source,options)}};}\n`,{mode:0o600});
 fixtureEnvironment=originalEnvironment.replace(/^CODEX_BOTS_AUTH_MODULE=.*$/m,'CODEX_BOTS_AUTH_MODULE='+wrapper)+'\nCODEX_AUTH_EXTERNAL_FIXTURE_CONFIG='+join(root,'host-config.json')+'\n';
 assert.equal(await readFile(environmentFile,'utf8'),originalEnvironment,'Owner configuration changed before effects');
 await writeFile(environmentFile+'.p245-next',fixtureEnvironment,{mode:0o600});await rename(environmentFile+'.p245-next',environmentFile);changed=true;
 await run('systemctl',['--user','restart','codex-bots.service']);
 await until(async()=>{try{const h=await api('/api/health');return h.ok&&h.capabilities.externalAuthConfigured&&h.build.identity===receipt.producer.identity;}catch{return false;}},'Installed hosted external helper not ready',60000);
 receipt.runningHostedService=true;
 caller=await connectHostedAuthHelper();const software=createPlaybookClient((name,args)=>client.send('tools/call',{name,arguments:args}));
 const file=join(root,'checkpoint.json');const checkpoints={load:async()=>{try{return JSON.parse(await readFile(file,'utf8'));}catch(error){if(error.code!=='ENOENT')throw error;return null;}},save:async(_key,value)=>{await writeFile(file+'.tmp',JSON.stringify(value),{mode:0o600});await rename(file+'.tmp',file);}};
 const options={taskId:task.taskId,guideId:'owned',accountRef:'dummy',organizationRef:'owned-company',scope:{person:'operator',tenant:'owned',account:'dummy',origin,source:'dummy-fixture',capabilities:['login']},expiresAt:Date.now()+600000,callerTaskId:caller.callerTaskId};
 const playbook=createLoginPlaybook({browser:software,helper:caller.helper,checkpoints});
 const pending=await playbook.advance(options);assert.equal(pending.reasonCode,'approval_required');requestId=pending.requestId;receipt.requestId=requestId;assert.notEqual(caller.callerTaskId,task.taskId);
 await api('/api/auth',{requestId,action:'approve'});
 const started=await playbook.advance(options);assert.equal(started.effectState,'unknown');
 await until(async()=>{try{return await readFile(join(root,'source-entered'));}catch(e){if(e.code!=='ENOENT')throw e;}},'Installed private source not entered');
 const host=JSON.parse(await readFile(join(root,'fixture-host.json'),'utf8'));const held=await fetch(host.observerUrl);assert.equal(held.status,409);assert.equal(JSON.parse(await readFile(join(root,'host-audit.json'),'utf8')).observerDispatches,0);receipt.privateObserverHeld=true;
 assert.equal((await createLoginPlaybook({browser:software,helper:caller.helper,checkpoints}).advance(options)).reasonCode,'helper_running_or_uncertain');
 await writeFile(join(root,'release-source'),'1',{mode:0o600});sourceRelease=true;
 const summary=await until(async()=>{const x=await caller.helper('inspect_request',{requestId});return !x.running?x:false;},'Installed private auth did not settle');
 assert.equal(summary.state,'authenticated');assert.equal(summary.captureState,'ready');receipt.outcome=summary;
 const completed=await playbook.advance(options);assert.equal(completed.status,'completed');assert.equal(completed.effectState,'verified');receipt.consumerResult=completed;
 const wrong=await software.verifyLogin({taskId:task.taskId,guideId:'owned',accountRef:'dummy',organizationRef:'wrong-company'});assert.equal(wrong.reasonCode,'organization_mismatch');receipt.independentWrongCompanyRejected=true;
 const visible=await fetch(host.observerUrl);assert.equal(visible.status,200);receipt.publicCaptureAfterSafe=await visible.json();receipt.nativeSafety=JSON.parse(await readFile(join(root,'native-safety.json'),'utf8'));assert(receipt.nativeSafety.safe);
 if(nativeStaged){
  const inventory=await run('systemctl',['--user','list-units','--all','--type=service','--plain','--no-legend']);
  const marker='-'+receipt.remoteIdentity.installationId.slice(0,12)+'-';
  const units=inventory.stdout.split('\n').map(line=>line.trim().split(/\s+/)[0]).filter(name=>name.startsWith('remote-view-')&&name.includes(marker));assert(units.length>0);
  const journal=await run('journalctl',['--user','--no-pager','--output=cat','--since',receipt.startedAt,...['codex-bots.service',...units].flatMap(unit=>['--unit',unit])],{maxBuffer:16*1024*1024});
  receipt.canaryJournalScan={unitCount:units.length+1,bytes:Buffer.byteLength(journal.stdout),matches:journal.stdout.includes(password)?1:0};assert.equal(receipt.canaryJournalScan.matches,0);
 }
 await run('systemctl',['--user','restart','codex-bots.service']);await until(async()=>{try{return (await api('/api/health')).ok;}catch{return false;}},'Installed restart not ready',60000);
 const resumed=await createLoginPlaybook({browser:software,helper:caller.helper,checkpoints}).advance(options);assert.equal(resumed.status,'completed');assert.equal(resumed.requestId,requestId);receipt.completedRestartPreserved=true;
 await assert.rejects(caller.helper('begin_authentication',{requestId}));receipt.replayRefused=true;
 assert.equal(submissions,1);assert(principalReads>0);receipt.websiteSubmissions=submissions;receipt.independentPrincipalReads=principalReads;receipt.hostAudit=JSON.parse(await readFile(join(root,'host-audit.json'),'utf8'));assert.equal(receipt.hostAudit.sourceReads,1);
 if(nativeStaged){assert.equal(identifierSubmissions,1);receipt.identifierSubmissions=identifierSubmissions;receipt.nativeInputAudit=JSON.parse(await readFile(join(root,'native-input-audit.json'),'utf8'));assert.equal(receipt.nativeInputAudit.privateSubmissions,1);assert.equal(receipt.nativeInputAudit.nativeTextDispatches,2);assert.equal(receipt.nativeInputAudit.nativeOperationDispatches,6);}
 else assert.equal(receipt.hostAudit.privateSubmissions,1);
 assert(JSON.stringify(await call('browser_snapshot',{taskId:peer.taskId})).includes('Peer work preserved'));receipt.peerTabPreserved=true;passed=true;
} catch(error) {receipt.failure='Inspect private failure artifact';await writeFile(join(root,'private-failure.txt'),String(error.stack),{mode:0o600});}
finally {
 const cleanup={};
 try {
  if(changed){
   if(!passed&&requestId){const response=await api('/api/auth',{requestId,action:'revoke'});cleanup.failedGrantRevoked=!!response;}
   if(!sourceRelease)await writeFile(join(root,'release-source'),'1',{mode:0o600});
   assert.equal(await readFile(environmentFile,'utf8'),fixtureEnvironment,'Do not overwrite changed foreign configuration');
   await writeFile(environmentFile+'.p245-restore',originalEnvironment,{mode:0o600});await rename(environmentFile+'.p245-restore',environmentFile);await run('systemctl',['--user','restart','codex-bots.service']);
   await until(async()=>{try{const h=await api('/api/health');return h.ok&&!h.capabilities.externalAuthConfigured;}catch{return false;}},'Original hosted config not ready',60000);
   assert.equal(await readFile(environmentFile,'utf8'),originalEnvironment);assert.deepEqual(await api('/api/bots'),receipt.beforeBots);cleanup.originalConfigAndBotsPreserved=true;
  }
  if(peer)await call('browser_close',{taskId:peer.taskId});if(task)await call('browser_close',{taskId:task.taskId});client?.close();
  try{const pid=Number(await readFile(join(context.socketDir,'runtime-host.pid'),'utf8')),row=(await processes()).find(p=>p.pid===pid);if(row){assert.equal(row.executable,receipt.agent.path);process.kill(pid,'SIGTERM');}}catch(error){if(error.code!=='ENOENT')throw error;}
  await until(async()=>(await processes()).length===0,'Owned process residue remains',30000);cleanup.ownedProcessResidue=0;
  if(desktop)await remote.release(desktop);
  if(remote){const active=(await remote.inventory()).assignments.filter(a=>a.state==='active');assert(!active.some(a=>a.assignmentId===desktop?.assignmentId));for(const p of receipt.peerBaseline??[])assert(active.some(a=>a.assignmentId===p.assignmentId&&a.desktopId===p.desktopId&&a.generation===p.generation));cleanup.peerAssignmentsPreserved=true;}
  cleanup.complete=true;
 }catch(error){passed=false;cleanup.complete=false;await writeFile(join(root,'private-cleanup-failure.txt'),String(error.stack),{mode:0o600});}
 remote?.close();website?.closeAllConnections();if(website?.listening)await new Promise(resolve=>website.close(resolve));receipt.cleanup=cleanup;receipt.passed=passed;receipt.finishedAt=new Date().toISOString();await save('qualification.json',receipt);console.log(JSON.stringify({fixtureRoot:root,passed,cleanup}));
}
process.exitCode=passed?0:1;
