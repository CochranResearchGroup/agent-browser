import { dashboardResponseFailureError, responseFailureMetadata } from "./response-failure.ts";
import type { BrowserBuildEvidence } from '@/components/browser-build-picker';
export type ManualProfile = {id:string;name:string;mode?:'cdp'|'cdp_free';family?:string;version?:string;buildId?:string;activeTaskCount:number;canShareTabs:boolean;accountLabels?:string[];taskId?:string;sessionName?:string;browserId?:string;tabId?:string};
export type ManualBrowserCatalog = {schemaVersion:'agent-browser.workspace-browser-catalog.v1';builds:BrowserBuildEvidence[];profiles:ManualProfile[]};
export type ManualBrowserOpened = {schemaVersion:'agent-browser.workspace-browser-launch.v1';operationId:string;state:'ready';warning?:string;initialNavigation?:'unconfirmed';taskId:string;sessionName:string;profileId:string;browserId:string;tabId:string};
export type ManualBrowserIntent = {operationId:string;profileMode:'disposable'|'persistent'|'existing';profileId?:string;profileName?:string;mode:'cdp'|'cdp_free';url:string;leaveOpen:true;confirmProfileBuildChange?:boolean;buildSelection:BrowserBuildEvidence['selection']};
export class ManualBrowserLaunchRefusal extends Error {}
const endpoint='/api/dashboard/browser-launch';
export async function loadManualBrowserCatalog():Promise<ManualBrowserCatalog>{
 const r=await fetch(endpoint,{credentials:'same-origin',cache:'no-store',signal:AbortSignal.timeout(15000)});const p=await r.json();
 if(!r.ok||p.success!==true||responseFailureMetadata(p)||p.data?.schemaVersion!=='agent-browser.workspace-browser-catalog.v1'||!Array.isArray(p.data.builds)||!Array.isArray(p.data.profiles))throw dashboardResponseFailureError(p,'browser_choices_unconfirmed');
 return p.data;
}
export async function openManualBrowser(intent:ManualBrowserIntent):Promise<ManualBrowserOpened>{
 const r=await fetch(endpoint,{method:'POST',credentials:'same-origin',headers:{'Content-Type':'application/json'},signal:AbortSignal.timeout(60000),body:JSON.stringify(intent)});return readOpened(r,intent.operationId);
}
export async function inspectManualBrowserLaunch(operationId:string):Promise<ManualBrowserOpened>{
 const r=await fetch(endpoint+'?operationId='+encodeURIComponent(operationId),{credentials:'same-origin',cache:'no-store',signal:AbortSignal.timeout(15000)});return readOpened(r,operationId);
}
async function readOpened(r:Response,id:string):Promise<ManualBrowserOpened>{const p=await r.json(),d=p.data;if(!r.ok&&p.outcome==='refused_no_effect')throw Object.assign(new ManualBrowserLaunchRefusal('No browser was created. Review the choices before trying again.'),dashboardResponseFailureError(p,'manual_browser_launch_refused_no_effect'));if(!r.ok||p.success!==true||responseFailureMetadata(p)||d?.schemaVersion!=='agent-browser.workspace-browser-launch.v1'||d.operationId!==id||d.state!=='ready'||['taskId','sessionName','profileId','browserId','tabId'].some(k=>typeof d[k]!=='string'||!d[k]))throw dashboardResponseFailureError(p,'manual_browser_launch_unconfirmed_inspect_original');return d;}
