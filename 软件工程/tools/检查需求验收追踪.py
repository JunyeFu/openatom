"""S03-05需求/输入/预期/责任追踪检查；不执行产品验收。"""
import json
from pathlib import Path

S = Path(__file__).resolve().parent.parent
D = S / 'docs/04_开发与验收'
REF = 'docs/04_开发与验收/首版需求验收追踪.json'


def read(ref):
    return json.loads((S / ref).read_text(encoding='utf-8-sig'))


def verify():
    trace = read(REF)
    selection = read(trace['source_refs']['selection'])
    stage = read(trace['source_refs']['stage'])
    nonfunctional = read(trace['source_refs']['nonfunctional'])
    user_tasks = read(trace['source_refs']['user_tasks'])['rows']
    operations = read(trace['source_refs']['operations'])['operation_catalog']
    tasks_doc = read('docs/04_开发与验收/tasks.json')
    task_map = {t['id']: t for t in selection['selected']}
    sf_map = {r['id']: r for r in stage['sf_evidence']}
    cases = {r['id']: r for r in trace['cases']}
    preparations = {r['id']: r for r in trace['preparation']}
    rows = trace['functional_requirements']
    expected_ids = selection['scope_freeze']['included_requirement_ids']
    assert [r['id'] for r in rows] == expected_ids
    assert len(cases) == len(trace['cases']) == 17
    assert trace['actual_runtime'] is None
    source_table = {}
    for line in (S / trace['source_refs']['requirements']).read_text(encoding='utf-8-sig').splitlines():
        if line.startswith('|SF-'):
            cells = line.strip('|').split('|')
            source_table[cells[0]] = cells[1:]

    parsed = {}
    checked_refs = 0

    def reference(ref):
        nonlocal checked_refs
        path = S / ref['path']
        assert path.exists(), ref
        checked_refs += 1
        pointer = ref['json_pointer']
        if path.suffix == '.json':
            if ref['path'] not in parsed:
                parsed[ref['path']] = read(ref['path'])
            node = parsed[ref['path']]
            for token in pointer.strip('/').split('/') if pointer else []:
                token = token.replace('~1', '/').replace('~0', '~')
                node = node[int(token)] if isinstance(node, list) else node[token]
            if 'expected_identity' in ref:
                assert node[ref['identity_field']] == ref['expected_identity'], ref
            return node
        assert not pointer, ref

    all_operations = set()
    for row in rows:
        assert row['priority'] == 'first_release_must'
        assert [row['title'], row['acceptance']] == source_table[row['id']]
        assert row['producer_tasks'] == sf_map[row['id']]['producer_tasks']
        assert row['evidence_owner_task'] in row['producer_tasks'] + [row['closure_task']]
        assert row['evidence_owner_window'] == task_map[row['evidence_owner_task']]['planned_window_id']
        assert row['closure_task'] == 'P5-01'
        assert row['verification_milestone'] == sf_map[row['id']]['verification_milestone']
        assert row['user_task_ids'] == [u['user_task'] for u in user_tasks if row['id'] in u['requirements']]
        assert row['nonfunctional_ids'] == [n['id'] for n in nonfunctional['requirements'] if row['id'] in n['related_SF']]
        assert row['case_ids'] and len(row['case_ids']) == len(set(row['case_ids']))
        assert set(row['case_ids']) <= set(cases)
        all_operations.update(row['operation_refs'])
        assert reference(row['actual_result_slot']) is None
    assert all_operations == {op['operation'] for op in operations}

    for row in cases.values():
        assert row['owner_task'] in task_map
        assert row['owner_window'] == task_map[row['owner_task']]['planned_window_id']
        assert row['input_refs'] and row['procedure'] and row['expected']
        assert row['preparation_id'] in preparations
        assert row['runtime_status'] == 'not_executed' and row['actual_command'] is None and row['actual_result'] is None
        for ref in row['input_refs'] + row['expected_refs']:
            reference(ref)
    for row in preparations.values():
        assert set(row['owner_tasks']) <= set(task_map)
        assert row['gate'] and row['input_state'] and row['required_artifacts'] and row['steps']
    assert cases['AC-12']['preparation_id'] == cases['AC-13']['preparation_id'] == 'PREP-PACKAGING'
    assert cases['AC-17']['preparation_id'] == 'PREP-AGENT'
    assert [ref['expected_identity'] for ref in cases['AC-17']['expected_refs']] == [f'GF-{i:02}' for i in range(1, 8)]

    # S03-02的规则边界必须进入真实验收路径，文件存在不能代替覆盖。
    boundary_ref = 'scenarios/真实业务设计/business-rule-boundaries.design.json'
    boundary_ids = {r['id'] for r in read(boundary_ref)['cases']}
    for kind in ['input_refs', 'expected_refs']:
        covered = {ref.get('expected_identity') for case in cases.values() for ref in case[kind]
                   if ref['path'] == boundary_ref}
        assert covered == boundary_ids, (kind, covered)
    category_ref = 'scenarios/九类模块采用/expected-category-adoption.design.json'
    first_release_ids = {r['id'] for r in read(category_ref)['cases']
                         if r['release_scope'] == 'first_release_design_expectation'}
    assert {ref.get('expected_identity') for ref in cases['AC-04']['expected_refs']
            if ref['path'] == category_ref} == first_release_ids
    assert 'AC-12' in next(r for r in rows if r['id'] == 'SF-02')['case_ids']
    assert 'AC-11' in next(r for r in rows if r['id'] == 'SF-04')['case_ids']
    assert sf_map['SF-15']['verification_milestone'] == 'MS-12'
    assert all(r['verification_milestone'] == 'MS-11' for r in rows if r['id'] != 'SF-15')
    assert 'SF-15' not in preparations['PREP-MATERIALS']['required_passed_functional_ids']
    assert set(preparations['PREP-MATERIALS']['required_passed_functional_ids']) == set(expected_ids) - {'SF-15'}

    nfr_rows = trace['nonfunctional_requirements']
    assert [r['id'] for r in nfr_rows] == [r['id'] for r in nonfunctional['requirements']]
    for row, original in zip(nfr_rows, nonfunctional['requirements']):
        assert reference(row['source']) == original
        assert row['priority'] == 'first_release_must'
        assert row['producer_tasks'] == original['implementation_tasks'] and row['related_SF'] == original['related_SF']
        assert row['evidence_owner_task'] in row['producer_tasks']
        assert row['evidence_owner_window'] == task_map[row['evidence_owner_task']]['planned_window_id']
        for ref in row['input_refs']:
            reference(ref)
        assert reference(row['actual_result_slot']) is None
    assert [r['id'] for r in trace['integration_checks']] == [r['id'] for r in selection['shared_scenario_checks']]
    for row, original in zip(trace['integration_checks'], selection['shared_scenario_checks']):
        assert row['owner_task'] == original['owner']
        assert row['producer_tasks'] == original['producer_tasks']
        assert row['input_refs'] == original['input_refs'] and row['required_observations'] == original['required_observations']
        assert row['release_gate'] == original['release_gate']
        assert set(row['related_SF']) <= set(expected_ids)
        assert reference(row['actual_result_slot']) is None

    assert [r['task_id'] for r in trace['task_projection']] == [r['id'] for r in selection['selected']]
    for row in trace['task_projection']:
        original = task_map[row['task_id']]
        assert row['functional_ids'] == original['requirement_ids'] and row['scope'] == original['scope']
        assert row['nonfunctional_ids'] == [n['id'] for n in nonfunctional['requirements'] if row['task_id'] in n['implementation_tasks']]
        assert row['runtime_status'] == 'planned' and row['actual_result'] is None
        assert reference(row['task_source'])['id'] == row['task_id']
    assert not set(task_map) & set(selection['deferred'])
    assert selection['requirement_trace_ref'] == tasks_doc['first_release_requirement_trace_ref'] == REF
    assert read('docs/04_开发与验收/编码任务包/任务包索引.json')['requirement_trace_ref'] == REF
    selected_ids = set(task_map)
    for row in selection['selected']:
        expected = row['requirement_ids'] + [n['id'] for n in nonfunctional['requirements'] if row['id'] in n['implementation_tasks']]
        mirror = next(t for t in tasks_doc['tasks'] if t['id'] == row['id'])
        assert row['acceptance_trace_ids'] == mirror['first_release_acceptance_trace_ids'] == expected
    assert all(t['first_release_acceptance_trace_ids'] == [] for t in tasks_doc['tasks'] if t['id'] not in selected_ids)
    for row in stage['sf_evidence']:
        assert row['requirement_trace_ref'] == REF

    for name in ['machining-delivery', 'purchased-assembly', 'parallel-electromechanical']:
        policy = read(f'modules/templates/{name}/project.design.json')['import_policy']['into_existing_project']
        assert all(part in policy for part in ['preserve_existing_stock_and_demand', 'without_sample_stock_or_demand', 'seed_requires_explicit_choice_in_new_scenario'])
    return dict(status='passed_static_acceptance_trace', functional_requirements=len(rows), nonfunctional_requirements=len(nfr_rows),
                acceptance_cases=len(cases), selected_tasks=len(task_map), planned_operations=len(all_operations), integration_checks=len(trace['integration_checks']),
                checked_input_expected_and_result_references=checked_refs, preparation_conditions=list(preparations),
                business_boundaries_traced=len(boundary_ids), first_release_category_expectations=len(first_release_ids),
                material_acceptance_cycle=False,
                actual_product_cases_executed=0, actual_runtime=None, errors=[])


if __name__ == '__main__':
    result = verify()
    (D / '需求验收追踪检查结果.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(result, ensure_ascii=False))
