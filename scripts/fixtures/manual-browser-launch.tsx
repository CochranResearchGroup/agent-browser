import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { ManualBrowserLaunchDialog } from '../../packages/dashboard/src/components/manual-browser-launch-dialog';
function Fixture() {
  const [open,setOpen]=useState(false);
  const [result,setResult]=useState('');
  return <><button onClick={()=>setOpen(true)}>Open browser</button><ManualBrowserLaunchDialog open={open} onOpenChange={setOpen} userLabel="operator" onOpened={async value=>setResult(JSON.stringify(value))}/><output id="opened">{result}</output></>;
}
createRoot(document.getElementById('root')!).render(<Fixture/>);
