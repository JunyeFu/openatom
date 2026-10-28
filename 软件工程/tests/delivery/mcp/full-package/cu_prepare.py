"""Freeze actual Host previews into narrow test grants before starting any peer."""
import copy,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4];OUT=ROOT/'.work/cu-delivery'
def read(p):return json.loads(p.read_text(encoding='utf-8'))
def write(p,v):p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(v,ensure_ascii=False,indent=2),encoding='utf-8')
def leaves(v,path=''):
    if isinstance(v,dict):return [p for k,x in v.items() for p in leaves(x,path+'/'+k.replace('~','~0').replace('/','~1'))]
    if isinstance(v,list):return [p for i,x in enumerate(v) for p in leaves(x,path+'/'+str(i))]
    return [path]
def fields(v):return [{'pointer':p,'element_id':None} for p in sorted(set(leaves({k:x for k,x in v.items() if k!='id'})))]
def roles():
    value=read(ROOT/'packages/commands/tests/fixtures/production/p1-legacy-goal.json')
    definitions=[];sources=[]
    for category in ['items','devices','resources','tasks','operations','transport','templates','professional','media']:
        source=ROOT/'tools/type-contract-proof/fixtures'/('module-'+category+'.json')
        definition=read(source)['data']['definition'];definition['id']='cu.draft.'+category
        definitions.append(definition);sources.append(str(source))
    for mode in ['generate','nine']:
        stage=OUT/'cases'/mode;seed=stage/'seed';seed.mkdir(parents=True,exist_ok=False)
        docs=[]
        def document(role,kind,identity,name,revision=None,reference=None):
            docs.append({'role':role,'object_ref':{'kind':kind,'id':identity},'revision':revision,'module_ref':reference,'file_ref':{'path':name,'kind':kind},'required':True})
        project=value['project'];scenario=value['scenario']
        document('active_project','project',project['id'],'active.json',project['revision']);write(seed/'active.json',{'schema_version':'1.0','data':project})
        document('scenario_input','scenario',scenario['id'],'scenario.json',scenario['revision']);write(seed/'scenario.json',{'schema_version':'1.0','data':scenario})
        for index,definition in enumerate(value['definitions']):
            name='module-'+str(index)+'.json';document('module_definition','module',definition['id'],name,reference={'id':definition['id'],'version':definition['version']})
            write(seed/name,{'schema_version':'1.0','data':{'definition':definition,'assets':[],'files':[],'dependencies':[],'rights':[],'issues':[]}})
        write(seed/'project.json',{'schema_version':'1.0','data':{'project_id':project['id'],'project_revision':project['revision'],'title':project['title'],
            'generation':1,'documents':docs,'module_refs':project['module_refs'],'assets':[],'missing':[],'restart_policy':'restart_from_initial_input'}})
        if mode=='generate':
            paths=sorted({f['pointer'] for d in definitions for f in fields(d)})
            grants=[{'scope':{'kind':'edit','data':{'kind':'project','data':'P1'}},'branches':['create_definition_draft'],'objects':[],
                'creates':[{'kind':'definition_draft','max_new_objects':9,'fields':[{'pointer':p,'element_id':None} for p in paths]}],'source_versions':[],'reusable_resources':[]}]
            drafts=[];permitted=[]
        else:
            grants=[{'scope':{'kind':'edit','data':{'kind':'definition_draft','data':{'id':d['id'],'revision':1}}},'branches':['put_definition'],
                'objects':[{'target':{'kind':'definition_draft','id':d['id']},'fields':[{'pointer':'/title','element_id':None}],'can_remove':False}],
                'creates':[],'source_versions':[],'reusable_resources':[]} for d in definitions]
            drafts=definitions;permitted=[d['id'] for d in definitions]
        profile={'project_id':'P1','operations':['get_context','describe_capabilities','apply_commands','get_object','run_scenario','get_task','get_run'],'definition_drafts':permitted,'allow_view_control':False,'edit':grants}
        write(stage/'input.json',value);write(stage/'drafts.json',definitions);write(stage/'source.json',{'sources':sources,'only_transform':'new private IDs; category payload/version/contribution unchanged; not provider or visual acceptance'})
        write(stage/'case.json',{'root':str(ROOT),'seed':str(seed),'python':sys.executable,'bridge':str(ROOT/'.work/target/debug/superfactory-mcp-bridge.exe'),
            'output':str(stage/'actual-r1'),'profiles':[profile],'mode':mode,'drafts':drafts})
def case_for(packet,preview,mode):
    p=packet['project']['id'];command=copy.deepcopy(preview['seeded']['data']['command']);scenario=copy.deepcopy(command['explicit_new_scenario_seed'])
    order=scenario['orders'][0];target=1 if mode=='workflow-success' else 2 if mode in ['workflow-repair','workflow-budget'] else 3
    if mode in ['workflow-repair','workflow-budget']:
        scenario['window']['end_ms']=80000
        scenario['production_tasks'][0]['release_at_ms']=30000 if mode=='workflow-repair' else 70000
    calendar=next(o['data']['id'] for o in command['proposed_objects'] if o['kind']=='calendar')
    goal={'industry':'paper_carton_manufacturing','product_family':'die_cut_fold_glued_paper_carton','product_specification_ref':order['product_ref'],
        'accepted_quantity_per_day':{'value':{'coefficient':target,'decimal_places':0},'unit':'piece'},
        'acceptance_scope':{'product_ref':order['product_ref'],'unit':'piece','destination':order['destination'],'stage':order['acceptance_stage']},
        'production_calendar_ref':calendar,'observation_window':scenario['window'],'site_envelope':preview['choices']['site_envelope'],
        'existing_equipment':[{'kind':'device','id':o['data']['id']} for o in command['proposed_objects'] if o['kind']=='device'],'constraints':[]}
    if mode=='workflow-missing':goal['production_calendar_ref']=None
    scenario['orders']=[]
    for task in scenario['production_tasks']:task['order_allocations']=[]
    scenario['acceptance_goal']=goal
    proposed=command['proposed_objects']+[{'kind':'scenario','data':scenario}];kinds={}
    for o in proposed:kinds.setdefault(o['kind'],[]).append(o['data'])
    creates=[{'kind':kind,'max_new_objects':len(objects),'fields':[{'pointer':path,'element_id':None} for path in sorted({f['pointer'] for o in objects for f in fields(o)})]} for kind,objects in kinds.items()]
    settings_fields=[{'pointer':'/title','element_id':None}]
    objects=[{'target':{'kind':'project','id':p},'fields':settings_fields,'can_remove':False},
        {'target':{'kind':'scenario','id':scenario['id']},'fields':[{'pointer':'/revision','element_id':None},{'pointer':'/project_revision','element_id':None},
        {'pointer':'/production_tasks/0/release_at_ms','element_id':scenario['production_tasks'][0]['id']}],'can_remove':False}]
    grant={'scope':{'kind':'edit','data':{'kind':'project','data':p}},'branches':['instantiate_template','put_object','set_project_settings'],
        'objects':objects,'creates':creates,'source_versions':[packet['template_ref']]+packet['source_project']['module_refs'],'reusable_resources':[]}
    operations=['describe_capabilities','get_context','get_object','apply_commands','query_module_library','check_factory_draft','render_view',
        'run_scenario','get_task','get_run','compare_runs','save_project','export_evaluation','cancel_task','import_package','export_package']
    profile={'project_id':p,'operations':operations,'definition_drafts':[],'allow_view_control':False,'edit':[grant]}
    stage=OUT/'cases'/mode
    expected=copy.deepcopy(packet['expected'])
    expected['initial_acceptance_times']=[75000] if mode=='workflow-repair' else [] if mode=='workflow-budget' else [45000,65000]
    expected['acceptance_counts']=[1,2] if mode=='workflow-repair' else [0,0,1,1] if mode=='workflow-budget' else [2]
    data={'project':packet['project'],'scenario':scenario,'goal':goal,'preview':preview,'template_ref':packet['template_ref'],
        'expected':expected,'input_basis':'existing native carton example; fixed two-piece supply; explicit manual acceptance without order quotas; no target-derived stock or cycle'}
    write(stage/'input.json',data)
    write(stage/'case.json',{'root':str(ROOT),'seed':str(Path(packet['_directory'])/'empty-project'),'python':sys.executable,
        'bridge':str(ROOT/'.work/target/debug/superfactory-mcp-bridge.exe'),'output':str(stage/'actual-r1'),'profiles':[profile],'mode':mode,'drafts':[]})
if __name__=='__main__':
    directory=Path(read(OUT/'fixtures/latest.json')['directory']);packet=read(directory/'template-input.json');packet['_directory']=str(directory)
    for preview,mode in zip(packet['previews'],['workflow-success','workflow-shortfall','workflow-repair','workflow-conflict']):case_for(packet,preview,mode)
    case_for(packet,packet['previews'][0],'workflow-budget');case_for(packet,packet['previews'][0],'workflow-missing')
    print('prepared6 independent public-Host/MCP workflow inputs and fixed exact-leaf authority')
