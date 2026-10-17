/** Connect to the verified installed Bots SDK without exposing private bearer material. */
export function connectHostedAuthHelper(options?:{installationRoot?:string;discoveryFile?:string}):Promise<{
  callerTaskId:string;
  installation:{identity:string;sourceRevision:string};
  helper(operation:string,args:Record<string,unknown>):Promise<Record<string,any>>;
}>;
