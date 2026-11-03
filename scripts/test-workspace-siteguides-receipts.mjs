import assert from 'node:assert/strict';import {mkdtemp,writeFile,rm}from'node:fs/promises';import{tmpdir}from'node:os';import{join}from'node:path';
import{readTeachingReceipt}from'../packages/client/src/workspace-siteguides-host.js';
const directory=await mkdtemp(join(tmpdir(),'teaching-receipts-'));
try{
 const scope={taskId:'task',rootGuideId:'guide',reviewedGuideId:'guide-r2',reviewedSha256:'a'.repeat(64),contentSha256:'b'.repeat(64)};
 const request={...scope,decisionRef:'review-1'},record={version:'codex-operator-confirmation.v1',status:'confirmed',confirmationId:'review-1',authenticatedUser:{username:'operator'},action:{kind:'operator_confirmation',request:{purpose:'teaching_review',...scope,decision:'approved',sourceUnavailableAcknowledged:true}}};
 const path=join(directory,'review-1.json');const save=()=>writeFile(path,JSON.stringify(record));await save();
 const receipt=await readTeachingReceipt({ledgerDirectory:directory,actor:'operator'},request,'teaching_review');assert.equal(receipt.actorRef,'dashboard:operator');assert.equal(receipt.provenance,'authenticated_operator');assert.equal(receipt.decisionId,'review-1');
 for(const patch of[{actor:'forged'}, {actor:'operator',ledgerDirectory:directory}]){if(patch.actor==='forged')await assert.rejects(readTeachingReceipt({ledgerDirectory:directory,...patch},request,'teaching_review'),/authority_mismatch/);}
 await assert.rejects(readTeachingReceipt({ledgerDirectory:directory,actor:'operator'},{...request,contentSha256:'c'.repeat(64)},'teaching_review'),/content_mismatch/);
 record.action.request.purpose='browser_action';await save();await assert.rejects(readTeachingReceipt({ledgerDirectory:directory,actor:'operator'},request,'teaching_review'),/authority_mismatch/);
 record.action.request.purpose='teaching_answer';record.action.request={...record.action.request,questionId:'refunds',questionSha256:'d'.repeat(64),answeredGuideId:'guide-r2',answeredSha256:'a'.repeat(64),answer:'Include refunds'};await save();
 const answerRequest={taskId:'task',rootGuideId:'guide',answeredGuideId:'guide-r2',answeredSha256:'a'.repeat(64),contentSha256:'b'.repeat(64),questionId:'refunds',questionSha256:'d'.repeat(64),answerRef:'review-1'};
 assert.equal((await readTeachingReceipt({ledgerDirectory:directory,actor:'operator'},answerRequest,'teaching_answer')).answer,'Include refunds');
 await assert.rejects(readTeachingReceipt({ledgerDirectory:directory,actor:'operator'},{...answerRequest,questionSha256:'e'.repeat(64)},'teaching_answer'),/content_mismatch/);
 console.log('Existing authenticated ledger: exact review/question binding, actual actor and dedicated purpose; browser confirmations and changed content rejected.');
}finally{await rm(directory,{recursive:true,force:true});}
