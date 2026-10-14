import assert from 'node:assert/strict';
import {readFileSync,readdirSync,writeFileSync,existsSync,renameSync,mkdirSync} from 'node:fs';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {chromium} from 'playwright';
import {createHash,randomBytes} from 'node:crypto';
import {createLoginPlaybook} from '../packages/client/src/login-playbook.js';

/** Constrained dummy acceptance using actual, explicitly pinned helper source.
 * No viewers, watches, recording or real credentials. Every fixture client
 * observation is wired to the producer's privacy barrier. This cannot qualify
 * desktop-wide privacy or production credential methods.
 */
export async function runAuthFixture({context,software,taskId,origin,guideId,accountRef,organizationRef,appHtml,qbo}) {
 const root=process.env.AGENT_BROWSER_SMOKE_AUTH_HELPER_SOURCE;
 assert(root,'Explicit actual auth-helper source is required');
 const {startAuthBroker,callAuthBroker}=await import(pathToFileURL(join(root,'src/auth-broker.mjs')));
 const {authPrivacy}=await import(pathToFileURL(join(root,'src/auth-privacy.mjs')));
 const {createBrowserLoginReceiver}=await import(pathToFileURL(join(root,'src/auth-browser.mjs')));
 let port;
 for(const pid of readdirSync('/proc').filter(name=>/^\d+$/.test(name))) {
  try {
   const env=readFileSync(`/proc/${pid}/environ`,'utf8').split('\0');
   if(!env.includes(`HOME=${context.tempHome}`)||!env.includes(`AGENT_BROWSER_HOME=${context.agentHome}`))continue;
   const args=readFileSync(`/proc/${pid}/cmdline`,'utf8').split('\0');
   const argument=args.find(arg=>/^--remote-debugging-port=\d+$/.test(arg));
   const index=args.indexOf('--remote-debugging-port');
   const candidate=argument?Number(argument.split('=')[1]):index>=0?Number(args[index+1]):0;
   if(candidate>0)port=candidate;
  }catch{}
 }
 if(!port) {
  const files=readdirSync(context.agentHome,{recursive:true});
  const active=files.filter(file=>file.endsWith('/DevToolsActivePort'));
  assert.equal(active.length,1,'Exactly one owned browser discovery file');
  port=Number(readFileSync(join(context.agentHome,active[0]),'utf8').split('\n')[0]);
 }
 assert(port>0,'Locate only the owned fixture browser CDP port');
 const browser=await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
 const page=browser.contexts().flatMap(item=>item.pages()).find(item=>item.url()===origin+'/');
 assert(page,'Use the existing task receiving page');
 const privateRoot=join(context.tempHome,'auth');mkdirSync(privateRoot,{mode:0o700});
 const records={};
 const persist=async()=>writeFileSync(join(privateRoot,'privacy.json'),JSON.stringify(records),{mode:0o600});
 const privacy=authPrivacy({records,persist});
 let broker;
 let privateReads=0;
 let finishMfa;
 const privatePassword='dummy-'+randomBytes(24).toString('hex');
 const credentialUsername=accountRef+'@example.invalid';
 const expectedPasswordHash=createHash('sha256').update(privatePassword).digest('hex');
 const guide=await software.inspectGuide(taskId,guideId);
 const controls=guide.controls;assert(controls,'Receiving controls come from the reviewed SiteGuide');
 try {
  const identifierStart=qbo && process.env.AGENT_BROWSER_SMOKE_LOGIN_VARIANT!=='password';
  const form=qbo?`<main data-auth-account="${accountRef}"><input name="userid" type="email"><button id="intuit-next" ${identifierStart?'':'hidden'}>Continue</button><input name="password" type="password" ${identifierStart?'hidden':''}><button id="intuit-signin">Sign in</button></main>`:
   `<main data-auth-account="${accountRef}"><input id="username"><input id="password" type="password"><button id="submit">Log in</button></main>`;
  await page.setContent(form);
  if(identifierStart)await page.evaluate(()=>document.querySelector('#intuit-next').onclick=()=>{
   document.querySelector('input[name=password]').hidden=false;
   document.querySelector('#intuit-next').hidden=true;
  });
  await page.evaluate(({controls,username,passwordHash,origin})=>document.querySelector(controls.submit).onclick=async()=>{
   window.syntheticSubmissions=(window.syntheticSubmissions??0)+1;
   const actualUsername=document.querySelector(controls.identifier).value;
   const actualPassword=document.querySelector(controls.password).value;
   const bytes=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(actualPassword));
   const hash=[...new Uint8Array(bytes)].map(byte=>byte.toString(16).padStart(2,'0')).join('');
   if(actualUsername!==username||hash!==passwordHash){document.body.innerHTML='<p id="login-error">Credentials rejected by fixture website</p>';return;}
   document.body.innerHTML='<div id="mfa"><input autocomplete="one-time-code"><button id="fixture-human-confirm">Approve this synthetic challenge</button></div>';
   document.querySelector('#fixture-human-confirm').onclick=()=>{
    window.syntheticMfaConfirmations=(window.syntheticMfaConfirmations??0)+1;
    document.body.innerHTML='<p>Login continued in password-manager destination</p>';
    window.open(origin+'/login-destination','fixture-auth-destination');
   };
  },{controls,username:credentialUsername,passwordHash:expectedPasswordHash,origin});
  assert.equal((await software.verifyLogin({taskId,guideId,accountRef,organizationRef})).reasonCode,identifierStart?'identifier_required':'password_required');
  let destination=page;
  const receivingPage={url:()=>destination.url(),locator:selector=>destination.locator(selector),
    evaluate:(...args)=>destination.evaluate(...args)};
  const receiver=createBrowserLoginReceiver({page:receivingPage,account:accountRef,origin,
   usernameSelector:controls.identifier,passwordSelector:controls.password,submitSelector:controls.submit,
   scopeSelector:controls.accountScope,verifiedSelector:guide.authenticatedSelector,expectedUsername:credentialUsername});
  const baseApply=receiver.apply;
  let challenge;
  const reachedChallenge=new Promise(resolve=>challenge=resolve);
  const humanMfa=new Promise(resolve=>finishMfa=resolve);
  receiver.apply=async(credential,scope,authorize)=>{
   if(identifierStart) {
    await authorize();await page.locator(controls.identifier).fill(credential.username);
    await authorize();await page.locator(controls.identifierSubmit).click();
   }
   const popup=page.waitForEvent('popup',{timeout:20000});
   // A failed credential test may never create a popup; consume rejection.
   popup.catch(()=>{});
   await baseApply(credential,scope,authorize);
   await page.locator('#mfa').waitFor({state:'visible',timeout:5000});challenge();
   await humanMfa;await authorize();
   destination=await popup;await destination.waitForLoadState('domcontentloaded');
  };
  broker=await startAuthBroker({root:join(privateRoot,'broker'),
   principals:[{role:'caller',id:'fixture-agent',taskId},{role:'approver',id:'fixture-person'}],
   accounts:[{person:'fixture-person',tenant:'fixture',account:accountRef,origin,source:'dummy-fixture',callers:['fixture-agent'],approvers:['fixture-person']}],
   privacy,receiver,store:{read:async(_source,{authorize})=>{await authorize();privateReads++;
    if(process.env.AGENT_BROWSER_SMOKE_AUTH_SOURCE_LOCKED==='1')throw Object.assign(Error('Synthetic password manager is locked'),{code:'auth_store_unavailable'});
    return {username:credentialUsername,password:process.env.AGENT_BROWSER_SMOKE_WRONG_CREDENTIAL==='1'?privatePassword+'-wrong':privatePassword};}}});
  const [caller,approver]=broker.connections.map(file=>JSON.parse(readFileSync(file,'utf8')));
  const checkpoints={
   load:async key=>{const file=join(privateRoot,'checkpoint.json');return existsSync(file)?JSON.parse(readFileSync(file,'utf8'))[key]??null:null;},
   save:async(key,value)=>{const file=join(privateRoot,'checkpoint.json'),temporary=file+'.tmp';writeFileSync(temporary,JSON.stringify({[key]:value}),{mode:0o600});renameSync(temporary,file);},
  };
  const helper=async(operation,args)=>{
   const response=await callAuthBroker(caller,operation,args);
   assert(!JSON.stringify(response).includes(privatePassword),'Producer never returns the opaque dummy secret');
   if(operation==='authenticate'&&process.env.AGENT_BROWSER_SMOKE_DROP_AUTH_ACK==='1')throw Error('Injected lost acknowledgement');
   return response;
  };
  const protectedBrowser={verifyLogin:options=>privacy.read('fixture-agent',()=>software.verifyLogin(options)),
    prepareLogin:options=>privacy.read('fixture-agent',()=>software.prepareLogin(options)),
    validatePlan:options=>privacy.read('fixture-agent',()=>software.validatePlan(options)),
    listTabs:id=>privacy.read('fixture-agent',()=>software.listTabs(id)),
    selectTab:(id,tab)=>privacy.read('fixture-agent',()=>software.selectTab(id,tab))};
  const playbook=createLoginPlaybook({browser:protectedBrowser,helper,checkpoints});
  const options={taskId,guideId,accountRef,organizationRef,
   scope:{person:'fixture-person',tenant:'fixture',account:accountRef,origin,source:'dummy-fixture',capabilities:['login']},expiresAt:Date.now()+60000};
  const pending=await playbook.advance(options);assert.equal(pending.reasonCode,'approval_required');
  await callAuthBroker(approver,'approve',{requestId:pending.requestId});
  if(process.env.AGENT_BROWSER_SMOKE_CHANGED_CONTROLS==='1') {
   await page.locator(controls.password).evaluate((element,qbo)=>{if(qbo)element.name='changed-control';else element.id='changed-control';},qbo);
   const changed=await playbook.advance(options);assert.equal(changed.status,'adaptation_required');
   assert.equal(changed.reasonCode,'prepared_controls_changed');assert.equal(changed.effectState,'no_effect');
   assert.equal(privateReads,0,'Changed prepared controls prevent even private credential retrieval');
   assert.equal((await helper('inspect_request',{requestId:pending.requestId})).inputAttemptedAt,undefined);
   await page.setContent(appHtml);
   console.log('Changed prepared controls prevented private retrieval and input using the original approved request');
   return;
  }
  const running=playbook.advance(options);
  if(process.env.AGENT_BROWSER_SMOKE_WRONG_CREDENTIAL==='1') {
   const rejected=await running;assert.equal(rejected.effectState,'unknown');assert.notEqual(rejected.status,'completed');
   const inspected=await helper('inspect_request',{requestId:pending.requestId});assert.equal(inspected.state,'uncertain');
   assert.equal(inspected.captureState,'held');assert.equal(privateReads,1);
   console.log('Independent website rejected a wrong private credential; no false login completion');
   return {negative:true};
  }
  if(process.env.AGENT_BROWSER_SMOKE_AUTH_SOURCE_LOCKED==='1') {
   const uncertain=await running;assert.equal(uncertain.effectState,'unknown');
   const locked=await playbook.advance(options);assert.equal(locked.reasonCode,'password_source_unavailable');assert.equal(locked.effectState,'no_effect');
   const inspected=await helper('inspect_request',{requestId:pending.requestId});assert.equal(inspected.inputAttemptedAt,undefined);
   assert.equal((await playbook.advance(options)).reasonCode,'password_source_unavailable');
   await page.setContent(appHtml);
   console.log('Actual helper-source locked dummy source projects actionable unavailable state with proven no private input');
   return;
  }
  await Promise.race([reachedChallenge,running.then(()=>{throw Error('Challenge was not reached');})]);
  await assert.rejects(()=>protectedBrowser.verifyLogin(options),error=>error.code==='auth_capture_held');
  const inspected=await helper('inspect_request',{requestId:pending.requestId});assert(inspected.inputAttemptedAt);
  // A new orchestrator inspecting the same durable checkpoint does not submit.
  const restart=createLoginPlaybook({browser:protectedBrowser,helper,checkpoints});
  const continued=await restart.advance(options);assert.equal(continued.reasonCode,'helper_running_or_uncertain');
  assert.equal(continued.actionId,pending.actionId,'Controller switch preserves action identity');
  await page.locator('#fixture-human-confirm').click();finishMfa();let completed=await running;
  if(process.env.AGENT_BROWSER_SMOKE_DROP_AUTH_ACK==='1'){
   assert.equal(completed.effectState,'unknown');completed=await restart.advance(options);
  }
  assert.equal(completed.status,'completed');
  assert.equal(completed.effectState,'verified');assert.equal(completed.taskId,taskId);
  assert.equal(await page.evaluate(()=>window.syntheticSubmissions),1);
  assert.equal(privateReads,1);
  assert(!JSON.stringify(completed).includes(privatePassword));
  assert.equal((await restart.advance(options)).status,'completed');
  console.log('Actual helper-source dummy private fill, human-MFA wait, attributed popup, capture hold and restart reconciliation passed');
 }finally{
  const closed=broker?.close(); // Shut down authority before unblocking a failed fixture.
  finishMfa?.();
  await closed;await browser.close();
 }
}
