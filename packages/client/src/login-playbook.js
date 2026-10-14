// @ts-check
/** @typedef {import('./login-playbook.d.ts').LoginOptions} LoginOptions */
/** @typedef {import('./login-playbook.d.ts').Dependencies} Dependencies */
/** @typedef {import('./login-playbook.d.ts').Checkpoint} Checkpoint */

/**
 * Compose the supported auth-helper operations with existing-task SiteGuide
 * verification. The trusted helper owns receiving input and capture privacy.
 * Checkpoints contain metadata only and must be durable before authentication.
 * @param {Dependencies} dependencies
 */
export function createLoginPlaybook({browser,helper,checkpoints}) {
  if (!browser || typeof helper !== 'function' || !checkpoints?.load || !checkpoints?.save) {
    throw new TypeError('login_playbook_dependencies_required');
  }
  const busy=new Set();
  return {
    /**
     * Advance once. A persisted attempted request can only be inspected; a lost
     * response never authorizes another authenticate call. Exact-task receiving
     * injection and observer privacy are deployment requirements, not inferred
     * from helper availability or supplied by this public client.
     * @param {LoginOptions} options
     */
    async advance(options) {
      const {taskId,guideId,accountRef,organizationRef,scope,expiresAt}=options;
      const key=`${taskId}/${guideId}`;
      if(busy.has(key))throw Error('login_playbook_busy');
      busy.add(key);
      try {
        const fields=['person','tenant','account','origin','source'];
        if(!scope || Object.keys(scope).length!==6 || fields.some(field=>typeof scope[field]!=='string') ||
          JSON.stringify(scope.capabilities)!=='["login"]' || !Number.isSafeInteger(expiresAt)) {
          throw Error('login_scope_invalid');
        }
        const binding=JSON.stringify({taskId,guideId,accountRef,organizationRef,
          scope:Object.fromEntries([...fields,'capabilities'].map(field=>[field,scope[field]])),expiresAt,
          authenticationMethod:options.authenticationMethod??'password'});
        /** @type {Checkpoint|null} */
        let checkpoint=await checkpoints.load(key);
        if(checkpoint && checkpoint.binding!==binding)throw Error('checkpoint_binding_mismatch');
        if(!checkpoint) {
          const observed=await browser.verifyLogin({taskId,guideId,accountRef,organizationRef});
          if(observed.status==='completed' || observed.status==='failed' || observed.status==='adaptation_required')return observed;
          if(options.authenticationMethod && options.authenticationMethod!=='password') {
            return result('waiting','authentication_method_unsupported','no_effect');
          }
          if(!['identifier_required','password_required','login_required','mfa_required'].includes(observed.reasonCode))return observed;
          let preparedPlan;
          if(browser.prepareLogin) {
            const preparation=await browser.prepareLogin({taskId,guideId,accountRef,organizationRef});
            if(!preparation.plan)return preparation;
            preparedPlan=preparation.plan;
          }
          let requested;
          try {requested=await helper('request_login',{...scope,expiresAt});}
          catch {return result('waiting','helper_unavailable','no_effect');}
          if(!matches(requested))throw Error('helper_binding_mismatch');
          checkpoint={binding,requestId:requested.requestId,attempted:false,
            profileId:observed.profileId,browserId:observed.browserId,tabId:observed.tabId,
            ...(preparedPlan?{preparedPlan}:{}),
            knownTabIds:browser.listTabs ? (await browser.listTabs(taskId)).tabs.map(tab=>tab.tabId) : [observed.tabId]};
          await checkpoints.save(key,checkpoint);
        }
        let summary;
        try {summary=await helper('inspect_request',{requestId:checkpoint.requestId});}
        catch {return result('waiting','helper_unavailable',checkpoint.attempted?'unknown':'no_effect',checkpoint);}
        if(!matches(summary) || summary.requestId!==checkpoint.requestId)throw Error('helper_binding_mismatch');
        if(summary.state==='pending')return result('waiting','approval_required','no_effect',checkpoint);
        if(['denied','revoked'].includes(summary.state))return result('failed',`helper_${summary.state}`,checkpoint.attempted?'unknown':'no_effect',checkpoint);
        if(summary.state==='authenticated') {
          if(summary.captureState!=='ready')return result('waiting','private_capture_held','unknown',checkpoint);
          return await verifyReceiving(checkpoint);
        }
        if(summary.state==='uncertain' && summary.captureState==='ready' && summary.inputAttemptedAt===undefined) {
          return result('waiting','password_source_unavailable','no_effect',checkpoint);
        }
        if(summary.state==='uncertain' && summary.captureState==='ready' && summary.inputAttemptedAt!==undefined) {
          return await verifyReceiving(checkpoint);
        }
        if(checkpoint.attempted || summary.startedAt!==undefined || summary.inputAttemptedAt!==undefined || summary.state==='uncertain') {
          return result('waiting','helper_running_or_uncertain','unknown',checkpoint);
        }
        if(summary.state!=='approved')return result('failed','helper_state_invalid','no_effect',checkpoint);
        if(summary.expiresAt<=Date.now())return result('waiting','helper_grant_expired','no_effect',checkpoint);
        if(checkpoint.preparedPlan && browser.validatePlan) {
          let validation=await browser.validatePlan({taskId,guideId,accountRef,organizationRef,plan:checkpoint.preparedPlan});
          if(validation.reasonCode==='plan_stale' && browser.prepareLogin) {
            const preparation=await browser.prepareLogin({taskId,guideId,accountRef,organizationRef});
            if(preparation.plan && preparation.plan.guideDigest===checkpoint.preparedPlan.guideDigest &&
              preparation.plan.controlFingerprint===checkpoint.preparedPlan.controlFingerprint) {
              checkpoint={...checkpoint,preparedPlan:preparation.plan};await checkpoints.save(key,checkpoint);
              validation=await browser.validatePlan({taskId,guideId,accountRef,organizationRef,plan:preparation.plan});
            }
          }
          if(validation.status!=='completed')return {...validation,routine:'login',requestId:checkpoint.requestId,actionId:checkpoint.requestId,effectState:'no_effect'};
        }
        // Persist before any call that can submit. Failure to save makes no input.
        checkpoint={...checkpoint,attempted:true};
        await checkpoints.save(key,checkpoint);
        try {summary=await helper('authenticate',{requestId:checkpoint.requestId});}
        catch(error) {
          const code=error && typeof error==='object' && 'code' in error ? error.code : undefined;
          const reason=code==='auth_not_authorized'?'helper_authorization_required':
            code==='auth_capture_held'?'private_capture_held':'helper_running_or_uncertain';
          return result('waiting',reason,'unknown',checkpoint);
        }
        if(!matches(summary) || summary.requestId!==checkpoint.requestId)throw Error('helper_binding_mismatch');
        if(summary.state!=='authenticated')return result('waiting','helper_running_or_uncertain','unknown',checkpoint);
        if(summary.captureState!=='ready')return result('waiting','private_capture_held','unknown',checkpoint);
        return await verifyReceiving(checkpoint);

        /** @param {Record<string, any>} value */
        function matches(value) {
          return typeof value?.requestId==='string' && value.taskId===taskId && value.expiresAt===expiresAt &&
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
                if(candidate.profileId!==current.profileId || candidate.browserId!==current.browserId)throw Error('receiving_binding_changed');
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
          return {schemaVersion:'agent-browser.routine-result.v1',routine:'login',taskId,guideId,
            status,reasonCode,effectState,...(current?{requestId:current.requestId,
              actionId:current.requestId,
              profileId:current.profileId,browserId:current.browserId,tabId:current.tabId}:{})};
        }
      } finally {busy.delete(key);}
    },
  };
}
