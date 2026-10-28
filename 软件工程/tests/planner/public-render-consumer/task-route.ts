const task=await fetch('/__render_task_fixture').then(response=>response.json());
const calls:unknown[]=[];
declare global { interface Window { __renderRoute: { calls: unknown[] } } }
window.__renderRoute={calls};
(window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__={async invoke(command:string,args:unknown){
  calls.push({command,args});
  if(command==='native_render_input')return task;
  if(command==='native_capture_probe_plan')return null;
  if(command==='native_render_asset'||command==='native_render_file'){const response=await fetch('/__render_task_bytes',{method:'POST',body:JSON.stringify({command,args})});if(!response.ok)throw new Error(await response.text());return response.arrayBuffer();}
  if(command==='native_render_ready'||command==='native_render_failure')return;
  if(command==='engineering_status')return {phase:'controlled-main-only'};
  if(command==='engineering_frontend_ready')return;
  throw new Error('Task route must not invoke main engineering/public Host: '+command);
}};
await import('../../../apps/planner/src/main');
export {};
