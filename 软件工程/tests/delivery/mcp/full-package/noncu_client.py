"""Actual stdio consumer; optional captures use the Host's real product renderer.

The controlled Native process remains hidden without interactive desktop input.
"""
import copy,json,queue,subprocess,sys,threading,time,zipfile
from pathlib import Path
BRIDGE,ENDPOINT,HOST,DIRECTORY,MODE,ROOT=sys.argv[1:]
ROOT=Path(ROOT);OUT=Path(DIRECTORY)
INPUT=json.loads((OUT.parent/'input.json').read_text(encoding='utf-8'))
CHECKS=[];CLIENTS=[];ADM=0;REQUESTS=[];ORDINAL=0
TRACE=(OUT/'transcript.jsonl').open('w',encoding='utf-8')
def keep(kind,value): TRACE.write(json.dumps({kind:value},ensure_ascii=False)+'\n');TRACE.flush()
def check(name,condition,**details):
    CHECKS.append({'name':name,'passed':bool(condition),**details})
    assert condition,(name,details)
def ok(reply):
    assert reply['outcome']['status']=='ok',reply
    return reply['outcome']['data']
def code(reply): return reply.get('outcome',{}).get('data',{}).get('code')
def canonical(operation,params,project='P1'):
    global ORDINAL
    ORDINAL+=1
    return {'api_version':'0.1','request_id':f'noncu-{MODE}-{ORDINAL}','project_id':project,
            'scope':{'kind':'workspace' if project is None else 'project'},'operation':operation,'params':params}
def admin(kind,session='',**value):
    global ADM
    ADM+=1;action={'kind':kind,'session':session,**value}
    temporary=OUT/f'admin-{ADM}.tmp'
    temporary.write_text(json.dumps(action,ensure_ascii=False),encoding='utf-8')
    temporary.replace(OUT/f'admin-{ADM}.json')
    response=OUT/f'admin-{ADM}.response.json'
    end=time.monotonic()+30
    while True:
        assert time.monotonic()<end,('Native admin deadline',action)
        if response.exists():
            try:
                result=json.loads(response.read_text(encoding='utf-8'))
                break
            except PermissionError:
                # Observed Windows rename-handle release race; read the same
                # response within the original deadline, never resubmit action.
                pass
        time.sleep(.005)
    keep('trusted_test_administration',{'action':action,'result':result})
    return result
def ui(operation,params,project='P1'): return admin('ui',request=canonical(operation,params,project))
class Client:
    def __init__(self):
        self.index=len(CLIENTS)+1;self.number=0
        self.stderr=(OUT/f'bridge-{self.index}.stderr.txt').open('wb')
        self.process=subprocess.Popen([BRIDGE,'--host-pipe',ENDPOINT,'--host-instance-id',HOST],stdin=subprocess.PIPE,stdout=subprocess.PIPE,
            stderr=self.stderr,creationflags=subprocess.CREATE_NO_WINDOW)
        self.lines=queue.Queue();CLIENTS.append(self)
        def reader():
            for line in self.process.stdout: self.lines.put(json.loads(line))
            self.lines.put(None)
        threading.Thread(target=reader,daemon=True).start()
        result=self.rpc('initialize',{'protocolVersion':'2025-06-18','capabilities':{},'clientInfo':{'name':'noncu-independent-protocol','version':'1'}},fragmented=True)
        check(f'initialize_real_stdio_peer_{self.index}','result' in result)
        self.send({'jsonrpc':'2.0','method':'notifications/initialized'})
    def send(self,value,fragmented=False):
        keep('stdio_send',value)
        data=json.dumps(value,ensure_ascii=False,separators=(',',':')).encode('utf-8')+b'\n'
        if fragmented:
            for index in range(0,len(data),7): self.process.stdin.write(data[index:index+7]);self.process.stdin.flush()
        else: self.process.stdin.write(data);self.process.stdin.flush()
    def receive(self,identity):
        while True:
            response=self.lines.get(timeout=30)
            assert response is not None,('bridge EOF',self.process.poll())
            keep('stdio_receive',response)
            if response.get('id')==identity: return response
    def rpc(self,method,params,fragmented=False):
        self.number+=1
        self.send({'jsonrpc':'2.0','id':self.number,'method':method,'params':params},fragmented)
        return self.receive(self.number)
    def raw_tool(self,request): return self.rpc('tools/call',{'name':'superfactory_'+request['operation'],'arguments':request})
    def call(self,operation,params,project='P1',definition=None):
        request=canonical(operation,params,project);REQUESTS.append(request)
        if definition is not None:request['scope']={'kind':'definition_draft','data':definition}
        wrapper=self.raw_tool(request)
        assert 'result' in wrapper,wrapper
        result=wrapper['result'];outer=result['structuredContent'];reply=outer
        if outer['outcome']['status']=='managed':
            resources=[item['resource'] for item in result['content'] if item['type']=='resource']
            assert len(resources)==1 and resources[0]['mimeType']=='application/json'
            resource=resources[0]
            assert resource['uri']=='superfactory-result://'+outer['outcome']['data']['result_id']
            assert len(resource['text'].encode('utf-8'))==outer['outcome']['data']['byte_length']
            reply=json.loads(resource['text']);assert reply['context']==outer['context']
            assert reply['outcome']['status']!='managed'
        assert reply['request_id']==request['request_id']
        assert bool(result.get('isError',False))==(reply['outcome']['status']=='error')
        if reply['context'] is not None: assert reply['context']['host_instance_id']==HOST
        self.last_wrapper=result
        return reply
    def context(self):
        reply=self.call('get_context',{'selection':[]});self.session=reply['context']['session_id'];return ok(reply)
    def object(self,kind='project',identity='P1'):
        return ok(self.call('get_object',{'object_id':{'kind':kind,'id':identity},'source':{'kind':'draft'},'node_index':None}))['data']
    def close(self,expected=0):
        if not self.process.stdin.closed: self.process.stdin.close()
        result=self.process.wait(timeout=10);self.stderr.close()
        assert result==expected,('bridge exit',result,expected)
def terminal(client,handle,project='P1'):
    assert 'id' in handle and 'state' in handle and 'result' not in handle
    deadline=time.monotonic()+120
    while True:
        state=ok(client.call('get_task',{'task_id':handle['id']},project))
        assert all(state['task'][key]==handle[key] for key in ('id','scope','kind','input_revision'))
        if state['task']['state'] in ('completed','failed','cancelled'): return state
        assert time.monotonic()<deadline,handle
        time.sleep(.005)
def complete(client,handle,operation,project='P1'):
    state=terminal(client,handle,project)
    assert state['originating_operation']==operation and state['task']['state']=='completed',state
    assert state['result'] is not None and state['failure'] is None
    return state['result']['data']
def run(client):
    project=client.object();scenario=client.object('scenario',INPUT['scenario']['id'])
    handle=ok(client.call('run_scenario',{'scenario_id':scenario['id'],'input_revision':project['revision'],
        'observation_window':scenario['window'],'intent':{'kind':'project_scenario'}}))
    summary=complete(client,handle,'run_scenario')
    full=ok(client.call('get_run',{'run_id':summary['run_id'],'query':{'kind':'full_result'}}))['data']
    assert full['id']==summary['run_id'] and full['input_ref']==summary['input']
    assert full['evidence_origin']=='actual_software'
    return handle,summary,full
def settings(project,title):
    return {'kind':'set_project_settings','data':{**copy.deepcopy({key:project[key] for key in ('time_reference','generation_context','sources','setup_policies')}),'title':title}}
def apply(client,revision,commands): return client.call('apply_commands',{'base_revision':revision,'commands':commands})
def eof_removed(session):
    end=time.monotonic()+5
    while time.monotonic()<end:
        reply=admin('probe_eof',session)
        if code(reply)=='PermissionDenied' and reply['context'] is None: return True
        time.sleep(.005)
    return False
def selected(client,purpose,relative,file=False): return admin('select',client.session,purpose=purpose,relative=relative,file=file)
def query_library(client):
    all_categories=['items','devices','resources','tasks','operations','transport','templates','professional','media']
    result=ok(client.call('query_module_library',{'category_filters':all_categories,'industry':None,'product_family':None,
        'site_envelope':None,'source_scope':['bundled','project_packages']}))
    expected=json.loads((ROOT/'modules/templates/carton-factory/candidate/module.json').read_text(encoding='utf-8'))['data']['definition']
    check('actual_bundled_template_exact_ref_discovered',any(c['module_ref']=={'id':expected['id'],'version':expected['version']} for c in result['candidates']))
    missing=ok(client.call('query_module_library',{'category_filters':['templates'],'industry':'noncu-explicitly-unsupported-industry',
        'product_family':'noncu-no-such-family','site_envelope':None,'source_scope':['bundled']}))
    known_refs=[candidate['module_ref'] for candidate in result['candidates']]
    check('missing_template_has_no_applicable_or_invented_candidate',all(not candidate['applicable'] and candidate['missing'] and candidate['module_ref'] in known_refs for candidate in missing['candidates']))
    (OUT/'library.json').write_text(json.dumps(result,ensure_ascii=False,indent=2),encoding='utf-8')
def local_goal():
    return {'industry':'self-built local assembly fixture','product_family':'packaged-assembly','product_specification_ref':None,
        'accepted_quantity_per_day':{'value':{'coefficient':2,'decimal_places':0},'unit':'piece'},
        'acceptance_scope':{'product_ref':{'id':'PACKAGE','revision':'r1'},'unit':'piece','destination':'packages-B','stage':'factory_released'},
        'production_calendar_ref':'ALL','observation_window':{'start_ms':0,'end_ms':240000},
        'site_envelope':{'regions':[{'id':'declared-usable-floor','kind':'usable','polygon_m':[{'x':0,'y':0},{'x':20,'y':0},{'x':20,'y':10},{'x':0,'y':10}],'clearance_m':None}],
            'length_m':20,'width_m':10,'height_m':4,'usable_polygon_m':[],'clearance_m':0},'existing_equipment':[],'constraints':[]}
def check_factory(client):
    before=client.object();goal=local_goal()
    params={'revision':before['revision'],'generation_goal':goal,'check_scope':{'objects':[{'kind':'project','id':'P1'}],
        'check_references':True,'check_business':True,'check_static_layout':True}}
    result=ok(client.call('check_factory_draft',params))
    check('same_Host_complete_static_check_passes',all(c['outcome']=='pass' for c in result['checks']),count=len(result['checks']))
    missing=copy.deepcopy(params);missing['generation_goal']['production_calendar_ref']=None
    result_missing=ok(client.call('check_factory_draft',missing))
    check('missing_calendar_remains_explicitly_undetermined',any(c['outcome']=='undetermined' for c in result_missing['checks']))
    conflict=copy.deepcopy(params)
    conflict['generation_goal']['site_envelope']['length_m']=2
    conflict['generation_goal']['site_envelope']['regions'][0]['polygon_m']=[{'x':0,'y':0},{'x':2,'y':0},{'x':2,'y':10},{'x':0,'y':10}]
    conflicted=ok(client.call('check_factory_draft',conflict))
    check('smaller_declared_site_reports_object_conflict',any(c['outcome']=='fail' for c in conflicted['checks']))
    stale=copy.deepcopy(params);stale['revision']=before['revision']+1
    check('static_checker_rejects_wrong_revision',code(client.call('check_factory_draft',stale))=='RevisionConflict')
    check('all_static_queries_preserve_entire_draft',client.object()==before)
    (OUT/'static-checks.json').write_text(json.dumps({'complete':result,'missing_calendar':result_missing,'site_conflict':conflicted},ensure_ascii=False,indent=2),encoding='utf-8')
def lifecycle():
    wrong=subprocess.run([BRIDGE,'--host-pipe',ENDPOINT,'--host-instance-id','desktop-host-0'],capture_output=True,creationflags=subprocess.CREATE_NO_WINDOW)
    check('wrong_OS_server_identity_denied_before_authorization',wrong.returncode==2 and b'selected desktop Host' in wrong.stderr)
    first=Client();first.context();project=first.object()
    check('actual_large_original_resource_complete',len(project['title'])==1200000 and first.last_wrapper['structuredContent']['outcome']['status']=='managed')
    names={tool['name'] for tool in first.rpc('tools/list',{})['result']['tools']}
    check('readonly_tools_hide_apply_commands','superfactory_apply_commands' not in names)
    denied=apply(first,project['revision'],[settings(project,'ungranted')])
    check('direct_unlisted_edit_retains_Host_permission_error',code(denied)=='PermissionDenied')
    forged=canonical('get_context',{'selection':[]});forged['caller']={'origin':'interactive_ui'}
    check('stdio_caller_injection_rejected_by_canonical_schema',first.raw_tool(forged).get('error',{}).get('code')==-32602)
    check('foreign_project_scope_denied',code(first.call('get_context',{'selection':[]},'not-granted'))=='PermissionDenied')
    query_library(first);check_factory(first)
    session1=first.session;first.close();check('EOF_revokes_actual_shared_Host_caller',eof_removed(session1))
    second=Client();second.context();project=second.object();session2=second.session
    check('reconnection_uses_fresh_verified_session',session2!=session1)
    edit=settings(project,'noncu external title')
    bad=settings(project,'must not commit');bad['data']['time_reference']['label']='not granted'
    check('mixed_allowed_and_denied_batch_is_atomic',code(apply(second,project['revision'],[edit,bad]))=='PermissionDenied' and second.object()==project)
    commit=ok(apply(second,project['revision'],[edit]));revision=commit['new_revision']
    check('external_title_commits_once',revision==project['revision']+1)
    check('old_revision_cannot_overwrite',code(apply(second,project['revision'],[edit]))=='RevisionConflict')
    item={'id':'noncu-created-item','revision':'r1','title':'explicit owned creation','unit':'piece','tracking':'individual','spec_ref':None,'attributes':{},'attribute_schema':[]}
    command={'kind':'put_object','data':{'kind':'item_type','data':item}}
    revision=ok(apply(second,revision,[command]))['new_revision']
    check('actual_created_object_readback',second.object('item_type',item['id'])==item)
    second_item=copy.deepcopy(command);second_item['data']['data']['id']='noncu-excess-item'
    check('cumulative_creation_budget_enforced',code(apply(second,revision,[second_item]))=='PermissionDenied')
    check('secondary_window_cannot_change_authority',not admin('secondary_authority',session2,profile=1)['ok'])
    check('field_downgrade_accepted',admin('authority',session2,profile=1,variant='remove_fields')['ok'])
    check('removed_field_denied',code(apply(second,revision,[edit]))=='PermissionDenied')
    check('same_session_restore_accepted',admin('authority',session2,profile=1)['ok'])
    check('restore_does_not_refund_creation_budget',code(apply(second,revision,[second_item]))=='PermissionDenied')
    token=selected(second,'save_project','lifecycle-save')
    admission=ok(second.call('save_project',{'revision':revision,'selected_directory_ref':token['token']}))
    cancelled=ok(second.call('cancel_task',{'task_id':admission['id'],'action':'cancel'}))
    state=terminal(second,admission)
    check('actual_cancel_commit_race_reaches_real_terminal',state['task']['state'] in ('cancelled','completed') and (cancelled['action_accepted'] or cancelled['terminal_won']),terminal=state['task']['state'])
    completed_save=ok(second.call('save_project',{'revision':revision,'selected_directory_ref':token['token']}))
    complete(second,completed_save,'save_project')
    late=ok(second.call('cancel_task',{'task_id':completed_save['id'],'action':'cancel'}))
    check('completed_commit_wins_later_cancel',late['terminal_won'] and not late['action_accepted'])
    scenario=second.object('scenario',INPUT['scenario']['id'])
    admitted_run=ok(second.call('run_scenario',{'scenario_id':scenario['id'],'input_revision':revision,'observation_window':scenario['window'],'intent':{'kind':'project_scenario'}}))
    admitted_save=ok(second.call('save_project',{'revision':revision,'selected_directory_ref':token['token']}))
    second.close();check('EOF_after_admission_revokes_owner_only',eof_removed(session2))
    third=Client();third.context();session3=third.session
    check('new_session_inherits_no_file_token',code(third.call('save_project',{'revision':revision,'selected_directory_ref':token['token']}))=='PermissionDenied')
    check('new_session_inherits_no_task',code(third.call('get_task',{'task_id':admitted_save['id']}))=='PermissionDenied')
    check('secondary_window_cannot_grant_task',not admin('secondary_task',session3,task=admitted_save['id'])['ok'])
    check('explicit_original_TaskId_read_grant',admin('task_read',session3,task=admitted_save['id'])['ok'])
    resumed=terminal(third,admitted_save)
    check('same_accepted_save_finishes_after_disconnect',resumed['task']['state']=='completed' and resumed['task']['id']==admitted_save['id'])
    check('readonly_task_grant_has_no_controls',resumed['allowed_controls']==[] and code(third.call('cancel_task',{'task_id':admitted_save['id'],'action':'cancel'}))=='PermissionDenied')
    check('explicit_task_control_grant',admin('task_control',session3,task=admitted_save['id'])['ok'])
    check('reconnected_completed_task_still_wins_cancel',ok(third.call('cancel_task',{'task_id':admitted_save['id'],'action':'cancel'}))['terminal_won'])
    check('unsupported_control_is_not_added',code(third.call('cancel_task',{'task_id':admitted_save['id'],'action':'pause_calculation'}))=='UnsupportedCapability')
    check('one_TaskId_grant_does_not_grant_Engine_task',code(third.call('get_task',{'task_id':admitted_run['id']}))=='PermissionDenied')
    check('explicit_original_Engine_TaskId_read_grant',admin('task_read',session3,task=admitted_run['id'])['ok'])
    resumed_run=complete(third,admitted_run,'run_scenario')
    frozen=ok(third.call('get_run',{'run_id':resumed_run['run_id'],'query':{'kind':'full_result'}}))['data']
    check('reconnect_keeps_actual_Run_identity_and_input',frozen['id']==resumed_run['run_id'] and frozen['input_ref']==resumed_run['input'])
    check('operation_downgrade_applies_to_granted_task',admin('authority',session3,profile=2,variant='drop_operations')['ok'] and code(third.call('get_task',{'task_id':admitted_save['id']}))=='PermissionDenied')
    admin('authority',session3,profile=2)
    check('scope_downgrade_applies_to_granted_task',admin('authority',session3,profile=2,variant='wrong_project')['ok'] and code(third.call('get_task',{'task_id':admitted_save['id']}))=='PermissionDenied')
    admin('authority',session3,profile=2)
    check('native_revoke_accepted',admin('revoke',session3)['ok'])
    third.number+=1
    third.send({'jsonrpc':'2.0','id':third.number,'method':'tools/call','params':{
        'name':'superfactory_get_context','arguments':canonical('get_context',{'selection':[]})}})
    check('revoked_transport_cannot_dispatch_another_request',third.lines.get(timeout=10) is None)
    third.close(expected=76)
    check('revocation_removes_shared_Host_caller',eof_removed(session3))
    check('no_accepted_action_replayed_on_reconnect',sum(r['operation']=='run_scenario' for r in REQUESTS)==1)
def business():
    client=Client();client.context();initial=client.object();query_library(client)
    def static_gate(label):
        current=client.object();scenario=client.object('scenario',INPUT['scenario']['id'])
        result=ok(client.call('check_factory_draft',{'revision':current['revision'],'generation_goal':scenario['acceptance_goal'],
            'check_scope':{'objects':[{'kind':'project','id':'P1'}],'check_references':True,'check_business':True,'check_static_layout':True}}))
        check(label+'_static_query_preserves_original_goal_and_draft',client.object()==current and client.object('scenario',scenario['id'])==scenario)
        if MODE=='complete':check('complete_declared_layout_all_static_checks_pass_before_Run',all(c['outcome']=='pass' for c in result['checks']))
        # Preserve fail/undetermined statuses from these independent fixtures;
        # a completed capacity run cannot certify their static-layout gate.
        (OUT/(label+'-static-gate.json')).write_text(json.dumps(result,ensure_ascii=False,indent=2),encoding='utf-8')
    static_gate('original')
    handle,summary,frozen=run(client)
    check('actual_nonempty_Run_completed',len(frozen['jobs'])>0 and frozen['computation_status']=='completed')
    expected=MODE in ('success','complete')
    check('fixed_original_goal_result',frozen['goal_met']==expected,goal_met=frozen['goal_met'])
    accepted=sum(event['quantity']['value']['coefficient']*10**(-event['quantity']['value']['decimal_places']) for event in frozen['acceptance_events'])
    check('fixed_original_accepted_output',accepted==(3 if expected else 2),accepted=accepted)
    for query in [{'kind':'events','data':{'interval':None,'page':{'after_index':None,'limit':100}}},
                  {'kind':'objects_at','data':{'at_ms':0,'objects':[],'page':{'after_index':None,'limit':100}}}]:
        view=ok(client.call('get_run',{'run_id':frozen['id'],'query':query}))['data']
        check('actual_timeline_'+query['kind'],view['run_id']==frozen['id'] and (query['kind']!='objects_at' or view['at_ms']==0))
    (OUT/'original-run.json').write_text(json.dumps(frozen,ensure_ascii=False,indent=2),encoding='utf-8')
    if not expected:
        scenario=client.object('scenario',INPUT['scenario']['id']);changed=copy.deepcopy(scenario)
        # Metadata names the current Scenario and Project base; the Host owns
        # the resulting revision increment after the command is committed.
        changed['production_tasks'][0]['release_at_ms']=1000
        edited=apply(client,initial['revision'],[{'kind':'put_object','data':{'kind':'scenario','data':changed}}])
        check('explicit_schedule_only_candidate_commits',edited['outcome']['status']=='ok',response=edited)
        static_gate('repaired')
        _,new_summary,new_run=run(client)
        check('repair_uses_new_actual_Run_and_keeps_target',new_run['id']!=frozen['id'] and new_run['input_ref']['project_revision']==initial['revision']+1 and client.object('scenario',scenario['id'])['acceptance_goal']==scenario['acceptance_goal'])
        comparison=ok(client.call('compare_runs',{'run_ids':[frozen['id'],new_run['id']],'metric_scope':{'keys':[],'objects':[],'acceptance':[]}}))
        check('actual_two_Run_comparison_consumed',comparison is not None)
        check('bounded_shortfall_does_not_invent_target_met',not new_run['goal_met'])
        (OUT/'shortfall-policy.json').write_text(json.dumps({'committed_repair_rounds':1,'capacity_runs':2,'max_rounds':3,'target_not_reduced':True,
            'stop':'No accepted-output gain from permitted release-only change; no authority to alter quality/BOM/supply/cycle. Preserve shortfall and both real Runs.'},ensure_ascii=False,indent=2),encoding='utf-8')
    current=client.object();reports=selected(client,'export_evaluation','reports')
    report_task=ok(client.call('export_evaluation',{'revision':current['revision'],'report_mode':'simulation_evaluation','scenario_refs':[],
        'run_refs':[frozen['id']],'metric_scope':{'keys':[],'objects':[],'acceptance':[]},'formats':['pdf','html','csv','json'],'selected_destination_ref':reports['token']}))
    report=complete(client,report_task,'export_evaluation')
    files=[Path(reports['path'])/file['path'] for file in report['files']]
    check('actual_four_format_files_exist',all(any(file.suffix=='.'+extension and file.is_file() and file.stat().st_size>0 for file in files) for extension in ['pdf','html','csv','json']))
    pdf=next(file for file in files if file.suffix=='.pdf');check('actual_pdf_has_PDF_signature',pdf.read_bytes().startswith(b'%PDF-'))
    actual_json=next(file for file in files if file.suffix=='.json')
    report_json=json.loads(actual_json.read_text(encoding='utf-8'))
    check('report_consumes_selected_actual_Run',frozen['id'] in json.dumps(report_json,ensure_ascii=False))
    saved=selected(client,'save_project','saved')
    saved_task=ok(client.call('save_project',{'revision':current['revision'],'selected_directory_ref':saved['token']}))
    saved_result=complete(client,saved_task,'save_project')
    check('editable_project_actual_save_completed',saved_result['saved_revision']==current['revision'] and (Path(saved['path'])/'project.json').is_file())
    check('read_report_save_preserve_frozen_Run',ok(client.call('get_run',{'run_id':frozen['id'],'query':{'kind':'full_result'}}))['data']==frozen)
    if expected:
        check('initial_success_direct_delivery_has_one_Run_no_edit_or_compare',sum(r['operation']=='run_scenario' for r in REQUESTS)==1 and not any(r['operation'] in ('apply_commands','compare_runs') for r in REQUESTS) and client.object()==initial)
    (OUT/'report-output.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
    client.close()
def handoff():
    client=Client();client.context();initial=client.object()
    old=json.loads((OUT.parent/'expected-run.json').read_text(encoding='utf-8'))
    check('new_Native_process_reads_saved_original_Run_exactly',ok(client.call('get_run',{'run_id':old['id'],'query':{'kind':'full_result'}}))['data']==old)
    _,_,rerun=run(client)
    check('reopened_editable_inputs_reach_actual_rerun',rerun['id']!=old['id'] and rerun['input_ref']['project_revision']==initial['revision'] and rerun['goal_met'])
    for kind,identity in [('project','P1'),('scenario',INPUT['scenario']['id'])]:
        snapshots=[]
        for frozen in (old,rerun):
            snapshots.append(ok(client.call('get_object',{'object_id':{'kind':kind,'id':identity},
                'source':{'kind':'run_snapshot','data':{'run_id':frozen['id'],'at_ms':0}},'node_index':None}))['data'])
        check('reopened_'+kind+'_frozen_snapshot_content_identical',snapshots[0]==snapshots[1])
    normalized=copy.deepcopy(rerun);normalized['id']=old['id']
    for field in ('frozen_project_file','frozen_scenario_file'):
        # Separate Run archives own separate storage paths. Their frozen object
        # contents were compared above; retain every other result/input field.
        normalized['input_ref'][field]['path']=old['input_ref'][field]['path']
    check('reopened_same_inputs_reproduce_business_Run_except_identity_and_archive_paths',normalized==old)
    edited=ok(apply(client,initial['revision'],[settings(initial,'noncu editable consumer title')]))
    current=client.object()
    check('reopened_native_draft_remains_editable',edited['new_revision']==initial['revision']+1 and current['title']=='noncu editable consumer title')
    check('later_edit_preserves_saved_original_Run',ok(client.call('get_run',{'run_id':old['id'],'query':{'kind':'full_result'}}))['data']==old)
    destination=selected(client,'export_package','editable-handoff.zip',file=True)
    task=ok(client.call('export_package',{'revision':current['revision'],'selected_destination_ref':destination['token'],
        'package_scope':{'purpose':'handoff_project','scenarios':[INPUT['scenario']['id']],'run_ids':[old['id'],rerun['id']],'artifact_ids':[],'module_refs':[],
            'include_redistributable_assets':False}}))
    complete(client,task,'export_package')
    archive=Path(destination['path'])
    check('actual_editable_ZIP_export_completed',archive.is_file() and zipfile.is_zipfile(archive))
    with zipfile.ZipFile(archive) as zipped:check('actual_ZIP_has_project_manifest',any(name.endswith('project.json') for name in zipped.namelist()))
    imported=selected(client,'import_package','editable-handoff.zip',file=True)
    check('external_workspace_open_requires_actual_UI',code(client.call('import_package',{'selected_file_ref':imported['token'],
        'import_policy':'open_project','base_revision':None},None))=='PermissionDenied' and client.object()==current)
    imported=admin('select','desktop-main-ui',purpose='import_package',relative='editable-handoff.zip',file=True)
    task=ok(ui('import_package',{'selected_file_ref':imported['token'],'import_policy':'open_project','base_revision':None},None))
    check('controlled_UI_import_task_explicitly_shared_for_read_only',admin('task_read',client.session,task=task['id'])['ok'])
    draft=complete(client,task,'import_package',None)
    check('actual_ZIP_import_is_proposal_not_implicit_adoption',client.object()==current)
    command={'kind':'adopt_imported_draft','data':{'task_id':task['id'],'choices':{'chosen_policy':'open_project',
        'objects_to_create':draft['proposed_objects'],'module_versions':draft['proposed_definitions'],
        'reused_bindings':{'item_types':{},'resources':{},'targets':{},'ports':{}},'selected_assets':[],
        'accept_missing_as_draft':False,'departure':'discard_after_tasks_terminal'}}}
    check('ungranted_adoption_branch_does_not_modify_draft',code(client.call('apply_commands',{'base_revision':None,'commands':[command]},None))=='PermissionDenied' and client.object()==current)
    client.close()
    ok(ui('apply_commands',{'base_revision':None,'commands':[command]},None))
    fresh=Client();fresh.context()
    check('explicit_controlled_adoption_reopens_same_saved_draft',fresh.object()==current)
    for frozen in (old,rerun):
        check('explicit_adoption_preserves_Run_'+frozen['id'],ok(fresh.call('get_run',{'run_id':frozen['id'],'query':{'kind':'full_result'}}))['data']==frozen)
    fresh.close()
def nine():
    client=Client();client.context();before=client.object()
    _,_,frozen=run(client)
    definitions=json.loads((OUT.parent/'drafts.json').read_text(encoding='utf-8'))
    created=json.loads((OUT/'controlled-draft-setup.json').read_text(encoding='utf-8'))
    check('nine_Host_allocated_draft_identities_are_distinct',len(created)==9 and len({c['scope']['data']['id'] for c in created})==9)
    seen=[]
    for expected,commit in zip(definitions,created):
        identity=commit['scope']['data']['id'];category=expected['payload']['category']
        params={'object_id':{'kind':'definition_draft','id':identity},'source':{'kind':'draft'},'node_index':None}
        reply=client.call('get_object',params,definition=identity)
        actual=ok(reply)['data']
        check(category+'_actual_definition_payload_roundtrip',actual==expected and reply['context']['scope_revision']==1)
        changed=copy.deepcopy(actual);changed['title']='noncu edited '+category
        commit=ok(client.call('apply_commands',{'base_revision':1,'commands':[{'kind':'put_definition','data':changed}]},definition=identity))
        readback=client.call('get_object',params,definition=identity)
        check(category+'_scoped_external_title_edit_preserves_payload',ok(readback)['data']==changed and commit['new_revision']==2 and readback['context']['scope_revision']==2)
        seen.append(category)
    check('nine_actual_payload_categories_consumed',set(seen)=={'items','devices','resources','tasks','operations','transport','templates','professional','media'})
    foreign=client.call('get_object',{'object_id':{'kind':'definition_draft','id':'outside-draft-grant'},'source':{'kind':'draft'},'node_index':None},definition='outside-draft-grant')
    check('definition_draft_scope_remains_explicit',code(foreign)=='PermissionDenied')
    check('definition_generation_does_not_implicitly_adopt_or_change_Project',client.object()==before)
    check('nine_draft_edits_preserve_existing_Run',ok(client.call('get_run',{'run_id':frozen['id'],'query':{'kind':'full_result'}}))['data']==frozen)
    client.close()
def generate():
    client=Client();client.context();before=client.object();_,_,frozen=run(client)
    definitions=json.loads((OUT.parent/'drafts.json').read_text(encoding='utf-8'))
    created=[]
    for definition in definitions:
        commit=ok(apply(client,before['revision'],[{'kind':'create_definition_draft','data':definition}]))
        identity=commit['scope']['data']['id'];category=definition['payload']['category']
        check(category+'_external_creation_returns_Host_identity',commit['scope']['kind']=='definition_draft' and commit['new_revision']==1 and identity!=definition['id'])
        reply=client.call('get_object',{'object_id':{'kind':'definition_draft','id':identity},'source':{'kind':'draft'},'node_index':None},definition=identity)
        check(category+'_externally_created_payload_matches_fixed_DTO',ok(reply)['data']==definition and reply['context']['scope_revision']==1)
        created.append(commit)
    check('nine_external_creations_have_distinct_runtime_identities',len({c['scope']['data']['id'] for c in created})==9)
    check('definition_creation_budget_is_cumulative',code(apply(client,before['revision'],[{'kind':'create_definition_draft','data':definitions[0]}]))=='PermissionDenied')
    check('external_generation_is_not_Project_adoption',client.object()==before)
    check('external_generation_preserves_frozen_Run',ok(client.call('get_run',{'run_id':frozen['id'],'query':{'kind':'full_result'}}))['data']==frozen)
    (OUT/'external-creations.json').write_text(json.dumps(created,ensure_ascii=False,indent=2),encoding='utf-8')
    client.close()
def transport_cancel():
    client=Client();context=client.context();scenario=client.object('scenario',INPUT['scenario']['id'])
    handle=ok(client.call('run_scenario',{'scenario_id':scenario['id'],'input_revision':context['project']['revision'],
        'observation_window':scenario['window'],'intent':{'kind':'project_scenario'}}))
    client.number+=1
    request=canonical('get_object',{'object_id':{'kind':'project','id':'P1'},'source':{'kind':'draft'},'node_index':None})
    client.send({'jsonrpc':'2.0','id':client.number,'method':'tools/call','params':{'name':'superfactory_get_object','arguments':request}})
    client.send({'jsonrpc':'2.0','method':'notifications/cancelled','params':{'requestId':client.number,'reason':'controlled transport read cancellation; never business cancel_task'}})
    reply=client.lines.get(timeout=30)
    if reply is None:
        client.close(expected=75)
        observed='active_MCP_request_cancelled_bridge_exit75'
    else:
        keep('stdio_receive',reply)
        check('cancellation_race_returns_whole_response_or_RPC_error',reply.get('id')==client.number and ('result' in reply or 'error' in reply))
        client.close()
        observed='response_won_transport_cancel_race'
    fresh=Client();fresh.context()
    check('transport_cancel_does_not_reauthorize_old_TaskId',code(fresh.call('get_task',{'task_id':handle['id']}))=='PermissionDenied')
    check('known_original_TaskId_explicitly_reauthorized_after_transport_cancel',admin('task_read',fresh.session,task=handle['id'])['ok'])
    completed=complete(fresh,handle,'run_scenario')
    full=ok(fresh.call('get_run',{'run_id':completed['run_id'],'query':{'kind':'full_result'}}))['data']
    check('MCP_request_cancel_never_cancels_accepted_Engine_work',full['computation_status']=='completed' and completed['input']['project_revision']==context['project']['revision'])
    check('transport_cancel_does_not_resubmit_or_business_cancel',sum(r['operation']=='run_scenario' for r in REQUESTS)==1 and not any(r['operation']=='cancel_task' for r in REQUESTS))
    (OUT/'transport-cancel.json').write_text(json.dumps({'observed':observed,'known_TaskId':handle['id'],'actual_completed_RunId':completed['run_id'],
        'business_cancel_called':False,'admission_replayed':False},ensure_ascii=False,indent=2),encoding='utf-8')
    fresh.close()
failure=None
try:
    if MODE=='lifecycle':lifecycle()
    elif MODE in ('success','complete','shortfall'):business()
    elif MODE=='handoff':handoff()
    elif MODE=='nine':nine()
    elif MODE=='generate':generate()
    elif MODE=='cancel':transport_cancel()
    elif MODE.startswith('workflow-'):
        import cu_workflow_cases
        cu_workflow_cases.execute(globals())
    elif MODE in ('render-native-edit','render-native-reopen'):
        import cu_native_edit
        cu_native_edit.execute(globals())
    elif MODE.startswith('render-'):
        import cu_render_cases
        cu_render_cases.execute(globals())
    else:raise AssertionError(MODE)
except Exception as error:
    failure=repr(error)
    raise
finally:
    for client in CLIENTS:
        if client.process.poll() is None:
            if not client.process.stdin.closed:client.process.stdin.close()
            client.process.wait(timeout=10)
            client.stderr.close()
    (OUT/'client-result.json').write_text(json.dumps({'mode':MODE,'checks':CHECKS,'failure':failure,'passed':sum(c['passed'] for c in CHECKS),
        'failed':sum(not c['passed'] for c in CHECKS),'actual_stdio':True,'actual_Native_Host':True,'actual_Codex':False,
        'GUI_or_rendered_images_executed':bool(globals().get('RENDER_IMAGES',[])),'render_images':globals().get('RENDER_IMAGES',[]),'human_acceptance':None,'whole_package_done':False},ensure_ascii=False,indent=2),encoding='utf-8')
    TRACE.close()
