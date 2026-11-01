import {createRoot} from 'react-dom/client';import {Workbench} from '../../../apps/planner/src/features/factory/Workbench';import '../../../apps/planner/src/styles/workbench.css';
const calls:unknown[]=[],held:(()=>void)[]=[];let hold=false;
const rpc=(request:unknown)=>fetch('/__normal_core',{method:'POST',body:JSON.stringify(request)}).then(r=>r.json());
(window as unknown as {__TAURI_INTERNALS__:unknown}).__TAURI_INTERNALS__={async invoke(command:string,args:{request:any}){
 const request=args.request;calls.push({command,request});
 if(command==='project_api')return rpc(request.operation==='get_task'?{kind:'await_task',scope:request.scope,task_id:request.params.task_id}:{kind:'dispatch',request});
 if(command==='project_read_media_asset'||command==='project_read_run_port_output'){const value=await rpc({kind:command==='project_read_media_asset'?'media':'port_output',...request});if(!value.ok)throw value.error;return command==='project_read_media_asset'?Uint8Array.from(value.bytes).buffer:value.data;}
 const response=await fetch('/__normal_private',{method:'POST',body:JSON.stringify({command,request})});
 if(!response.ok)throw new Error(await response.text());
 if(command==='project_read_stored_manufacturing_file'){if(hold)await new Promise<void>(resolve=>held.push(resolve));return response.arrayBuffer();}
 return response.json();
}};
declare global {interface Window {__normalRender:any}}
window.__normalRender={read:()=>({calls,held:held.length}),hold:()=>{hold=true;},release:()=>{hold=false;held.splice(0).forEach(resolve=>resolve());}};
const container=document.getElementById('root') as HTMLElement&{normalRoot?:ReturnType<typeof createRoot>};const root=container.normalRoot??(container.normalRoot=createRoot(container));root.render(<Workbench/>);if(import.meta.hot)import.meta.hot.dispose(()=>{root.unmount();if(container.normalRoot===root)delete container.normalRoot;});
