"""Prepare fixed, source-labeled non-GUI cases before running the actual client."""
import copy,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4]
OUT=ROOT/'.work/delivery'
def write(path,value):
    path.parent.mkdir(parents=True,exist_ok=True)
    path.write_text(json.dumps(value,ensure_ascii=False,indent=2),encoding='utf-8')
OPS=['describe_capabilities','get_context','get_object','apply_commands','render_view','run_scenario','get_run','compare_runs',
     'compute_packaging','get_task','cancel_task','adopt_artifact','import_package','save_project','export_package','export_evaluation','query_module_library','check_factory_draft']
def profile(scenario,edit):
    fields=lambda names:[{'pointer':name,'element_id':None} for name in names]
    grants=[]
    if edit:
        grants=[{'scope':{'kind':'edit','data':{'kind':'project','data':'P1'}},'branches':['put_object','set_project_settings'],
            'objects':[{'target':{'kind':'project','id':'P1'},'fields':fields(['/title']),'can_remove':False},
                       {'target':{'kind':'scenario','id':scenario},'fields':fields(['/revision','/project_revision'])+
                        [{'pointer':'/production_tasks/0/release_at_ms','element_id':'TASK-LOCAL'}],'can_remove':False}],
            'creates':[{'kind':'item_type','max_new_objects':1,'fields':fields(['/revision','/title','/unit','/tracking','/spec_ref','/attributes','/attribute_schema'])}],
            'source_versions':[],'reusable_resources':[]}]
    return {'project_id':'P1','operations':OPS,'definition_drafts':[],'allow_view_control':False,'edit':grants}
def document(role,kind,identity,file,revision=None,module=None):
    return {'role':role,'object_ref':{'kind':kind,'id':identity},'revision':revision,'module_ref':module,'file_ref':{'path':file,'kind':kind},'required':True}
def package(seed,value):
    seed.mkdir(parents=True,exist_ok=False)
    project=value['project'];scenario=value['scenario']
    docs=[document('active_project','project',project['id'],'active.json',project['revision']),
          document('scenario_input','scenario',scenario['id'],'scenario.json',scenario['revision'])]
    write(seed/'active.json',{'schema_version':'1.0','data':project})
    write(seed/'scenario.json',{'schema_version':'1.0','data':scenario})
    for index,definition in enumerate(value['definitions']):
        file=f'module-{index}.json'
        docs.append(document('module_definition','module',definition['id'],file,module={'id':definition['id'],'version':definition['version']}))
        write(seed/file,{'schema_version':'1.0','data':{'definition':definition,'assets':[],'files':[],'dependencies':[],'rights':[],'issues':[]}})
    write(seed/'project.json',{'schema_version':'1.0','data':{'project_id':project['id'],'project_revision':project['revision'],
        'title':project['title'],'generation':1,'documents':docs,'module_refs':project['module_refs'],'assets':[],'missing':[],
        'restart_policy':'restart_from_initial_input'}})
expectations={
 'transport':{'same_Host_session_and_canonical_requests':True,'large_project_title_chars':1200000,
              'managed_resource_uri_prefix':'superfactory-result://','malformed_or_unbound_requests':'reject without mutation'},
 'authority':{'readonly_direct_edit':'PermissionDenied','stale_revision':'RevisionConflict','forged_caller':'JSON-RPC -32602',
              'unauthorized_batch':'atomic no revision change','create_budget':1,'reconnect':'fresh session, no file/task inheritance',
              'explicit_TaskId_access':'read/control separate, original IDs retained, operations and scope still checked'},
 'tasks':{'TaskHandle':'admission only','cancel_race':'actual completed or cancelled terminal, never fabricated cancelled',
          'after_commit_cancel':'terminal_won=true, action_accepted=false','disconnect':'accepted work remains, no replay'},
 'success':{'source':'production/p1-legacy-goal.json','goal_met':True,'accepted_events':3,'goal_quantity_per_day':1,
            'direct_delivery_actual_runs':1,'formats':['pdf','html','csv','json'],'no_forced_edit_or_comparison':True},
 'shortfall':{'source':'quality/quality-original.json','goal_met':False,'accepted_quantity':2,'target_quantity':3,
              'authorized_candidate_field':'production_tasks/TASK-LOCAL/release_at_ms','new_release_ms':1000,
              'max_committed_repairs':3,'max_capacity_runs':4,'frozen_prior_Run':'unchanged'},
 'excluded':{'real_Codex_account':False,'ordinary_GUI':False,'rendered_image_or_visual_acceptance':False,'human_acceptance':None,'whole_package_done':False}}
write(OUT/'expectations.json',expectations)
for mode,relative in [('lifecycle','production/local-assembly-pack.json'),('success','production/p1-legacy-goal.json'),('shortfall','quality/quality-original.json')]:
    source=ROOT/'packages/commands/tests/fixtures'/relative
    value=json.loads(source.read_text(encoding='utf-8'))
    changes=[]
    if mode=='lifecycle':
        value['project']['title']='L'*1200000
        # Exact existing static-check fixture's declared positions, no geometry scaling.
        for device,x in zip(value['project']['devices'],[1,4]):
            device['pose']['translation']['x']=x;device['pose']['translation']['y']=1
        changes=['project.title transport payload only','two device x/y positions from runtime-factory-check/fixture_support.rs']
    stage=OUT/'cases'/mode
    package(stage/'seed',value)
    profiles=[profile(value['scenario']['id'],False),profile(value['scenario']['id'],True)]+[profile(value['scenario']['id'],False) for _ in range(5)] if mode=='lifecycle' else [profile(value['scenario']['id'],True)]
    write(stage/'input.json',value)
    write(stage/'source.json',{'frozen_source':str(source),'controlled_transformations':changes,'expected_source':str(OUT/'expectations.json')})
    write(stage/'case.json',{'root':str(ROOT),'seed':str(stage/'seed'),'python':sys.executable,
        'bridge':str(ROOT/'.work/target/debug/superfactory-mcp-bridge.exe'),'output':str(stage/'actual'),
        'profiles':profiles,'mode':mode,'drafts':[]})
print('Prepared lifecycle, initial-success and independent-shortfall cases; no client executed')
