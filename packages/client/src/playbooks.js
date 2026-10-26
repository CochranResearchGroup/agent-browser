// @ts-check
/** @typedef {import('./playbooks.d.ts').CallTool} CallTool */
/** @typedef {import('./playbooks.d.ts').VerifyLoginOptions} VerifyLoginOptions */
/** @typedef {import('./playbooks.d.ts').RoutineResult} RoutineResult */

/**
 * Decode the existing public MCP envelope; no alternate task or transport owner.
 * @param {CallTool} callTool
 * @param {string} name
 * @param {Record<string, unknown>} args
 */
async function invoke(callTool, name, args) {
  const result = await callTool(name, args);
  const text = result?.content?.find(item => item.type === 'text')?.text;
  if (typeof text !== 'string') throw new Error('playbook_response_invalid');
  const envelope = JSON.parse(text);
  if (envelope.success !== true) throw new Error('playbook_request_failed');
  return envelope.data;
}

/**
 * Compose SiteGuide routines through an existing authenticated MCP client's
 * tools/call method. Credentials never enter this wrapper.
 * @param {CallTool} callTool
 */
export function createPlaybookClient(callTool) {
  if (typeof callTool !== 'function') throw new TypeError('playbook_transport_required');
  /** @param {string} taskId @param {string} operation @param {Record<string, unknown>} [params] */
  const run = (taskId, operation, params = {}) => invoke(callTool, 'browser_command', {
    taskId, action: 'playbook', params: {operation, ...params},
  });
  return {
    /** @param {string} taskId */
    listTabs: taskId => invoke(callTool,'browser_tab',{taskId,operation:'list'}),
    /** @param {string} taskId @param {string} tabId */
    selectTab: (taskId,tabId) => invoke(callTool,'browser_tab',{taskId,operation:'select',tabId}),
    /** @param {string} [operation] */
    discover: operation => invoke(callTool, 'browser_discover', {
      category: 'playbooks', ...(operation ? {operation} : {}),
    }),
    /** @param {string} taskId */
    listGuides: taskId => run(taskId, 'siteguides'),
    /** @param {string} taskId @param {string} guideId */
    inspectGuide: (taskId, guideId) => run(taskId, 'inspect_guide', {guideId}),
    /** Inspect an inert existing-model teaching candidate; never execute or publish it.
     * @param {string} taskId @param {string} guideId */
    inspectTeachingDraft: async (taskId, guideId) => {
      const guide=await run(taskId,'inspect_guide',{guideId});
      if(guide.teaching?.schemaVersion!=='agent-browser.teaching-draft.v1' || guide.teaching.state!=='draft') throw Error('teaching_draft_required');
      return guide;
    },
    /** @param {VerifyLoginOptions} options @returns {Promise<RoutineResult>} */
    verifyLogin: ({taskId, ...params}) => run(taskId, 'verify_login', params),
    /** @param {VerifyLoginOptions} options */
    prepareLogin: ({taskId,...params}) => run(taskId,'prepare_login',params),
    /** @param {VerifyLoginOptions & {plan:Record<string, unknown>}} options */
    validatePlan: ({taskId,...params}) => run(taskId,'validate_plan',params),
    /** @param {import('./playbooks.d.ts').CollectPostOptions} options */
    collectPosts: ({taskId,...params}) => run(taskId,'collect_posts',params),
  };
}
