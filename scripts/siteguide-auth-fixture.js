import assert from 'node:assert/strict';
import {readFileSync,readdirSync,writeFileSync,existsSync,renameSync,mkdirSync} from 'node:fs';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {chromium} from 'playwright';
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
  await page.evaluate(submit=>document.querySelector(submit).onclick=()=>{
   window.syntheticSubmissions=(window.syntheticSubmissions??0)+1;
   document.body.innerHTML='<div id="mfa"><input autocomplete="one-time-code">Waiting for synthetic person</div>';
  },controls.submit);
  assert.equal((await software.verifyLogin({taskId,guideId,accountRef,organizationRef})).reasonCode,identifierStart?'identifier_required':'password_required');
  let destination=page;
  const receivingPage={url:()=>destination.url(),locator:selector=>destination.locator(selector),
    evaluate:(...args)=>destination.evaluate(...args)};
  const receiver=createBrowserLoginReceiver({page:receivingPage,account:accountRef,origin,
   usernameSelector:controls.identifier,passwordSelector:controls.password,submitSelector:controls.submit,
   scopeSelector:controls.accountScope,verifiedSelector:guide.authenticatedSelector,expectedUsername:accountRef});
  const baseApply=receiver.apply;
  let challenge;
  const reachedChallenge=new Promise(resolve=>challenge=resolve);
  let finishMfa;
  const humanMfa=new Promise(resolve=>finishMfa=resolve);
  receiver.apply=async(credential,scope,authorize)=>{
   if(identifierStart) {
    await authorize();await page.locator(controls.identifier).fill(credential.username);
    await authorize();await page.locator(controls.identifierSubmit).click();
   }
   await baseApply(credential,scope,authorize);challenge();
   await humanMfa;await authorize();
   await page.setContent('<p>Login continued in password-manager destination</p>');
   const popup=page.waitForEvent('popup');
   await page.evaluate(url=>window.open(url,'fixture-auth-destination'),origin+'/login-destination');
   destination=await popup;await destination.waitForLoadState('domcontentloaded');
   await destination.setContent(appHtml);
  };
  broker=await startAuthBroker({root:join(privateRoot,'broker'),
   principals:[{role:'caller',id:'fixture-agent',taskId},{role:'approver',id:'fixture-person'}],
   accounts:[{person:'fixture-person',tenant:'fixture',account:accountRef,origin,source:'dummy-fixture',callers:['fixture-agent'],approvers:['fixture-person']}],
   privacy,receiver,store:{read:async(_source,{authorize})=>{await authorize();
    if(process.env.AGENT_BROWSER_SMOKE_AUTH_SOURCE_LOCKED==='1')throw Object.assign(Error('Synthetic password manager is locked'),{code:'auth_store_unavailable'});
    return {username:accountRef,password:'dummy-not-a-real-credential'};}}});
  const [caller,approver]=broker.connections.map(file=>JSON.parse(readFileSync(file,'utf8')));
  const checkpoints={
   load:async key=>{const file=join(privateRoot,'checkpoint.json');return existsSync(file)?JSON.parse(readFileSync(file,'utf8'))[key]??null:null;},
   save:async(key,value)=>{const file=join(privateRoot,'checkpoint.json'),temporary=file+'.tmp';writeFileSync(temporary,JSON.stringify({[key]:value}),{mode:0o600});renameSync(temporary,file);},
  };
  const helper=(operation,args)=>callAuthBroker(caller,operation,args);
  const protectedBrowser={verifyLogin:options=>privacy.read('fixture-agent',()=>software.verifyLogin(options)),
    listTabs:id=>privacy.read('fixture-agent',()=>software.listTabs(id)),
    selectTab:(id,tab)=>privacy.read('fixture-agent',()=>software.selectTab(id,tab))};
  const playbook=createLoginPlaybook({browser:protectedBrowser,helper,checkpoints});
  const options={taskId,guideId,accountRef,organizationRef,
   scope:{person:'fixture-person',tenant:'fixture',account:accountRef,origin,source:'dummy-fixture',capabilities:['login']},expiresAt:Date.now()+60000};
  const pending=await playbook.advance(options);assert.equal(pending.reasonCode,'approval_required');
  await callAuthBroker(approver,'approve',{requestId:pending.requestId});
  const running=playbook.advance(options);
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
  assert.equal((await restart.advance(options)).reasonCode,'helper_running_or_uncertain');
  finishMfa();const completed=await running;assert.equal(completed.status,'completed');
  assert.equal(completed.effectState,'verified');assert.equal(completed.taskId,taskId);
  assert.equal(await page.evaluate(()=>window.syntheticSubmissions),1);
  assert.equal((await restart.advance(options)).status,'completed');
  console.log('Actual helper-source dummy private fill, human-MFA wait, attributed popup, capture hold and restart reconciliation passed');
 }finally{await broker?.close();await browser.close();}
}
