import {mkdtemp,mkdir,writeFile,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,dirname} from 'node:path';
import {pathToFileURL} from 'node:url';
import {connectHostedAuthHelper} from '../../packages/client/src/hosted-auth-helper.js';

/** Actual Auth client/authority and parent hub/relay with synthetic external ports.
 * Never loads installed discovery or a live owner configuration. */
export async function createOwnerAuthFixture({source=process.env.AUTH_RECEIPTS_SOURCE,parentSource=process.env.RECEIPTS_SOURCE,sdkSource=process.env.AUTH_RECEIPTS_SDK_SOURCE,configure}={}){
 if(!source||!parentSource||!sdkSource)throw Error('Set reviewed AUTH_RECEIPTS_SOURCE, RECEIPTS_SOURCE and AUTH_RECEIPTS_SDK_SOURCE');
 const load=(root,file)=>import(pathToFileURL(join(root,file)).href);
 const [{startStandalone},{openCredentialService},{createReceiptsApprovalAdapter,credentialApprovalActionSets},{openRequestHub},{openNotificationRelay},{publishClientSdk}]=await Promise.all([
  load(source,'src/standalone.mjs'),load(source,'src/credential-service.mjs'),load(source,'src/receipts-approval.mjs'),
  load(parentSource,'apps/receipts-interoperation/src/request-hub.mjs'),load(parentSource,'apps/receipts-interoperation/src/notifications.mjs'),load(sdkSource,'src/client-sdk.mjs')]);
 const root=await mkdtemp(join(tmpdir(),'owner-playbook-'));
 const state={time:1800000000000,reads:0,inputs:0,notificationsReady:true,authenticated:false,sourceAccount:'owner@example.test',channelActor:'slack:owner',decisions:[],prompts:[],events:[],verificationAccount:'owner@example.test',verificationOrganization:'company'};
 const now=()=>state.time;
 let host,authority;
 const hub=openRequestHub({statePath:join(root,'hub.sqlite'),now,applications:{'auth-receipts':{route:'synthetic'}},routes:{synthetic:{destination:'synthetic-owner',adapter:{
  async install(){return {status:'installed'};},
  async send(body){state.prompts.push(body);return {status:'delivered'};},
  async receive({requestId}){return state.decisions.filter(item=>item.requestId===requestId);}
 }}}});
 for(const definition of credentialApprovalActionSets)await hub.installActionSet('auth-receipts',definition);
 const relay=await openNotificationRelay({directory:join(root,'relay'),ownerId:'owner',producerId:'auth-receipts',route:{baseUrl:'https://synthetic.invalid',workspace:'synthetic',channelRef:'synthetic-owner'},now,
  fetch:async(_url,options)=>{const body=JSON.parse(options.body);state.events.push(body);return Response.json(state.notificationsReady?{ok:true,action:{id:'sent-'+state.events.length,status:'sent'}}:{ok:false});}});
 const notifications={idempotentEvents:true,maxPurposeLength:240,async deliver(event){const accepted=await relay.accept(event);return state.notificationsReady?relay.deliver(event.eventId):accepted;}};
 const parent=createReceiptsApprovalAdapter({application:hub.forApplication('auth-receipts'),actors:{'slack:owner':'owner'},notifications,now});
 const receiving={taskId:'browser-task',profileId:'profile',browserId:'browser',tabId:'tab',generation:'generation-1'};
 const sourcePort={id:'synthetic',async discover(){return [{recordId:'synthetic:billing',origin:'https://billing.example',name:'Billing',account:'owner@example.test',modifiedAt:1}];},async read(_id,{authorize}){await authorize();state.reads++;return {username:state.sourceAccount,password:'SYNTHETIC-PASSWORD-ONLY'};}};
 const receiver={async validate(_principal,request){if(request.targetId!=='target')throw Object.assign(Error('Wrong synthetic target'),{code:'auth_not_authorized'});},async use({authorize}){await authorize();state.inputs++;state.authenticated=true;return {released:true,authenticated:true};},async reconcile(){return {released:state.inputs>0,authenticated:state.authenticated};}};
 // Only synthetic test hosts provide this callback. It supplies enrolled owner
 // ports before startup; it is never exposed by a production browser client.
 const ownerOptions=await configure?.({root,state,receiving,receiver,sourcePort,now})??{};
 const start=async()=>{
  host=await startStandalone({tenantOptions:{stateHome:join(root,'ipc')},auth:{now,principals:[{id:'client',role:'caller'}],resolveCaller:async(p,context)=>({taskId:context?.threadId??p.taskId}),credentialServiceFactory:context=>openCredentialService({...context,root:join(root,'authority'),now,approvers:['owner'],policyEditors:['owner'],sources:[sourcePort],parent,receiver,...ownerOptions}).then(value=>authority=value)}});
  const file=join(dirname(host.connections[0]),'external-discovery.json');
  await writeFile(file,JSON.stringify({contractVersion:2,transport:'codex-bots-auth',receivingProvider:'agent-browser',connectionFile:host.connections[0]}),{mode:0o600});return file;
 };
 const discoveryFile=await start(),candidate=join(root,'sdk');await publishClientSdk({destination:candidate});
 const oldThread=process.env.CODEX_THREAD_ID;process.env.CODEX_THREAD_ID='synthetic-caller';let auth;
 try{auth=await connectHostedAuthHelper({installationRoot:candidate,discoveryFile});}finally{if(oldThread===undefined)delete process.env.CODEX_THREAD_ID;else process.env.CODEX_THREAD_ID=oldThread;}
 const store=join(root,'checkpoints');await mkdir(store);
 const checkpoints={async load(key){try{return JSON.parse(await readFile(join(store,encodeURIComponent(key)+'.json'),'utf8'));}catch(error){if(error.code==='ENOENT')return null;throw error;}},async save(key,value){await writeFile(join(store,encodeURIComponent(key)+'.json'),JSON.stringify(value),{mode:0o600});}};
 const browser={async verifyLogin(){return {...receiving,status:state.authenticated&&state.verificationAccount==='owner@example.test'&&state.verificationOrganization==='company'?'completed':'waiting',reasonCode:state.authenticated?(state.verificationAccount==='owner@example.test'&&state.verificationOrganization==='company'?'authenticated':'identity_mismatch'):'password_required',effectState:'no_effect'};},async prepareLogin(){return {...receiving,status:'completed',plan:{guideDigest:'reviewed-guide',controlFingerprint:'reviewed-controls'}};},async validatePlan(){return {...receiving,status:'completed',reasonCode:'plan_current'};}};
 const options={...receiving,guideId:'billing',accountRef:'owner',organizationRef:'company',query:'billing.example',accountHint:'owner@example.test',origin:'https://billing.example',targetId:'target',purpose:'Synthetic owner workflow',expiresAt:state.time+60000,idempotencyKey:'synthetic-login'};
 return {root,state,auth,browser,checkpoints,options,receiver,
  respond(action='allow_once',{actor='slack:owner',payload={}}={}){const prompt=state.prompts.at(-1);if(!prompt)throw Error('No synthetic prompt');state.decisions.push({id:'decision-'+state.decisions.length,requestId:prompt.requestId,applicationId:prompt.applicationId,actionSet:prompt.actionSet,version:prompt.version,actor,action,payload});},
  ownerCall:(operation,args={})=>authority.call({role:'approver',id:'owner'},operation,args),
  async connectClient(taskId){const previous=process.env.CODEX_THREAD_ID;process.env.CODEX_THREAD_ID=taskId;try{return await connectHostedAuthHelper({installationRoot:candidate,discoveryFile});}finally{if(previous===undefined)delete process.env.CODEX_THREAD_ID;else process.env.CODEX_THREAD_ID=previous;}},
  async restart(){await host.close();await start();},
  async close(){await host?.close();relay.close();hub.close();await rm(root,{recursive:true,force:true});}
 };
}
