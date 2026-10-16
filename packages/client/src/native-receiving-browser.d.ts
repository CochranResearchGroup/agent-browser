import type {Binding,Guard,Observation} from './hosted-login-receiver.js';
export interface Region {x:number;y:number;width:number;height:number}
export type NativeAction={kind:'click';x:number;y:number;button:number;count:number}|{kind:'key';chord:string}|{kind:'text';method:'keys';text:string};
/** Trusted observations alone may mark a recognized popup dismissible by Escape. */
export interface NativeObservation extends Observation {
 controls:Record<string,{selector:string;count:number;tag:string|null;type:string|null;visible:boolean;enabled:boolean;hitTarget:boolean;dismissiblePopup?:boolean;rect:Region|null}>;
 viewport:{width:number;height:number;devicePixelRatio:number;focused:boolean;visible:boolean};
 fingerprint?:string;
}
export interface Desktop {assignmentId:string;desktopId:string;generation:number;viewingGeneration:number;[key:string]:unknown}
export interface NativeGuards {windowId:number;expectedProcessIdentity:{pid:number;start_token:string;[key:string]:unknown};expectedWidth:number;expectedHeight:number;expectedWindow:Region}
export interface NativeOptions {
 readObservation(binding:Binding):Promise<NativeObservation>;
 inputMatches(input:{binding:Binding;control:string;selector:string;value:string;guard:Guard}):Promise<boolean>;
 desktop:Desktop;guards:NativeGuards;timeoutMs?:number;
 remote:{
  observe(desktop:Desktop):Promise<Desktop>;
  observation(desktop:Desktop):Promise<{windows:{windows:Array<{id:number;active:boolean;processIdentity?:{pid:number;start_token:string}}>}}>;
  captureRegion(desktop:Desktop,input:NativeGuards&{region:Region}):Promise<{region:Region;capture:{sha256:string}}>;
  input(desktop:Desktop,input:NativeGuards&{expectedVisual:{region:Region;sha256:string};actions:NativeAction[]}):Promise<{delivery:string}>;
 };
}
export interface NativeInput {binding:Binding;observation:NativeObservation&{fingerprint:string};control:string;guard:Guard}
/** Trusted host only: native input arguments and equality checks remain private. */
export function createNativeReceivingBrowser(options:NativeOptions):{
 observe(binding:Binding):Promise<NativeObservation&{fingerprint:string}>;
 fill(input:NativeInput&{value:string}):Promise<void>;
 activate(input:NativeInput):Promise<void>;
 waitForChange(input:Omit<NativeInput,'control'>):Promise<void>;
};
