import {mkdir,readFile,writeFile,readdir} from 'node:fs/promises';import {root,out} from './load-source.mjs';
const files=[
 'apps/planner/src/features/factory/Workbench.tsx','apps/planner/src/features/factory/DeviceProcessWorkspace.tsx',
 'apps/planner/src/features/presentation-editor/ImageTimelineEditor.tsx','apps/planner/src/features/presentation-editor/types.ts',
 'apps/planner/src/services/presentationHost.ts','apps/planner/src/features/media/MediaPresentationView.tsx',
 'apps/planner/src/features/media/load.ts','apps/planner/src/features/media/types.ts','apps/planner/src/features/media/viewport.ts',
 'apps/planner/src/features/media/harness.tsx','apps/planner/src/features/media/line-grade-harness.tsx',
 'apps/planner/src/features/presentation-editor/sharedMedia.tsx',
 'packages/presentation/src/scene.ts','packages/presentation/src/glb.ts',
 'scenarios/媒体表现/runtime-node-inspector/core-build.json',
];
for(const name of await readdir(new URL('apps/planner/src/features/node-inspector/',root)))if(/\.(tsx?|css)$/.test(name))files.push('apps/planner/src/features/node-inspector/'+name);
const snapshots=[];
for(const path of files){const bytes=await readFile(new URL(path,root)),target=new URL('frozen-source/'+path,out);await mkdir(new URL('./',target),{recursive:true});await writeFile(target,bytes);snapshots.push({source:path,snapshot:'scenarios/媒体表现/runtime-node-independent/frozen-source/'+path,bytes:bytes.length,exact_after_copy:bytes.equals(await readFile(new URL(path,root)))});}
await writeFile(new URL('source-cut.json',out),JSON.stringify({snapshots,no_hash_or_latest_rlib_selection:true,source_authority:'Root final freeze must be recorded by handoff; copying alone is not approval'},null,2));console.log('Exact source snapshots '+snapshots.length);
