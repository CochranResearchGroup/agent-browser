/** Synthetic hosted acceptance measurements. No runtime starts or global effects.
 * All durations use one monotonic process clock. Native SDK coordination, public
 * Agent Browser responses, and cold viewer pixels are separate workload axes.
 */
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {performance} from 'node:perf_hooks';

const defaultNow=()=>performance.now();
export const HOSTED_CLOCK='node:perf_hooks.performance.now (monotonic milliseconds)';
function finiteTime(value){assert(Number.isFinite(value)&&value>=0,'Clock sample must be finite and nonnegative');return value;}
function percentile(values,fraction){if(!values.length)return null;const sorted=[...values].sort((a,b)=>a-b);return sorted[Math.ceil(sorted.length*fraction)-1];}
export function summarizeLatency(samples,{workload,targetMs=250,clock=HOSTED_CLOCK}={}){
 assert(typeof workload==='string'&&workload.length>0);assert(Number.isFinite(targetMs)&&targetMs>0);
 const successful=samples.filter(sample=>sample.outcome==='validated_response');
 for(const sample of samples){finiteTime(sample.startedMs);finiteTime(sample.finishedMs);assert(sample.finishedMs>=sample.startedMs);assert.equal(sample.durationMs,sample.finishedMs-sample.startedMs);}
 const durations=successful.map(sample=>sample.durationMs),p95Ms=percentile(durations,.95);
 return {workload,clock,unit:'milliseconds',quantileMethod:'nearest-rank ceil(n * 0.95)',targetMs,targetComparison:'p95 < target',
  attempted:samples.length,validated:successful.length,failedOrUnknown:samples.length-successful.length,
  minMs:durations.length?Math.min(...durations):null,medianMs:percentile(durations,.5),p95Ms,maxMs:durations.length?Math.max(...durations):null,
  targetMet:p95Ms!==null&&p95Ms<targetMs&&successful.length===samples.length,samples:samples.map(sample=>({...sample}))};
}
export function createLatencyWorkload({workload,targetMs=250,now=defaultNow}={}){
 const clock=now===defaultNow?HOSTED_CLOCK:'caller-supplied monotonic millisecond clock';
 const samples=[];
 return {
  async measure(operation,dispatch,validate=()=>{}){
   const startedMs=finiteTime(now());let finishedMs,decoded=false;
   try{const result=await dispatch();finishedMs=finiteTime(now());assert(finishedMs>=startedMs,'Monotonic clock moved backwards');decoded=true;await validate(result);samples.push({index:samples.length,operation,startedMs,finishedMs,durationMs:finishedMs-startedMs,outcome:'validated_response'});return result;}
   catch(error){finishedMs??=finiteTime(now());assert(finishedMs>=startedMs,'Monotonic clock moved backwards');samples.push({index:samples.length,operation,startedMs,finishedMs,durationMs:finishedMs-startedMs,outcome:decoded?'response_evidence_failed':'failed_or_unknown'});throw error;}
  },
  receipt(){return {...summarizeLatency(samples,{workload,targetMs,clock}),clockOriginUnixMs:now===defaultNow?performance.timeOrigin:null,byOperation:Object.fromEntries([...new Set(samples.map(sample=>sample.operation))].map(operation=>{const {samples:raw,...summary}=summarizeLatency(samples.filter(sample=>sample.operation===operation),{workload:workload+':'+operation,targetMs,clock});return [operation,summary];})),acknowledgmentBoundary:'dispatch to decoded API response; post-response evidence checks excluded'};},
 };
}
const identity=value=>Object.fromEntries(['taskId','browserId','profileId','tabId'].map(key=>[key,value?.[key]]));
function validateControl(value,owner,kind,expectedIdentity){
 assert.equal(value?.control?.owner,owner);assert(Number.isSafeInteger(value.control.epoch)&&value.control.epoch>=0);
 if(expectedIdentity&&kind==='agent_browser_public_control'){assert.equal(value.taskId,expectedIdentity.taskId);assert.equal(value.browserId,expectedIdentity.browserId);}
 if(kind==='consumer_native_coordination'){
  assert.equal(value?.native?.owner,owner==='human'?'operator':'agent','Native and Agent Browser ownership must agree');
  if(owner==='human')assert.equal(value.native.state,'ready');
 }
}
/** Repeated effects require explicit callback ownership from the outer fixture.
 * Every mutation uses a fresh immediately observed epoch and one operation ID.
 * A failed/unknown effect is retained and never dispatched a second time.
 */
export async function qualifyControlTransitions({status,change,observeTask,task,pairs=10,kind='agent_browser_public_control',save=async()=>{},now=defaultNow}){
 assert(['agent_browser_public_control','consumer_native_coordination'].includes(kind));
 assert(Number.isSafeInteger(pairs)&&pairs>0&&pairs<=10);
 if(kind==='consumer_native_coordination')assert(pairs>=3,'Native coordination needs at least three bounded pairs');
 assert.equal(typeof status,'function');assert.equal(typeof change,'function');assert.equal(typeof observeTask,'function');
 const original=identity(await observeTask());for(const value of Object.values(original))assert.equal(typeof value,'string');assert.equal(original.taskId,task.taskId);
 const timings=createLatencyWorkload({workload:kind,targetMs:250,now});
 const receipt={version:1,kind,requestedPairs:pairs,completedPairs:0,transitions:[],passed:false,
  includesNativeCoordination:kind==='consumer_native_coordination',includesNativeToolbarClick:false,includesViewerFocusCompletion:false,
  excludes:['cold viewer navigation and first pixels','browser/UI pointer-to-toolbar interaction','focus completion'],identity:original};
 const file=`hosted-control-performance-${kind.replaceAll('_','-')}.json`;
 const persist=async()=>{receipt.latency=timings.receipt();await save(file,receipt);};
 let human=null,returnAttempted=false;
 async function transition(operation,current,cleanup=false){
  const operationId=randomUUID(),intent={operation,operationId,expectedEpoch:current.control.epoch,cleanup,phase:'dispatch_intent'};
  receipt.transitions.push(intent);await persist();
  if(operation==='return')returnAttempted=true;
  let value;
  try{
   value=await timings.measure(cleanup?'cleanup_return':operation,()=>change({operation,operationId,expectedEpoch:current.control.epoch,actor:'synthetic hosted performance fixture',reason:cleanup?'Finish a known successful performance takeover':'Measure bounded warm control response',pauseReads:false}),result=>{
    validateControl(result,operation==='takeover'?'human':'agent',kind,original);assert.equal(result.control.epoch,current.control.epoch+1);
   });
  }catch(error){intent.phase='failed_or_unknown_no_replay';await persist();throw error;}
  intent.phase='validated_response';intent.resultEpoch=value.control.epoch;
  if(operation==='takeover'){human=value;returnAttempted=false;}else human=null;
  await persist();return value;
 }
 try{
  for(let pair=0;pair<pairs;pair++){
   const before=await status();validateControl(before,'agent',kind,original);
   human=await transition('takeover',before);
   const fresh=await status();validateControl(fresh,'human',kind,original);assert.equal(fresh.control.epoch,human.control.epoch,'Control changed between paired transitions');
   const returned=await transition('return',fresh);human=null;
   assert.deepEqual(identity(await observeTask()),original,'Warm control must preserve the exact task/browser/profile/tab');
   const after=await status();validateControl(after,'agent',kind,original);assert.equal(after.control.epoch,returned.control.epoch);
   receipt.completedPairs++;
  }
  receipt.passed=timings.receipt().targetMet;await persist();return receipt;
 }catch(error){
  // Cleanup is a new deliberate return only after a known successful takeover.
  // If a return was already dispatched, its uncertain effect is never repeated.
  if(human&&!returnAttempted){try{const fresh=await status();validateControl(fresh,'human',kind,original);assert.equal(fresh.control.epoch,human.control.epoch);await transition('return',fresh,true);receipt.cleanupReturn='validated_response';}catch{receipt.cleanupReturn='failed_or_unknown_no_replay';}}
  receipt.failureCode=String(error?.code??error?.name??'Error');await persist();throw error;
 }
}
/** Start immediately before navigating to the durable app handoff. The caller
 * must supply samples from real canvas pixel inspection and real Native frames.
 * This measures one navigation, not a distribution or an authentication bypass.
 */
export function createColdFirstPixelClock({now=defaultNow,minimumMarkers=8,minimumWidth=100,minimumHeight=100,minimumWebSocketFrames=1}={}){
 for(const number of [minimumMarkers,minimumWidth,minimumHeight,minimumWebSocketFrames])assert(Number.isSafeInteger(number)&&number>0);
 const startedMs=finiteTime(now());let first=null;const clock=now===defaultNow?HOSTED_CLOCK:'caller-supplied monotonic millisecond clock';
 return {
  observe({canvas,receivedWebSocketFrames}){
   if(first)return true;
   if(!canvas||!Number.isSafeInteger(canvas.width)||canvas.width<minimumWidth||!Number.isSafeInteger(canvas.height)||canvas.height<minimumHeight||!Number.isSafeInteger(canvas.markers)||canvas.markers<minimumMarkers||!Number.isSafeInteger(receivedWebSocketFrames)||receivedWebSocketFrames<minimumWebSocketFrames)return false;
   const firstPixelMs=finiteTime(now());assert(firstPixelMs>=startedMs);first={firstPixelMs,durationMs:firstPixelMs-startedMs,canvas:{width:canvas.width,height:canvas.height,markers:canvas.markers},receivedWebSocketFrames};return true;
  },
  receipt(){return {version:1,workload:'cold_native_viewer_first_pixels',clock,clockOriginUnixMs:now===defaultNow?performance.timeOrigin:null,unit:'milliseconds',sampleCount:first?1:0,
   boundary:'durable app handoff navigation to first observed synthetic Native canvas markers plus Native WebSocket frames',startedMs,
   markerThreshold:{minimumMarkers,minimumWidth,minimumHeight,minimumWebSocketFrames},validated:first!==null,...(first??{}),targetMs:null,targetMet:null,
   authentication:'existing caller-provided state',distributionClaim:false};},
 };
}

/** Summarize the predeclared cold attempts separately from warm API latency.
 * This validates receipts; the creating live harness remains responsible for
 * actual fresh contexts, Native frames and independently inspected pixels. */
export function summarizeColdFirstPixels(samples,{runId,pairDigest,expectedSamples=5}={}){
 assert(Array.isArray(samples));assert(Number.isSafeInteger(expectedSamples)&&expectedSamples>0);
 assert(samples.length<=expectedSamples,'Cold attempts exceed the predeclared sample budget');
 assert(typeof runId==='string'&&runId.length>0);assert(/^[a-f0-9]{64}$/.test(pairDigest));
 const ids=new Set(),contexts=new Set();
 for(const sample of samples){
  assert.equal(sample.runId,runId,'Cold sample belongs to another run');assert.equal(sample.pairDigest,pairDigest,'Cold sample belongs to another source pair');
  assert(typeof sample.id==='string'&&sample.id.length>0);assert(typeof sample.contextId==='string'&&sample.contextId.length>0);
  assert(!ids.has(sample.id),'Cold sample ID was reused');assert(!contexts.has(sample.contextId),'Cold browser context was reused');ids.add(sample.id);contexts.add(sample.contextId);
 }
 const validated=samples.filter(sample=>sample.outcome==='validated_response'&&sample.pixels?.validated===true);
 const durations=validated.map(sample=>{
  const pixels=sample.pixels;assert.equal(pixels.sampleCount,1);
  assert.deepEqual(Object.keys(pixels.markerThreshold??{}).sort(),['minimumHeight','minimumMarkers','minimumWebSocketFrames','minimumWidth']);
  let instant=finiteTime(pixels.startedMs);const check=createColdFirstPixelClock({...pixels.markerThreshold,now:()=>instant});instant=finiteTime(pixels.firstPixelMs);
  assert(check.observe({canvas:pixels.canvas,receivedWebSocketFrames:pixels.receivedWebSocketFrames}),'Cold receipt lacks validated Native frames and canvas markers');
  assert.equal(pixels.durationMs,instant-pixels.startedMs,'Cold receipt duration differs from its monotonic boundary');
  return finiteTime(pixels.durationMs);
 });
 const complete=samples.length===expectedSamples&&validated.length===expectedSamples;
 return {version:1,workload:'cold_native_viewer_first_pixels',runId,pairDigest,
  expectedSamples,attempted:samples.length,validated:validated.length,failedOrUnknown:samples.length-validated.length,
  unit:'milliseconds',quantileMethod:'nearest-rank ceil(n * 0.95)',
  minMs:durations.length?Math.min(...durations):null,medianMs:percentile(durations,.5),
  p95Ms:percentile(durations,.95),maxMs:durations.length?Math.max(...durations):null,
  complete,distributionClaim:complete,targetMs:null,targetMet:null,samples:structuredClone(samples)};
}
