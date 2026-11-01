// Actual Workbench and original UI adapter; isolated HTTP transports the existing real Core.
import React from 'react';
import {createRoot} from 'react-dom/client';
import {Workbench} from '../../../apps/planner/src/features/factory/Workbench';
import '../../../apps/planner/src/styles/workbench.css';

const calls: unknown[] = [];
const rpc = (query: unknown) => fetch('/__root_navigation_core', {method: 'POST', headers: {'content-type': 'application/json'}, body: JSON.stringify(query)}).then(reply => reply.json());
(window as any).__rootNavigation = {calls};
(window as any).__TAURI_INTERNALS__ = {
  async invoke(command: string, args: any) {
    calls.push({command, args});
    if (command === 'project_api') return rpc({kind: 'dispatch', request: args.request});
    if (command === 'project_read_media_asset') {
      const result = await rpc({kind: 'media', ...args.request});
      if (!result.ok) throw result.error;
      return Uint8Array.from(result.bytes).buffer;
    }
    throw new Error(`This isolated Core transport does not implement Native ${command}`);
  },
};
createRoot(document.getElementById('root')!).render(<React.StrictMode><Workbench /></React.StrictMode>);
