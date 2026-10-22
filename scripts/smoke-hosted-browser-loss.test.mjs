import test from 'node:test';
import assert from 'node:assert/strict';
import {qualifyHostedBrowserLoss} from './smoke-hosted-browser-loss.mjs';

function fixture({unknownReopen=false,absenceUnproven=false}={}){
 const task={taskId:'fresh-task',browserId:'fresh-browser',profileId:'fresh-profile',tabId:'fresh-tab'},desktop={desktopId:'owned-assignment',generation:7};
 const calls=[],saved=[],kills=[];let revision=0,state='connected',lossPolicy='report_only',recoveryDesktop=null,replaced=false;
 const original={pid:1234,startToken:'linux:fixture:10',uid:1000,executablePath:'/fixture/chrome',profilePath:'/fixture/profile',freshOwned:true,custody:'managed',alive:true};
 const workflow=async(name,args)=>{
  calls.push({name,args:structuredClone(args)});
  if(name==='browser_command'){
   if(args.params.script.includes('textContent.includes'))return {result:true};
   if(args.params.script.includes('=1'))return {result:1};
   assert(replaced,'Only inspect blank page after a qualified replacement');return {result:{url:'about:blank',oldMarker:'undefined',body:''}};
  }
  assert.equal(name,'browser_reconnect');
  if(args.operation!=='status'){
   assert.equal(args.expectedRevision,revision);assert.equal(typeof args.requestId,'string');
   if(args.operation==='configure')lossPolicy=args.lossPolicy;
   if(args.operation==='reconnect')state='browser_lost';
   if(args.operation==='rebind'){assert.equal(state,'browser_lost');recoveryDesktop=args.desktop;}
   if(args.operation==='reopen'){
    if(unknownReopen)throw Object.assign(Error('Unknown reopen result'),{code:'transport_unknown'});
    assert.equal(lossPolicy,'reopen');assert.deepEqual(recoveryDesktop,desktop);replaced=true;state='connected';recoveryDesktop=null;
   }
   revision++;
  }
  return {host:{...task,desktop},reconnect:{revision,state,lossPolicy,recoveryDesktop,pending:false,automaticReplay:false}};
 };
 return {task,desktop,workflow,syntheticOnly:true,calls,saved,kills,
  observeProcess:async({expected})=>expected?{...original,alive:false,pidAbsent:!absenceUnproven}:{...original,...(replaced?{pid:1235,startToken:'linux:fixture:20'}:{})},
  terminateOwnedBrowser:async proof=>{kills.push(structuredClone(proof));return {pid:proof.pid,startToken:proof.startToken,originalGenerationAbsent:true};},
  observeDesktop:async()=>({...desktop,state:'active'}),observePeers:async()=>({baselinePid:999,startToken:'baseline-generation'}),
  save:async(_,receipt)=>saved.push(structuredClone(receipt))};
}

test('loss oracle qualifies report-only loss, exact recovery intent and one blank managed replacement',async()=>{
 const context=fixture();const result=await qualifyHostedBrowserLoss(context);
 assert.equal(result.passed,true);assert.equal(result.checks.length,5);
 assert.equal(result.desktopRestart.state,'not_exercised');assert.equal(result.physicalRelocation,'not_exercised');
 assert.equal(context.kills.length,1);assert.deepEqual(context.calls.filter(call=>call.name==='browser_reconnect').map(call=>call.args.operation),['status','configure','reconnect','configure','rebind','reopen']);
 assert.equal(context.saved.at(-1).passed,true);
});

test('unknown reopen result stops without retry, page observation or fabricated success',async()=>{
 const context=fixture({unknownReopen:true});await assert.rejects(qualifyHostedBrowserLoss(context),error=>error.code==='transport_unknown');
 const reopen=context.calls.findIndex(call=>call.args.operation==='reopen');assert(reopen>0);assert.equal(reopen,context.calls.length-1);
 assert.equal(context.calls.filter(call=>call.args.operation==='reopen').length,1);assert(!context.saved.some(receipt=>receipt.passed));
});

test('PID reuse or unproven original absence stops before loss observation and recovery',async()=>{
 const context=fixture({absenceUnproven:true});await assert.rejects(qualifyHostedBrowserLoss(context),/PID reuse or inaccessible process/);
 assert.equal(context.kills.length,1);assert(!context.calls.some(call=>['reconnect','rebind','reopen'].includes(call.args.operation)));assert(!context.saved.some(receipt=>receipt.passed));
});
