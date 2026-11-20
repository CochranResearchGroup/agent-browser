// @ts-check
/** Ordinary invocation wait budget. It makes no trusted age/expiry assertion.
 * @param {number} timeoutMs @param {string} code
 */
export function clientWaitBudget(timeoutMs, code) {
  const controller=new AbortController();let ended=false,finish;
  const end=new Promise(resolve=>{finish=resolve});
  const timer=setTimeout(()=>{ended=true;controller.abort();finish();},timeoutMs);
  const fail=()=>{throw Object.assign(new Error(code),{code,budgetKind:'caller_wait_only'})};
  return {
    signal:controller.signal,
    check(){if(ended)fail()},
    /** @template T @param {()=>Promise<T>} work @returns {Promise<T>} */
    async run(work){if(ended)fail();try{return await Promise.race([work(),end.then(fail)])}catch(error){if(ended)fail();throw error}},
    close(){clearTimeout(timer)},
  };
}
