import test from 'node:test';
import assert from 'node:assert/strict';
import {qualifyHostedBrowserReconnect} from './smoke-hosted-browser-reconnect.mjs';
function fixture({failReturn=false}={}){
 const calls=[],saved=[];let owner='agent',epoch=0,revision=0,policy='report_only',state='unverified',hold=false;
 const task={taskId:'task',browserId:'browser',profileId:'profile',tabId:'tab'};
 const snapshot=()=>({taskId:'task',host:{...task,controllerState:{owner}},reconnect:{revision,state,lossPolicy:policy,disconnectHold:hold,retained:hold,pending:false,automaticReplay:false}});
 const workflow=async(name,args)=>{
  calls.push({name,args});
  if(name==='browser_control'){
   if(args.operation==='takeover'){owner='human';epoch++;}
   if(args.operation==='return'){if(failReturn)throw Error('unknown return outcome');owner='agent';epoch++;}
   return {control:{owner,epoch}};
  }
  if(name==='browser_continue')return task;
  assert.equal(name,'browser_reconnect');
  if(args.operation!=='status'){assert.equal(args.expectedRevision,revision);revision++;if(args.operation==='disconnect'){state='disconnected';hold=true;}if(args.operation==='reconnect'){state='connected';hold=false;}if(args.operation==='configure')policy=args.lossPolicy;}
  return snapshot();
 };
 return {workflow,task,save:async(name,value)=>saved.push({name,value}),syntheticOnly:true,calls,saved};
}
test('hosted reconnect helper qualifies public transitions and measured metadata only',async()=>{
 const input=fixture(),result=await qualifyHostedBrowserReconnect(input);assert.equal(result.passed,true);assert.equal(result.warmStatusMs.length,12);assert.equal(result.actualBrowserLossExercised,false);assert.equal(result.actualDesktopRestartExercised,false);
 assert.equal(input.calls.filter(c=>c.args.operation==='return').length,1);assert.equal(input.calls.filter(c=>c.args.operation==='reopen'||c.args.operation==='rebind').length,0);assert.equal(input.saved.length,1);
});
test('unknown return outcome is retained without another dispatch',async()=>{
 const input=fixture({failReturn:true});await assert.rejects(qualifyHostedBrowserReconnect(input),/unknown return/);assert.equal(input.calls.filter(c=>c.args.operation==='return').length,1);assert.equal(input.saved[0].value.passed,false);
});
