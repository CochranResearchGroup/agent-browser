import React from 'react';
import {createRoot} from 'react-dom/client';
import {SiteGuidesLibrary} from '../../packages/dashboard/src/components/siteguides-library';
const selected=new URLSearchParams(location.search).has('selected');
createRoot(document.getElementById('root')!).render(<SiteGuidesLibrary context={selected?{taskId:'task',profileId:'work',sitePolicyId:'sales',account:'operator@example.test',organization:'Research'}:null} onNativeView={()=>{document.getElementById('native-result')!.textContent='native-owner';}} onInterpret={async packet=>{document.getElementById('agent-result')!.textContent=JSON.stringify(packet);}}/>);
