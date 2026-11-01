import {readFile,writeFile,mkdir,access} from 'node:fs/promises';
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/媒体表现/runtime-node-inspector/before/',root);await mkdir(out,{recursive:true});
for(const path of ['apps/planner/src/features/media/load.ts','apps/planner/src/features/media/types.ts','apps/planner/src/features/media/MediaPresentationView.tsx','apps/planner/src/features/media/harness.tsx','apps/planner/src/features/media/line-grade-harness.tsx','apps/planner/src/services/presentationHost.ts','apps/planner/src/features/factory/DeviceProcessWorkspace.tsx','apps/planner/src/features/presentation-editor/sharedMedia.tsx']){
 const target=new URL(path.split('/').slice(-2).join('__')+'.txt',out);let exists=true;try{await access(target);}catch{exists=false;}if(!exists)await writeFile(target,await readFile(new URL(path,root)));
}
