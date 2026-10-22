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
