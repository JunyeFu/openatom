"""Author canonical P4-01 candidates; never modifies original design inputs.

The existing frozen design adapter is reused only at authoring time. The Rust
module-kit consumes the resulting current domain DTOs without Python at runtime.
"""
from pathlib import Path
from copy import deepcopy as cp
import importlib.util
import json
import sys

ROOT = Path(__file__).resolve().parents[2]
BASE = Path(__file__).resolve().parent
sys.dont_write_bytecode = True
ADAPTER = ROOT / 'scenarios/九类模块采用'
sys.path.insert(0, str(ADAPTER))
spec = importlib.util.spec_from_file_location('frozen_template_adapter', ADAPTER/'DRF03_C04_内层模板适配.py')
adapter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(adapter)


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')


def qty(n, unit='piece'):
    return {'value': {'coefficient': n, 'decimal_places': 0}, 'unit': unit}


def ref(ident):
    return {'id': ident, 'revision': 'r1'}


def mref(definition):
    return {k: definition[k] for k in ['id', 'version']}


def definition(ident, title, category, payload):
    return {'id': ident, 'version': '0.1.0', 'title': title, 'definition_state': 'design',
            'contribution': {'author': None, 'license': None, 'assets': []},
            'parameters': [], 'attribute_schema': [], 'payload': {'category': category, 'data': payload}}


def package(d):
    return {'schema_version': '1.0', 'data': {'definition': d, 'assets': [], 'files': [],
                                            'dependencies': [], 'rights': [], 'issues': []}}


def issue(code, message, path, blocks=None):
    return {'code': code, 'message': message, 'field_path': path, 'object_ref': None,
            'blocks': blocks if blocks is not None else ['complete_conclusion']}


def build(name, supplied_source=None):
    src_path = BASE/name/'project.design.json'
    source = read(src_path) if supplied_source is None else supplied_source
    template, material, generated, archives = adapter.convert(source, src_path)
    t = template['data']
    # Runtime derivatives have NEW exact identities; never change the content
    # behind the older local.*@0.1.0 declaration-only fixtures.
    identity_map = {v['data']['definition']['id']: 'p401.'+name+'.'+v['data']['definition']['id'].split('.device.')[-1] if '.device.' in v['data']['definition']['id'] else 'p401.'+name+'.preparation.'+v['data']['definition']['id'].split('.preparation.')[-1] for v in generated.values()}
    def rewrite_module_refs(x):
        if isinstance(x, dict):
            if x.get('id') in identity_map and 'version' in x:
                x['id'] = identity_map[x['id']]; x['version'] = '1.0.0'
            for v in x.values(): rewrite_module_refs(v)
        elif isinstance(x,list):
            for v in x: rewrite_module_refs(v)
    for x in [t, material, generated]: rewrite_module_refs(x)
    for value in generated.values():
        d = value['data']['definition']
        # The archive and definition describe the very same source assets.
        # Copy the existing rights verbatim, including unknown license/False;
        # this does not grant redistribution or replace the source declaration.
        d['contribution']['assets'] = cp(value['data']['rights'])
        if d['payload']['category'] == 'devices':
            op = d['payload']['data']['behavior']['inline']
            for row in op['inputs']+op['outputs']:
                row.setdefault('declared_output_quality', None)
            if op['builtin'] != 'inspect_and_route':
                for row in op['inputs']:
                    if not row['required_quality']:
                        row['required_quality']=['unchecked','released']
            if op['builtin'] == 'inspect_and_route':
                # max_reworks=0: rework/quarantine were two source aliases for
                # the same isolation store. Compile one physical fail branch.
                op['outputs'] = [r for r in op['outputs'] if r['port'] != 'quarantine']
                op['quality_and_routes']['result_ref'] = None
                # Current Engine instance overrides consume durations/phases.
                # The finite rework limit remains an editable definition field;
                # do not advertise an unimplemented instance scalar consumer.
                d['payload']['data']['configuration'].pop('max_reworks',None)
                d['parameters']=[p for p in d['parameters'] if p['id']!='max_reworks']
                d['payload']['data']['ports'] = [r for r in d['payload']['data']['ports'] if r['id'] != 'quarantine']
                if d['payload']['data']['presentation'] is not None: d['payload']['data']['presentation']['port_anchors'].pop('quarantine',None)
                for dev in t['devices']:
                    if dev['module_ref']['id'] == d['id']:
                        dev['role_bindings']['item_types'].pop('quarantine',None)
                        dev['role_bindings']['targets'].pop('material_store.quarantine',None)
                        dev['configuration'].pop('max_reworks',None)
            if op['builtin'] == 'pack':
                op['pack_identity'] = {'output_role': 'packed', 'container_material_roles': ['carton'], 'content_roles': ['product']}
                op['outputs'][0]['declared_output_quality'] = 'released'
    t['connections'] = [c for c in t['connections'] if not (c['data']['source']['kind']=='port' and c['data']['source']['data']['port_id']=='quarantine')]
    # The source explicitly declares this output released at completion, not input.
    if name == 'parallel-electromechanical':
        generated['definitions/BoardFinish01/module.json']['data']['definition']['payload']['data']['behavior']['inline']['outputs'][0]['declared_output_quality'] = 'released'
    prefix = 'p401.'+name
    template_ref = {'id': 'community.template-'+name, 'version': '1.0.0'}
    defs = [x['data']['definition'] for x in generated.values()]
    by_device = {x['id']: x for x in t['devices']}
    missing = []
    for i, h in enumerate(t['halls']):
        if h['height_m'] is None:
            missing.append({'kind': 'site_geometry', 'object_ref': {'kind': 'hall', 'id': h['id']}, 'source_pointer': f'/halls/{i}', 'required_fields': ['height_m']})
    for i, d in enumerate(t['devices']):
        missing.append({'kind': 'site_geometry', 'object_ref': {'kind': 'device', 'id': d['id']}, 'source_pointer': f'/devices/{i}', 'required_fields': ['footprint_m', 'height_m', 'clearance_m']})
    for i, item in enumerate(t['item_types']):
        missing.append({'kind': 'specification', 'object_ref': {'kind': 'item_type', 'id': item['id']}, 'source_pointer': f'/item_types/{i}/spec_ref', 'required_fields': ['spec_ref']})
    claims = []
    for i, raw in enumerate(source['professional_artifacts']):
        required = raw.get('required_fields', ['board_revision','coordinate_frame','component_library_ref','placements','fiducials_mm','feeders','nozzles','machine_profile','native_job_ref','phase_plan'])
        absent = [k for k in required if raw.get(k) is None]
        claims.append({'id': raw['id'], 'artifact_type': raw['artifact_type'], 'mode': 'declared_process',
                       'consumer_device_ids': [d['id'] for d in source['devices'] if d.get('artifact_ref') == raw['id']],
                       'source_pointer': f'/professional_artifacts/{i}', 'declared_scope': [raw['accepted_scope']], 'required_fields': required, 'missing_fields': absent})
        missing.append({'kind': 'professional_input', 'object_ref': {'kind': 'device', 'id': claims[-1]['consumer_device_ids'][0]}, 'source_pointer': f'/professional_artifacts/{i}', 'required_fields': absent+['producer_result']})
    prep = [{'task_ref': p['module_ref'], 'target_device_id': p['target_device_id'], 'before_device_ids': p['before_device_ids'], 'worker_roles': p['resources'], 'route_refs': []} for p in material['preparation_bindings']]
    for p in prep:
        by_device[p['target_device_id']]['role_bindings']['targets']['target'] = {'kind': 'device', 'id': p['target_device_id']}
    adoption = {'id': prefix+'.adoption', 'template_ref': template_ref,
                'source_archive': {'path': 'source-project.design.json', 'kind': 'asset'},
                'resource_choices': [{'source_id': r['id'], 'requirement_index': i, 'resource_id': None} for i,r in enumerate(source['resources'])],
                'preparation_bindings': prep, 'imported_claims': claims,
                'inspection_presets': [{'id': material['quality_declarations']['id'], 'consumer_device_id': 'Inspection01',
                    'inspection_module': by_device['Inspection01']['module_ref'], 'item_or_ancestor': None, 'match_kind': None,
                    'results': [{'kind': 'string', 'value': x} for x in source['quality_inputs']]}],
                'deferred_seed': {'time_reference': {'label': 'scenario_start', 'origin_iso8601': None, 'timezone': None},
                    'window': {'start_ms': 0, 'end_ms': source['time']['observation_end']},
                    'initial_stock': [{'location_id': x['store'], 'item_type_id': x['item_type'], 'quantity': qty(x['quantity'], x['unit']), 'quality': None} for x in source['initial_stock']],
                    'demand': {'item_type_id': source['demand']['item_type'], 'quantity': qty(source['demand']['quantity']), 'destination_location_id': source['demand']['destination'], 'acceptance_stage': 'factory_released'},
                    'product_ref': None, 'bom_ref': None, 'routing_ref': None}, 'missing_inputs': missing}
    t['adoption'] = adoption
    t['seed'] = None
    # This is a new, explicitly selectable example product, not recovered facts.
    blueprint = read(ROOT/'scenarios/九类模块采用/adoption-output/project-draft.json')['data']
    for k in ['halls','locations','lines','devices','connections','item_types']:
        blueprint[k] = cp(t[k])
    blueprint.update(id=prefix+'.blueprint', revision=1, title=source['title']+' · 显式示例产品', resources=[], calendars=[], products=[], boms=[], routes=[], scenario_refs=[], artifact_refs=[], acknowledgments=[], bindings=[], template_adoptions=[], setup_policies=[], generation_context=None, parameter_sources=[], sources=[], issues=[])
    blueprint['views']['selected_hall'] = t['halls'][0]['id']
    resources = []
    source_resources = [(r['id'], cp(req)) for r,req in zip(source['resources'],t['resource_requirements'])]
    # One explicit execution fixture per machine, not an additional worker.
    for dev in t['devices']:
        rid = 'capacity-'+dev['id']
        req = {'kind': 'fixture', 'role': 'execution_capacity', 'capacity': qty(1,'slot'), 'qualifications': [], 'default_calendar_ms': [{'start_ms':0,'end_ms':source['time']['observation_end']}], 'presentation': None}
        i = len(t['resource_requirements']);t['resource_requirements'].append(cp(req))
        adoption['resource_choices'].append({'source_id':rid,'requirement_index':i,'resource_id':None})
        dev['role_bindings']['resources']['execution_capacity'] = rid
        source_resources.append((rid,req))
    blueprint['devices'] = cp(t['devices'])
    for rid,req in source_resources:
        rd=definition(prefix+'.resource.'+rid,rid,'resources',req);defs.append(rd)
        calendar_id='calendar-'+rid
        blueprint['calendars'].append({'id':calendar_id,'title':calendar_id,'intervals':cp(req['default_calendar_ms']),'source':'template_declared_example'})
        resources.append({'id':rid,'definition_ref':mref(rd),'title':rid,'kind':req['kind'],'role':req['role'],'capacity':cp(req['capacity']),'qualifications':cp(req['qualifications']),'calendar_id':calendar_id,'home_location':None,'presentation':None})
    blueprint['resources']=resources
    device_defs={d['id']:d for d in defs if d['payload']['category']=='devices'}
    steps=[];boms=[];producer={}
    for dev in t['devices']:
        op=device_defs[dev['module_ref']['id']]['payload']['data']['behavior']['inline']
        predecessors=[]
        for p in prep:
            if dev['id'] in p['before_device_ids']:
                stepid='step-prepare-'+dev['id']
                steps.append({'id':stepid,'operation_ref':cp(p['task_ref']),'device_id':dev['id'],'predecessors':[],'item_bindings':{},'resource_bindings':cp(p['worker_roles'])})
                predecessors.append(stepid)
        for row in op['inputs']:
            item=dev['role_bindings']['item_types'][row['role']]
            if item in producer:predecessors.append(producer[item])
            boms.append({'item_type_ref':ref(item),'quantity':cp(row['quantity']),'consume_at_operation':'step-'+dev['id']})
        step={'id':'step-'+dev['id'],'operation_ref':cp(dev['module_ref']),'device_id':dev['id'],'predecessors':list(dict.fromkeys(predecessors)),
              'item_bindings':cp(dev['role_bindings']['item_types']),'resource_bindings':cp(dev['role_bindings']['resources'])}
        steps.append(step)
        for row in op['outputs']:
            if op['builtin']!='inspect_and_route':producer[dev['role_bindings']['item_types'][row['role']]]=step['id']
        # Passing inspection must be a real predecessor of packing, not a type rename.
        if op['builtin']=='pack':
            step['predecessors']=list(dict.fromkeys(step['predecessors']+['step-Inspection01']))
    blueprint['products']=[{'id':'example-product','revision':'r1','item_type_ref':ref(source['demand']['item_type']),'bom_ref':ref('example-bom'),'routing_ref':ref('example-route'),'artifact_refs':[]}]
    blueprint['boms']=[{'id':'example-bom','revision':'r1','rows':boms}]
    blueprint['routes']=[{'id':'example-route','revision':'r1','steps':steps}]
    for p in prep:p['route_refs']=[ref('example-route')]
    blueprint['sources']=[{'id':'example-source','title':'Original template design plus separately selected synthetic example Product/BOM/Route',
        'locator':str(src_path.relative_to(ROOT)).replace('\\','/'),'read_scope':'all declared source rows; no field measurement or professional result', 'read_date':None,'basis':'template_declared_example'}]
    for collection,kind,fields in [('halls','hall',['width_m','depth_m','height_m']),('devices','device',['pose','footprint_m','height_m']),('products','product',['item_type_ref','bom_ref','routing_ref'])]:
        for row in blueprint[collection]:
            for field in fields:
                blueprint['parameter_sources'].append({'target':{'kind':kind,'id':row['id']},'field_path':'/'+field,'value':None,
                    'basis':'unknown' if row[field] is None or row[field]==[] else 'template_declared_example','source_ref':'example-source'})
    for dev in blueprint['devices']:
        for key,value in dev['configuration'].items():
            blueprint['parameter_sources'].append({'target':{'kind':'device','id':dev['id']},'field_path':'/configuration/'+key+'/value','value':cp(value),'basis':'template_declared_example','source_ref':'example-source'})
    blueprint['module_refs']=[mref(d) for d in defs]
    t['module_refs']=cp(blueprint['module_refs'])
    blueprint['template_adoptions']=[cp(adoption)]
    out=BASE/name/'candidate'
    for path,value in generated.items():write(out/path,value)
    for path,value in archives.items():write(out/path,value)
    write(out/'template.json',template)
    write(out/'example-project.json',{'schema_version':'1.0','data':blueprint})
    write(out/'definitions.json',defs)
    write(out/'source-project.design.json',source)
    for d in defs:
        if d['payload']['category']=='resources':write(out/'definitions'/d['id']/'module.json',package(d))
    md=definition(template_ref['id'],source['title'],'templates',{'project':'template.json','generation_profile':None});md['version']=template_ref['version']
    outer=package(md);outer['data']['dependencies']=cp(t['module_refs'])
    outer['data']['files']=[{'path':str(p.relative_to(out)).replace('\\','/'),'kind':'template' if p.name=='template.json' else 'generation_profile' if p.name=='generation-profile.json' else 'asset'} for p in sorted(out.rglob('*')) if p.is_file() and p!=out/'module.json']
    outer['data']['issues']=[issue('template_example_not_measured','Declared process/example only; geometry and professional producer inputs remain missing.','/definition/payload'),issue('license_unconfirmed','Original source author and redistribution license not confirmed.','/definition/contribution',['redistribution'])]
    write(out/'module.json',outer)
    return blueprint,t,defs


def carton_candidate():
    source=read(BASE/'machining-delivery/project.design.json')
    source.update(id='template-carton-factory',title='采购纸板→模切→折糊→检验：纸箱制造声明例',professional_artifacts=[],preparation_tasks=[])
    source['assumptions'].append('New self-designed runtime example: 20s die-cut + 20s fold-glue + 5s inspection. No measured capacity, packaging geometry or production license is asserted.')
    source['item_types']=[{'id':x,'unit':'piece','spec_ref':'self-'+x+'-r1'} for x in ['board','carton-blank','carton']]
    source['resources']=[{'id':x,'kind':'worker','capacity':1,'calendar_ms':[[0,300000]]} for x in ['W-B','W-C','W-A']]
    source['stores']=[{'id':loc,'hall':'H1','item_type':item,'capacity':4,'unit':'piece','layout_m':[2+3*i,2]} for i,(loc,item) in enumerate([('store-board','board'),('store-blank','carton-blank'),('store-carton','carton'),('store-released','carton'),('store-isolate','carton')])]
    base=cp(source['devices'][0]);base.pop('artifact_ref',None);base.pop('phases',None)
    def transform(ident, a,b,la,lb,worker,x):
        d=cp(base);d.update(id=ident,parameters={'input_item':a,'output_item':b,'cycle_seconds':20},port_bindings={'input':la,'output':lb},job_resources=[ident,worker],resource_bindings={'legacy.worker.'+worker:worker},layout_m=[x,8]);return d
    inspect=cp(source['devices'][1]);inspect['parameters'].update(product_item='carton',cycle_seconds=5);inspect['port_bindings']['input']='store-carton'
    source['devices']=[transform('DieCut01','board','carton-blank','store-board','store-blank','W-C',4),transform('FoldGlue01','carton-blank','carton','store-blank','store-carton','W-A',8),inspect]
    source['lines']=[{'id':'L-carton','hall':'H1','members':[d['id'] for d in source['devices']]}]
    source['connections']=[]
    for d in source['devices']:
        for port,store in d['port_bindings'].items():
            item=next(x['item_type'] for x in source['stores'] if x['id']==store)
            incoming=port=='input'
            source['connections'].append({'kind':'material','item_type':item,'from':store if incoming else d['id']+'.'+port,'to':d['id']+'.'+port if incoming else store})
    source['module_refs']=[r for r in source['module_refs'] if r['id']!='community.parameterized-pack']
    source['initial_stock']=[{'store':'store-board','item_type':'board','quantity':2,'unit':'piece'}]
    source['demand']={'item_type':'carton','quantity':2,'unit':'piece','destination':'store-released','acceptance_scope':'released_cartons_at_factory_destination'}
    source['entry_exit']={'input_types':['board'],'output_type':'carton','track_start':'purchased_board_arrival','track_end':'released_carton_store'}
    b,t,defs=build('carton-factory',source)
    p=BASE/'carton-factory/candidate'
    profile={'industry':'paper_carton_manufacturing','product_family':'die_cut_fold_glued_paper_carton',
        'site_applicability':{'regions':[],'length_m':None,'width_m':None,'height_m':None,'usable_polygon_m':[],'clearance_m':None},
        'process_graph':cp(b['routes'][0]['steps']),'nine_category_roles':[{'role':category+'.'+role,'quantity':qty(1,'definition')} for category,roles in read(BASE/'carton-factory/generation-profile.design.json')['nine_category_roles'].items() for role in roles],
        'required_parameters':[{'id':key,'label':key,'value_type':'number','binding':binding,'unit':unit,'minimum':0,'maximum':None,'integer':False,'options':[],'basis':'unknown'} for key,binding,unit in [('site_length_m','/site_envelope/length_m','m'),('site_width_m','/site_envelope/width_m','m'),('site_height_m','/site_envelope/height_m','m'),('daily_accepted_quantity','/accepted_quantity_per_day/value','piece')]],
        'parameter_sources':cp(b['parameter_sources']),'layout_modes':['straight_line','u_shape','parallel_lines'],'role_bindings':adapter.empty_bindings()}
    write(p/'generation-profile.json',{'schema_version':'1.0','data':profile})
    outer=read(p/'module.json');outer['data']['definition']['payload']['data']['generation_profile']='generation-profile.json';outer['data']['files']=[x for x in outer['data']['files'] if x['path']!='generation-profile.json'];outer['data']['files'].append({'path':'generation-profile.json','kind':'generation_profile'});write(p/'module.json',outer)
    write(p/'configuration-notes.json',{'basis':'self-designed example, not measured production','active_route':['die_cut','fold_glue','inspection'],'output':'released carton pieces, never filled package or bundle count',
        'optional_not_enabled':['printing requires selected device/time/material routing','bundling requires band material, content relation, destination/counting choice'],
        'no_goal_to_result_binding':True,'geometry_missing':['actual device footprints','height','clearances'],'professional_result':None})
    return b,t,defs


def support_catalog():
    out=ROOT/'modules/devices/template-support'
    packages=[]
    for category in ['items','devices','resources','tasks','operations','transport','templates','professional','media']:
        src=ROOT/'scenarios/九类模块采用/adapter-output'/category
        value=read(src/'module.json')
        if category=='templates':
            value=read(BASE/'carton-factory/candidate/module.json')
            # This entry points to the actual full candidate rather than copying
            # an old uncompiled TemplateSubset masquerading as an adopted asset.
            directory='../../../templates/carton-factory/candidate'
        else:
            value['data']['definition']['id']='p401.support.'+category
            value['data']['definition']['version']='1.0.0'
            directory=category
            for f in src.rglob('*'):
                if f.is_file() and f.name!='module.json':
                    p=out/category/f.relative_to(src);p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(f.read_bytes())
            write(out/category/'module.json',value)
        packages.append(value['data'])
    write(out/'catalog.json',packages)
    write(out/'README.json',{'kind':'current_domain_creatable_examples_not_provider_or_full_adoption_proof','categories':[p['definition']['payload']['category'] for p in packages],
        'template_source':'modules/templates/carton-factory/candidate','rules':['No arbitrary 12-object threshold','Professional definition is an input contract, not a registered available solver','Media has production_effect none','Unknown rights remain unknown; not redistribution approval']})


if __name__=='__main__':
    for name in ['machining-delivery','purchased-assembly','parallel-electromechanical']:
        b,t,d=build(name)
        print(name, 'canonical devices',len(t['devices']),'definitions',len(d))
    b,t,d=carton_candidate()
    print('carton-factory canonical devices',len(t['devices']),'definitions',len(d))
    support_catalog()
