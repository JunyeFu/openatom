/// <reference types="vite/client" />
// CSS-only independent declaration probe; real product/Host checks use harness.tsx.
import {createRoot} from 'react-dom/client';
import {MediaPresentationView} from '../../../apps/planner/src/features/media/MediaPresentationView';
import type {Presentation} from '../../../packages/commands/generated/typescript/Presentation';
import native from '../../../scenarios/媒体表现/runtime-node-static/inline-native.json';
const container=document.getElementById('root') as HTMLElement&{cssRoot?:ReturnType<typeof createRoot>};
const root=container.cssRoot??(container.cssRoot=createRoot(container));
root.render(<MediaPresentationView mode="asset_inspection" presentation={native as unknown as Presentation} lod="detailed" rights={[]}
  binding={{session_id:'controlled-css-probe',request:{project_id:'P1',scope:{kind:'project'},scope_revision:1,source:{kind:'module_version',data:{module_ref:{id:'inspector.native',version:'1.0.0'}}}}}}
  readAsset={async()=>{throw new Error('Native declaration has no byte assets');}}/>);
if(import.meta.hot)import.meta.hot.dispose(()=>{root.unmount();if(container.cssRoot===root)delete container.cssRoot;});
