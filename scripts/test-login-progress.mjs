import assert from 'node:assert/strict';
import {test} from 'node:test';
import {summarizeLoginProgress} from '../packages/client/src/hosted-auth-helper.js';

test('review without a request is selection work, not a delivered user decision', () => {
  const result = summarizeLoginProgress({loginId:'login-1',state:'owner_review'});
  assert.equal(result.category, 'credential_selection_required');
  assert.equal(result.waitingForUser, false);
  assert.equal(result.nextAction.operation, 'inspect_credential_login');
  assert.deepEqual(result.nextAction.args, {loginId:'login-1'});
});

test('operator metadata points to observation and never claims human activity', () => {
  const result=summarizeLoginProgress({loginId:'login-1',control:{owner:'operator',epoch:1}});
  assert.equal(result.category,'control_metadata');
  assert.equal(result.component,'remote-view');
  assert.equal(result.waitingForUser,false);
  assert.equal(result.nextAction.operation,'inspect_credential_login');
  assert.equal('humanActivity' in result,false);
});

test('progress contains no raw private payload and input uncertainty stays incomplete', () => {
  const result=summarizeLoginProgress({request:{requestId:'request-1',state:'unknown',attemptId:'attempt-1',password:'SECRET'},page:'PRIVATE',cookies:'COOKIE'});
  assert.equal(result.category,'input_outcome_unknown');
  assert.equal(result.outcome,'incomplete');
  assert.equal(JSON.stringify(result).includes('SECRET'),false);
  assert.equal(JSON.stringify(result).includes('PRIVATE'),false);
  assert.equal(JSON.stringify(result).includes('COOKIE'),false);
});

test('approval creation, transport, delivery, expiry and denial stay distinct', () => {
  for (const [state, submission, category] of [
    ['pending',undefined,'approval_not_submitted'],
    ['pending','unknown','approval_submission_unknown'],
    ['pending','accepted','approval_submitted'],
    ['pending','receiving_target_unavailable','receiving_target_unavailable'],
    ['expired','accepted','request_expired'],
    ['denied','accepted','authority_denied'],
  ]) {
    const result=summarizeLoginProgress({loginId:'login-1',request:{requestId:'request-1',state,approvalSubmission:submission}});
    assert.equal(result.category,category);
    assert.equal(result.waitingForUser,false);
  }
  const delivered={state:'delivered',actionable:true,evidenceRef:'delivery-1'};
  const pending={request:{requestId:'request-1',state:'pending'},approvalDelivery:delivered};
  assert.equal(summarizeLoginProgress(pending).waitingForUser,true);
  assert.equal(summarizeLoginProgress({...pending,approvalDelivery:{state:'delivered'}}).waitingForUser,false);
  assert.equal(summarizeLoginProgress({...pending,request:{requestId:'request-1',state:'expired'}}).waitingForUser,false);
});
