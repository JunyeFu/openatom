// Read the unchanged pre-render SDK's actual response. This baseline has no
// Native backend; it cannot qualify the later injected-backend control tests.
import assert from 'node:assert/strict';
import {writeFile,mkdir} from 'node:fs/promises';
import {openCore,root} from '../planner/node-inspector/core.mjs';
const out=new URL('scenarios/Agent共建/runtime-public-render-host/',root);
await mkdir(out,{recursive:true});
const core=await openCore('scenarios/图片时间轴/runtime/phase.input.json');
try {
  const p=core.ready.snapshot.project;
  const request={api_version:'0.1',request_id:'public-render-before',project_id:p.id,scope:{kind:'project'},operation:'render_view',params:{revision:p.revision,view:{kind:'line_canvas',canvas:'line_canvas',hall_id:p.halls[0].id,object_ref:null,project_revision:p.revision,run_id:null,view_time_ms:null,lod:'detailed',camera:p.views.line_camera},object_id:null,preview:null,delivery:'capture_only'}};
  const response=await core.rpc({kind:'dispatch',request});
  await writeFile(new URL('baseline-unsupported.json',out),JSON.stringify({request,response,SDK:'scenarios/媒体表现/runtime-node-inspector/core_consumer.exe',cut:'pre-render Node8 + frozen-instance4',backend_present:false,expected_product_goal:'actual Rendering Task only after a real trusted product backend is injected',Native:false},null,2));
  assert.equal(response.outcome.status,'ok','Pre-render actual Host still has no public render_view producer/backend; not a Native PNG test');
}finally{await core.close();}
