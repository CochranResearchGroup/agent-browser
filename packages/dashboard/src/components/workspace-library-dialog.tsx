'use client';
import {useCallback,useEffect,useState} from 'react';
import {Dialog,DialogContent,DialogHeader,DialogTitle,DialogDescription} from '@/components/ui/dialog';
import {ProfileLibrary} from '@/components/profile-library';
import {ProfileLab,type ProfileLabSource} from '@/components/profile-lab';
import {SiteGuidesLibrary} from '@/components/siteguides-library';
import {loadProfileLibrary,type LibraryProfile,type ProfileLibraryData} from '@/lib/workspace-profile-library';
import {requestSiteGuides,type TeachingContext} from '@/lib/workspace-siteguides';
type Section='profiles'|'guides'|'lab';
/** One library dialog consumes the existing profile and teaching owners. Native
 * recording, narration and input stay in the mounted Remote View desktop. */
export function WorkspaceLibraryDialog({open,onOpenChange,initialSection,source,task,onRefresh,onOpenProfile,diagnosticsHref}:{open:boolean;onOpenChange:(open:boolean)=>void;initialSection:Section;source:ProfileLabSource|null;task:{taskId:string;profileId:string}|null;onRefresh:()=>Promise<void>;onOpenProfile:(profileId:string)=>void;diagnosticsHref:string}) {
 const [section,setSection]=useState<Section>(initialSection),[profiles,setProfiles]=useState<LibraryProfile[]>([]),[sites,setSites]=useState<ProfileLibraryData['sitePolicies']>([]);
 const [site,setSite]=useState(''),[account,setAccount]=useState(''),[organization,setOrganization]=useState(''),[error,setError]=useState('');
 const [accountUnknown,setAccountUnknown]=useState(false),[personal,setPersonal]=useState(false);
 const profilesChanged=useCallback((next:LibraryProfile[])=>setProfiles(next),[]);
 useEffect(()=>{if(!open)return;let active=true;setSection(initialSection);setSite('');setAccount('');setOrganization('');setAccountUnknown(false);setPersonal(false);setError('');loadProfileLibrary().then(data=>{if(active){setProfiles(data.profiles.map(row=>row.profile));setSites(data.sitePolicies);}}).catch(()=>{if(active)setError('Site and profile choices could not be read. Reopen the library to refresh.');});return()=>{active=false};},[open,initialSection,task?.taskId]);
 const profile=profiles.find(row=>row.id===task?.profileId);
 const context:TeachingContext|null=task&&site&&(account.trim()||accountUnknown)&&(organization.trim()||personal)?{...task,sitePolicyId:site,account:accountUnknown?'':account.trim(),organization:personal?'':organization.trim()}:null;
 const interpret=useCallback(async(packet:Record<string,unknown>)=>{
  const request=packet.agentRequest;
  if(!request||typeof request!=='object'||Array.isArray(request))throw Error('Teaching agent request is unavailable. Refresh the original source packet.');
  const result=await requestSiteGuides({...request as Record<string,unknown>,operation:'request_agent'});
  if(result.status!=='saved'||typeof result.guideId!=='string')throw Error('The original interpretation is not confirmed. Inspect its original operation before continuing.');
  return {guideId:result.guideId};
 },[]);
 return <Dialog open={open} onOpenChange={onOpenChange}><DialogContent className="native-task-dialog workspace-library"><DialogHeader><DialogTitle>Profiles and SiteGuides</DialogTitle><DialogDescription>Intended site identities and retained authentication evidence remain separate.</DialogDescription></DialogHeader>
 <nav aria-label="Library sections"><button type="button" aria-pressed={section==='profiles'} onClick={()=>setSection('profiles')}>Profiles</button><button type="button" aria-pressed={section==='guides'} onClick={()=>setSection('guides')}>SiteGuides</button><button type="button" aria-pressed={section==='lab'} disabled={!profiles.length&&!source} onClick={()=>setSection('lab')}>Profile Lab</button></nav>
 {error&&<p role="alert">{error}</p>}
 {section==='profiles'&&<ProfileLibrary onProfilesChange={profilesChanged} onOpenGuides={siteId=>{setSite(siteId);setSection('guides')}}/>}
 {section==='lab'&&<ProfileLab profiles={source?[source]:profiles.map(row=>({id:row.id,name:row.name||row.id}))} onRefresh={onRefresh}/>}
 {section==='guides'&&<><fieldset><legend>Teach this site's intended context</legend>{task?<p>Selected task: {task.taskId}. Profile: {profile?.name||task.profileId}.</p>:<p>Select a managed browser task to begin teaching. Retained guides remain available below.</p>}
 <label>Site<select aria-label="Teaching site" value={site} onChange={event=>{setSite(event.target.value);setAccount('');setOrganization('');setAccountUnknown(false);setPersonal(false)}}><option value="">Choose a configured site</option>{sites.map(row=><option key={row.id} value={row.id}>{row.name||row.id}</option>)}</select></label>
 {!!profile?.siteAssociations.filter(row=>row.sitePolicyId===site).length&&<label>Saved intended identity<select aria-label="Saved teaching identity" value={JSON.stringify([account,organization])} onChange={event=>{const [a,o]=JSON.parse(event.target.value);setAccount(a);setOrganization(o);setAccountUnknown(!a);setPersonal(!o)}}><option value={JSON.stringify(['',''])}>Choose explicitly or enter below</option>{profile.siteAssociations.filter(row=>row.sitePolicyId===site).map((row,index)=><option key={index} value={JSON.stringify([row.account,row.organization])}>{row.account} · {row.organization}</option>)}</select></label>}
 <label>Intended account<input aria-label="Teaching intended account" disabled={accountUnknown} value={account} onChange={event=>setAccount(event.target.value)}/></label><label><input type="checkbox" checked={accountUnknown} onChange={event=>setAccountUnknown(event.target.checked)}/>Account unknown or unverified</label><label>Organization or workspace<input aria-label="Teaching organization" disabled={personal} value={organization} onChange={event=>setOrganization(event.target.value)}/></label><label><input type="checkbox" checked={personal} onChange={event=>setPersonal(event.target.checked)}/>Personal / no organization</label><p>These labels describe your intent. The site must independently verify the signed-in identity. Record video and human narration using the existing Remote View controls.</p></fieldset>
 <SiteGuidesLibrary profiles={profiles} sitePolicyId={site||undefined} context={context} onNativeView={()=>onOpenChange(false)} onInterpret={interpret} onProfile={profileId=>{onOpenChange(false);onOpenProfile(profileId)}}/></>}
 <a href={diagnosticsHref}>Service diagnostics</a></DialogContent></Dialog>;
}
