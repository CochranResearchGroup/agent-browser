import {withClientResultRecovery} from './client-recovery.js';
import {withClientErrorRecovery} from './client-recovery.js';
import { clientArgumentError } from './client-argument-error.js';
// @ts-check
export {createOwnerLoginPlaybook} from './owner-login-playbook.js';
/** @typedef {import('./login-playbook.d.ts').LoginOptions} LoginOptions */
/** @typedef {import('./login-playbook.d.ts').Dependencies} Dependencies */
/** @typedef {import('./login-playbook.d.ts').Checkpoint} Checkpoint */

/**
 * Compose the supported auth-helper operations with existing-task SiteGuide
 * verification. The trusted helper owns receiving input and capture privacy.
 * Checkpoints contain metadata only and must be durable before authentication.
 * Explicit reconcile mode reads back an existing hosted request without creating
 * a replacement or replaying private input; company verification remains fresh.
 * @param {Dependencies} dependencies
 */
export function createLoginPlaybook({browser,helper,checkpoints}) {
  if (!browser || typeof browser.prepareLogin !== 'function' || typeof browser.validatePlan !== 'function' || typeof helper !== 'function' || !checkpoints?.load || !checkpoints?.save) {
    throw clientArgumentError('login_playbook_dependencies_required', "packages/client/src/login-playbook.js", "createLoginPlaybook", ["browser", "helper", "checkpoints"]);
  }
  const busy=new Set();
  return {
    /**
     * Advance once. A persisted attempted request requires reconciliation; a lost
     * response never authorizes another authenticate call. Exact-task receiving
     * injection and observer privacy are deployment requirements, not inferred
     * from helper availability or supplied by this public client.
     * @param {LoginOptions} options
     */
    async advance(options) {
      const {taskId,guideId,accountRef,organizationRef,scope,expiresAt,callerTaskId}=options;
      if(callerTaskId!==undefined && (typeof callerTaskId!=='string' || !callerTaskId))throw withClientErrorRecovery(Error('login_caller_invalid'),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"},'login_caller_invalid');
      if(options.reconcile!==undefined&&typeof options.reconcile!=='boolean')throw withClientErrorRecovery(Error('login_reconciliation_invalid'),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"},'login_reconciliation_invalid');
      if(options.reconcile&&callerTaskId===undefined)throw withClientErrorRecovery(Error('hosted_caller_required_for_reconciliation'),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"},'hosted_caller_required_for_reconciliation');
      const key=`${taskId}/${guideId}`;
      if(busy.has(key))throw withClientErrorRecovery(Error('login_playbook_busy'),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"},'login_playbook_busy');
      busy.add(key);
      try {
        const fields=['person','tenant','account','origin','source'];
        if(!scope || Object.keys(scope).length!==6 || fields.some(field=>typeof scope[field]!=='string') ||
          JSON.stringify(scope.capabilities)!=='["login"]' || !Number.isSafeInteger(expiresAt)) {
          throw withClientErrorRecovery(Error('login_scope_invalid'),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"},'login_scope_invalid');
        }
        const binding=JSON.stringify({taskId,guideId,accountRef,organizationRef,
          scope:Object.fromEntries([...fields,'capabilities'].map(field=>[field,scope[field]])),expiresAt,
          authenticationMethod:options.authenticationMethod??'password',...(callerTaskId===undefined?{}:{callerTaskId})});
        /** @type {Checkpoint|null} */
        let checkpoint=await checkpoints.load(key);
        let receivingTarget=checkpoint?.receivingTarget;
        if(checkpoint && checkpoint.binding!==binding)throw withClientErrorRecovery(Error('checkpoint_binding_mismatch'),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"},'checkpoint_binding_mismatch');
        if(!checkpoint&&options.reconcile)throw withClientErrorRecovery(Error('checkpoint_required_for_reconciliation'),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"},'checkpoint_required_for_reconciliation');
        if(!checkpoint) {
          const observed=await browser.verifyLogin({taskId,guideId,accountRef,organizationRef});
          if(observed.status==='completed' || observed.status==='failed' || observed.status==='adaptation_required')return withClientResultRecovery(await (observed),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});
          if(options.authenticationMethod && options.authenticationMethod!=='password') {
            return withClientResultRecovery(await (result('waiting','authentication_method_unsupported','no_effect')),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});
          }
          if(!['identifier_required','password_required','login_required','mfa_required'].includes(observed.reasonCode))return withClientResultRecovery(await (observed),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});
          const preparation=await browser.prepareLogin({taskId,guideId,accountRef,organizationRef});
          if(!preparation.plan)return withClientResultRecovery(await (preparation),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});
          const preparedPlan=preparation.plan;
          if(callerTaskId!==undefined) {
            if(['profileId','browserId','tabId'].some(field=>typeof observed[field]!=='string' || !observed[field]))throw withClientErrorRecovery(Error('receiving_binding_required'),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"},'receiving_binding_required');
            receivingTarget={provider:'agent-browser',taskId,profileId:observed.profileId,browserId:observed.browserId,tabId:observed.tabId};
          }
          let requested;
          try {requested=await helper('request_login',{...scope,expiresAt,...(callerTaskId===undefined?{}:{contractVersion:2,receivingTarget})});}
          catch {return withClientResultRecovery(await (result('waiting','helper_unavailable','no_effect')),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});}
          if(!matches(requested))throw withClientErrorRecovery(Error('helper_binding_mismatch'),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"},'helper_binding_mismatch');
          checkpoint={binding,requestId:requested.requestId,attempted:false,
            ...(callerTaskId===undefined?{}:{callerTaskId,receivingTarget}),
            profileId:observed.profileId,browserId:observed.browserId,tabId:observed.tabId,
            ...(preparedPlan?{preparedPlan}:{}),
            knownTabIds:browser.listTabs ? (await browser.listTabs(taskId)).tabs.map(tab=>tab.tabId) : [observed.tabId]};
          await checkpoints.save(key,checkpoint);
        }
        let summary;
        try {summary=await helper('inspect_request',{requestId:checkpoint.requestId});}
        catch {return withClientResultRecovery(await (result('waiting','helper_unavailable',checkpoint.attempted?'unknown':'no_effect',checkpoint)),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});}
        if(!matches(summary) || summary.requestId!==checkpoint.requestId)throw withClientErrorRecovery(Error('helper_binding_mismatch'),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"},'helper_binding_mismatch');
        if(summary.state==='pending')return withClientResultRecovery(await (result('waiting','approval_required','no_effect',checkpoint)),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});
        if(['denied','revoked'].includes(summary.state))return withClientResultRecovery(await (result('failed',`helper_${summary.state}`,checkpoint.attempted?'unknown':'no_effect',checkpoint)),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});
        if(summary.state==='released')return withClientResultRecovery(await (result('failed','helper_attempt_released',checkpoint.attempted?'unknown':'no_effect',checkpoint)),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});
        if(options.reconcile){
          if(summary.running||!['uncertain','authenticated'].includes(summary.state))return withClientResultRecovery(await (result('waiting','helper_running_or_uncertain',checkpoint.attempted?'unknown':'no_effect',checkpoint)),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});
          // Explicit readback only. Never create a replacement request or make
          // this checkpoint eligible for another private authentication.
          try{summary=await helper('reconcile_authentication',{requestId:checkpoint.requestId});}
          catch{return withClientResultRecovery(await (result('waiting','helper_reconciliation_unavailable','unknown',checkpoint)),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});}
          if(!matches(summary)||summary.requestId!==checkpoint.requestId)throw withClientErrorRecovery(Error('helper_binding_mismatch'),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"},'helper_binding_mismatch');
          if(summary.state==='released')return withClientResultRecovery(await (result('failed','helper_attempt_released','unknown',checkpoint)),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});
          if(!['uncertain','authenticated'].includes(summary.state))return withClientResultRecovery(await (result('waiting','helper_running_or_uncertain','unknown',checkpoint)),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});
        }
        if(summary.state==='authenticated') {
          if(summary.captureState!=='ready')return withClientResultRecovery(await (result('waiting','private_capture_held','unknown',checkpoint)),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});
          return withClientResultRecovery(await (await verifyReceiving(checkpoint)),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});
        }
        if(summary.state==='uncertain' && summary.captureState==='ready' && summary.inputAttemptedAt===undefined) {
          return withClientResultRecovery(await (result('waiting','password_source_unavailable','unknown',checkpoint)),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});
        }
        if(summary.state==='uncertain' && summary.captureState==='ready' && summary.inputAttemptedAt!==undefined) {
          return withClientResultRecovery(await (await verifyReceiving(checkpoint)),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});
        }
        // Only a typed pre-input refusal plus fresh exact-request evidence can
        // clear local intent. Missing acknowledgement alone never permits retry.
        if(checkpoint.attempted && checkpoint.preInputRefusal && summary.state==='approved' &&
          summary.startedAt===undefined && summary.inputAttemptedAt===undefined) {
          checkpoint={...checkpoint,attempted:false,preInputRefusal:undefined};
          await checkpoints.save(key,checkpoint);
        }
        if(checkpoint.attempted || summary.startedAt!==undefined || summary.inputAttemptedAt!==undefined || summary.state==='uncertain') {
          return withClientResultRecovery(await (result('waiting','helper_running_or_uncertain','unknown',checkpoint)),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});
        }
        if(summary.state!=='approved')return withClientResultRecovery(await (result('failed','helper_state_invalid','no_effect',checkpoint)),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});
        if(options.reviewedPlan) {
          const replacement=await browser.validatePlan({taskId,guideId,accountRef,organizationRef,plan:options.reviewedPlan});
          if(replacement.status!=='completed')return withClientResultRecovery(await ({...replacement,routine:'login',requestId:checkpoint.requestId,actionId:checkpoint.requestId,effectState:'no_effect'}),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});
          const retainedTarget=checkpoint;
          if(['profileId','browserId','tabId'].some(field=>replacement[field]!==retainedTarget[field])) {
            return withClientResultRecovery(await (result('adaptation_required','task_target_changed','no_effect',checkpoint)),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});
          }
          checkpoint={...checkpoint,preparedPlan:options.reviewedPlan};
          await checkpoints.save(key,checkpoint);
        }
        if(!checkpoint.preparedPlan) {
          const preparation=await browser.prepareLogin({taskId,guideId,accountRef,organizationRef});
          if(!preparation.plan)return withClientResultRecovery(await (preparation),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});
          checkpoint={...checkpoint,preparedPlan:preparation.plan};await checkpoints.save(key,checkpoint);
        }
        {
          const retainedPlan=checkpoint.preparedPlan;
          if(!retainedPlan)throw withClientErrorRecovery(Error('prepared_plan_required'),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"},'prepared_plan_required');
          let validation=await browser.validatePlan({taskId,guideId,accountRef,organizationRef,plan:retainedPlan});
          if(validation.reasonCode==='plan_stale') {
            const preparation=await browser.prepareLogin({taskId,guideId,accountRef,organizationRef});
            if(preparation.plan && preparation.plan.guideDigest===retainedPlan.guideDigest &&
              preparation.plan.controlFingerprint===retainedPlan.controlFingerprint &&
              preparation.plan.expectedState===retainedPlan.expectedState && preparation.plan.bindingDigest===retainedPlan.bindingDigest) {
              checkpoint={...checkpoint,preparedPlan:preparation.plan};await checkpoints.save(key,checkpoint);
              validation=await browser.validatePlan({taskId,guideId,accountRef,organizationRef,plan:preparation.plan});
            }
          }
          const retainedTarget=checkpoint;
          if(validation.status==='completed' && ['profileId','browserId','tabId'].some(field=>validation[field]!==retainedTarget[field])) {
            return withClientResultRecovery(await (result('adaptation_required','task_target_changed','no_effect',checkpoint)),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});
          }
          if(validation.status!=='completed')return withClientResultRecovery(await ({...validation,routine:'login',requestId:checkpoint.requestId,actionId:checkpoint.requestId,effectState:'no_effect'}),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});
        }
        // Legacy callback metadata has no qualified executable expiry authority.
        // Keep its original readback available; current owner SDK workflows own
        // admission and expiry at the actual private-use boundary.
        return withClientResultRecovery(await (result('waiting','helper_grant_time_unqualified','no_effect',checkpoint)),{source:"packages/client/src/login-playbook.js",owningFunction:"createLoginPlaybook.advance"});

        /** @param {Record<string, any>} value */
        function matches(value) {
          const target=checkpoint?.receivingTarget??receivingTarget;
          const receivingMatches=callerTaskId===undefined || value.contractVersion===2 && target &&
            Object.keys(value.receivingTarget??{}).length===5 &&
            (/** @type {const} */ (['provider','taskId','profileId','browserId','tabId'])).every(field=>value.receivingTarget?.[field]===target[field]);
          return receivingMatches && typeof value?.requestId==='string' && value.taskId===(callerTaskId??taskId) && value.expiresAt===expiresAt &&
            fields.every(field=>value.scope?.[field]===scope[field]) && JSON.stringify(value.scope?.capabilities)==='["login"]';
        }
        /** @param {Checkpoint} current */
        async function verifyReceiving(current) {
          let observed=await browser.verifyLogin({taskId,guideId,accountRef,organizationRef});
          if(['profileId','browserId','tabId'].some(field=>observed[field]!==current[field])) {
            return result('adaptation_required','task_target_changed','unknown',current);
          }
          if(observed.status!=='completed' && browser.listTabs && browser.selectTab) {
            const tabs=await browser.listTabs(taskId);
            const candidates=tabs.tabs.filter(tab=>!current.knownTabIds?.includes(tab.tabId));
            if(candidates.length>16)return result('adaptation_required','destination_limit_exceeded','unknown',current);
            const matches=[];
            try {
              for(const tab of candidates) {
                await browser.selectTab(taskId,tab.tabId);
                const candidate=await browser.verifyLogin({taskId,guideId,accountRef,organizationRef});
                if(candidate.profileId!==current.profileId || candidate.browserId!==current.browserId)throw withClientErrorRecovery(Error('receiving_binding_changed'),{source:"packages/client/src/login-playbook.js",owningFunction:"verifyReceiving"},'receiving_binding_changed');
                if(candidate.status==='completed')matches.push(candidate);
              }
            } finally {await browser.selectTab(taskId,current.tabId);}
            if(matches.length>1)return result('adaptation_required','receiving_destination_ambiguous','unknown',current);
            if(matches.length===1) {
              observed=matches[0];
              const updated={...current,tabId:observed.tabId};
              await checkpoints.save(key,updated);
              await browser.selectTab(taskId,observed.tabId);
            }
          }
          return {...observed,routine:'login',requestId:current.requestId,
            actionId:current.requestId,
            effectState:observed.status==='completed'?'verified':'unknown'};
        }
        /** @param {string} status @param {string} reasonCode @param {string} effectState @param {Checkpoint} [current] */
        function result(status,reasonCode,effectState,current) {
          return withClientResultRecovery({schemaVersion:'agent-browser.routine-result.v1',routine:'login',taskId,guideId,
            status,reasonCode,effectState,...(current?{requestId:current.requestId,
              actionId:current.requestId,
              profileId:current.profileId,browserId:current.browserId,tabId:current.tabId}:{})},{source:"packages/client/src/login-playbook.js",owningFunction:"result"});
        }
      } finally {busy.delete(key);}
    },
  };
}

export {createOwnerTotpContinuation} from './owner-totp-continuation.js';
export {createOwnerNumberMatchingContinuation} from './owner-number-matching.js';
