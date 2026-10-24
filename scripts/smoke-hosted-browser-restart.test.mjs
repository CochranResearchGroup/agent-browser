import test from 'node:test';
import assert from 'node:assert/strict';
import {qualifyHostedBrowserRestart} from './smoke-hosted-browser-restart.mjs';
function fixture({unknownReopen=false,unchangedGeneration=false,unprovenAbsence=false}={}){
 const old={desktopId:'owned',generation:1},task={taskId:'t',browserId:'b',profileId:'p',tabId:'tab',host:{desktop:old}};
 let revision=0,state='connected',target=old,recoveryDesktop=null;const calls=[],saved=[];
 const workflow=async(name,args)=>{
  calls.push(args.operation??name);
  if(name==='browser_command')return {result:{url:'about:blank',body:''}};
  if(args.operation!=='status'){
   assert.equal(args.expectedRevision,revision);revision++;
   if(args.operation==='reconnect')state='browser_lost';
   if(args.operation==='rebind')recoveryDesktop=args.desktop;
   if(args.operation==='reopen'){
    if(unknownReopen)throw Error('unknown reopen');
    target=recoveryDesktop;recoveryDesktop=null;state='connected';
   }
  }
  return {host:{...task,desktop:target},reconnect:{revision,state,automaticReplay:false,recoveryDesktop,pending:false}};
 };
 return {task,workflow,syntheticOnly:true,calls,saved,
  observeProcess:async expected=>expected?{pidAbsent:!unprovenAbsence}:{alive:true,pid:10,startToken:state==='connected'&&target.generation===2?'new':'old',profilePath:'/owned/profile'},
  observePeers:async()=>({peer:'same-birth'}),save:async(_,value)=>saved.push(structuredClone(value)),
  restartNative:async()=>({state:'ready_for_explicit_rebind',before:{desktopId:'native',assignmentId:'a',viewingGeneration:1},after:{desktopId:'native',assignmentId:'a',viewingGeneration:unchangedGeneration?1:2},launchBinding:{readinessScope:'live_resource'},applicationLoss:{observed:'lost'},websiteEffectsReplayed:false})};
}
test('new Native generation remains recovery intent until one explicit blank reopen',async()=>{
 const f=fixture(),result=await qualifyHostedBrowserRestart(f);assert.equal(result.passed,true);
 assert.deepEqual(f.calls,['status','configure','reconnect','configure','rebind','reopen','browser_command']);
});
test('unknown reopen is not retried or followed by page effects',async()=>{
 const f=fixture({unknownReopen:true});await assert.rejects(qualifyHostedBrowserRestart(f),/unknown reopen/);
 assert.equal(f.calls.at(-1),'reopen');assert.equal(f.calls.filter(x=>x==='reopen').length,1);assert(!f.saved.some(x=>x.passed));
});
test('unchanged Native generation or unproven browser absence stops before rebind',async()=>{
 for(const options of [{unchangedGeneration:true},{unprovenAbsence:true}]){
  const f=fixture(options);await assert.rejects(qualifyHostedBrowserRestart(f));assert(!f.calls.includes('rebind'));assert(!f.calls.includes('reopen'));
 }
});
