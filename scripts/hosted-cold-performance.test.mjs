import test from 'node:test';
import assert from 'node:assert/strict';
import {createColdFirstPixelClock,summarizeColdFirstPixels} from './hosted-performance.mjs';

const runId='p279-synthetic-run';
const pairDigest='a'.repeat(64);
function sample(durationMs,index,{outcome='validated_response',run=runId,pair=pairDigest}={}){
 let now=100;
 const pixels=createColdFirstPixelClock({now:()=>now});
 now+=durationMs;
 if(outcome==='validated_response')assert.equal(pixels.observe({canvas:{width:1440,height:960,markers:63},receivedWebSocketFrames:12}),true);
 return {id:`sample-${index}`,contextId:`fresh-context-${index}`,runId:run,pairDigest:pair,outcome,pixels:pixels.receipt()};
}

test('five fresh same-pair cold contexts retain their receipts and qualify a distribution without a latency target',()=>{
 const samples=[1000,2000,3000,4000,5000].map((duration,index)=>sample(duration,index));
 const summary=summarizeColdFirstPixels(samples,{runId,pairDigest,expectedSamples:5});
 assert.equal(summary.complete,true);
 assert.equal(summary.attempted,5);
 assert.equal(summary.validated,5);
 assert.equal(summary.failedOrUnknown,0);
 assert.equal(summary.p95Ms,5000);
 assert.equal(summary.medianMs,3000);
 assert.equal(summary.targetMs,null);
 assert.equal(summary.targetMet,null);
 assert.equal(summary.distributionClaim,true);
 assert.deepEqual(summary.samples,samples,'Every original context and cold-pixel receipt remains in the report');
});

test('a failed cold attempt consumes its declared slot and remains visible without replacement',()=>{
 const samples=[1000,2000,3000,4000,5000].map((duration,index)=>sample(duration,index,index===1?{outcome:'failed_or_unknown'}:{}));
 const summary=summarizeColdFirstPixels(samples,{runId,pairDigest,expectedSamples:5});
 assert.equal(summary.attempted,5);
 assert.equal(summary.validated,4);
 assert.equal(summary.failedOrUnknown,1);
 assert.equal(summary.complete,false);
 assert.equal(summary.distributionClaim,false);
 assert.equal(summary.targetMs,null);
 assert.equal(summary.targetMet,null);
 assert.deepEqual(summary.samples,samples,'Failure receipts are not filtered out or replaced by another context');
 assert.equal(summary.samples[1].id,'sample-1');
 assert.equal(summary.samples[1].outcome,'failed_or_unknown');
 assert.equal(summary.samples[1].pixels.validated,false);
});

for(const [field,foreign] of [['pairDigest','b'.repeat(64)],['runId','another-run']]){
 test(`a cold distribution refuses a sample from another ${field}`,()=>{
  const samples=[1000,2000,3000,4000,5000].map((duration,index)=>sample(duration,index));
  samples[2][field]=foreign;
  assert.throws(()=>summarizeColdFirstPixels(samples,{runId,pairDigest,expectedSamples:5}),`Mixed ${field} must not become a distribution`);
 });
}


for(const field of ['id','contextId']){
 test(`a cold distribution refuses duplicate ${field} rather than counting reused evidence`,()=>{
  const samples=[1000,2000,3000,4000,5000].map((duration,index)=>sample(duration,index));
  samples[3][field]=samples[0][field];
  assert.throws(()=>summarizeColdFirstPixels(samples,{runId,pairDigest,expectedSamples:5}),`Duplicate ${field} must not be counted as a new cold attempt`);
 });
}

for(const replaceFailure of [false,true]){
 test(`the declared five-attempt budget refuses a sixth context${replaceFailure?' even when it would replace a failed slot':''}`,()=>{
  const samples=[1000,2000,3000,4000,5000,6000].map((duration,index)=>sample(duration,index,replaceFailure&&index===1?{outcome:'failed_or_unknown'}:{}));
  assert.throws(()=>summarizeColdFirstPixels(samples,{runId,pairDigest,expectedSamples:5}),'Extra contexts cannot improve a predeclared cold distribution');
 });
}

test('four retained cold attempts remain an incomplete observation set rather than a distribution',()=>{
 const samples=[1000,2000,3000,4000].map((duration,index)=>sample(duration,index));
 const summary=summarizeColdFirstPixels(samples,{runId,pairDigest,expectedSamples:5});
 assert.equal(summary.attempted,4);
 assert.equal(summary.validated,4);
 assert.equal(summary.failedOrUnknown,0);
 assert.equal(summary.complete,false);
 assert.equal(summary.distributionClaim,false);
 assert.equal(summary.targetMs,null);
 assert.equal(summary.targetMet,null);
 assert.deepEqual(summary.samples,samples,'An incomplete receipt keeps all original attempts without inventing missing contexts');
});

test('a validated cold receipt without the Native frames required by its clock is refused',()=>{
 const samples=[1000,2000,3000,4000,5000].map((duration,index)=>sample(duration,index));
 samples[2].pixels.receivedWebSocketFrames=0;
 assert.equal(samples[2].pixels.validated,true);
 assert.equal(samples[2].pixels.sampleCount,1);
 assert.throws(()=>summarizeColdFirstPixels(samples,{runId,pairDigest,expectedSamples:5}),'A success flag cannot replace the Native frame evidence required by the public clock');
});
