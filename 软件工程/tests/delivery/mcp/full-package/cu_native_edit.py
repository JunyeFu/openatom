"""One actually generated native device: ports, timeline, tag, paint, tag edit."""
import copy,importlib.util,json
from cu_render_cases import record_image

def execute(n):
    root=n['ROOT'];out=n['OUT'];data=n['INPUT'];check=n['check'];ok=n['ok'];p=data['project']['id'];client=n['Client']();images=[];n['RENDER_IMAGES']=images
    spec=importlib.util.spec_from_file_location('workflow',root/'integrations/agent/workflow.py');w=importlib.util.module_from_spec(spec);spec.loader.exec_module(w)
    def call(op,params,definition=None):
        reply=client.call(op,params,project=p,definition=definition)
        if op=='get_task':record_image(client,reply,out,images,check)
        return reply
    context=call('get_context',{'selection':[]});client.session=context['context']['session_id'];before=w.unwrap(call('get_object',{'object_id':{'kind':'project','id':p},'source':{'kind':'draft'},'node_index':None}))['data']
    reopened=n['MODE']=='render-native-reopen'
    if reopened:
        expected=data['native_definition'];reference={'id':expected['id'],'version':expected['version']}
        snapshot=w.unwrap(call('get_object',{'object_id':{'kind':'module','id':expected['id']},'source':{'kind':'module_version','data':{'module_ref':reference}},'node_index':None}))['data']
        check('relocated_actual_module_package_reopens_exact_committed_definition',snapshot['definition']==expected)
    created=w.unwrap(call('apply_commands',{'base_revision':before['revision'],'commands':[{'kind':'create_definition_draft','data':data['native_definition']}]}));identity=created['scope']['data']['id']
    read={'object_id':{'kind':'definition_draft','id':identity},'source':{'kind':'draft'},'node_index':None};actual=w.unwrap(call('get_object',read,identity))['data']
    check('external_Agent_generated_exact_native_parts_ports_timeline_tag',actual==data['native_definition'] and len(actual['payload']['data']['presentation']['nodes'])==9 and len(actual['payload']['data']['ports'])==3 and len(actual['payload']['data']['presentation']['timeline'])==1 and len(actual['payload']['data']['presentation']['tags'])==1)
    check('creation_is_not_Project_adoption',w.unwrap(call('get_object',{'object_id':{'kind':'project','id':p},'source':{'kind':'draft'},'node_index':None}))['data']==before)
    granted=n['admin']('authority',client.session,profile=1,variant='bind_single_definition');check('real_UI_grant_binds_actual_generated_draft',granted['ok'])
    draft_call=lambda op,params:call(op,params,identity)
    tools=[t['name'] for t in client.rpc('tools/list',{})['result']['tools']];(out/'actual-tools.json').write_text(json.dumps(tools,indent=2),encoding='utf-8')
    request=copy.deepcopy(data['render_request']);request['revision']=1;request['view']['project_revision']=before['revision'];request['object_id']={'kind':'definition_draft','id':identity};request['view']['object_ref']=request['object_id']
    if reopened:request['view']['hall_id']=None
    first=w.complete(draft_call,'render_view',request);check('actual_generated_device_painted_in_its_scope',first['task']['state']=='completed' and first['result']['data']['scene']['scene_revision']==1)
    changed=copy.deepcopy(actual);changed['payload']['data']['presentation']['tags'][0]['label']='New-process receiver edited the delivered native tag' if reopened else 'MCP edited tag after real product feedback'
    commit=w.unwrap(draft_call('apply_commands',{'base_revision':1,'commands':[{'kind':'put_definition','data':changed}]}))
    check('granted_native_tag_edit_preserves_parts_ports_and_timeline',commit['new_revision']==2 and w.unwrap(draft_call('get_object',read))['data']==changed)
    request['revision']=2;second=w.complete(draft_call,'render_view',request)
    check('actual_product_repaints_edited_native_definition',second['task']['state']=='completed' and second['result']['data']['scene']['scene_revision']==2 and len(images)==2 and (out/images[0]['path']).read_bytes()!=(out/images[1]['path']).read_bytes())
    check('generated_device_edit_never_adopts_or_changes_Project',w.unwrap(call('get_object',{'object_id':{'kind':'project','id':p},'source':{'kind':'draft'},'node_index':None}))['data']==before)
    for run_id,original in data['workflow']['frozen_runs'].items():check('generated_native_work_preserves_existing_Run',w.unwrap(call('get_run',{'run_id':run_id,'query':{'kind':'full_result'}}))['data']==original)
    if reopened:
        check('receiver_edits_draft_without_changing_committed_module',w.unwrap(call('get_object',{'object_id':{'kind':'module','id':expected['id']},'source':{'kind':'module_version','data':{'module_ref':reference}},'node_index':None}))['data']==snapshot)
        (out/'native-edit-result.json').write_text(json.dumps({'before':actual,'after':changed,'first_render':first,'second_render':second,'Project_adopted':False,'formal_package_exported':False,'community_published':False,'relocated_package_reopened':True},ensure_ascii=False,indent=2),encoding='utf-8')
        (out/'render-images.json').write_text(json.dumps(images,ensure_ascii=False,indent=2),encoding='utf-8');client.close();return
    permitted=n['admin']('authority',client.session,profile=2,variant='bind_single_definition');check('explicit_UI_version_grant_uses_current_native_draft',permitted['ok'])
    versioned=w.unwrap(draft_call('apply_commands',{'base_revision':2,'commands':[{'kind':'commit_definition_version','data':{'version':'0.2.0'}}]}))
    committed=w.unwrap(draft_call('get_object',read))['data'];check('explicit_native_version_commit_preserves_edited_payload',versioned['new_revision']==3 and committed=={**changed,'version':'0.2.0'})
    selected=n['selected'](client,'export_package','generated-native.sfmodule',file=True)
    exported=w.complete(draft_call,'export_package',{'revision':3,'package_scope':{'purpose':'private_module','scenarios':[],'run_ids':[],'artifact_ids':[],'module_refs':[{'id':committed['id'],'version':committed['version']}],'include_redistributable_assets':False},'selected_destination_ref':selected['token']})
    check('actual_native_module_package_export_completed',exported['task']['state']=='completed' and n['Path'](selected['path']).is_file())
    (out/'native-edit-result.json').write_text(json.dumps({'created':created,'commit':commit,'version_commit':versioned,'before':actual,'after':committed,'first_render':first,'second_render':second,'export':exported,'package_path':selected['path'],'Project_adopted':False,'formal_package_exported':True,'community_published':False},ensure_ascii=False,indent=2),encoding='utf-8');(out/'render-images.json').write_text(json.dumps(images,ensure_ascii=False,indent=2),encoding='utf-8');client.close()
