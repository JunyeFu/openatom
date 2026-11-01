"""有限设计消费者核验；不执行共享生成器的顶层写入或实际Host。"""
import ast
import copy
import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True
from DRF03_C04_内层模板适配 import BASE, ROOT, OUT, check_references, remap_layout, read
from jsonschema import Draft202012Validator, ValidationError


def main():
    source = read(ROOT/'modules/templates/parallel-electromechanical/project.design.json')
    template, companion, outer = [read(OUT/name) for name in ['template.json','adoption-material.json','module.json']]
    generated = {str(p.relative_to(OUT)).replace('\\','/'):read(p) for p in (OUT/'definitions').rglob('module.json')}
    schemas = ROOT/'tools/type-contract-proof/output/schemas'
    tv = Draft202012Validator(read(schemas/'template.schema.json'))
    mv = Draft202012Validator(read(schemas/'module.schema.json'))
    data = template['data']
    by_id = {d['id']:d for d in data['devices']}
    def operation(device):
        return generated['definitions/'+device+'/module.json']['data']['definition']['payload']['data']['behavior']['inline']
    checks = {}
    def register(ident):
        def add(fn):
            checks[ident] = fn
            return fn
        return add
    @register('T3-01')
    def layout():
        tv.validate(template)
        assert [len(data[k]) for k in ['locations','connections','devices','lines']] == [12,18,6,3]
        assert data['halls'][0]['height_m'] is None
        check_references(template,companion,generated)
        return 'passed'
    @register('T3-02')
    def modules():
        mv.validate(outer)
        assert outer['data']['definition']['version'] == '0.2.0' and len(generated) == 7
        for value in generated.values():
            mv.validate(value)
        for entry in outer['data']['files']:
            assert (OUT/entry['path']).is_file()
        for path,value in generated.items():
            for entry in value['data']['files']:
                assert (OUT/path).parent.joinpath(entry['path']).is_file()
        return 'passed'
    @register('T3-03')
    def phases():
        for device,times in [('CNC01',[10000,45000,5000]),('Board01',[5000,20000,5000])]:
            op = operation(device)
            assert op['duration'] is None and [p['duration']['value'] for p in op['phases']] == times
            assert [bool(p['phase_resources']) for p in op['phases']] == [True,False,True]
            assert all(p['interruption_policy'] == 'pause_resume' for p in op['phases'])
        return 'passed'
    @register('T3-04')
    def preparation():
        prep, = companion['preparation_bindings']
        assert prep['target_device_id'] == 'Board01' and prep['before_device_ids'] == ['Board01']
        assert prep['resources'] == {'legacy.worker.W-E':'W-E'} and prep['material_delta'] == 0
        task = generated['definitions/preparation-board-preparation/module.json']['data']['definition']['payload']['data']
        assert task['duration']['value'] == 30000 and task['equipment_access'] == 'exclusive'
        assert task['completion_effect'] == {'target_role':'target','marker':'board-preparation','produces_material':False}
        return 'passed'
    @register('T3-05')
    def claims():
        assert len(companion['professional_claims']) == 2
        assert [c['original_payload'] for c in companion['professional_claims']] == source['professional_artifacts']
        assert all(c['evidence_origin'] == 'imported_claim' and not c['producer_result_created'] and not c['human_acknowledgment_created'] for c in companion['professional_claims'])
        return 'passed'
    @register('T3-06')
    def workers_seed():
        assert companion['source_resources'] == source['resources']
        assert [r['role'] for r in data['resource_requirements']] == ['legacy.worker.'+r['id'] for r in source['resources']]
        assert len(data['resource_requirements']) == 4
        for row in data['resource_requirements']:
            assert row['capacity']['value']['coefficient'] == 1
            assert row['default_calendar_ms'] == [{'start_ms':0,'end_ms':300000}]
        assert companion['deferred_seed']['initial_stock'] == source['initial_stock']
        assert companion['deferred_seed']['demand'] == source['demand']
        return 'passed'
    @register('T3-07')
    def real_missing():
        codes = {x['code'] for x in companion['unresolved_fields']}
        assert {'missing_hall_height','missing_device_geometry','unresolved_spec_reference','missing_product_BOM_route'} <= codes
        assert data['seed'] is None and all(d['footprint_m'] == [] and d['height_m'] is None for d in data['devices'])
        assert 'template_real_inputs_missing' in {x['code'] for x in outer['data']['issues']}
        return 'draft_blocked'
    @register('T3-08')
    def isolation():
        isolate = next(x for x in data['locations'] if x['id'] == 'store-isolate')
        assert isolate['purpose'] == 'isolation' and not isolate['ordinary_issue_enabled']
        check_references(template,companion,generated)
        return 'passed'
    def ref_rejected(mutator):
        candidate = copy.deepcopy(template)
        mutator(candidate['data']['connections'][0]['data'])
        tv.validate(candidate)
        try:
            check_references(candidate,companion,generated)
        except (AssertionError,KeyError,StopIteration):
            return 'reference_rejected'
        raise AssertionError('bad reference accepted')
    @register('T3-09')
    def unknown_location():
        return ref_rejected(lambda edge:edge.update(source={'kind':'location','data':'MISSING'}))
    @register('T3-10')
    def wrong_shape():
        candidate = copy.deepcopy(template)
        candidate['data']['connections'][0]['data']['source'] = {'kind':'location','data':{'module_ref':{'id':'x','version':'0.1.0'},'instance_id':'CNC01','port_id':'input'}}
        try:
            tv.validate(candidate)
        except ValidationError:
            return 'shape_rejected'
        raise AssertionError('wrong location shape accepted')
    @register('T3-11')
    def wrong_definition():
        def change(edge):
            edge['destination']['data']['module_ref']['version'] = '9.9.9'
        return ref_rejected(change)
    @register('T3-12')
    def wrong_store():
        return ref_rejected(lambda edge:edge.update(destination_store='store-shell'))
    @register('T3-13')
    def wrong_direction():
        def change(edge):
            edge['source'] = copy.deepcopy(edge['destination'])
        return ref_rejected(change)
    @register('T3-14')
    def namespace():
        t,c = remap_layout(template,companion,'trial2')
        tv.validate(t)
        check_references(t,c,generated)
        assert len(c['source_to_instance_ids']) == 40
        assert all(x.startswith('trial2.') for x in c['source_to_instance_ids'].values())
        assert t['data']['module_refs'] == data['module_refs']
        assert c['preparation_bindings'][0]['target_device_id'] == 'trial2.Board01'
        assert c['professional_claims'][0]['consumer_device_ids'] == ['trial2.CNC01']
        assert template['data']['devices'][0]['id'] == 'CNC01'
        return 'passed'
    @register('T3-15')
    def source_preserved():
        assert read(OUT/'source-project.design.json') == source
        fields = read(BASE/'DRF03_C04_逐字段去向.json')
        assert not fields['unknown_or_dropped_fields']
        def count(value):
            if isinstance(value,dict) and value:return sum(count(x) for x in value.values())
            if isinstance(value,list) and value:return sum(count(x) for x in value)
            return 1
        assert len(fields['fields']) == count(source)
        assert len({r['source_path'] for r in fields['fields']}) == count(source)
        assert companion['source_controls']['time'] == source['time']
        assert companion['quality_declarations']['source_values'] == ['PASS','PASS']
        assert companion['quality_declarations']['output_quality_declarations'] == [{'device_id':'BoardFinish01','values':{'pcb':'released','source':'declared_preset_for_flow_only'}}]
        return 'passed'
    @register('T3-16')
    def semantic_integration():
        contract = read(ROOT/companion['consumer_contract_ref'])
        # This case targets the preserved legacy0.2.0 outer candidate, not the new typed0.3.0.
        assert contract['legacy_round3_snapshot']['status'] == 'selected_owned_rule_public_import_integration_required'
        assert 'companion_consumer_requires_public_integration' in {x['code'] for x in outer['data']['issues']}
        return 'integration_required'
    # Execute only the actual pure function AST, not the generator's top-level writes.
    src = ast.parse((ROOT/'modules/templates/生成三类模板设计.py').read_text(encoding='utf-8'))
    fn = next(node for node in src.body if isinstance(node,ast.FunctionDef) and node.name == 'select_packaging_branch')
    ns = {'copy':copy,'PROJECT':ROOT,'read':read}
    exec(compile(ast.Module(body=[fn],type_ignores=[]),str(ROOT/'modules/templates/生成三类模板设计.py'),'exec'),ns)
    select = ns['select_packaging_branch']
    original = next(d for d in source['devices'] if d['id'] == 'Packing01')
    @register('C2G-01')
    def declared():
        candidate,selection = select(original,'declared_process')
        assert candidate == original and selection is None
        return 'passed'
    @register('C2G-02')
    def missing_selection():
        candidate,selection = select(original,'calculated_nominal')
        assert candidate['configuration'] == {} and selection['missing_selection'] == ['packaging_job_id','packaging_job_revision']
        return 'missing_selection'
    @register('C2G-03')
    def explicit():
        candidate,selection = select(original,'calculated_nominal',{'packaging_job_id':'EXPLICIT_JOB','packaging_job_revision':3})
        assert candidate['module_ref'] == {'id':'community.parameterized-pack','version':'0.2.0'}
        assert candidate['configuration'] == {'packaging_job_id':{'kind':'string','value':'EXPLICIT_JOB'},'packaging_job_revision':{'kind':'integer','value':3}}
        definition = read(ROOT/'packages/packaging/consumer-device.design.json')['data']['definition']
        assert selection['parameter_declarations'] == [p for p in definition['parameters'] if p['id'] in {'packaging_job_id','packaging_job_revision'}]
        assert read(ROOT/'modules/devices/parameterized-pack/manifest.json')['version'] == '0.1.0'
        return 'passed'
    @register('C2G-04')
    def boolean_revision():
        try:
            select(original,'calculated_nominal',{'packaging_job_id':'EXPLICIT_JOB','packaging_job_revision':True})
        except ValueError:
            return 'input_rejected'
        raise AssertionError('bool revision accepted')
    @register('C2G-05')
    def projection():
        selection = read(ROOT/'docs/00_项目基线/首版任务选择.json')
        tasks = {row['id']:row for row in selection['selected']}
        definition_ref = 'packages/packaging/consumer-device.design.json'
        for ident in ['P0-03','P4-02']:
            task = tasks[ident]
            assert definition_ref in task['execution_plan']['input_refs']
            assert definition_ref in (ROOT/task['coding_package_ref']).read_text(encoding='utf-8-sig')
        return 'passed'
    expected = read(BASE/'DRF03_第三轮独立预期.json')['cases']
    assert set(checks) == {row['id'] for row in expected}
    results = []
    for row in expected:
        actual = checks[row['id']]()
        results.append({'id':row['id'],'expected':row['expected'],'actual':actual,'matched':actual == row['expected']})
    result = {'document_kind':'actual_round3_finite_design_consumer_checks_not_Host', 'cases':results,'case_count':len(results),
              'all_independent_expectations_matched':all(x['matched'] for x in results),
              'current_canonical_Schema_used':True,'source_leaf_dispositions':len(read(BASE/'DRF03_C04_逐字段去向.json')['fields']),
              'public_companion_integration':'required_not_observed','real_geometry_Product_BOM_Route':'missing',
              'old_22_29_7_changed':False,'actual_Host_or_Engine':None,'public_progress_written':False}
    (BASE/'DRF03_第三轮核验结果.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({k:v for k,v in result.items() if k != 'cases'},ensure_ascii=False))
    assert result['all_independent_expectations_matched']


if __name__ == '__main__':
    main()
