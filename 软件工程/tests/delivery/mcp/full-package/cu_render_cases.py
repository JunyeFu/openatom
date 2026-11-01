"""Actual product Native renderer through MCP; no supplied/fabricated pixels."""
import base64,copy,importlib.util,json,struct

def record_image(client,reply,out,images,check):
    if reply['outcome']['status']!='ok':return
    state=reply['outcome']['data']
    if state['task']['state']!='completed' or state['result']['kind']!='render':return
    content=[c for c in client.last_wrapper['content'] if c['type']=='image']
    check('actual_MCP_render_has_one_native_image',len(content)==1 and content[0]['mimeType']=='image/png')
    png=base64.b64decode(content[0]['data'],validate=True)
    check('actual_native_capture_is_nonempty_PNG',png.startswith(b'\x89PNG\r\n\x1a\n') and len(png)>24)
    pixels=struct.unpack('>II',png[16:24]);check('actual_native_PNG_dimensions',pixels[0]>0 and pixels[1]>0)
    name=f'product-render-{len(images)+1}.png';(out/name).write_bytes(png)
    images.append({'path':name,'task':state['task'],'output':state['result']['data'],'pixels':pixels,'bytes':len(png)})

def execute(n):
    root=n['ROOT'];out=n['OUT'];data=n['INPUT'];check=n['check'];ok=n['ok'];p=data['project']['id']
    spec=importlib.util.spec_from_file_location('workflow',root/'integrations/agent/workflow.py');w=importlib.util.module_from_spec(spec);spec.loader.exec_module(w)
    client=n['Client']();images=[];n['RENDER_IMAGES']=images
    def call(op,params):
        reply=client.call(op,params,project=p)
        if op=='get_task':record_image(client,reply,out,images,check)
        return reply
    reply=call('get_context',{'selection':[]});client.session=reply['context']['session_id'];context=ok(reply)
    tools=[t['name'] for t in client.rpc('tools/list',{})['result']['tools']];(out/'actual-tools.json').write_text(json.dumps(tools,indent=2),encoding='utf-8')
    check('real_renderer_advertised_iff_registered',('superfactory_render_view' in tools)==data['expect_backend'])
    project=w.object_at_draft(call,'project',p)
    request=copy.deepcopy(data['render_request']);request['revision']=project['revision'];request['view']['project_revision']=project['revision']
    if n['MODE']=='render-structural':
        device=next(d for d in project['devices'] if d['id'].endswith(':Inspection01'));candidate=copy.deepcopy(device);a,b=candidate['footprint_m'][:2];candidate['footprint_m'].insert(1,{'x':(a['x']+b['x'])/2,'y':(a['y']+b['y'])/2})
        settings={k:copy.deepcopy(project[k]) for k in ['title','time_reference','generation_context','sources','setup_policies']};settings['title']='Must remain uncommitted after structural rejection'
        params={'base_revision':project['revision'],'commands':[{'kind':'set_project_settings','data':settings},{'kind':'put_object','data':{'kind':'device','data':candidate}}]}
        response=call('apply_commands',params)
        check('actual_whole_polygon_grant_remains_Unsupported',response['outcome']['status']=='error' and response['outcome']['data']['code']=='UnsupportedCapability')
        check('structural_rejection_is_atomic',w.object_at_draft(call,'project',p)==project and w.object_at_draft(call,'device',device['id'])==device)
        for identity,original in data['workflow']['frozen_runs'].items():check('structural_refusal_keeps_original_Run',w.unwrap(call('get_run',{'run_id':identity,'query':{'kind':'full_result'}}))['data']==original)
        (out/'structural-repro.json').write_text(json.dumps({'params':params,'actual_response':response,'before':device,'proposed':candidate,'project_unchanged':True,'production_patch_applied':False},ensure_ascii=False,indent=2),encoding='utf-8');client.close();return
    if not data['expect_backend']:
        rejected=call('render_view',request)
        check('unregistered_renderer_is_explicitly_Unsupported',rejected['outcome']['status']=='error' and rejected['outcome']['data']['code']=='UnsupportedCapability')
        check('unsupported_render_never_emits_an_image',not any(c['type']=='image' for c in client.last_wrapper.get('content',[])))
        (out/'unsupported.json').write_text(json.dumps(rejected,ensure_ascii=False,indent=2),encoding='utf-8');client.close();return
    state=w.complete(call,'render_view',request)
    (out/'render-state.json').write_text(json.dumps(state,ensure_ascii=False,indent=2),encoding='utf-8')
    check('real_product_renderer_completed',state['task']['state']=='completed')
    output=state['result']['data'];check('same_revision_real_software_scene',output['scene']['evidence_origin']=='actual_software' and output['scene']['scene_revision']==project['revision'] and output['scene']['view']==request['view'])
    check('real_product_camera_sees_declared_objects',bool(output['visible_objects']))
    check('render_does_not_edit_project',w.object_at_draft(call,'project',p)==project)
    if n['MODE']=='render-detail':
        for identity,original in data['workflow']['frozen_runs'].items():check('detail_preview_keeps_original_Run',w.unwrap(call('get_run',{'run_id':identity,'query':{'kind':'full_result'}}))['data']==original)
        (out/'render-images.json').write_text(json.dumps(images,ensure_ascii=False,indent=2),encoding='utf-8');client.close();return
    stale=copy.deepcopy(request);stale['revision']=project['revision']-1
    rejected=call('render_view',stale)
    check('stale_render_admission_is_RevisionConflict',rejected['outcome']['status']=='error' and rejected['outcome']['data']['code']=='RevisionConflict')
    shown=copy.deepcopy(request);shown['delivery']='show_and_capture';denied=call('render_view',shown)
    check('capture_permission_never_grants_visible_view_control',denied['outcome']['status']=='error' and denied['outcome']['data']['code']=='PermissionDenied')
    detail=copy.deepcopy(request);detail['view']['kind']='device_detail';detail['view']['object_ref']={'kind':'device','id':project['devices'][-1]['id']};detail['object_id']=detail['view']['object_ref']
    unsupported=call('render_view',detail)
    check('unregistered_view_kind_is_explicitly_Unsupported',unsupported['outcome']['status']=='error' and unsupported['outcome']['data']['code']=='UnsupportedCapability')
    cancelled=w.unwrap(call('render_view',request));control=w.unwrap(call('cancel_task',{'task_id':cancelled['id'],'action':'cancel'}))
    terminal=w.wait_task(call,'render_view',cancelled)
    check('real_render_cancel_reaches_actual_terminal',control['action_accepted'] and terminal['task']['state']=='cancelled')
    check('cancelled_render_returns_no_success_image',not any(c['type']=='image' for c in client.last_wrapper['content']))
    (out/'cancelled-render.json').write_text(json.dumps({'control':control,'terminal':terminal},ensure_ascii=False,indent=2),encoding='utf-8')
    device=next(d for d in project['devices'] if d['id'].endswith(':Inspection01'));moved=copy.deepcopy(device);moved['pose']['translation']['x']+=2
    edited=w.unwrap(call('apply_commands',{'base_revision':project['revision'],'commands':[{'kind':'put_object','data':{'kind':'device','data':moved}}]}))
    check('actual_MCP_feedback_followed_by_granted_pose_edit',edited['new_revision']==project['revision']+1 and w.object_at_draft(call,'device',device['id'])==moved)
    current=w.object_at_draft(call,'project',p)
    after=copy.deepcopy(request);after['revision']=current['revision'];after['view']['project_revision']=current['revision']
    changed=w.complete(call,'render_view',after)
    check('actual_product_feedback_of_edited_next_draft',changed['task']['state']=='completed' and changed['result']['data']['scene']['view']==after['view'] and (out/images[0]['path']).read_bytes()!=(out/images[1]['path']).read_bytes())
    for run_id,original in data['workflow']['frozen_runs'].items():
        frozen=copy.deepcopy(request);frozen['revision']=current['revision'];frozen['view']['project_revision']=original['input_ref']['project_revision'];frozen['view']['run_id']=run_id;frozen['view']['view_time_ms']=min(45000,original['computed_until_ms'])
        actual=w.complete(call,'render_view',frozen)
        check('edited_draft_renders_exact_old_Run_source',actual['task']['state']=='completed' and actual['result']['data']['scene']['scene_revision']==current['revision'] and actual['result']['data']['scene']['view']==frozen['view'])
        check('rendering_frozen_Run_never_rewrites_it',w.unwrap(call('get_run',{'run_id':run_id,'query':{'kind':'full_result'}}))['data']==original)
    check('frozen_Run_render_keeps_next_draft_pose',w.object_at_draft(call,'device',device['id'])==moved)
    selected=n['selected'](client,'save_project','edited-layout-project');saved=w.complete(call,'save_project',{'revision':current['revision'],'selected_directory_ref':selected['token']})
    check('actual_visual_pose_edit_saved_as_editable_project',saved['task']['state']=='completed')
    (out/'edited-layout.json').write_text(json.dumps({'before':device,'after':moved,'commit':edited,'save':saved,'saved_directory':selected['path']},ensure_ascii=False,indent=2),encoding='utf-8')
    (out/'render-images.json').write_text(json.dumps(images,ensure_ascii=False,indent=2),encoding='utf-8')
    client.close()
