export interface Binding {provider:'agent-browser';taskId:string;profileId:string;browserId:string;tabId:string;[key:string]:string}
export interface Scope {person:string;tenant:string;account:string;origin:string;source:string;capabilities:string[];[key:string]:unknown}
export type Guard=()=>Promise<void>;
export interface Observation {binding:Binding;origin:string;state:string;accountMatched:boolean;visiblePassword:boolean;maskedPassword:boolean;[key:string]:unknown}
export interface ReceiverOptions {
 binding:Binding;scope:Scope;expectedUsername:string;maxTransitions?:number;authenticatedOrigins?:string[];
 browser:{
  observe(binding:Binding):Promise<Observation>;
  fill(input:{binding:Binding;observation:Observation;control:'identifier'|'password';value:string;guard:Guard}):Promise<void>;
  activate(input:{binding:Binding;observation:Observation;control:'identifierSubmit'|'submit';guard:Guard}):Promise<void>;
  waitForChange(input:{binding:Binding;observation:Observation;guard:Guard}):Promise<void>;
 };
 awaitMfa?(input:{receivingTarget:Binding;scope:Scope;guard:Guard}):Promise<{dispatched:boolean}>;
 readPrincipal(input:{binding:Binding;guard:Guard}):Promise<{authenticated:boolean;origin:string;username:string}>;
 readReconciliationSafety?(input:{binding:Binding;guard:Guard;authenticated:boolean}):Promise<{safe:boolean;protectedInputNotAttempted?:boolean;receivingLoginVerified?:boolean}>;
 readCaptureSafety(input:{binding:Binding;credential:{username:string;password:string}}):Promise<{safe:boolean}>;
}
/** Trusted hosted-service adapter only; never pass credential values through public client tools. */
export function createHostedLoginReceiver(options:ReceiverOptions):{
 inspect(scope:Scope,guard?:Guard):Promise<{origin:string;account:string}>;
 apply(credential:{username:string;password:string},scope:Scope,guard:Guard):Promise<void>;
 verify(scope:Scope,guard?:Guard):Promise<{origin:string;account:string|null;authenticated:boolean}>;
 reconcile(scope:Scope,guard?:Guard):Promise<{origin:string;account:string|null;authenticated:boolean;captureSafe:boolean;protectedInputNotAttempted?:boolean;receivingLoginVerified?:boolean}>;
 safeForCapture(credential:{username:string;password:string},scope:Scope):Promise<boolean>;
};
