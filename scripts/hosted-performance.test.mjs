import test from 'node:test';
import assert from 'node:assert/strict';
import {createLatencyWorkload,summarizeLatency,createColdFirstPixelClock,qualifyControlTransitions} from './hosted-performance.mjs';
const task={taskId:'task',browserId:'browser',profileId:'profile',tabId:'tab'};
function fixture({failTake=false,failReturn=false,failReadAfterTake=false}={}){
 const calls=[],saved=[];let epoch=0,owner='agent',nativeOwner='agent',tick=0,reads=0;
 return {task,pairs:3,kind:'consumer_native_coordination',now:()=>{tick+=10;return tick;},observeTask:async()=>task,
  status:async()=>{reads++;if(failReadAfterTake&&reads===2)throw Error('read failed');return {control:{owner,epoch},native:{owner:nativeOwner,state:'ready'}};},
  change:async args=>{calls.push(args);if(args.operation==='takeover'&&failTake)throw Error('unknown takeover');if(args.operation==='return'&&failReturn)throw Error('unknown return');assert.equal(args.expectedEpoch,epoch);epoch++;owner=args.operation==='takeover'?'human':'agent';nativeOwner=owner==='human'?'operator':'agent';return {control:{owner,epoch},native:{owner:nativeOwner,state:'ready'}};},
  save:async(name,value)=>saved.push({name,value:structuredClone(value)}),calls,saved};
}
test('nearest-rank p95 uses actual validated samples and strict target',()=>{
 const samples=Array.from({length:20},(_,i)=>({operation:'status',startedMs:0,finishedMs:i+1,durationMs:i+1,outcome:'validated_response'}));
 assert.equal(summarizeLatency(samples,{workload:'status'}).p95Ms,19);
 assert.equal(summarizeLatency(samples,{workload:'status',targetMs:19}).targetMet,false);
 assert.equal(summarizeLatency([],{workload:'status'}).targetMet,false);
 assert.equal(summarizeLatency([...samples,{operation:'status',startedMs:0,finishedMs:1,durationMs:1,outcome:'failed_or_unknown'}],{workload:'status'}).targetMet,false);
});
test('clock measures decoded response boundary and retains failed validation',async()=>{
 let clock=0;const timing=createLatencyWorkload({workload:'test',now:()=>clock});
 await timing.measure('status',async()=>{clock=12;return 1;},()=>{clock=100;});
 await assert.rejects(timing.measure('return',async()=>{clock=110;return 1;},()=>{throw Error('wrong identity');}),/wrong identity/);
 const receipt=timing.receipt();assert.equal(receipt.samples[0].durationMs,12);assert.equal(receipt.samples[1].durationMs,10);assert.equal(receipt.samples[1].outcome,'response_evidence_failed');assert.equal(receipt.targetMet,false);
});
test('Native coordinated workload uses fresh epochs and exact bounded operation IDs',async()=>{
 const input=fixture(),result=await qualifyControlTransitions(input);assert.equal(result.passed,true);assert.equal(result.completedPairs,3);assert.equal(result.latency.attempted,6);assert.equal(result.latency.byOperation.takeover.validated,3);assert.equal(result.latency.byOperation.return.validated,3);assert.equal(result.latency.p95Ms,10);assert.equal(result.includesNativeToolbarClick,false);assert.equal(result.includesNativeCoordination,true);
 assert.equal(new Set(input.calls.map(call=>call.operationId)).size,6);assert.deepEqual(input.calls.map(call=>call.expectedEpoch),[0,1,2,3,4,5]);assert(input.saved.some(row=>row.value.transitions.some(t=>t.phase==='dispatch_intent')));
});
test('unknown Native takeover never triggers automatic return',async()=>{
 const input=fixture({failTake:true});await assert.rejects(qualifyControlTransitions(input),/unknown takeover/);assert.deepEqual(input.calls.map(c=>c.operation),['takeover']);assert.equal(input.saved.at(-1).value.passed,false);assert.equal(input.saved.at(-1).value.transitions[0].phase,'failed_or_unknown_no_replay');
});
test('unknown Native return is never dispatched twice',async()=>{
 const input=fixture({failReturn:true});await assert.rejects(qualifyControlTransitions(input),/unknown return/);assert.deepEqual(input.calls.map(c=>c.operation),['takeover','return']);
});
test('known successful takeover gets one deliberate cleanup return after a read failure',async()=>{
 const input=fixture({failReadAfterTake:true});await assert.rejects(qualifyControlTransitions(input),/read failed/);assert.deepEqual(input.calls.map(c=>c.operation),['takeover','return']);assert.equal(input.saved.at(-1).value.cleanupReturn,'validated_response');assert.equal(input.saved.at(-1).value.transitions.at(-1).cleanup,true);
});
test('cold first-pixel clock requires markers and actual frames, preserves first sample',()=>{
 let now=100;const clock=createColdFirstPixelClock({now:()=>now});now=200;
 assert.equal(clock.observe({canvas:{width:1440,height:960,markers:63},receivedWebSocketFrames:0}),false);assert.equal(clock.receipt().validated,false);
 assert.equal(clock.observe({canvas:{width:1440,height:960,markers:0},receivedWebSocketFrames:12}),false);
 now=450;assert.equal(clock.observe({canvas:{width:1440,height:960,markers:63},receivedWebSocketFrames:14}),true);now=900;clock.observe({canvas:{width:1440,height:960,markers:64},receivedWebSocketFrames:30});
 const receipt=clock.receipt();assert.equal(receipt.durationMs,350);assert.equal(receipt.canvas.markers,63);assert.equal(receipt.receivedWebSocketFrames,14);assert.equal(receipt.sampleCount,1);assert.equal(receipt.distributionClaim,false);assert.equal(receipt.targetMs,null);
});
