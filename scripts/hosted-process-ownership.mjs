/** Exact current kernel generation and executable, never merely the PID. */
export function sameProcessInstance(row,prior) {
 return !!row&&!!prior&&row.pid===prior.pid&&row.uid===prior.uid&&
  typeof row.start==='string'&&/^\d+$/.test(row.start)&&row.start===prior.start&&
  typeof row.executable==='string'&&row.executable.startsWith('/')&&row.executable===prior.executable;
}
/** Retained positive custody survives env-marker loss, reparenting and SID changes.
 * A reused PID does not belong to the original custody; an incomplete or changed
 * executable on that same generation is unknown and must block release proof.
 */
export function retainedProcessOwned(row,observed=new Map()) {
 const prior=observed.get(row.pid);
 if(!prior||prior.start!==row.start)return false;
 if(!sameProcessInstance(row,prior))throw Error('fixture_retained_owned_identity_unknown');
 return true;
}
/** Pure Linux fixture custody projection. No PID-only or process-group kills. */
export function appProcessOwned(row,root,observed=new Map()) {
 if(!root||row.uid!==root.uid)return false;
 const prior=observed.get(row.pid);
 if(prior&&prior.start!==row.start)throw Error('fixture_owned_pid_reused');
 if(prior)return true; // Exact observed generation remains owned after reparenting.
 if(row.pid===root.pid){if(row.start!==root.start)throw Error('fixture_root_pid_reused');return true;}
 // detached spawn creates a new kernel session whose ID is the exact root PID.
 // A foreign session or process predating that root cannot qualify.
 return row.session===root.pid&&row.processGroup===root.pid&&BigInt(row.start)>=BigInt(root.start);
}
export function privateProfileReferenced(cmdline,profile) {
 // Chrome can expose either NUL-delimited argv or a flattened process title.
 return cmdline.includes(profile);
}

/** Retain viewer kernel custody independently of executable readability.
 * New descendants require the exact still-live launched executable and its
 * private detached session. Retained generations survive root exit/reparenting,
 * but never grant signal authority without a fresh executable observation.
 */
export async function retainViewerProcess(row,{root,observed,readRoot,readKernel}) {
 if(!root)return false;
 const prior=observed.get(row.pid);
 if(prior){
  if(prior.start!==row.start)throw Error('fixture_viewer_pid_reused');
  if(prior.uid!==row.uid)throw Error('fixture_viewer_identity_unknown');
 }else{
  if(row.uid!==root.uid||root.session!==root.pid||root.processGroup!==root.pid||
   row.session!==root.pid||row.processGroup!==root.pid||BigInt(row.start)<BigInt(root.start))return false;
  if(!sameProcessInstance(await readRoot(root.pid),root))throw Error('fixture_viewer_root_identity_unknown');
 }
 const fresh=await readKernel(row.pid);
 if(['pid','uid','start','parentPid','session','processGroup'].some(key=>fresh[key]!==row[key]))throw Error('fixture_viewer_identity_changed');
 observed.set(row.pid,{...fresh});return true;
}

/** Kernel-only custody is never kill authority or absence proof. readIdentity
 * must return null only after a fresh kernel ENOENT/ESRCH observation.
 */
export async function viewerCleanupCensus(observed,readIdentity) {
 const owned=[],unresolved=[];
 for(const prior of observed.values()){
  try{
   const row=await readIdentity(prior.pid);
   if(row===null)continue;
   if(row.pid!==prior.pid||row.start!==prior.start||row.uid!==prior.uid||typeof row.executable!=='string'||!row.executable.startsWith('/'))throw Error('fixture_viewer_cleanup_identity_unknown');
   owned.push(row);
  }catch(error){unresolved.push({pid:prior.pid,start:prior.start,code:error.code??error.message});}
 }
 return {owned,unresolved};
}
