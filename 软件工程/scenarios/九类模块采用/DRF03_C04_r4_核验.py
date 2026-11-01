"""独立正反材料；选定Schema与实际公共Schema严格分记，未调用Host/Engine。"""
from copy import deepcopy
import json
import sys

sys.dont_write_bytecode=True
from DRF03_C04_r4_采用输入 import (ROOT,BASE,PRIOR,OUT,DOCS,read,write,selected_defs,validator,
    has_public_delta,validate_semantics,remap_typed_adoption,bound_preset,output_quality_assignments)
from DRF03_C04_内层模板适配 import check_references
from jsonschema import ValidationError


def main():
    source=read(OUT/'source-project.design.json');template=read(OUT/'template.json');a=template['data']['adoption']
    modules={str(p.relative_to(OUT)).replace('\\','/'):read(p) for p in (OUT/'definitions').rglob('module.json')}
    current=read(ROOT/'tools/type-contract-proof/output/schemas/all-types.schema.json')['$defs']
    canonical=has_public_delta(current);defs=current if canonical else selected_defs(current)
    checks={};evidence={}
    def register(ident):
        def add(fn):checks[ident]=fn;return fn
        return add
    def semantic_reject(mutator):
        t,m=deepcopy(template),deepcopy(modules)
        mutator(t,m)
        try:validate_semantics(t,m,source)
        except (AssertionError,IndexError,KeyError,StopIteration,ValueError):return True
        raise AssertionError('bad semantic input admitted')
    @register('A4-01')
    def typed_source():
        validator('TemplateSubset',defs).validate(template['data'])
        validate_semantics(template,modules,source)
        assert len(template['data']['locations'])==12 and len(template['data']['connections'])==18 and len(modules)==7
        assert source==read(PRIOR/'source-project.design.json')
        return 'passed'
    @register('A4-02')
    def free_json():
        for fn in [lambda x:x.update(host_instructions={'create_stock':999}),lambda x:x['imported_claims'][0].update(accepted=True)]:
            candidate=deepcopy(a);fn(candidate)
            try:validator('TemplateAdoption',defs).validate(candidate)
            except ValidationError:continue
            raise AssertionError('unknown instruction or acceptance field admitted')
        return 'shape_rejected'
    @register('A4-03')
    def phases_old():
        for ident,times in [('CNC01',[10000,45000,5000]),('Board01',[5000,20000,5000])]:
            definition=modules['definitions/'+ident+'/module.json']['data']['definition']
            op=definition['payload']['data']['behavior']['inline']
            assert [p['duration']['value'] for p in op['phases']]==times and op['duration'] is None
            assert all(p['interruption_policy']=='pause_resume' for p in op['phases'])
        for path,package in modules.items():
            old=read(PRIOR/path)
            if 'BoardFinish01' not in path:assert package==old
            else:
                assert old['data']['definition']['version']=='0.1.0'
                assert 'declared_output_quality' not in old['data']['definition']['payload']['data']['behavior']['inline']['outputs'][0]
        assert read(ROOT/'modules/devices/parameterized-pack/manifest.json')['version']=='0.1.0'
        return 'passed'
    @register('A4-04')
    def prepare_pending():
        prep,=a['preparation_bindings'];task=modules['definitions/preparation-board-preparation/module.json']['data']['definition']['payload']['data']
        assert prep['route_refs']==[] and prep['worker_roles']=={'legacy.worker.W-E':'W-E'}
        assert task['duration']['value']==30000 and not task['completion_effect']['produces_material']
        assert next(r for r in a['resource_choices'] if r['source_id']=='W-E')['resource_id'] is None
        return 'unresolved'
    @register('A4-05')
    def task_existing_contract():
        task=modules['definitions/preparation-board-preparation/module.json']['data']['definition']
        assert task['payload']['category']=='tasks'
        algorithm=read(ROOT/'docs/03_架构与契约/调度与状态转换算法.json')
        text=json.dumps(algorithm,ensure_ascii=False)
        assert '已完成同配置用已有marker判就绪' in text and '不重复取得资源' in text
        prior=read(BASE/'DRF03_第二轮核验结果.json')
        case=next(row for row in prior['cases'] if row['id']=='C1-04')
        assert case['matched']
        evidence['A4-05']='Existing Task lowering evidence + current common normative marker reuse; no new route or actual Engine schedule claimed.'
        return 'passed'
    @register('A4-06')
    def workers():
        assert len(a['resource_choices'])==4 and all(r['resource_id'] is None for r in a['resource_choices'])
        for r in a['resource_choices']:
            req=template['data']['resource_requirements'][r['requirement_index']]
            assert req['role']=='legacy.worker.'+r['source_id'] and req['capacity']['unit']=='person'
            assert req['default_calendar_ms']==[{'start_ms':0,'end_ms':300000}]
        project=read(OUT/'project-draft.json')['data']
        assert project['resources']==[] and project['calendars']==[]
        return 'unresolved'
    @register('A4-07')
    def bad_worker_target():
        assert semantic_reject(lambda t,m:t['data']['adoption']['resource_choices'][0].update(requirement_index=3))
        assert semantic_reject(lambda t,m:t['data']['adoption']['preparation_bindings'][0].update(target_device_id='MISSING'))
        return 'reference_rejected'
    @register('A4-08')
    def claims():
        assert len(a['imported_claims'])==2
        for i,c in enumerate(a['imported_claims']):
            assert c['source_pointer']==f'/professional_artifacts/{i}' and c['mode']=='declared_process'
            assert c['id']==source['professional_artifacts'][i]['id']
            assert not {'result_ref','actual_outcome','acknowledgment','accepted'} & set(c)
        project=read(OUT/'project-draft.json')['data'];assert project['acknowledgments']==[] and project['bindings']==[]
        return 'passed'
    @register('A4-09')
    def no_global_preset():
        p,=a['inspection_presets'];assert p['results']==[{'kind':'string','value':'PASS'}]*2
        assert bound_preset(a,p,set()) is None
        return 'unresolved'
    @register('A4-10')
    def explicit_preset():
        p=deepcopy(a['inspection_presets'][0]);p.update(item_or_ancestor='TEST_ACTUAL_ANCESTOR',match_kind='ancestor')
        projected=bound_preset(a,p,{'TEST_ACTUAL_ANCESTOR'})
        validator('InspectionPreset',current).validate(projected)
        assert projected['id']==a['id']+'.'+p['id'] and projected['results']==p['results']
        evidence['A4-10']='Explicit known-item type_fixture only; no actual source item/Scenario manufactured.'
        return 'passed'
    op=modules['definitions/BoardFinish01/module.json']['data']['definition']['payload']['data']['behavior']['inline']
    slots=[{'item_id':'ACTUAL_OUTPUT_B','job_id':'JOB_B','port':'output'},
           {'item_id':'OTHER_JOB_PCB','job_id':'JOB_A','port':'output'},
           {'item_id':'UNRELATED_PORT','job_id':'JOB_B','port':'other'}]
    @register('A4-11')
    def actual_slot_only():
        assert output_quality_assignments(op,'last_effective_phase_completed',True,'JOB_B',slots)==[{'item_id':'ACTUAL_OUTPUT_B','quality':'released'}]
        assert slots[1]['item_id']=='OTHER_JOB_PCB'
        evidence['A4-11']='Finite existing-output-slot effect reference, no material creation/inventory/scheduler.'
        return 'passed'
    @register('A4-12')
    def early_release():
        assert all(output_quality_assignments(op,moment,True,'JOB_B',slots)==[] for moment in ['import','adopt','job_started','intermediate_phase_completed','cancelled'])
        return 'no_effect'
    @register('A4-13')
    def retry():
        assert output_quality_assignments(op,'last_effective_phase_completed',False,'JOB_B',slots)==[]
        return 'no_effect'
    @register('A4-14')
    def bad_quality():
        path='definitions/BoardFinish01/module.json'
        assert semantic_reject(lambda t,m:m[path]['data']['definition']['payload']['data']['behavior']['inline']['inputs'][0].update(declared_output_quality='released'))
        assert semantic_reject(lambda t,m:m[path]['data']['definition']['payload']['data']['behavior']['inline']['outputs'][0].update(port='input'))
        return 'semantic_rejected'
    @register('A4-15')
    def seed_pending():
        seed=a['deferred_seed'];assert len(seed['initial_stock'])==5 and seed['demand']['quantity']['value']['coefficient']==2
        assert all(seed[k] is None for k in ['product_ref','bom_ref','routing_ref'])
        assert all(row['quality'] is None for row in seed['initial_stock'])
        assert template['data']['seed'] is None
        assert {'site_geometry','professional_input','specification','product_bom_route','initial_quality'} <= {row['kind'] for row in a['missing_inputs']}
        return 'unresolved'
    @register('A4-16')
    def remapping():
        types={row['id']:'test2.'+row['id'] for row in template['data']['item_types']}
        t,c=remap_typed_adoption(template,read(OUT/'adoption-material.json'),'test2',types)
        validator('TemplateSubset',defs).validate(t['data']);validate_semantics(t,modules,source)
        check_references(t,c,modules)
        mapped=t['data']['adoption']
        assert mapped['preparation_bindings'][0]['target_device_id']=='test2.Board01'
        assert mapped['imported_claims'][0]['consumer_device_ids']==['test2.CNC01']
        assert mapped['deferred_seed']['demand']['destination_location_id']=='test2.store-package'
        assert mapped['deferred_seed']['demand']['item_type_id']=='test2.package'
        assert mapped['resource_choices']==a['resource_choices'] and t['data']['module_refs']==template['data']['module_refs']
        return 'passed'
    @register('A4-17')
    def reopen():
        for path,key in [(OUT/'template.json','adoption'),(OUT/'project-draft.json','template_adoptions')]:
            restored=json.loads(json.dumps(read(path),ensure_ascii=False))
            record=restored['data'][key] if key=='adoption' else restored['data'][key][0]
            assert record==a and (OUT/record['source_archive']['path']).is_file()
        project=read(OUT/'project-draft.json')['data'];project['issues']=[]
        assert all(x['resource_id'] is None for x in project['template_adoptions'][0]['resource_choices'])
        validator('ProjectState',defs).validate(project)
        return 'passed'
    @register('A4-18')
    def bad_archive():
        candidate=deepcopy(a);candidate['source_archive']['path']='../outside.json'
        try:validator('TemplateAdoption',defs).validate(candidate)
        except ValidationError:pass
        else:raise AssertionError('escaping path admitted')
        assert semantic_reject(lambda t,m:t['data']['adoption']['imported_claims'][0].update(source_pointer='/professional_artifacts/99'))
        return 'input_rejected'
    @register('A4-19')
    def actual_vs_selected():
        record=read(DOCS/'DRF03_C04_r4_适配结果.json')
        assert canonical and record['public_merge_observed'] and record['schema_kind']=='actual_canonical'
        assert 'template_adoptions' in current['ProjectState']['required']
        assert 'declared_output_quality' in current['MaterialRoleQuantity']['properties']
        validator('TemplateAdoption',current).validate(a)
        validator('ProjectState',current).validate(read(OUT/'project-draft.json')['data'])
        api_defs=read(ROOT/'tools/type-contract-proof/output/schemas/api-all-types.schema.json')['$defs']
        validator('ImportDraft',api_defs).validate(read(OUT/'import-draft.json'))
        validator('TemplateInstantiation',api_defs).validate(read(OUT/'template-instantiation.json'))
        return 'canonical_matched'
    expected=read(BASE/'DRF03_C04_r4_独立预期.json')['cases'];assert set(checks)=={row['id'] for row in expected}
    results=[]
    for row in expected:
        actual=checks[row['id']]();results.append({'id':row['id'],'expected':row['expected'],'actual':actual,'matched':actual==row['expected']})
    result={'document_kind':'actual_finite_typed_adoption_design_consumer_not_runtime','schema_kind':'actual_canonical' if canonical else 'selected_not_canonical',
        'case_count':len(results),'all_matched':all(x['matched'] for x in results),'cases':results,'evidence_limits':evidence,
        'prior_22_29_7_21_changed':False,'public_progress_written':False,'actual_Host_or_Engine':None,'actual_complete_professional':None}
    write(DOCS/'DRF03_C04_r4_核验结果.json',result)
    print(json.dumps({k:v for k,v in result.items() if k not in ['cases','evidence_limits']},ensure_ascii=False))
    assert result['all_matched']


if __name__=='__main__':main()
