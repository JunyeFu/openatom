import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
export const root=new URL('../../../',import.meta.url), out=new URL('scenarios/媒体表现/runtime-node-independent/',root);
const compiled=new URL('compiled/',out);await mkdir(compiled,{recursive:true});
for(const [name,path] of [
  ['formSchema','features/modules/formSchema'],['authoringModel','features/modules/model'],
  ['presentationHost','services/presentationHost'],['model','features/node-inspector/model'],['read','features/node-inspector/read'],
]) {
  let source=stripTypeScriptTypes(await readFile(new URL(`apps/planner/src/${path}.ts`,root),'utf8'),{mode:'transform'})
    .replace(/import \{ dispatch \} from '(?:\.\/projectHost|\.\.\/\.\.\/services\/projectHost)';/,"const dispatch=()=>{throw new Error('Independent test requires actual fixed SDK port; no Native')};")
    .replace(/import \{ readMediaAsset[^\n]*from '.\/mediaAssets';/,"const readMediaAsset=()=>{throw new Error('No Native bytes in independent metadata reader')};")
    .replaceAll("from '../../services/presentationHost'","from './presentationHost.mjs'")
    .replace("from './model'","from './model.mjs'").replace("from './formSchema'","from './formSchema.mjs'");
  await writeFile(new URL(name+'.mjs',compiled),source);
}
export const model=await import(new URL('model.mjs',compiled));
export const authoring=await import(new URL('authoringModel.mjs',compiled));
export const {createNodeInspectorReader}=await import(new URL('read.mjs',compiled));
export async function workbenchEditor(dependencies){
  const path=new URL('apps/planner/src/features/factory/Workbench.tsx',root),text=await readFile(path,'utf8');
  const start=text.indexOf('  async function editProcessNodeSource('),end=text.indexOf('\n  const processActions:',start);
  if(start<0||end<0)throw new Error('Exact reviewed Workbench source region not found');
  const source=text.slice(start,end);await writeFile(new URL('reviewed-workbench-function.ts',out),source);
  const code=stripTypeScriptTypes(source,{mode:'transform'}),names=Object.keys(dependencies);
  return new Function(...names,`${code}\nreturn editProcessNodeSource;`)(...names.map(name=>dependencies[name]));
}
export async function consumerEditNode(dependencies){
  const text=await readFile(new URL('apps/planner/src/features/factory/DeviceProcessWorkspace.tsx',root),'utf8');
  const start=text.indexOf('  async function editNode('),end=text.indexOf('\n  const nodePanel',start);
  if(start<0||end<0)throw new Error('Exact consumer editNode function not found');
  const source=text.slice(start,end);await writeFile(new URL('reviewed-consumer-edit-node.ts',out),source);
  const code=stripTypeScriptTypes(source,{mode:'transform'}),names=Object.keys(dependencies);
  return new Function(...names,`${code}\nreturn editNode;`)(...names.map(name=>dependencies[name]));
}
export async function sampleReceiver(dependencies){
  const text=await readFile(new URL('apps/planner/src/features/factory/Workbench.tsx',root),'utf8');
  const start=text.indexOf('  async function receiveProcessSample('),end=text.indexOf('\n  async function runProcessCycle',start);
  if(start<0||end<0)throw new Error('Exact sample receiver source not found');
  const source=text.slice(start,end);await writeFile(new URL('reviewed-sample-receiver.ts',out),source);
  const code=stripTypeScriptTypes(source,{mode:'transform'}),names=Object.keys(dependencies);
  return new Function(...names,`${code}\nreturn receiveProcessSample;`)(...names.map(name=>dependencies[name]));
}
