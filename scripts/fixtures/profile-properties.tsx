import React from 'react';
import { createRoot } from 'react-dom/client';
import { NativeTabActions } from '../../packages/dashboard/src/components/native-tab-actions';
const tab = { id: 'tab-a', browserId: 'browser-a', targetId: 'target-a', managedTask: true, profileId: 'stable-profile',
  createdAtMs: 1700000000000, taskMetadata: null, title: 'Research', profileProperties: { id: 'stable-profile', name: 'Temporary research', kind: 'disposable' as const } };
createRoot(document.getElementById('root')!).render(<NativeTabActions node={{ id: 'node-a', profileId: 'stable-profile' } as any} tab={tab}><button className="native-workspace-tab">Research</button></NativeTabActions>);
