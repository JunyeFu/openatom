"""构造实际源的强类型采用候选；公共未合入时明确只用选定Schema核验。"""
from copy import deepcopy
import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True
from DRF03_C04_内层模板适配 import ROOT, BASE, OUT as PRIOR, read, write
from DRF03_C04_内层模板适配 import remap_layout
from jsonschema import Draft202012Validator

OUT = BASE/'adoption-output'
DOCS = ROOT/'docs/04_开发与验收/特征细化'


def selected_defs(current):
    d = deepcopy(current)
    def ref(name):return {'$ref':'#/$defs/'+name}
    def arr(name):return {'type':'array','items':ref(name)}
    def opt(name):return {'anyOf':[ref(name),{'type':'null'}]}
    strings = {'type':'array','items':{'type':'string'}}
    def obj(name,properties,optional=()):
        d[name] = {'type':'object','properties':properties,'required':[k for k in properties if k not in optional],'additionalProperties':False}
    d['MaterialRoleQuantity']['properties']['declared_output_quality'] = opt('QualityState')
    d['TemplateSubset']['properties']['adoption'] = opt('TemplateAdoption')
    d['ProjectState']['properties']['template_adoptions'] = arr('TemplateAdoption')
    if 'template_adoptions' not in d['ProjectState']['required']:
        d['ProjectState']['required'].append('template_adoptions')
    obj('TemplateAdoption',{'id':ref('ObjectId'),'template_ref':ref('ModuleRef'),'source_archive':ref('FileRef'),
        'resource_choices':arr('TemplateResourceChoice'),'preparation_bindings':arr('TemplatePreparationBinding'),
        'imported_claims':arr('ImportedProfessionalClaim'),'inspection_presets':arr('TemplateInspectionPreset'),
        'deferred_seed':opt('DeferredTemplateSeed'),'missing_inputs':arr('TemplateMissingInput')},('deferred_seed',))
    obj('TemplateResourceChoice',{'source_id':ref('ObjectId'),'requirement_index':ref('SafeUint'),'resource_id':opt('ObjectId')},('resource_id',))
    obj('TemplatePreparationBinding',{'task_ref':ref('ModuleRef'),'target_device_id':ref('ObjectId'),
        'before_device_ids':arr('ObjectId'),'worker_roles':{'type':'object','additionalProperties':ref('ObjectId')},'route_refs':arr('SubjectRevisionRef')})
    obj('ImportedProfessionalClaim',{'id':ref('ObjectId'),'artifact_type':{'type':'string'},'mode':ref('ProfessionalMode'),
        'consumer_device_ids':arr('ObjectId'),'source_pointer':{'type':'string'},'declared_scope':strings,'required_fields':strings,'missing_fields':strings})
    obj('TemplateInspectionPreset',{'id':ref('ObjectId'),'consumer_device_id':ref('ObjectId'),'inspection_module':ref('ModuleRef'),
        'item_or_ancestor':opt('ObjectId'),'match_kind':opt('QualityMatchKind'),'results':{'type':'array','items':opt('TypedValue')}},('item_or_ancestor','match_kind'))
    obj('DeferredTemplateSeed',{'time_reference':ref('TimeReference'),'window':ref('WindowMs'),'initial_stock':arr('TemplateStockSeed'),
        'demand':ref('TemplateDemandSeed'),'product_ref':opt('SubjectRevisionRef'),'bom_ref':opt('SubjectRevisionRef'),'routing_ref':opt('SubjectRevisionRef')},('product_ref','bom_ref','routing_ref'))
    obj('TemplateStockSeed',{'location_id':ref('ObjectId'),'item_type_id':ref('ObjectId'),'quantity':ref('Quantity'),'quality':opt('QualityState')},('quality',))
    obj('TemplateDemandSeed',{'item_type_id':ref('ObjectId'),'quantity':ref('Quantity'),'destination_location_id':ref('ObjectId'),'acceptance_stage':ref('AcceptanceStage')})
    obj('TemplateMissingInput',{'kind':ref('TemplateMissingInputKind'),'object_ref':opt('ObjectRef'),'source_pointer':{'type':'string'},'required_fields':strings},('object_ref',))
    d['TemplateMissingInputKind'] = {'type':'string','enum':['site_geometry','specification','professional_input','product_bom_route','initial_quality']}
    return d


def validator(name,defs):
    return Draft202012Validator({'$defs':defs,'$ref':'#/$defs/'+name})


def has_public_delta(defs):
    return ('TemplateAdoption' in defs and 'adoption' in defs['TemplateSubset']['properties']
            and 'template_adoptions' in defs['ProjectState']['properties']
            and 'declared_output_quality' in defs['MaterialRoleQuantity']['properties'])


def build():
    source = read(PRIOR/'source-project.design.json')
    template,outer = read(PRIOR/'template.json'),read(PRIOR/'module.json')
    companion = read(PRIOR/'adoption-material.json')
    modules = {str(p.relative_to(PRIOR)).replace('\\','/'):read(p) for p in (PRIOR/'definitions').rglob('module.json')}
    prefix = 'local.'+source['id']
    template_ref = {'id':outer['data']['definition']['id'],'version':'0.3.0'}
    changed = modules['definitions/BoardFinish01/module.json']['data']['definition']
    old_ref = {'id':changed['id'],'version':changed['version']}
    changed['version'] = '0.2.0'
    new_ref = {'id':changed['id'],'version':changed['version']}
    operation = changed['payload']['data']['behavior']['inline']
    assert operation['builtin'] == 'transform' and len(operation['outputs']) == 1
    assert operation['outputs'][0]['port'] == 'output'
    operation['outputs'][0]['declared_output_quality'] = 'released'
    for row in template['data']['devices']:
        if row['module_ref'] == old_ref:
            row['module_ref'] = deepcopy(new_ref)
        if row['id'] == 'Board01':
            row['role_bindings']['targets']['target'] = {'kind':'device','id':'Board01'}
    for ref in template['data']['module_refs']:
        if ref == old_ref:ref.update(new_ref)
    for edge in template['data']['connections']:
        for key in ['source','destination']:
            endpoint = edge['data'][key]
            if endpoint['kind'] == 'port' and endpoint['data']['module_ref'] == old_ref:
                endpoint['data']['module_ref'] = deepcopy(new_ref)
    # Quantity construction follows the source's integral piece counts, no RuntimeItems created.
    def quantity(value,unit):return {'value':{'coefficient':value,'decimal_places':0},'unit':unit}
    claims = []
    for i,raw in enumerate(source['professional_artifacts']):
        required = raw.get('required_fields',['board_revision','coordinate_frame','component_library_ref','placements','fiducials_mm','feeders','nozzles','machine_profile','native_job_ref','phase_plan'])
        missing = [key for key in required if raw.get(key) is None]
        claims.append({'id':raw['id'],'artifact_type':raw['artifact_type'],'mode':'declared_process',
            'consumer_device_ids':[d['id'] for d in source['devices'] if d.get('artifact_ref') == raw['id']],
            'source_pointer':f'/professional_artifacts/{i}','declared_scope':[raw['accepted_scope']],
            'required_fields':required,'missing_fields':missing})
    source_devices = {d['id']:d for d in template['data']['devices']}
    prep, = companion['preparation_bindings']
    missing_inputs = [{'kind':'site_geometry','object_ref':{'kind':'hall','id':source['halls'][0]['id']},'source_pointer':'/halls/0','required_fields':['height_m']}]
    for i,d in enumerate(source['devices']):
        missing_inputs.append({'kind':'site_geometry','object_ref':{'kind':'device','id':d['id']},'source_pointer':f'/devices/{i}','required_fields':['footprint_m','height_m','clearance_m']})
    for i,t in enumerate(source['item_types']):
        missing_inputs.append({'kind':'specification','object_ref':{'kind':'item_type','id':t['id']},'source_pointer':f'/item_types/{i}/spec_ref','required_fields':['spec_ref']})
    for i,c in enumerate(claims):
        missing_inputs.append({'kind':'professional_input','object_ref':{'kind':'device','id':c['consumer_device_ids'][0]},'source_pointer':c['source_pointer'],'required_fields':c['missing_fields']+['producer_result']})
    missing_inputs.append({'kind':'product_bom_route','object_ref':None,'source_pointer':'/demand','required_fields':['product_ref','bom_ref','routing_ref']})
    for i,stock in enumerate(source['initial_stock']):
        missing_inputs.append({'kind':'initial_quality','object_ref':{'kind':'location','id':stock['store']},'source_pointer':f'/initial_stock/{i}','required_fields':['quality']})
    adoption = {'id':prefix+'.adoption','template_ref':template_ref,'source_archive':{'path':'source-project.design.json','kind':'asset'},
        'resource_choices':[{'source_id':raw['id'],'requirement_index':i,'resource_id':None} for i,raw in enumerate(source['resources'])],
        'preparation_bindings':[{'task_ref':prep['module_ref'],'target_device_id':prep['target_device_id'],'before_device_ids':prep['before_device_ids'],'worker_roles':prep['resources'],'route_refs':[]}],
        'imported_claims':claims,
        'inspection_presets':[{'id':companion['quality_declarations']['id'],'consumer_device_id':'Inspection01','inspection_module':source_devices['Inspection01']['module_ref'],
            'item_or_ancestor':None,'match_kind':None,'results':[{'kind':'string','value':value} for value in source['quality_inputs']]}],
        'deferred_seed':{'time_reference':{'label':source['time']['origin'],'origin_iso8601':None,'timezone':None},'window':{'start_ms':0,'end_ms':source['time']['observation_end']},
            'initial_stock':[{'location_id':stock['store'],'item_type_id':stock['item_type'],'quantity':quantity(stock['quantity'],stock['unit']),'quality':None} for stock in source['initial_stock']],
            'demand':{'item_type_id':source['demand']['item_type'],'quantity':quantity(source['demand']['quantity'],source['demand']['unit']),
                      'destination_location_id':source['demand']['destination'],'acceptance_stage':'factory_released'},'product_ref':None,'bom_ref':None,'routing_ref':None},
        'missing_inputs':missing_inputs}
    template['data']['adoption'] = adoption
    outer['data']['definition']['version'] = template_ref['version']
    outer['data']['dependencies'] = deepcopy(template['data']['module_refs'])
    outer['data']['issues'] = [x for x in outer['data']['issues'] if x['code'] != 'companion_consumer_requires_public_integration']
    outer['data']['issues'].append({'code':'typed_adoption_public_merge_pending','message':'Selected typed adoption delta; canonical public merge and consumers must be independently checked before this is admitted.',
        'field_path':'/definition/payload/data/project','object_ref':None,'blocks':['execution','complete_conclusion']})
    # The legacy JSON companion is an archive only. Runtime instructions are all typed now.
    return source,template,outer,modules,old_ref,new_ref


def validate_semantics(template,modules,source):
    data = template['data'];a=data['adoption'];devices={d['id']:d for d in data['devices']}
    definitions={(v['data']['definition']['id'],v['data']['definition']['version']):v['data']['definition'] for v in modules.values()}
    assert a['template_ref']['version'] == '0.3.0'
    assert a['source_archive']['kind'] == 'asset'
    assert [r['source_id'] for r in a['resource_choices']] == [r['id'] for r in source['resources']]
    assert [r['requirement_index'] for r in a['resource_choices']] == list(range(4))
    for r in a['resource_choices']:
        req=data['resource_requirements'][r['requirement_index']]
        assert req['role'] == 'legacy.worker.'+r['source_id']
    for prep in a['preparation_bindings']:
        assert prep['target_device_id'] in devices and all(i in devices for i in prep['before_device_ids'])
        task=definitions[(prep['task_ref']['id'],prep['task_ref']['version'])]
        assert task['payload']['category']=='tasks' and not task['payload']['data']['completion_effect']['produces_material']
        assert set(prep['worker_roles'].values()) <= {r['source_id'] for r in a['resource_choices']}
        assert devices[prep['target_device_id']]['role_bindings']['targets']['target'] == {'kind':'device','id':prep['target_device_id']}
    for claim in a['imported_claims']:
        parts=claim['source_pointer'].split('/')
        assert parts[1] == 'professional_artifacts'
        raw=source['professional_artifacts'][int(parts[2])]
        assert raw['id']==claim['id'] and raw['artifact_type']==claim['artifact_type']
        assert all(x in devices for x in claim['consumer_device_ids'])
    for preset in a['inspection_presets']:
        assert preset['consumer_device_id'] in devices
        assert devices[preset['consumer_device_id']]['module_ref'] == preset['inspection_module']
        assert definitions[(preset['inspection_module']['id'],preset['inspection_module']['version'])]['payload']['data']['behavior']['inline']['quality_and_routes']['result_ref'] == preset['id']
    for definition in definitions.values():
        if definition['payload']['category'] != 'devices':continue
        op=definition['payload']['data']['behavior']['inline']
        assert all(row.get('declared_output_quality') is None for row in op['inputs'])
        for out in op['outputs']:
            if out.get('declared_output_quality') is not None:
                assert out['declared_output_quality'] == 'released'
                assert op['builtin'] in ['transform','assemble','pack']
                port=next(p for p in definition['payload']['data']['ports'] if p['id']==out['port'])
                assert port['direction']=='output' and port['payload']['kind']=='material'
    return True


def remap_typed_adoption(template,companion,namespace,item_type_map):
    t,c=remap_layout(template,companion,namespace)
    data=t['data'];a=data['adoption'];mapping=c['source_to_instance_ids']
    a['id']=namespace+'.adoption'
    a['source_archive']['path']='sources/'+namespace+'/source-project.design.json'
    for device in data['devices']:
        for target in device['role_bindings']['targets'].values():
            if target['id'] in mapping:target['id']=mapping[target['id']]
        for role,ident in device['role_bindings']['item_types'].items():
            device['role_bindings']['item_types'][role]=item_type_map[ident]
    for row in data['item_types']:row['id']=item_type_map[row['id']]
    for loc in data['locations']:loc['accepted_item_types']=[item_type_map[x] for x in loc['accepted_item_types']]
    for edge in data['connections']:edge['data']['item_type']=item_type_map[edge['data']['item_type']]
    for prep in a['preparation_bindings']:
        prep['target_device_id']=mapping[prep['target_device_id']]
        prep['before_device_ids']=[mapping[x] for x in prep['before_device_ids']]
    for claim in a['imported_claims']:claim['consumer_device_ids']=[mapping[x] for x in claim['consumer_device_ids']]
    for preset in a['inspection_presets']:
        preset['consumer_device_id']=mapping[preset['consumer_device_id']]
        # id is the immutable template-local preset key. Project/Run key is (a.id,preset.id).
    seed=a['deferred_seed']
    for row in seed['initial_stock']:
        row['location_id']=mapping[row['location_id']];row['item_type_id']=item_type_map[row['item_type_id']]
    seed['demand']['destination_location_id']=mapping[seed['demand']['destination_location_id']]
    seed['demand']['item_type_id']=item_type_map[seed['demand']['item_type_id']]
    for missing in a['missing_inputs']:
        ref=missing['object_ref']
        if ref is not None:
            ref['id']=item_type_map[ref['id']] if ref['kind']=='item_type' else mapping[ref['id']]
    return t,c


def bound_preset(adoption,preset,known_item_ids):
    """Typed projection only: does not inspect or create actual materials."""
    if preset['item_or_ancestor'] is None or preset['match_kind'] is None:
        return None
    assert preset['item_or_ancestor'] in known_item_ids
    assert preset['match_kind'] in ['exact_item','ancestor','batch']
    return {'id':adoption['id']+'.'+preset['id'],'item_or_ancestor':preset['item_or_ancestor'],
            'match_kind':preset['match_kind'],'inspection_module':deepcopy(preset['inspection_module']),
            'results':deepcopy(preset['results'])}


def output_quality_assignments(operation,moment,slot_just_created,job_id,actual_output_slots):
    """Finite reference for the new completion effect, not an inventory/scheduler."""
    if moment!='last_effective_phase_completed' or not slot_just_created:
        return []
    effects={row['port']:row['declared_output_quality'] for row in operation['outputs'] if row.get('declared_output_quality') is not None}
    assert all(value=='released' for value in effects.values())
    assert len({slot['item_id'] for slot in actual_output_slots})==len(actual_output_slots)
    return [{'item_id':slot['item_id'],'quality':effects[slot['port']]} for slot in actual_output_slots
            if slot['job_id']==job_id and slot['port'] in effects]


def proposed_project(template):
    """Shape/draft projection: all unsupplied product/worker/runtime inputs stay empty."""
    fixture=read(ROOT/'tools/type-contract-proof/fixtures/project.json')
    project=fixture['data'];data=template['data']
    project.update(id='DRAFT_C04_TYPED',revision=0,title='C04 typed adoption draft',
        time_reference=deepcopy(data['adoption']['deferred_seed']['time_reference']),
        setup_policies=[],resources=[],calendars=[],products=[],boms=[],routes=[],scenario_refs=[],artifact_refs=[],
        acknowledgments=[],bindings=[],generation_context=None,sources=[],issues=[])
    for key in ['item_types','locations','halls','lines','devices','connections','module_refs']:
        project[key]=deepcopy(data[key])
    project['module_refs'].append(deepcopy(data['adoption']['template_ref']))
    project['template_adoptions']=[deepcopy(data['adoption'])]
    # Only the UI camera wrapper is a type fixture; it is not measured geometry or runtime.
    project['views']['selected_hall']=data['halls'][0]['id']
    return fixture


def main():
    current=read(ROOT/'tools/type-contract-proof/output/schemas/all-types.schema.json')['$defs']
    canonical=has_public_delta(current)
    chosen=selected_defs(current)
    # Both before/after are explicit; a selected schema is never labelled canonical.
    defs=current if canonical else chosen
    source,template,outer,modules,old_ref,new_ref=build()
    if canonical:
        outer['data']['issues']=[x for x in outer['data']['issues'] if x['code']!='typed_adoption_public_merge_pending']
    validator('TemplateSubset',defs).validate(template['data'])
    validator('ModulePackage',defs).validate(outer['data'])
    for package in modules.values():validator('ModulePackage',defs).validate(package['data'])
    validate_semantics(template,modules,source)
    project=proposed_project(template)
    validator('ProjectState',defs).validate(project['data'])
    OUT.mkdir(exist_ok=True)
    for old in PRIOR.rglob('*.json'):
        rel=old.relative_to(PRIOR)
        dest=OUT/rel;dest.parent.mkdir(parents=True,exist_ok=True)
        write(dest,read(old))
    write(OUT/'template.json',template);write(OUT/'module.json',outer)
    write(OUT/'project-draft.json',project)
    for name,value in modules.items():write(OUT/name,value)
    api_consumers=None
    if canonical:
        api_defs=read(ROOT/'tools/type-contract-proof/output/schemas/api-all-types.schema.json')['$defs']
        objects=[]
        for collection,kind in [('item_types','item_type'),('locations','location'),('halls','hall'),('lines','line'),('devices','device'),('connections','connection')]:
            objects.extend({'kind':kind,'data':deepcopy(row)} for row in template['data'][collection])
        instantiation={'template_ref':deepcopy(template['data']['adoption']['template_ref']),'subset':deepcopy(template['data']),
            'mapped_bindings':{'item_types':{},'resources':{},'targets':{},'ports':{}},'proposed_objects':objects,'explicit_new_scenario_seed':None}
        import_draft={'input_selection':{'selection_id':'TYPE_C04_IMPORT_ONLY','purpose':'import_package'},
            'proposed_definitions':deepcopy(project['data']['module_refs']),
            'proposed_objects':[{'kind':'project','id':project['data']['id']}],
            'missing':deepcopy(outer['data']['issues']),'conflicts':[],
            'required_human_choices':['actual worker bindings','actual route/inspection target','actual Product/BOM/Route/initial quality if choosing seed'],
            'proposed_data':[{'kind':'project','data':deepcopy(project['data'])}]}
        validator('TemplateInstantiation',api_defs).validate(instantiation)
        validator('ImportDraft',api_defs).validate(import_draft)
        write(OUT/'template-instantiation.json',instantiation);write(OUT/'import-draft.json',import_draft)
        api_consumers={'TemplateInstantiation':'current_canonical_shape_passed','ImportDraft':'current_canonical_shape_passed',
            'actual_call':False,'type_fixture_only':'Existing wrapper/identity namespace for shape proof; not an adopted Project or real selected file.'}
    write(DOCS/'DRF03_C04_r4_选定Schema.json',{'$defs':chosen,'$ref':'#/$defs/TemplateAdoption'})
    record={'document_kind':'actual_owned_typed_adoption_candidate_not_Host','status':'actual_canonical_schema_consumer_passed' if canonical else 'selected_schema_and_semantics_passed_canonical_merge_pending',
        'candidate_ref':'scenarios/九类模块采用/adoption-output/template.json','schema_kind':'actual_canonical' if canonical else 'selected_not_canonical',
        'public_merge_observed':canonical,'typed_rules':9,'locations':12,'connections':18,'definitions':7,
        'changed_definition':{'old':old_ref,'new':new_ref,'field':'outputs[0].declared_output_quality=released'},
        'project_draft_ref':'scenarios/九类模块采用/adoption-output/project-draft.json',
        'project_draft_shape_only':'Unsupplied resources/Products/BOMs/routes remain empty; only ViewState wrapper uses existing type fixture.',
        'actual_API_consumer_shapes':api_consumers,
        'old0_1_0_and_seven_prior_files_written':False,'runtime_or_ack_created':False,'actual_Host_or_Engine':None}
    write(DOCS/'DRF03_C04_r4_适配结果.json',record)
    print(json.dumps(record,ensure_ascii=False))


if __name__=='__main__':
    main()
