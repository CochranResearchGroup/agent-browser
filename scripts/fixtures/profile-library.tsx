import React from 'react';
import { createRoot } from 'react-dom/client';
import { ProfileLibrary } from '../../packages/dashboard/src/components/profile-library';
createRoot(document.getElementById('root')!).render(<ProfileLibrary onOpenGuides={site => { document.getElementById('guide-result')!.textContent = site; }}/>);
