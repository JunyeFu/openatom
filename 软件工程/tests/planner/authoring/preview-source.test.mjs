import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { build } from 'vite';
import { fileURLToPath } from 'node:url';
const root = new URL('../../../', import.meta.url);
const out = new URL('.work/single-window-20261018/p301-native36/Native51-template/preview/compiled/', root);
await build({configFile:false,root:fileURLToPath(root),logLevel:'warn',build:{ssr:true,outDir:fileURLToPath(out),emptyOutDir:false,minify:false,
  lib:{entry:fileURLToPath(new URL('tests/planner/authoring/preview-source-entry.ts',root)),formats:['es']},rolldownOptions:{output:{entryFileNames:'source.mjs'}}}});
const {authoringPresentationSource,createPresentationHost,ProcessSourceChanged,resolveMedia}=await import(new URL('source.mjs',out));
const module=JSON.parse(await readFile(new URL('modules/media/p302-controlled/image.module.json',root),'utf8'));
const workspace={project:{id:'preview-project',revision:17},context:{host:{session_id:'actual-ui-session'}}};
const draft={summary:{draft_id:'actual-definition-draft',revision:3,definition_ref:{id:'actual.media',version:'0.1.1'}}};
const source=(d=null)=>authoringPresentationSource(workspace,d,module,module.rights);
function reader(value,ports={}) {
  let current=value.context;const requests=[];
  const host=createPresentationHost(()=>current,{dispatch:async q=>{requests.push(q);return {kind:'module',data:module};},readAsset:async q=>{requests.push(q);return new Uint8Array([9,8,7]).buffer;},...ports});
  return {...host.media(value.context,value.binding),requests,switchTo:next=>current=next};
}
test('new local input has no invented Host scope; exact library source carries current project revision',()=>{
  assert.equal(authoringPresentationSource(workspace,null,null,[]),null);
  assert.equal(authoringPresentationSource({...workspace,project:null},draft,module,[]),null);
  const value=source();assert.equal(value.context.scope_revision,17);assert.deepEqual(value.context.scope,{kind:'project'});
  assert.deepEqual(value.binding.request.source,{kind:'module_version',data:{module_ref:{id:module.definition.id,version:module.definition.version}}});
  assert.equal(value.binding.session_id,'actual-ui-session');assert.equal(value.rights,module.rights);
});
test('local definition edits retain actual Draft identity and revision, distinct from project and current Run',()=>{
  const local={...module.definition,id:'unsaved-name',version:'9.9.9'},value=authoringPresentationSource(workspace,{...draft,definition:local},module,[]);
  assert.equal(value.context.scope_revision,3);assert.deepEqual(value.context.scope,{kind:'definition_draft',data:'actual-definition-draft'});
  assert.deepEqual(value.binding.request.source.data.module_ref,draft.summary.definition_ref);
  assert.equal(JSON.stringify(value).includes('run_id'),false);assert.equal(JSON.stringify(value).includes('unsaved-name'),false);
});
test('rights retain unknown license and redistribution=false without preventing local actual-byte inspection',async()=>{
  const rights=[{asset_ref:module.assets[0].id,source:'local only',license:null,redistribution_confirmed:false,missing_reason:'No redistribution confirmation'}];
  const value=authoringPresentationSource(workspace,null,module,rights),read=reader(value);
  const resolved=await resolveMedia({...value.binding,...read,binding:value.binding,rights:value.rights,presentation:module.definition.payload.data,lod:'detailed',mode:'asset_inspection'});
  const bytes=await resolved.read(module.assets[0]);assert.deepEqual([...new Uint8Array(bytes)],[9,8,7]);
  assert.equal(value.rights,rights);assert.equal(value.rights[0].license,null);assert.equal(value.rights[0].redistribution_confirmed,false);
  assert.deepEqual(read.requests[0],{...value.binding.request,asset:module.assets[0]});
});
test('an actual missing byte error is retained; no native or current-Run fallback is attempted',async()=>{
  const value=source(draft),failure=new Error('Actual draft-listed media bytes are absent');let reads=0;
  const read=reader(value,{readAsset:async()=>{reads++;throw failure;}});
  const resolved=await resolveMedia({...read,binding:value.binding,rights:value.rights,presentation:module.definition.payload.data,lod:'detailed',mode:'asset_inspection'});
  await assert.rejects(resolved.read(module.assets[0]),error=>error===failure);assert.equal(reads,1);assert.equal(read.requests.length,0);
});
test('exact media_ref traversal retains Draft Scope/revision and fetches the resolved version',async()=>{
  const value=source(draft),read=reader(value),presentation=structuredClone(module.definition.payload.data);
  presentation.source={kind:'media_ref',data:{id:module.definition.id,version:module.definition.version}};
  const resolved=await resolveMedia({...read,binding:value.binding,rights:[],presentation,lod:'detailed',mode:'asset_inspection'});
  await resolved.read(module.assets[0]);assert.equal(read.requests.length,2);
  assert.equal(read.requests[0].scope.data,'actual-definition-draft');assert.equal(read.requests[0].params.source.data.module_ref.version,module.definition.version);
  assert.equal(read.requests[1].scope_revision,3);assert.equal(read.requests[1].source.data.module_ref.id,module.definition.id);
});
test('a late byte response from an old session or Draft revision is rejected',async()=>{
  const value=source(draft);let release;const pending=new Promise(resolve=>release=resolve);
  const read=reader(value,{readAsset:async()=>pending});const request=read.readAsset({...value.binding.request,asset:module.assets[0]});
  read.switchTo({...value.context,session_id:'new-ui-session',scope_revision:4});release(new ArrayBuffer(0));
  await assert.rejects(request,ProcessSourceChanged);
});
