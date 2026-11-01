"""检查用户要求的跨文件主线同步；不运行产品、不增加成熟度或S阶段通过项。"""
from pathlib import Path
from collections import Counter
from urllib.parse import unquote
from html.parser import HTMLParser
import ast
import csv
import json
import re
import runpy
import sqlite3
from datetime import datetime, timedelta, timezone

S = Path(__file__).resolve().parent.parent
P = S.parent
D = S / 'docs/04_开发与验收'
W = P.parents[2]
CHECK_DATE = datetime.now(timezone(timedelta(hours=8))).date().isoformat()


def read_json(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def preserved(path):
    if not path.is_relative_to(P):
        return False
    parts = path.relative_to(P).parts
    return any(x in parts for x in ['原始资料', '历史课题总结', '临时文件', '__pycache__']) or path.name.endswith('.pyc')


def source_record(path):
    if not path.is_relative_to(P):
        return False
    parts = path.relative_to(P).parts
    return '阅读卡' in parts or path.name in ['来源目录.json', '来源目录.csv', '原资料引用目录.json', '模块资料映射.json', '补充来源.json', '补充人因与工时来源.json', '本轮补充来源.json', '投用与专业来源.json', '业务设计来源.json', '补充示例来源.json', '附加技术来源.json', 'NBS表格提取临时.json']


def reference_parent(path):
    # 已声明的逐字源码快照保留原相对链接；按清单中的原文件位置核验，不改快照。
    if path.is_relative_to(S) and 'final-source' in path.relative_to(S).parts:
        parts = path.relative_to(S).parts
        cut = S.joinpath(*parts[:parts.index('final-source')]) / 'source-cut.json'
        if cut.is_file():
            for entry in read_json(cut).get('sources', []):
                if (S / entry['snapshot']).resolve() == path.resolve():
                    return (S / entry['path']).parent
    return path.parent


class Links(HTMLParser):
    def __init__(self):
        super().__init__()
        self.links = []

    def handle_starttag(self, tag, attrs):
        if tag in ['a', 'img']:
            self.links += [v for k, v in attrs if k in ['href', 'src'] and v]


def main():
    errors = []
    backend_ref = read_json(D / '完整设计与执行准入.json')['execution_entry']['current_backend_slice_ref']
    backend_path = S / backend_ref
    backend = read_json(backend_path) if backend_path.exists() else None
    backend_dispatched = bool(backend and backend.get('dispatch'))
    backend_reviewed = bool(backend and backend['status'] == 'implemented_and_reviewed')
    selection = read_json(S / 'docs/00_项目基线/首版任务选择.json')
    selected_tasks = {row['id']: row for row in selection['selected']}
    package_index = read_json(S / selection['coding_package_index_ref'])
    dispatched_packages = {row['task_id']: row for row in package_index['packages'] if row['actual_dispatch']}
    dispatched_receipts = {ident: read_json(S / row['current_execution_ref'])
                           for ident, row in dispatched_packages.items()}
    for ident, receipt in dispatched_receipts.items():
        assert ident in selected_tasks and receipt['task_id'] == ident
        assert receipt['dispatch'] == dispatched_packages[ident]['actual_dispatch']
        assert 1 <= receipt['within_original_step'] <= len(selected_tasks[ident]['execution_plan']['steps'])
        if receipt['dispatch'].get('dispatch_mode') == 'whole_task_package':
            source = selected_tasks[ident]
            assert receipt['scope']['original_steps_scope'] == list(range(1, len(source['execution_plan']['steps']) + 1))
            assert receipt['source_task_snapshot']['execution_plan']['steps'] == source['execution_plan']['steps']
            assert receipt['source_task_snapshot']['release_acceptance'] == source['release_acceptance']
            packet = (S / receipt['full_dispatch_packet_ref']).read_text(encoding='utf-8-sig')
            assert all(step in packet for step in source['execution_plan']['steps'])
            assert source['release_acceptance'] in packet
    if backend_reviewed:
        assert backend['task_id'] in selected_tasks
        test_count = backend['actual_result']['tests']['distinct_tests']
        assert test_count > 0
        for run in backend['actual_result']['command_runs']:
            assert run['exit_code'] == 0
            if 'cargo test ' in run['command'] or run['command'] in ['npm run test:rust', 'npm.cmd run test:rust']:
                log = (S / run['stdout_ref']).read_text(encoding='utf-8-sig')
                passed = sum(map(int, re.findall(r'test result: ok\. (\d+) passed;', log)))
                assert passed == run.get('distinct_tests_executed', test_count)
        assert backend['actual_result']['tests']['failed'] == 0
        assert backend['review']['confirmed_defects'] == [] and backend['whole_task_done'] is False
    row_issues = {}
    checked_links = 0
    matrix = read_json(D / '主线同步矩阵.json')
    results = []
    for check in matrix['checks']:
        joined = []
        missing = []
        for ref in check['consumers']:
            path = S / ref
            if not path.exists():
                missing.append(ref)
            elif path.suffix != '.pdf':
                joined.append(path.read_text(encoding='utf-8-sig'))
        text = '\n'.join(joined)
        missing += [token for token in check['combined_required_tokens'] if token not in text]
        results.append({**check, 'status': '设计同步通过' if not missing else '待修正', 'missing': missing})
        if missing:
            errors.append({'check': check['id'], 'missing': missing})

    catalog = read_json(S / 'modules/catalog.json')
    contracts = read_json(S / 'packages/module-kit/category-contracts.design.json')
    commands = read_json(S / 'packages/commands/command-catalog.design.json')['operation_catalog']
    tasks_doc = read_json(S / 'docs/04_开发与验收/tasks.json')
    tasks = tasks_doc['tasks']
    mapping_doc = read_json(S / 'docs/00_项目基线/用户任务与需求映射.json')
    mappings = mapping_doc['rows']
    maturity = read_json(S / 'docs/04_开发与验收/软件开发流程与当前成熟度评估.json')
    ids = ['items', 'devices', 'resources', 'tasks', 'operations', 'transport', 'templates', 'professional', 'media']
    assert [x['id'] for x in catalog['categories']] == ids == [x['id'] for x in contracts['categories']]
    assert len(catalog['modules']) == 21
    assert len(commands) == 18 == len({x['operation'] for x in commands})
    available_operations = [x['operation'] for x in commands if x['runtime_available']]
    for operation in commands:
        assert operation['implementation_status'] == ('implemented_minimal_scope' if operation['runtime_available'] else 'planned')
    if available_operations:
        execution = read_json(S / 'docs/03_架构与契约/公开操作与宿主调用契约.json')['actual_product_API_execution']
        assert available_operations == execution['implemented_operations']
        for operation in commands:
            if operation['runtime_available']:
                receipt = read_json(S / operation['implementation_ref'])
                assert receipt['status'] == 'implemented_and_reviewed'
                assert operation['operation'] in receipt['actual_result']['implemented_operations']
    assert len(tasks) == 28 and len(selection['selected']) == 21 and len(selection['deferred']) == 7
    for task in tasks + selection['selected']:
        assert task['status'] == ('in_progress' if task['id'] in dispatched_packages else 'planned')
        assert task['actual_result'] is None
    assert {r for row in mappings for r in row['requirements']} == {f'SF-{i:02}' for i in range(1, 18)}
    # 联合验收的三处已观测问题：逐行任务映射、通用计量与达标即交付分支。
    task_table = (S / 'docs/00_项目基线/用户任务与产品边界.md').read_text(encoding='utf-8')
    for row in mappings:
        table_row = next(line for line in task_table.splitlines() if line.startswith('|' + row['user_task'] + ' '))
        refs = {f'SF-{n}' for n in re.findall(r'\d{2}', table_row.split('|')[-2])}
        assert refs == set(row['requirements']), ('UT_SF_mismatch', row['user_task'], refs)
    # S03-01实际补齐七UT的功能交接，引用原操作目录/所选任务，不增加运行结果。
    functional_contract = mapping_doc['functional_contract']
    functional_review = read_json(S / functional_contract['review_ref'])
    assert [row['user_task'] for row in mappings] == [f'UT-{i:02}' for i in range(1, 8)]
    referenced_operations = {op for row in mappings for op in row['operation_refs']}
    assert referenced_operations == {op['operation'] for op in commands}
    assert {task for row in mappings for task in row['implementation_tasks']} == {task['id'] for task in selection['selected']}
    for row in mappings:
        assert all(row[field] for field in ['input', 'human_actions', 'agent_actions', 'success_condition', 'incomplete_or_unsatisfied', 'handoff'])
        assert row['actual_result'] is None
    assert functional_contract['checkpoint'] == functional_review['checkpoint'] == 'S03-01'
    assert functional_contract['runtime_available'] is False and functional_contract['actual_user_validation'] is None
    assert functional_review['actual_runtime'] is None and functional_review['actual_target_user_validation'] is None
    assert functional_review['external_actions'] == []
    assert set(functional_review['spec_refs']) >= {mapping_doc['requirements'], mapping_doc['product_definition']}
    # S03-02修正实际状态/接受/取消缺口，有限复核不运行或改写产品与冻结预期。
    business = read_json(S / 'packages/domain/business-objects.design.json')
    business_review = read_json(S / business['review_ref'])
    assert business_review['checkpoint'] == 'S03-02'
    assert business['objects']['item_state']['fields']['lifecycle'].split('|') == business['state_axes']['lifecycle']
    assert 'reserved' not in business['state_axes']['lifecycle']
    assert business['objects']['material_allocation']['fields']['mode'].split('|') == business['state_axes']['allocation']
    assert business['state_axes']['reserved_is_allocation_only'] and business['state_axes']['available_is_not_free_stock']
    assert [row['id'] for row in business['exception_paths']] == [f'EX-{i:02}' for i in range(1, 10)]
    assert all(row['trigger'] and row['preserved_state_and_output'] and row['continuation_or_exit'] for row in business['exception_paths'])
    assert all((S / ref).exists() for ref in business['object_contract_refs'].values())
    assert read_json(S / 'packages/domain/factory-contracts.design.json')['business_rules_ref'] == 'business-objects.design.json'
    binding = read_json(S / 'packages/domain/professional-artifact-binding.design.json')
    assert set(binding['first_release_modes']) | set(binding['deferred_modes']) == set(binding['modes'])
    assert 'software_in_loop' in binding['deferred_modes'] and 'software_in_loop' not in binding['first_release_modes']
    rule_verification = runpy.run_path(str(S / 'scenarios/真实业务设计/复核业务规则.py'))['verify']()
    assert rule_verification == read_json(S / 'scenarios/真实业务设计/business-rule-verification.json')
    assert rule_verification['existing_expectations_rewritten'] is False
    assert rule_verification['actual_runtime'] is None and business['actual_result'] is None
    assert business_review['actual_runtime'] is None and business_review['external_actions'] == []
    # S03-05验证具体节点/身份和真实消费者，不把准备配方当产品运行通过。
    trace_verification = runpy.run_path(str(S / 'tools/检查需求验收追踪.py'))['verify']()
    assert trace_verification == read_json(D / '需求验收追踪检查结果.json')
    trace = read_json(D / '首版需求验收追踪.json')
    trace_review = read_json(S / trace['report_ref'])
    assert trace_review['checkpoint'] == 'S03-05'
    assert trace_review['actual_runtime'] is None and trace_review['external_actions'] == []
    # S03-04引用独立非功能规格及有限数学结果，不把声明负载当实际基准。
    nfr_ref = selection['nonfunctional_spec_ref']
    nfr = read_json(S / nfr_ref)
    nfr_review = read_json(S / nfr['review_ref'])
    nfr_result = read_json(S / 'docs/04_开发与验收/非功能规格检查结果.json')
    assert nfr['checkpoint'] == nfr_review['checkpoint'] == 'S03-04'
    assert nfr_review['status'] == 'passed_document_specification'
    assert nfr_result['status'] == 'passed_static_specification_and_finite_math'
    assert len(nfr['requirements']) == nfr_result['NFR_rows'] == 11
    assert len(nfr_result['synthetic_math_results']) == 11 and nfr_result['errors'] == []
    assert nfr_result['readiness_and_result_classification_checks'] == 'passed_static_specification_and_finite_cases'
    assert nfr_result['confirmed_choices_covered'] == nfr['confirmation_coverage']
    assert len(nfr['confirmation_coverage']) == 10
    assert nfr_result['measurement_matrix'] == nfr['test_execution_contract']['matrix']
    assert nfr['actual_runtime'] is None and nfr['actual_measurements'] == []
    assert nfr_review['actual_performance_result'] is None and nfr_result['actual_performance_result'] is None
    assert nfr_review['external_actions'] == [] and nfr_review['functional_source_edit_by_this_review'] is False
    nav_ref = selection['navigation_spec_ref']
    nav = read_json(S / nav_ref)
    nav_review = read_json(S / nav['review_ref'])
    nav_result = read_json(D / '交互导航检查结果.json')
    assert nav_review['checkpoint'] == 'S04-01' and nav_review['status'] == 'passed_document_specification'
    assert nav_result['errors'] == [] and nav_result['status'] == 'passed_static_navigation_specification'
    assert nav['normal_canvases'] == ['line_canvas', 'multi_hall_canvas']
    assert nav['zoom_opens_detail'] is False and nav['zoom_switches_canvas'] is False
    assert nav_result['task_routes'] == 7 and nav_result['SF_coverage'] == 17 and nav_result['planned_operation_refs'] == 18
    assert nav['runtime_available'] is False and nav_review['runtime_or_frontend'] is None
    assert tasks_doc['first_release_navigation_spec_ref'] == nav_ref
    assert read_json(D / '完整设计与执行准入.json')['navigation_spec_ref'] == nav_ref
    form_ref = selection['form_spec_ref']
    form_spec = read_json(S / form_ref)
    form_review = read_json(S / form_spec['review_ref'])
    assert form_review['checkpoint'] == 'S04-03' and form_review['status'] == 'passed_document_specification'
    assert nav_result['form_spec_checks']['status'] == 'passed_static_form_and_feedback_specification'
    assert tasks_doc['first_release_form_spec_ref'] == form_ref
    assert form_spec['actual_frontend'] is None and form_review['actual_UI_or_product_execution'] is None
    architecture_ref = selection['component_architecture_ref']
    architecture = read_json(S / architecture_ref)
    architecture_result = read_json(D / '架构接点检查结果.json')
    architecture_review = read_json(S / architecture['review_ref'])
    ownership = read_json(S / selection['ownership_spec_ref'])
    ownership_review = read_json(S / ownership['review_ref'])
    assert architecture_result['errors'] == [] and architecture_result['status'] == 'passed_static_architecture_specification'
    assert architecture_review['status'] == ownership_review['status'] == 'passed_document_specification'
    assert architecture_result['ownership_checks']['status'] == 'passed_static_ownership_and_finite_reasoning'
    assert architecture['actual_runtime'] is None and ownership['actual_runtime'] is None
    assert tasks_doc['first_release_component_architecture_ref'] == architecture_ref
    assert tasks_doc['first_release_ownership_spec_ref'] == selection['ownership_spec_ref']
    module_protocol = read_json(S / selection['module_protocol_ref'])
    module_protocol_review = read_json(S / module_protocol['review_ref'])
    module_protocol_result = read_json(S / 'docs/04_开发与验收/九类协议检查结果.json')
    assert module_protocol_review['status'] == 'passed_document_specification'
    assert module_protocol_result['status'] == 'passed_static_protocol_and_finite_reasoning'
    assert module_protocol_result['payload_field_contracts'] == 61
    assert module_protocol_result['checked_parameter_leaf_bindings'] == 28
    assert len(module_protocol_result['finite_reasoning_cases']) == 9
    assert module_protocol_result['actual_loader_or_engine_execution'] is None
    assert tasks_doc['first_release_module_protocol_ref'] == selection['module_protocol_ref']
    technical = read_json(S / selection['technical_selection_ref'])
    technical_review = read_json(S / technical['review_ref'])
    technical_result = read_json(S / 'docs/04_开发与验收/技术部署检查结果.json')
    assert technical_review['status'] == 'passed_document_specification'
    assert technical_result['status'] == 'passed_static_technical_and_deployment_design'
    assert technical_result['selected_dependencies'] == 22
    assert technical['actual_runtime'] is None and technical_result['actual_product_compile'] is None
    assert tasks_doc['first_release_technical_selection_ref'] == selection['technical_selection_ref']
    scheduler_ref = selection['scheduler_spec_ref']
    scheduler = read_json(S / scheduler_ref)
    scheduler_review = read_json(S / scheduler['review_ref'])
    scheduler_result = read_json(D / '调度算法检查结果.json')
    scheduler_trial = read_json(S / scheduler['isolated_std_trial_ref'])
    assert scheduler_review['status'] == 'passed_document_specification'
    assert scheduler_result['status'] == 'passed_static_specification_and_finite_reasoning'
    assert scheduler_result['algorithm_rules'] == len(scheduler['rules']) == 14
    assert len(scheduler_result['finite_cases']) == 21
    assert scheduler_result['logical_handoffs'] == scheduler_result['applicable_views'] == 6
    assert scheduler_result['actual_product_runtime'] is None and scheduler_result['actual_product_cases_executed'] == 0
    assert scheduler_trial['status'] == 'passed_actual_std_queue_subset' and scheduler_trial['actual_product_engine'] is False
    type_ref = selection['type_format_spec_ref']
    type_spec = read_json(S / type_ref)
    type_review = read_json(S / type_spec['review_ref'])
    type_result = runpy.run_path(str(S / 'tools/检查类型与格式.py'))['verify']()
    assert type_spec['status'] == type_review['status'] == 'passed_document_specification'
    assert type_review['checkpoint'] == 'S06-02' and type_review['actual_product_runtime'] is None
    assert type_result['category_fields_bound'] == 61 and type_result['common_fields_bound'] == 10
    assert tasks_doc['first_release_type_format_spec_ref'] == type_ref
    for ref in ['docs/04_开发与验收/完整设计与执行准入.json',
                'docs/04_开发与验收/软件开发流程与当前成熟度评估.json',
                'docs/04_开发与验收/任务框架与工量里程碑.json',
                'docs/04_开发与验收/编码任务包/任务包索引.json']:
        assert read_json(S / ref)['type_format_spec_ref'] == type_ref
    assert contracts['type_field_bindings_ref'] == type_spec['module_field_bindings_ref']
    api_ref = selection['api_contract_spec_ref']
    api_spec = read_json(S / api_ref)
    api_review = read_json(S / api_spec['review_ref'])
    api_result = runpy.run_path(str(S / 'tools/检查公开操作契约.py'))['verify']()
    assert api_spec['status'] == api_review['status'] == 'passed_document_specification'
    assert api_result['operations'] == api_result['service_signatures'] == 18
    assert api_result['product_operations_implemented'] == 0 and api_result['actual_product_API_execution'] is None
    packaging_review_ref = 'docs/04_开发与验收/S06-04_纸包装算法与数值方法复审.json'
    packaging_review = read_json(S / packaging_review_ref)
    assert packaging_review['status'] == 'passed_document_specification'
    assert packaging_review['review_passed'] is True
    assert packaging_review['open_findings'] == [] and packaging_review['remaining_public_blocker'] is None
    assert packaging_review['actual_product_runtime'] is None
    assert packaging_review['actual_complete_professional_trial'] is None
    packaging_proof = read_json(S / packaging_review['actual_isolated_methods']['result_ref'])
    hold_proof = read_json(S / packaging_review['actual_isolated_methods']['fixed_pose_recheck']['result_ref'])
    numeric_proof = read_json(S / packaging_review['actual_isolated_methods']['native_result_ref'])
    assert packaging_proof['case_count'] == len(packaging_proof['cases']) == 32
    assert packaging_proof['all_expected_matched'] is True
    assert all(row['matches_independent_expectation'] for row in packaging_proof['cases'])
    assert hold_proof['case_count'] == len(hold_proof['cases']) == 6
    assert hold_proof['all_expected_matched'] is True
    assert all(row['matches_independent_expectation'] for row in hold_proof['cases'])
    assert hold_proof['carrier_positive_checks'] == 21 and hold_proof['schema_negative_checks'] == 9
    assert hold_proof['existing_32_expectations_modified'] is False
    assert hold_proof['actual_Host_invocation'] is False and hold_proof['actual_product_runtime'] is None
    assert numeric_proof['case_count'] == len(numeric_proof['cases']) == 16
    assert all(row['actual_Rust_f64_encloses_exact_reference'] for row in numeric_proof['cases'])
    assert numeric_proof['full_geometry_kernel_proved'] is False
    assert tasks_doc['first_release_api_contract_spec_ref'] == api_ref
    for ref in ['docs/04_开发与验收/完整设计与执行准入.json',
                'docs/04_开发与验收/软件开发流程与当前成熟度评估.json',
                'docs/04_开发与验收/任务框架与工量里程碑.json',
                'docs/04_开发与验收/编码任务包/任务包索引.json']:
        assert read_json(S / ref)['api_contract_spec_ref'] == api_ref
    assert scheduler['actual_full_engine_implemented'] is False
    assert tasks_doc['first_release_scheduler_spec_ref'] == scheduler_ref
    for task in selection['selected']:
        if task['id'] in ['P1-01', 'P1-02', 'P1-03', 'P1-04', 'P1-05', 'P2-02', 'P2-04', 'P2-06']:
            assert task['algorithm_spec_ref'] == scheduler_ref and scheduler_ref in task['execution_plan']['input_refs']
    assert tasks_doc['first_release_nonfunctional_spec_ref'] == nfr_ref
    for ref in ['docs/04_开发与验收/完整设计与执行准入.json', 'docs/04_开发与验收/任务框架与工量里程碑.json']:
        assert read_json(S / ref)['nonfunctional_spec_ref'] == nfr_ref
    assert read_json(S / 'docs/00_项目基线/项目主线.json')['development_responsibility']['nonfunctional_spec_ref'] == nfr_ref
    assert maturity['phases'][2]['checkpoints'][3]['review']['status'] == 'passed'
    generation = read_json(S / 'packages/domain/factory-generation.design.json')
    policy = generation['delivery_policy']['initial_target_met']
    assert 'accepted_unit' in generation['goal_fields'] and 'acceptance_scope' in generation['goal_fields']
    assert all(policy[k] is False for k in ['requires_repair', 'requires_revised_run', 'requires_comparison'])
    scenarios = read_json(S / 'scenarios/目标驱动建厂/acceptance.design.json')
    assert 'compare_runs_or_explain_unrepairable' not in scenarios['required_steps']
    cases = {c['id']: c for c in scenarios['cases']}
    assert 'no_required_change_revised_run_or_comparison' in cases['GF-07']['expectation']
    assert 'new_run_comparison_or_inability_reason' in cases['GF-05']['expectation']
    assert {(e['product_family'], e['accepted_unit']) for e in scenarios['generic_unit_examples']} == {
        ('die_cut_fold_glued_paper_carton', 'piece'), ('purchased_parts_assembly', 'piece')}
    assert next(e for e in scenarios['generic_unit_examples']
                if e['product_family'] == 'die_cut_fold_glued_paper_carton')['accepted_item_type'] == 'carton'
    reporting = read_json(S / 'packages/reporting/evaluation-report.contract.design.json')
    assert 'revised references optional' in reporting['optional_generation_context']['field_usage']['initial_and_revised_scenario_run_refs']
    assert scenarios['runtime_available'] is False and scenarios['actual_result'] is None
    # S02-01冻结清单：首版范围/验收投影必须和真实任务消费者一致。
    freeze = selection['scope_freeze']
    assert freeze['status'] == 'scope_frozen_not_implemented'
    assert freeze['included_requirement_ids'] == [f'SF-{i:02}' for i in range(1, 18)]
    assert freeze['nine_category_ids'] == ids
    assert set(selection['deferred_scopes']) == set(selection['deferred'])
    task_by_id = {task['id']: task for task in tasks}
    assert {row['id'] for row in selection['selected']} | set(selection['deferred']) == set(task_by_id)
    assert not ({row['id'] for row in selection['selected']} & set(selection['deferred']))
    scope_text = (S / selection['scope_baseline']).read_text(encoding='utf-8-sig')
    for row in selection['selected']:
        task = task_by_id[row['id']]
        assert row['scope'] and '按原任务' not in row['scope']
        assert row['release_acceptance'] and row['requirement_ids']
        assert set(row['requirement_ids']) <= set(freeze['included_requirement_ids'])
        assert task['first_release_disposition'] == row['disposition']
        for suffix, key in [('scope', 'scope'), ('requirement_ids', 'requirement_ids'),
                            ('acceptance', 'release_acceptance'), ('paths', 'release_paths')]:
            assert task['first_release_' + suffix] == row[key], ('release_projection_mismatch', row['id'], suffix)
        for suffix, key in [('dependencies', 'release_dependencies'),
                            ('implementation_dependencies', 'implementation_dependencies'),
                            ('dependency_reasons', 'dependency_reasons'), ('execution_plan', 'execution_plan')]:
            assert task['first_release_' + suffix] == row[key], ('work_package_projection_mismatch', row['id'], suffix)
        plan = row['execution_plan']
        assert plan['inputs'] and plan['outputs'] and plan['steps'] and plan['verification_method']
        assert plan['actual_result'] is None
        assert plan['input_refs'] and all((S / ref).exists() for ref in plan['input_refs'])
        assert set(row['dependency_reasons']) == set(row['release_dependencies'])
        assert all(row['dependency_reasons'].values())
        assert all((S / path).exists() for path in row['release_paths'])
        table_line = '|' + row['id'] + '|' + row['scope'] + '|' + '、'.join(row['requirement_ids']) + '|'
        assert table_line in scope_text, ('scope_document_mismatch', row['id'])
    assert {r for row in selection['selected'] for r in row['requirement_ids']} == set(freeze['included_requirement_ids'])
    assert [row['id'] for row in selection['selected'] if row['disposition'] == 'partial'] == selection['partial_scope'] == ['P0-03']
    assert all(task_by_id[ident]['first_release_disposition'] == 'deferred' and
               task_by_id[ident]['first_release_scope'] == selection['deferred_scopes'][ident]
               for ident in selection['deferred'])
    partial = next(row for row in selection['selected'] if row['id'] == 'P0-03')
    assert partial['requirement_ids'] == ['SF-12', 'SF-13']
    assert '不要求实现建设日历' in partial['release_acceptance']
    assert freeze['external_registration_and_submission_separate'] is True
    for key in ['confirmed_choices_ref', 'current_mainline_ref', 'joint_corrections_ref',
                'industry_generation_profile_ref', 'delivery_policy_source_ref']:
        assert (S / freeze[key]).exists()
    assert len(freeze['general_template_refs']) == 3 and all((S / path).exists() for path in freeze['general_template_refs'])
    chosen = {x['id'] for x in selection['selected']}
    assert all(set(row['implementation_tasks']) <= chosen for row in mappings)
    graph = {x['id']: x['release_dependencies'] for x in selection['selected']}
    stack, seen = set(), set()

    def visit(n):
        assert n not in stack, ('dependency_cycle', n)
        if n in seen:
            return
        stack.add(n)
        for v in graph[n]:
            assert v in chosen
            visit(v)
        stack.remove(n)
        seen.add(n)

    for n in graph:
        visit(n)
    assert 'P4-01' in graph['P3-04']
    # S02-02：启动/完整验收的产物可达，最终交付不能漏掉选入任务。
    def ancestors(n):
        parents = set(graph[n])
        for parent in graph[n]:
            parents.update(ancestors(parent))
        return parents

    assert ancestors('P5-01') == chosen - {'P5-01'}
    assert 'P0-02' in graph['P1-01'] and 'P2-02' in graph['P1-04']
    assert 'P1-03' in graph['P2-02'] and 'P2-06' in graph['P3-04']
    assert not ({'P3-02', 'P3-03'} & set(graph['P4-01']))
    for row in selection['selected']:
        assert set(row['implementation_dependencies']) <= ancestors(row['id'])
    remaining, complete, waves = dict(graph), set(), []
    while remaining:
        wave = [n for n in graph if n in remaining and set(remaining[n]) <= complete]
        assert wave
        waves.append(wave)
        complete.update(wave)
        for n in wave:
            remaining.pop(n)
    assert waves == selection['logical_completion_waves']
    assert {x['id'] for x in selection['shared_scenario_checks']} == {'IX-01', 'IX-02', 'IX-03'}
    for check in selection['shared_scenario_checks']:
        assert check['owner'] in chosen and check['release_gate'] == 'P5-01'
        assert set(check['producer_tasks']) <= ancestors(check['owner'])
        assert check['owner'] in ancestors('P5-01') and check['actual_result'] is None
        assert check['input_refs'] and all((S / ref).exists() for ref in check['input_refs'])
    # S02-03只检查计划归属/写入边界，不调度窗口或代替真实交接。
    window_plan = read_json(S / selection['parallel_window_plan_ref'])
    roles = {r['id']: r for r in window_plan['window_roles']}
    assert set(roles) == {'W00', 'W01', 'W02', 'W03', 'W04', 'W05'}
    assert window_plan['project_owner']['id'] == 'user'
    assert all(t['accountable_owner'] == 'user' for t in tasks)
    assert window_plan['write_policy']['shared_contract_owner'] == 'W00'
    assert window_plan['write_policy']['workspace_control_owner'] == 'W00'
    assert window_plan['write_policy']['one_writer_per_file'] is True
    assert window_plan['execution_policy']['integration_window'] == 'W00'
    assert window_plan['execution_policy']['planned_window_count'] == 6
    assert window_plan['execution_policy']['maximum_simultaneously_active_implementation_windows'] == 5
    support = next((entry for key in ['current_host_slice_dispatch', 'current_persistence_history_dispatch', 'current_business_inputs_dispatch', 'current_whole_package_dispatch']
                    if (entry := window_plan.get(key, {})).get('scope_ref') == backend_ref), {})
    support_by_role = ({r['window_id']: r for r in support['dispatches']}
                       if support.get('scope_ref') == backend_ref else {})
    for role in roles.values():
        if role['id'] in support_by_role:
            assert role['actual_thread_id'] == support_by_role[role['id']]['thread_id']
            assert role['dispatch_status'] == ('current_slice_reviewed' if backend_reviewed else 'active_scoped_backend')
        elif backend_dispatched and role['id'] == 'W00':
            assert role['actual_thread_id'] == backend['dispatch']['thread_id']
            assert role['dispatch_status'] == ('current_slice_reviewed' if backend_reviewed else 'active_scoped_backend')
        else:
            assert role['actual_thread_id'] is None and role['dispatch_status'] == 'planned'
    expected_windows = len(support_by_role) if support_by_role else int(backend_dispatched)
    assert window_plan['actual_execution']['windows_dispatched_by_this_plan'] == expected_windows
    if backend_dispatched:
        assert len(window_plan['actual_execution']['actual_windows']) == expected_windows
        assert window_plan['actual_execution']['actual_windows'][0]['thread_id'] == backend['dispatch']['thread_id']
    else:
        assert window_plan['actual_execution']['actual_windows'] == []
    assert window_plan['actual_execution']['parallel_handoff_rehearsal'] is None
    assigned = {r['task_id']: r for r in window_plan['task_assignments']}
    assert len(assigned) == len(window_plan['task_assignments']) == 21 and set(assigned) == chosen
    role_task_ids = [ident for r in roles.values() for ident in r['primary_tasks']]
    assert len(role_task_ids) == len(set(role_task_ids)) == 21 and set(role_task_ids) == chosen
    window_text = (D / '本机并行窗口计划.md').read_text(encoding='utf-8-sig')
    for row in selection['selected']:
        owner = row['planned_window_id']
        assignment = assigned[row['id']]
        assert owner in roles and row['id'] in roles[owner]['primary_tasks']
        assert assignment['primary_window'] == owner and assignment['accountable_owner'] == 'user'
        assert set(assignment['support_windows']) <= set(roles) - {owner}
        assert assignment['status'] == ('in_progress' if row['id'] in dispatched_packages else 'planned')
        assert assignment['actual_result'] is None
        assert row['implementation_owner'] == 'user_with_ai_windows' and row['acceptance_owner'] == 'user'
        assert row['accountable_owner'] == 'user'
        for key in ['planned_window_id', 'implementation_owner', 'acceptance_owner', 'parallel_plan_ref']:
            assert task_by_id[row['id']]['first_release_' + key] == row[key]
        assert row['parallel_plan_ref'] == selection['parallel_window_plan_ref']
        assert '|' + row['id'] + '|' + owner + '|' in window_text
    assert tasks_doc['parallel_window_plan_ref'] == selection['parallel_window_plan_ref']
    assert all(task_by_id[n]['first_release_planned_window_id'] is None for n in selection['deferred'])
    write_ranges = [(ident, p.rstrip('/').lower()) for ident, r in roles.items()
                    for p in r['write_roots'] + r['write_files']]
    for index, (owner, path) in enumerate(write_ranges):
        for other_owner, other_path in write_ranges[index + 1:]:
            if owner != other_owner:
                assert not (path == other_path or path.startswith(other_path + '/') or
                            other_path.startswith(path + '/')), ('overlapping_window_write_range', owner, path, other_owner, other_path)
    # 完整设计/执行关口与提示词只是准备，不代尚未通过的复审或授权。
    design_gate = read_json(D / '完整设计与执行准入.json')
    design_checkpoints = [c for phase in maturity['phases'][:6] for c in phase['checkpoints']]
    pending_design_ids = [c['id'] for c in design_checkpoints if c['review']['status'] != 'passed']
    assert design_gate['design_freeze']['required_review_ids'] == [c['id'] for c in design_checkpoints]
    assert len(design_checkpoints) == 30
    assert design_gate['design_freeze']['currently_passed_design_reviews'] == 30 - len(pending_design_ids)
    assert design_gate['design_freeze']['currently_pending_design_reviews'] == pending_design_ids
    assert design_gate['design_freeze']['last_checkpoint'] == 'S06-05'
    design_frozen = design_gate['design_freeze']['status'] == 'passed'
    assert design_gate['design_freeze']['status'] in ['not_ready', 'passed']
    if design_frozen:
        assert not pending_design_ids
        freeze_review = read_json(D / 'S06-05_全层贯通设计与冻结复审.json')
        assert freeze_review['status'] == 'passed_all_layer_design_freeze'
        assert not freeze_review['unresolved_design_findings'] and freeze_review['actual_freeze']
        assert len(freeze_review['flows']) == 5 and len(freeze_review['task_design_matrix']) == 21
    assert design_gate['execution_entry']['last_checkpoint_before_product_coding'] == 'S07-05'
    assert design_gate['execution_entry']['first_product_coding_phase'] == 'S08'
    preparation_accepted = all(c['review']['status'] == 'passed' for c in maturity['phases'][6]['checkpoints'])
    if preparation_accepted:
        engineering = read_json(D / 'S07_工程准备与执行准入.json')
        assert engineering['status'] == 'passed_engineering_preparation'
        assert read_json(S / engineering['verification_ref'])['status'] == 'passed'
        assert design_gate['execution_entry']['status'] == ('active_scoped_backend_execution' if backend_dispatched else 'ready_for_scoped_backend_dispatch')
        assert design_gate['execution_entry']['actual_S08_dispatch_started'] is backend_dispatched
    pdf_delivery = design_gate['final_topic_pdf']
    if pdf_delivery['actual_output'] is not None:
        assert design_frozen and pdf_delivery['status'] == 'delivered'
        assert (S / pdf_delivery['software_engineering_relative_path']).is_file()
        assert (S / pdf_delivery['editable_DOCX_relative_path']).is_file()
    else:
        assert pdf_delivery['generated_this_turn'] is False
    assert design_gate['frontend_supervision']['software'] == 'Figma'
    figma_authorization = design_gate['frontend_supervision']['design_start_authorization']
    assert figma_authorization['decision'] == 'start_supervised_figma_design_and_clickable_prototype'
    assert figma_authorization['checkpoints'] == ['S04-02', 'S04-04']
    assert figma_authorization['user_supervision_required'] is True
    assert figma_authorization['product_frontend_coding_authorized'] is False
    for key in ['frontend_build_authorization', 'approved_figma_artifact']:
        assert design_gate['frontend_supervision'][key] is None
    assert maturity['frontend_supervision'] == design_gate['frontend_supervision']
    assert window_plan['frontend_supervision'] == design_gate['frontend_supervision']
    figma_execution = read_json(S / 'docs/02_交互设计/Figma执行状态.json')
    assert design_gate['frontend_supervision']['current_figma_batch']['status'] == figma_execution['execution_state']
    delivery_file, delivery_section = design_gate['current_report_delivery_ref'].split('#/')
    current_report = read_json(S / delivery_file)[delivery_section]
    assert current_report['artifacts']['PDF'] == pdf_delivery['software_engineering_relative_path']
    assert current_report['verified_actual_PDF_pages'] == pdf_delivery['actual_output']['actual_pages']
    assert maturity['phases'][3]['checkpoints'][4]['checkpoint'] == '负责人实际原型走查'
    if maturity['phases'][3]['checkpoints'][4]['review']['status'] == 'passed':
        walk = read_json(S / 'docs/02_交互设计/S04_可操作设计原型/output/delegated-walkthrough/walkthrough-result.json')
        assert walk['completed'] and walk['operator_kind'] == 'AI_delegated'
        assert walk['authorization']['user_message_id'] == '01a13aec-61f5-7c82-9ef9-46d9a202d275'
        assert len(walk['flows']) == 5 and len(walk['failure_checks']) == 8
        assert all(step['result'] == 'passed' for flow in walk['flows'] for step in flow['steps'])
        assert all(row['result'] == 'passed' for row in walk['failure_checks'])
        assert not walk['page_errors'] and walk['actual_user_walkthrough'] is None
        assert not walk['product_API_called']
    else:
        assert maturity['phases'][3]['checkpoints'][4]['review']['status'] == 'pending'
    prompt_index = read_json(D / '窗口启动提示词/启动提示词索引.json')
    assert {row['window_id'] for row in prompt_index['prompts']} == set(roles)
    assert len(prompt_index['prompts']) == 6
    for row in prompt_index['prompts']:
        assert row['primary_tasks'] == roles[row['window_id']]['primary_tasks']
        assert row['launch_authorized'] is (backend_dispatched and (row['window_id'] == 'W00' or row['window_id'] in support_by_role))
        if row['launch_authorized']:
            if row['window_id'] == 'W00':
                if support.get('dispatch_mode') == 'whole_task_packages':
                    assignment = support_by_role['W00']
                    current = dispatched_receipts[assignment['task_id']]
                    assert row['actual_dispatch'] == current['dispatch']
                    assert row['actual_dispatch']['scope_ref'] == assignment['scope_ref']
                    assert row['actual_dispatch']['thread_id'] == assignment['thread_id']
                else:
                    assert row['actual_dispatch'] == backend['dispatch']
            else:
                if support.get('dispatch_mode') == 'whole_task_packages':
                    assignment = support_by_role[row['window_id']]
                    assert row['actual_dispatch']['scope_ref'] == assignment['scope_ref']
                    assert row['actual_dispatch']['task_id'] == assignment['task_id']
                    assert assignment['task_id'] in dispatched_receipts
                    assert assignment['scope_ref'] == dispatched_packages[assignment['task_id']]['current_execution_ref']
                else:
                    assert row['actual_dispatch']['scope_ref'] == backend_ref
                    assert row['actual_dispatch']['task_id'] == backend['task_id']
                assert row['actual_dispatch']['thread_id'] == support_by_role[row['window_id']]['thread_id']
        text = (S / row['prompt_ref']).read_text(encoding='utf-8-sig')
        assert all(token in text for token in ['S06-05', 'S07-05', 'Figma', 'explicit approval', 'module views', 'read-only review'])
        assert '阶段实现计划.json' in text and 'P work families do not introduce whole-family start barriers' in text
    package_by_id = {r['task_id']: r for r in package_index['packages']}
    assert len(package_index['packages']) == len(package_by_id) == 21 and set(package_by_id) == chosen
    for row in selection['selected']:
        package = package_by_id[row['id']]
        assert package['primary_window'] == row['planned_window_id']
        if row['id'] in dispatched_packages:
            receipt = dispatched_receipts[row['id']]
            assert package['status'] == 'in_progress'
            assert package['prepared_slice']['within_original_step'] == receipt['within_original_step']
            frontend = package['prepared_slice']['business_frontend_changes']
            assert frontend is receipt['scope']['business_frontend_changes']
            if frontend:
                assert design_frozen and preparation_accepted
                assert receipt['dispatch']['business_frontend_authorized'] is True
                assert receipt['dispatch']['dispatch_mode'] == 'whole_task_package'
        elif preparation_accepted and row['id'] == 'P0-01':
            assert package['status'] == 'ready_for_dispatch'
        else:
            assert package['status'] == 'draft_not_dispatchable'
        assert package['design_accepted'] is design_frozen
        if design_frozen:
            assert package['design_acceptance_ref'] == 'docs/04_开发与验收/S06-05_全层贯通设计与冻结复审.json'
            assert len(package['design_view_refs']) == 6
        if row['id'] in dispatched_packages:
            receipt = dispatched_receipts[row['id']]
            assert package['actual_dispatch'] == receipt['dispatch']
            assert row['current_slice_result_ref'] == task_by_id[row['id']]['current_slice_result_ref'] == package['current_execution_ref']
            if receipt['status'] == 'implemented_and_reviewed':
                assert package['actual_commands'] == receipt['actual_result']['verification_commands']
        else:
            assert package['actual_dispatch'] is None
            assert package['actual_commands'] == (engineering['verified_commands'] if preparation_accepted else None)
        assert package['package_ref'] == row['coding_package_ref'] == task_by_id[row['id']]['first_release_coding_package_ref']
        assert row['coding_protocol_ref'] == task_by_id[row['id']]['first_release_coding_protocol_ref'] == design_gate['coding_protocol_ref']
        text = (S / package['package_ref']).read_text(encoding='utf-8-sig')
        assert row['scope'] in text and row['release_acceptance'] in text and package['status'] in text
    # 八项现行基线与无日程的任务工量/成果框架。
    effort_frame = read_json(S / selection['effort_framework_ref'])
    effort_rows = {r['task_id']: r for r in effort_frame['tasks']}
    assert len(effort_rows) == len(effort_frame['tasks']) == 21 and set(effort_rows) == chosen
    assert effort_frame['planning_policy']['daily_schedule_requested'] is False
    assert effort_frame['planning_policy']['daily_capacity_required'] is False
    assert effort_frame['planning_policy']['human_person_count'] == 1
    assert effort_frame['deadline']['material_preparation_before'] == '2026-10-30T00:00:00+08:00'
    sum_ranges = lambda records, key: [sum(r[key][i] for r in records) for i in [0, 1]]
    human_total = sum_ranges(effort_frame['tasks'], 'human_effort_hours')
    shared_total = sum_ranges(effort_frame['shared_work'], 'human_effort_hours')
    window_total = sum_ranges(effort_frame['tasks'], 'window_active_hours')
    assert effort_frame['totals']['selected_task_human_hours'] == human_total
    assert effort_frame['totals']['shared_human_hours'] == shared_total
    assert effort_frame['totals']['remaining_human_hours'] == [human_total[i] + shared_total[i] for i in [0, 1]]
    assert effort_frame['totals']['selected_task_window_active_hours'] == window_total
    assert effort_frame['totals']['actual_human_hours'] is None
    assert effort_frame['totals']['calendar_duration_or_deadline_guarantee'] is None
    milestones = {r['id']: r for r in effort_frame['milestones']}
    assert milestones['MS-11']['required_functional_ids'] == [
        sf for sf in selection['scope_freeze']['included_requirement_ids'] if sf != 'SF-15']
    assert milestones['MS-12']['required_functional_ids'] == ['SF-15']
    assert len(milestones) == len(effort_frame['milestones']) == 13
    milestone_task_ids = [ident for r in milestones.values() for ident in r['complete_task_ids']]
    assert len(milestone_task_ids) == len(set(milestone_task_ids)) == 21 and set(milestone_task_ids) == chosen
    milestone_done, milestone_left, milestone_ancestors = set(), dict(milestones), {}
    while milestone_left:
        ready = [ident for ident, r in milestone_left.items() if set(r['depends_on']) <= milestone_done]
        assert ready, 'milestone_cycle_or_unknown_dependency'
        for ident in ready:
            parents = set(milestones[ident]['depends_on'])
            milestone_ancestors[ident] = parents | set().union(*(milestone_ancestors[p] for p in parents))
            milestone_done.add(ident)
            milestone_left.pop(ident)
    for row in selection['selected']:
        effort = effort_rows[row['id']]
        assert effort['window'] == row['planned_window_id']
        assert effort['implementation_dependencies'] == row['implementation_dependencies']
        assert effort['release_dependencies'] == row['release_dependencies']
        assert row['effort_estimate_ref'] == task_by_id[row['id']]['first_release_effort_estimate_ref']
        assert row['deliverable_milestone_id'] == task_by_id[row['id']]['first_release_deliverable_milestone_id'] == effort['complete_at_milestone']
        assert row['id'] in milestones[effort['complete_at_milestone']]['complete_task_ids']
        for parent in row['release_dependencies']:
            parent_ms = effort_rows[parent]['complete_at_milestone']
            assert parent_ms == effort['complete_at_milestone'] or parent_ms in milestone_ancestors[effort['complete_at_milestone']]
        assert effort['actual_human_hours'] is None and effort['actual_window_hours'] is None
        assert all(0 <= values[0] <= values[1] for values in [effort['human_effort_hours'], effort['window_active_hours']])
    for summary in effort_frame['window_budget_summary']:
        selected_efforts = [r for r in effort_frame['tasks'] if r['window'] == summary['window']]
        assert summary['primary_task_ids'] == roles[summary['window']]['primary_tasks']
        assert summary['task_human_hours'] == sum_ranges(selected_efforts, 'human_effort_hours')
        assert summary['task_window_hours'] == sum_ranges(selected_efforts, 'window_active_hours')
    assert all(r['planned_start'] is None and r['planned_finish'] is None for r in milestones.values())
    joint_current = read_json(D / 'S01-01至S02-03_联合复核.json')
    expected_joint_ids = [f'S01-{i:02}' for i in range(1, 6)] + [f'S02-{i:02}' for i in range(1, 4)]
    assert joint_current['scope'] == expected_joint_ids == [r['id'] for r in joint_current['checkpoint_results']]
    assert all(r['status'] == 'passed_document_alignment' and (S / r['spec_ref']).exists() for r in joint_current['checkpoint_results'])
    assert joint_current['effort_framework_ref'] == selection['effort_framework_ref']
    assert joint_current['actual_runtime_result'] is None and joint_current['external_actions'] == []
    assert maturity['phases'][1]['checkpoints'][3]['checkpoint'] == '工量与成果里程碑框架'
    assert maturity['phases'][1]['checkpoints'][3]['review']['status'] == 'passed'
    maturity_md = (D / '软件开发流程与当前成熟度评估.md').read_text(encoding='utf-8-sig')
    effort_item = next(line for line in maturity_md.splitlines() if line.startswith('- **S02-04 '))
    assert '独立文档复审已通过' in effort_item and 'pending' not in effort_item
    effort_review = read_json(S / effort_frame['independent_review_ref'])
    assert effort_review['checkpoint'] == 'S02-04' and effort_review['status'] == 'passed_document_specification'
    assert effort_review['actual_runtime'] is None and effort_review['actual_figma_start'] is None
    assert effort_review['actual_window_dispatches'] == 0 and effort_review['external_actions'] == []
    assert len(effort_review['review_criteria']) == 7 and len(effort_review['finite_walkthroughs']) == 4
    assert all(r['status'] == 'passed_document_or_finite_reasoning' for r in effort_review['review_criteria'])
    assert all(r['status'] == 'passed_finite_reasoning' for r in effort_review['finite_walkthroughs'])
    assert effort_frame['calibration']['daily_time_log_required'] is False
    assert effort_frame['calibration']['actual_calibrations'] == []
    assert effort_frame['effort_accounting']['observed_estimation_samples'] == []
    assert effort_frame['effort_accounting']['measured_speedup'] is None
    for row in selection['selected']:
        assert effort_rows[row['id']]['estimate_input_refs'] == row['execution_plan']['input_refs']
    for milestone in milestones.values():
        assert milestone['delivery_owner'] in roles and milestone['acceptance_owner'] == 'user'
        assert set(milestone['contributors']) <= set(roles) - {milestone['delivery_owner']}
        assert milestone['required_deliverables'] and milestone['acceptance_source_refs']
        assert all((S / ref).exists() for ref in milestone['acceptance_source_refs'])
        if milestone['id'] in ['MS-01', 'MS-02', 'MS-03'] and design_frozen:
            assert milestone['status'] == 'accepted_design_milestone'
            assert milestone['actual_result']['kind'] == 'accepted_design_not_product'
        elif milestone['id'] == 'MS-04' and pdf_delivery['status'] == 'delivered':
            assert milestone['status'] == 'accepted_design_milestone'
            assert milestone['actual_result']['kind'] == 'accepted_design_not_product'
        elif milestone['id'] != 'MS-00':
            assert milestone['status'] == 'planned' and milestone['actual_result'] is None
    assert next(r for r in effort_frame['shared_work'] if r['id'] == 'E-DESIGN')['scope_checkpoint_ids'] == pending_design_ids
    for row in effort_frame['shared_work']:
        assert 0 <= row['human_effort_hours'][0] <= row['human_effort_hours'][1]
        assert row['actual_human_hours'] is None
    assert {r['id'] for r in effort_frame['material_checklist']} == {f'MAT-{i:02}' for i in range(1, 6)}
    ready_materials = [r['id'] for r in effort_frame['material_checklist'] if r['required_for_material_readiness']]
    external_materials = [r['id'] for r in effort_frame['material_checklist'] if not r['required_for_material_readiness']]
    assert ready_materials == effort_frame['deadline_control']['required_material_ids'] == [f'MAT-{i:02}' for i in range(1, 5)]
    assert external_materials == effort_frame['deadline_control']['external_followup_ids'] == ['MAT-05']
    assert all(r['readiness_milestone'] == 'MS-12' for r in effort_frame['material_checklist'] if r['required_for_material_readiness'])
    assert effort_frame['material_checklist'][-1]['readiness_milestone'] is None
    assert selection['effort_framework_review_ref'] == tasks_doc['effort_framework_review_ref'] == effort_frame['independent_review_ref']
    # S02-05检查退出定义，不运行产品或把退出定义当已退出。
    stage_plan = read_json(S / selection['stage_plan_ref'])
    stage_review = read_json(S / selection['stage_exit_review_ref'])
    assert stage_plan['checkpoint'] == stage_review['checkpoint'] == 'S02-05'
    assert stage_review['status'] == 'passed_document_specification'
    assert maturity['phases'][1]['checkpoints'][4]['review']['status'] == 'passed'
    stage_item = next(line for line in maturity_md.splitlines() if line.startswith('- **S02-05 '))
    assert '独立文档复审已通过' in stage_item and '待复审' not in stage_item
    assert len(stage_plan['lifecycle_phases']) == 12
    for phase, lifecycle in zip(maturity['phases'], stage_plan['lifecycle_phases']):
        assert lifecycle['id'] == phase['id'] and lifecycle['name'] == phase['name']
        assert lifecycle['required_review_ids'] == [row['id'] for row in phase['checkpoints']]
        assert lifecycle['delivery_owner'] in roles and lifecycle['acceptance_owner'] == 'user'
        assert lifecycle['inputs'] and lifecycle['outputs'] and lifecycle['exit_conditions']
        if design_frozen and (phase['id'] in [f'S{i:02}' for i in range(1, 6)] or
                              (phase['id'] == 'S06' and pdf_delivery['status'] == 'delivered')):
            assert lifecycle['actual_exit_result']['kind'] == 'accepted_design_stage_exit_not_product_execution'
        elif phase['id'] == 'S07' and preparation_accepted:
            assert lifecycle['actual_exit_result']['status'] == 'passed_actual_engineering_preparation'
            assert lifecycle['actual_exit_result']['ref'] == 'docs/04_开发与验收/S07_工程准备与执行准入.json'
        else:
            assert lifecycle['actual_exit_result'] is None
        expected_review_state = 'definitions_reviewed' if all(row['review']['status'] == 'passed' for row in phase['checkpoints']) else 'pending_reviews'
        if phase['id'] == 'S07' and preparation_accepted:
            expected_review_state = 'passed_actual_preparation_review'
        assert lifecycle['document_review_state'] == expected_review_state
    families = {row['id']: row for row in stage_plan['product_families']}
    assert len(families) == len(stage_plan['product_families']) == 6
    family_members = [ident for row in families.values() for ident in row['task_ids']]
    assert len(family_members) == len(set(family_members)) == 21 and set(family_members) == chosen
    for family_id, family in families.items():
        members = [row for row in selection['selected'] if row['id'].split('-')[0] == family_id]
        assert family['task_ids'] == [row['id'] for row in members]
        assert family['task_owners'] == {row['id']: row['planned_window_id'] for row in members}
        assert family['requirement_ids'] == sorted({sf for row in members for sf in row['requirement_ids']})
        assert family['delivery_owner'] in roles and family['acceptance_owner'] == 'user'
        assert family['inputs'] and family['outputs'] and family['exit_conditions']
        assert family['status'] == ('in_progress' if backend_dispatched and family_id == 'P0' else 'planned')
        assert family['actual_result'] is None
        assert family['owned_ix_ids'] == [row['id'] for row in selection['shared_scenario_checks'] if row['owner'].split('-')[0] == family_id]
        assert len(family['task_contracts']) == len(members)
        for contract, row in zip(family['task_contracts'], members):
            assert contract['task_id'] == row['id'] and contract['scope'] == row['scope']
            assert contract['start_dependencies'] == row['implementation_dependencies']
            assert contract['complete_dependencies'] == row['release_dependencies']
            assert contract['outputs'] == row['execution_plan']['outputs']
            assert row['phase_exit_ref'] == task_by_id[row['id']]['first_release_phase_exit_ref'] == selection['stage_plan_ref'] + '#' + family_id
    assert [row['id'] for row in stage_plan['sf_evidence']] == selection['scope_freeze']['included_requirement_ids']
    for row in stage_plan['sf_evidence']:
        assert row['producer_tasks'] == [task['id'] for task in selection['selected'] if task['id'] != 'P5-01' and row['id'] in task['requirement_ids']]
        assert row['producer_tasks'] and row['closure_task'] == 'P5-01' and row['closure_window'] == 'W00'
        assert row['verification_milestone'] == ('MS-12' if row['id'] == 'SF-15' else 'MS-11')
        assert row['material_closure_milestone'] == 'MS-12'
        assert row['current_result'] == 'not_executed' and row['actual_result'] is None
    assert len(stage_plan['ix_evidence']) == 3
    for row, source in zip(stage_plan['ix_evidence'], selection['shared_scenario_checks']):
        assert row['id'] == source['id'] and row['owner_task'] == source['owner']
        assert row['producer_tasks'] == source['producer_tasks'] and row['input_refs'] == source['input_refs']
        assert row['required_observations'] == source['required_observations'] and row['release_gate'] == source['release_gate']
        assert row['owner_window'] == task_by_id[row['owner_task']]['first_release_planned_window_id']
        assert row['current_result'] == 'not_executed' and row['actual_result'] is None
    assert stage_plan['entry_policy']['no_new_family_start_dependencies'] is True
    assert stage_plan['entry_policy']['S08_exit_does_not_require_P5_material_completion'] is True
    assert stage_plan['lifecycle_phases'][-1]['gate_for_current_material_readiness'] is False
    assert all(token in stage_plan['evidence_policy']['business_outcome_vs_verdict'] for token in ['Fail', 'Undetermined', 'RevisionConflict'])
    assert stage_plan['failure_policy']['actions'] and stage_plan['evidence_policy']['minimum_exit_record']
    assert all(stage_plan[key] is None for key in ['actual_product_family_exits', 'actual_sf_execution', 'actual_ix_execution'])
    if backend_dispatched:
        assert stage_plan['actual_product_execution']['task_id'] == backend['task_id']
        assert stage_plan['actual_product_execution']['whole_task_done'] is False
    else:
        assert stage_plan['actual_product_execution'] is None
    if preparation_accepted:
        assert set(stage_plan['actual_lifecycle_exit_results']) == {'S07'}
    else:
        assert stage_plan['actual_lifecycle_exit_results'] is None
    assert stage_plan['external_actions'] == stage_review['external_actions'] == []
    assert stage_review['actual_runtime'] is None and stage_review['actual_stage_exits'] is None
    assert len(stage_review['criteria']) == 7 and len(stage_review['finite_walkthroughs']) == 6
    assert all(row['status'] == 'passed_document_or_finite_reasoning' for row in stage_review['criteria'])
    assert all(row['status'] == 'passed_finite_reasoning' for row in stage_review['finite_walkthroughs'])
    assert effort_frame['stage_exit_contract_ref'] == design_gate['stage_exit_contract_ref'] == window_plan['stage_exit_contract_ref'] == tasks_doc['first_release_stage_plan_ref'] == selection['stage_plan_ref']
    assert package_index['stage_plan_ref'] == selection['stage_plan_ref']
    assert all(task_by_id[ident]['first_release_phase_exit_ref'] is None for ident in selection['deferred'])
    # 两个关键早开案例仍消费原任务前置，不能被新工作族规则改成整族等待。
    assert next(row for row in selection['selected'] if row['id'] == 'P1-01')['implementation_dependencies'] == ['P0-02']
    assert next(row for row in selection['selected'] if row['id'] == 'P4-02')['implementation_dependencies'] == ['P0-03']
    assert 'P4-02' not in next(row for row in selection['selected'] if row['id'] == 'P2-06')['release_dependencies']
    for phase in maturity['phases']:
        assert phase['checkpoint_count'] == len(phase['checkpoints']) == 5
        assert phase['completion_percent'] == sum(c['completion_credit'] for c in phase['checkpoints']) / 5 * 100
        assert phase['strength_percent'] == sum(c['evidence_credit'] for c in phase['checkpoints']) / 5 * 100
    checkpoints = [c for phase in maturity['phases'] for c in phase['checkpoints']]
    passed_ids = {c['id'] for c in checkpoints if c['review']['status'] == 'passed'}
    assert maturity['checkpoint_review']['passed_reviews'] == len(passed_ids)
    assert maturity['checkpoint_review']['pending_reviews'] == len(checkpoints) - len(passed_ids)
    assert set(maturity['checkpoint_review']['completed_review']) == passed_ids
    assert maturity['observations']['planned_operations'] == 18 and maturity['observations']['runtime_available_operations'] == len(available_operations)
    assert maturity['observations']['requirement_ids'] == 17
    for entry in catalog['modules']:
        manifest_path = S / 'modules' / entry['manifest']
        manifest = read_json(manifest_path)
        assert manifest['category'] == entry['category']
        for key in ['project', 'generation_profile']:
            if key in manifest:
                assert (manifest_path.parent / manifest[key]).exists()
    seed = read_json(S / 'modules/templates/carton-factory/project.design.json')
    assert seed['ready_for_simulation'] is False and seed['demand']['quantity'] is None
    assert all(v is None for k, v in seed['halls'][0].items() if k != 'id')
    profile = read_json(S / 'modules/templates/carton-factory/generation-profile.design.json')
    assert profile['ready_for_runtime'] is False and profile['actual_result'] is None
    assert set(profile['nine_category_roles']) == set(ids)

    # Each actual file receives a disposition; original evidence and pixels are not rewritten as requirements.
    inventory_path = D / '逐文件主线同步检查.csv'
    result_path = D / '主线同步检查结果.json'
    report_path = D / '全项目主线同步检查.md'
    for path, initial in [(inventory_path, 'path,action,basis,check_status\n'), (result_path, '{}'), (report_path, '# 主线同步检查\n')]:
        if not path.exists():
            path.write_text(initial, encoding='utf-8')
    edited = {x.replace('\\', '/') for x in read_json(D / '本轮已修改文件.json')['files']}
    lab_builds = [S / 'tools/architecture-proof/target',
                  S / 'tools/type-contract-proof/node_modules',
                  S / 'tools/type-contract-proof/python-libs',
                  S / 'target', S / 'node_modules', S / '.work',
                  S / 'apps/planner/dist', S / 'apps/planner/src-tauri/gen']
    # A fixture Junction can point back to canonical modules. Check each real
    # project file once, with its real parent for relative documentation links.
    files = set()
    for candidate in P.rglob('*'):
        if not candidate.is_file():
            continue
        physical = candidate.resolve()
        path = physical if physical.is_relative_to(P) else candidate
        if not any(path.is_relative_to(directory) for directory in lab_builds):
            files.add(path)
    files = sorted(files)
    files += [W / r for r in ['README.md', '大作业/README.md', '比赛/01_开放原子/README.md']]
    inventory = []
    for path in files:
        external = not path.is_relative_to(P)
        key = '外围入口/' + path.relative_to(W).as_posix() if external else path.relative_to(P).as_posix()
        issues = []
        if external:
            action, basis = '已更新/重新生成', '工作区/课程/赛事入口直接指向现行项目主线'
        elif preserved(path):
            action, basis = '保留原状', '原始证据/历史研究/缓存；不承载现行产品规格'
        elif path.suffix in ['.png', '.jpg', '.jpeg'] or path.name.endswith('_prompt.txt') or '模块配图' in path.parts or '从零搭建案例图包' in path.parts:
            action, basis = '保留原图/生成记录', '素材及原提示词保留；适用说明明确非完整SF-17或运行证据'
        elif source_record(path):
            action, basis = '保留证据内容/同步索引', '来源事实、范围和历史名称不改写为新功能证明'
        elif key in edited:
            action, basis = '已更新/重新生成', '按现行主线和该文件消费者责任更新'
        else:
            action, basis = '已检查—无需语义修改', '类别/参数/旧情景/素材定义继续适用；不单独声明产品范围'
        if not preserved(path):
            try:
                if path.suffix == '.json':
                    read_json(path)
                elif path.suffix == '.py':
                    ast.parse(path.read_text(encoding='utf-8-sig'), filename=key)
                elif path.suffix == '.csv':
                    with path.open(encoding='utf-8-sig', newline='') as f:
                        rows = csv.reader(f)
                        header = next(rows)
                        assert all(len(row) == len(header) for row in rows)
            except (ValueError, AssertionError, SyntaxError, StopIteration) as exc:
                issues.append('结构错误: ' + str(exc))
            links = []
            if path.suffix == '.md':
                links = re.findall(r'\]\(([^)]+)\)', path.read_text(encoding='utf-8-sig'))
            elif path.suffix == '.html' and key in ['设计预览/图集.html', '研究资料/研究资料索引.html']:
                parser = Links()
                parser.feed(path.read_text(encoding='utf-8-sig'))
                links = parser.links
            link_parent = reference_parent(path) if links else path.parent
            for target in links:
                target = unquote(target.strip('<>').split('#')[0])
                if not target or re.match(r'(?:[a-zA-Z]+:|//)', target) and not re.match(r'^[A-Za-z]:[/\\]', target):
                    continue
                checked_links += 1
                if not (link_parent / target).exists():
                    issues.append('引用不存在: ' + target)
        if issues:
            errors.append({'path': key, 'issues': issues})
            row_issues[key] = issues
        inventory.append({'path': key, 'action': action, 'basis': basis, 'check_status': '待修正' if issues else '已核对', 'issues': ' | '.join(issues)})

    for task in tasks:
        for ref in task.get('design_references', []) + task.get('paths', []):
            checked_links += 1
            if not (S / ref).exists():
                errors.append({'task': task['id'], 'missing_reference': ref})
    topics = read_json(P / '模块研究/模块目录.json')
    assert len(topics) == 41 and sum(t['first_release_role'] == 'selected_real_professional_loop' for t in topics) == 1
    con = sqlite3.connect(P / '模块研究/专业模块研究.sqlite')
    for ident, metadata in con.execute('SELECT id,metadata FROM topics'):
        assert json.loads(metadata)['first_release_role'] == next(t['first_release_role'] for t in topics if t['id'] == ident)
    con.close()
    sources = read_json(P / '研究资料/来源目录.json')
    assert len(sources) == 87 and sum(x['download_status'] == 'downloaded' for x in sources) == 75
    con = sqlite3.connect(P / '研究资料/项目研究资料.sqlite')
    assert con.execute('SELECT count(*) FROM sources').fetchone()[0] == 87
    con.close()
    with (P / '研究资料/数据/采用与任务价值待填.csv').open(encoding='utf-8-sig') as f:
        assert len(list(csv.DictReader(f))) == 0
    with inventory_path.open('w', encoding='utf-8-sig', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=['path', 'action', 'basis', 'check_status', 'issues'])
        writer.writeheader()
        writer.writerows(inventory)
    output = {'check_kind': 'all_project_file_design_sync_not_runtime_acceptance', 'date': CHECK_DATE, 'files': len(files), 'actions': dict(Counter(x['action'] for x in inventory)), 'matrix': results, 'checked_local_references': checked_links, 'errors': errors, 'runtime_available_operations': len(available_operations), 'completed_product_tasks': 0, 'maturity_percentages_unchanged': True, 'source_records': 87, 'professional_research_directions': 41, 'selected_professional_kernels': 1, 'existing_design_images': 22, 'preserved_assets_are_not_new_UI_coverage': True, 'joint_correction_checks': {'F01_generic_quantity_contract': 'passed', 'F02_initial_success_and_conditional_repair': 'passed', 'F03_all_seven_UT_SF_sets': 'passed'}}
    output['task_decomposition_checks'] = {
        'checkpoint': 'S02-02', 'status': 'passed_static_specification',
        'selected_work_packages': len(selection['selected']),
        'start_and_acceptance_dependencies_and_projections': 'passed',
        'inputs_outputs_steps_verification_and_refs': 'passed',
        'release_dependency_cycles': False, 'deferred_dependencies': [],
        'final_gate_covered_predecessors': len(ancestors('P5-01')),
        'logical_completion_waves': waves,
        'shared_scenario_checks': [x['id'] for x in selection['shared_scenario_checks']],
        'actual_runtime_results': None,
    }
    output['maturity_percentages_unchanged'] = False
    output['maturity_credit_change_this_revision'] = {
        **maturity['latest_review_merge'], 'changed_checkpoints': maturity['latest_review_merge'].get('checkpoints', ['S06-04']),
        'changed_by_checker': False,
    }
    output['parallel_window_plan_checks'] = {
        'checkpoint': 'S02-03', 'status': 'passed_static_plan', 'accountable_owner': 'user',
        'planned_window_roles': len(roles), 'assigned_selected_tasks': len(assigned),
        'declared_write_ranges': len(write_ranges), 'overlapping_base_write_ranges': [],
        'shared_contract_and_build_configuration_owner': 'W00',
        'task_projection_and_document_mapping': 'passed',
        'deferred_tasks_assigned_windows': [], 'actual_dispatched_by_this_plan': int(backend_dispatched),
        'actual_parallel_handoff': None, 'speedup_measurement': None,
    }
    output['design_and_coding_entry_checks'] = {
        'status': 'frozen_design_and_actual_preparation_synced' if preparation_accepted else 'frozen_design_and_pending_preparation_synced',
        'design_review_gate': 'S06-05', 'required_design_reviews': len(design_checkpoints),
        'pending_design_reviews': len(pending_design_ids), 'execution_preparation_gate': 'S07-05',
        'startup_prompts': 6, 'coding_package_drafts': sum(p['status'] == 'draft_not_dispatchable' for p in package_index['packages']),
        'coding_packages_ready_for_dispatch': sum(p['status'] == 'ready_for_dispatch' for p in package_index['packages']),
        'S07_preparation_accepted': preparation_accepted,
        'S08_backend_slice_dispatched': backend_dispatched,
        'S08_backend_slice_reviewed': backend_reviewed,
        'frontend_software': 'Figma', 'frontend_design_authorized': True,
        'frontend_design_artifact_available': design_gate['frontend_supervision'].get('design_source_artifact') is not None,
        'frontend_product_coding_authorized': False,
        'module_views_technology_interfaces_before_coding': True,
        'final_topic_pdf_deferred': pdf_delivery['status'] != 'delivered',
    }
    output['joint_baseline_and_effort_framework_checks'] = {
        'status': 'passed_static_document_and_arithmetic', 'joint_checkpoints': expected_joint_ids,
        'selected_tasks': len(effort_rows), 'milestones': len(milestones),
        'milestone_cycles_or_task_dependency_conflicts': [], 'daily_calendar_assigned': False,
        'task_human_hours': human_total, 'shared_human_hours': shared_total,
        'remaining_human_hours': effort_frame['totals']['remaining_human_hours'],
        'task_window_active_hours': window_total, 'human_and_window_hours_separate': True,
        'formal_review_count_increased': False, 'deadline_feasibility': None,
        'actual_runtime_or_frontend_start': None,
    }
    output['S02_04_independent_review_checks'] = {
        'checkpoint': 'S02-04', 'status': 'passed_static_specification_and_finite_reasoning',
        'selected_tasks': len(effort_rows), 'shared_work_groups': len(effort_frame['shared_work']),
        'milestones_with_delivery_acceptance_and_evidence': len(milestones),
        'required_material_ids': ready_materials, 'external_followup_ids': external_materials,
        'finite_walkthrough_cases': len(effort_review['finite_walkthroughs']),
        'formal_review_count': len(passed_ids), 'pending_review_count': len(checkpoints) - len(passed_ids),
        'pending_design_reviews': len(pending_design_ids), 'next_checkpoint': maturity['checkpoint_review']['next_checkpoint'],
        'scope_and_dependency_change': False, 'estimates_measured': False,
        'actual_runtime_or_dispatch': None, 'review_ref': effort_frame['independent_review_ref'],
    }
    output['S02_05_independent_review_checks'] = {
        'checkpoint': 'S02-05', 'status': 'passed_static_specification_and_finite_reasoning',
        'lifecycle_phases': len(stage_plan['lifecycle_phases']), 'product_work_families': len(families),
        'selected_task_coverage': len(family_members), 'SF_evidence_mappings': len(stage_plan['sf_evidence']),
        'IX_evidence_mappings': len(stage_plan['ix_evidence']), 'finite_walkthrough_cases': len(stage_review['finite_walkthroughs']),
        'no_new_family_barriers_or_dependency_changes': True,
        'business_outcome_separate_from_test_verdict': True,
        'P5_completion_not_candidate_test_entry': True, 'S12_runtime_not_new_material_prerequisite': True,
        'formal_reviews': len(passed_ids), 'pending_reviews': len(checkpoints) - len(passed_ids),
        'pending_design_reviews': len(pending_design_ids), 'next_checkpoint': maturity['checkpoint_review']['next_checkpoint'],
        'actual_stage_exit_or_product_runtime': None, 'review_ref': selection['stage_exit_review_ref'],
    }
    output['S03_01_independent_review_checks'] = {
        'checkpoint': 'S03-01', 'status': 'passed_static_functional_contract',
        'user_tasks': len(mappings), 'requirements': len({sf for row in mappings for sf in row['requirements']}),
        'selected_task_coverage': len({task for row in mappings for task in row['implementation_tasks']}),
        'planned_operation_references': len(referenced_operations),
        'human_agent_inputs_success_incomplete_and_handoff': 'present_for_all_seven_tasks',
        'UT_SF_document_and_mapping_sets': 'passed',
        'finite_walkthroughs_recorded': len(functional_review['finite_walkthroughs']),
        'formal_reviews': len(passed_ids), 'pending_reviews': len(checkpoints) - len(passed_ids),
        'pending_design_reviews': len(pending_design_ids), 'next_checkpoint': maturity['checkpoint_review']['next_checkpoint'],
        'actual_runtime_or_target_user_validation': None, 'review_ref': functional_contract['review_ref'],
    }
    output['S03_02_independent_review_checks'] = {
        'checkpoint': 'S03-02', 'status': 'passed_static_business_contract_and_finite_calculation',
        'business_objects': len(business['objects']), 'exception_paths': len(business['exception_paths']),
        'reservation_quality_lifecycle_and_holder_separate': 'passed',
        'partial_acceptance_dispatch_and_cancel_semantics': 'passed_finite_cases',
        'finite_business_cases': len(rule_verification['cases']), 'existing_expectations_rewritten': False,
        'professional_first_release_and_deferred_modes': 'passed',
        'formal_reviews': len(passed_ids), 'pending_reviews': len(checkpoints) - len(passed_ids),
        'pending_design_reviews': len(pending_design_ids), 'next_checkpoint': maturity['checkpoint_review']['next_checkpoint'],
        'actual_product_runtime': None, 'review_ref': business['review_ref'],
    }
    output['S03_04_independent_review_checks'] = {
        'checkpoint': 'S03-04', 'status': nfr_result['status'],
        'NFR_rows': len(nfr['requirements']), 'synthetic_math_cases': len(nfr_result['synthetic_math_results']),
        'confirmed_choices_covered': nfr_result['confirmed_choices_covered'],
        'measurement_matrix': nfr_result['measurement_matrix'],
        'readiness_and_result_classification_checks': nfr_result['readiness_and_result_classification_checks'],
        'minimal_measurement_record_checks': nfr_result['minimal_measurement_record_checks'],
        'required_check_group_count': nfr_result['required_check_group_count'],
        'coordinate_mapping_checks': nfr_result['coordinate_mapping_checks'],
        'readiness_and_clock_boundaries': nfr_result['readiness_and_clock_boundaries'],
        'editor_attempt_plan_checks': nfr_result['editor_attempt_plan_checks'],
        'measurement_type_binding_checks': nfr_result['measurement_type_binding_checks'],
        'load_profile': nfr_result['load_profile'], 'numeric_targets': nfr['numeric_targets'],
        'machine_evidence': 'inventory_only_not_rendering_or_performance',
        'consumer_reference_sync': 'passed', 'no_new_wall_clock_engine': nfr_review['view_time_not_simulation_time'],
        'actual_performance_result': None, 'actual_product_runtime': None,
        'formal_reviews': len(passed_ids), 'pending_reviews': len(checkpoints) - len(passed_ids),
        'pending_design_reviews': len(pending_design_ids), 'next_checkpoint': maturity['checkpoint_review']['next_checkpoint'],
        'spec_ref': nfr_ref, 'review_ref': nfr['review_ref'],
    }
    output['S06_01_independent_review_checks'] = {
        'checkpoint': 'S06-01', 'status': scheduler_result['status'],
        'algorithm_rules': 14, 'finite_reasoning_cases': 21, 'logical_handoffs': 6,
        'applicable_views': 6, 'std_API_trial': scheduler_trial['status'],
        'actual_product_runtime': None, 'actual_product_cases_executed': 0,
        'spec_ref': scheduler_ref, 'review_ref': scheduler['review_ref'],
        'formal_reviews': len(passed_ids), 'pending_reviews': len(checkpoints) - len(passed_ids),
        'pending_design_reviews': len(pending_design_ids),
        'next_checkpoint': maturity['checkpoint_review']['next_checkpoint'],
    }
    output['S06_02_independent_review_checks'] = {
        **type_result, 'spec_ref': type_ref, 'review_ref': type_spec['review_ref'],
        'formal_reviews': len(passed_ids), 'pending_reviews': len(checkpoints) - len(passed_ids),
        'pending_design_reviews': len(pending_design_ids),
        'next_checkpoint': maturity['checkpoint_review']['next_checkpoint'],
    }
    output['S06_03_independent_review_checks'] = {
        **api_result, 'spec_ref': api_ref, 'review_ref': api_spec['review_ref'],
        'formal_reviews': len(passed_ids), 'pending_reviews': len(checkpoints) - len(passed_ids),
        'pending_design_reviews': len(pending_design_ids),
        'next_checkpoint': maturity['checkpoint_review']['next_checkpoint'],
    }
    output['S06_04_independent_review_checks'] = {
        'checkpoint': 'S06-04', 'status': packaging_review['status'],
        'evidence_check': 'recorded_owned_outputs_and_shared_carrier_consistency_not_product_execution',
        'review_ref': packaging_review_ref,
        'closed_findings': [row['id'] for row in packaging_review['findings']],
        'remaining_public_blocker': None, 'finite_method_cases': packaging_proof['case_count'],
        'fixed_pose_cases': hold_proof['case_count'],
        'carrier_positive_checks': hold_proof['carrier_positive_checks'],
        'carrier_negative_checks': hold_proof['schema_negative_checks'],
        'native_numeric_cases': numeric_proof['case_count'],
        'actual_product_runtime': None, 'complete_professional_trial': None,
        'formal_reviews': len(passed_ids), 'pending_reviews': len(checkpoints) - len(passed_ids),
        'pending_design_reviews': len(pending_design_ids),
        'next_checkpoint': maturity['checkpoint_review']['next_checkpoint'],
    }
    output['S03_05_independent_review_checks'] = {
        **trace_verification, 'checkpoint': 'S03-05', 'review_ref': trace['report_ref'],
        'formal_reviews': len(passed_ids), 'pending_reviews': len(checkpoints) - len(passed_ids),
        'pending_design_reviews': len(pending_design_ids), 'next_checkpoint': maturity['checkpoint_review']['next_checkpoint'],
        'S03_completion_percent': maturity['phases'][2]['completion_percent'],
        'S03_strength_percent': maturity['phases'][2]['strength_percent'],
    }
    output['S04_01_independent_review_checks'] = {
        **nav_result, 'checkpoint': 'S04-01', 'review_ref': nav['review_ref'], 'spec_ref': nav_ref,
        'formal_reviews': len(passed_ids), 'pending_reviews': len(checkpoints) - len(passed_ids),
        'pending_design_reviews': len(pending_design_ids), 'next_checkpoint': maturity['checkpoint_review']['next_checkpoint'],
    }
    output['S04_03_independent_review_checks'] = {
        **nav_result['form_spec_checks'], 'spec_ref': form_ref, 'review_ref': form_spec['review_ref'],
        'formal_reviews': len(passed_ids), 'pending_reviews': len(checkpoints) - len(passed_ids),
        'pending_design_reviews': len(pending_design_ids), 'earliest_pending_checkpoint': maturity['checkpoint_review']['next_checkpoint'],
    }
    output['S05_01_independent_review_checks'] = {
        **architecture_result, 'checkpoint': 'S05-01', 'spec_ref': architecture_ref,
        'review_ref': architecture['review_ref'], 'formal_reviews': len(passed_ids),
        'pending_design_reviews': len(pending_design_ids),
    }
    output['S05_02_independent_review_checks'] = {
        **architecture_result['ownership_checks'], 'spec_ref': selection['ownership_spec_ref'],
        'review_ref': ownership['review_ref'], 'formal_reviews': len(passed_ids),
        'pending_design_reviews': len(pending_design_ids),
    }
    output['S05_03_independent_review_checks'] = {
        **module_protocol_result, 'checkpoint': 'S05-03', 'spec_ref': selection['module_protocol_ref'],
        'review_ref': module_protocol['review_ref'], 'formal_reviews': len(passed_ids),
        'pending_design_reviews': len(pending_design_ids),
    }
    output['S05_04_independent_review_checks'] = {
        **technical_result, 'checkpoint': 'S05-04', 'spec_ref': selection['technical_selection_ref'],
        'review_ref': technical['review_ref'], 'formal_reviews': len(passed_ids),
        'pending_design_reviews': len(pending_design_ids),
    }
    result_path.write_text(json.dumps(output, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    lines = ['# 全项目主线同步检查', '', f'{CHECK_DATE}。范围：超级工厂整个目录，以及工作区/大作业/开放原子三个阅读入口。检查的是文件主线一致与结构，不是软件功能验收；当前文档复审{len(passed_ids)}项通过/其余{len(checkpoints)-len(passed_ids)}项待复审。成熟度信用值与比例另按真实检查项记录，本工具不增加通过项。', '', '现行主线：九类自由创作与社区扩展的Windows工业模拟平台；外部Agent一句话产品/日产/场地→原生或导入社区行业模板/九类库→可编辑布局/连接/排期→静态检查/真实模拟→初案达标直接交付，不足时有限有据修正/说明→四格式评估和可编辑工程。', '', '|编号|逐项要求|更新检查|消费者数|', '|---|---|---|---|']
    lines += [f"|{x['id']}|{x['requirement']}|{x['status']}|{len(x['consumers'])}|" for x in results]
    lines += ['', f'逐文件清单共{len(files)}项，已检查{checked_links}处本地引用。处理方式：' + '；'.join(f'{k} {v}项' for k, v in output['actions'].items()) + '。', '', '[逐文件CSV](逐文件主线同步检查.csv) · [机器结果及消费者](主线同步检查结果.json) · [现行主线](../00_项目基线/项目主线.json) · [目标生成验收](../../scenarios/目标驱动建厂/README.md)', '', '已直接消解旧正文把目标生成列为后置、API/需求/清单旧计数、入口只列部分九类、生成源覆盖新字段、专业候选版与平台首版混称及报告使用改名前路径等问题。', '', '原厂原文、历史判断、旧图像像素和生成记录保留，不能改成新要求的证据。原有22张AI图未完整展示新增目标生成面板，其阅读说明和计划已明确。S04-02的GPT v03已作为布局起点；用户指定draw.io接手后的[可编辑母版与字段细化](../02_交互设计/工业工作台母版与字段细化.md)是当前制作依据。26状态母版及24帧隔离原型为设计资产，视觉/操作接受取S04实际复审，不能当产品运行截图；[AI迭代记录](../02_交互设计/S04-02_AI母版迭代_20261014/README.md)保留原始版本。', '', '首版17项需求、18项计划操作、21项选择/7项后置；三通用离散模板＋纸箱行业配置，只选纸包装真实专业内核。实际Agent/软件调用、产能达标和用户效果仍待实现及验证。']
    if errors:
        lines += ['', '## 尚待修正', ''] + ['- ' + json.dumps(e, ensure_ascii=False) for e in errors]
    else:
        lines += ['', '上述12项设计同步检查通过，无未消解的本地引用/结构问题。历史复审中的15/16计数只对应当时记录，现行入口按17/18解释。']
    lines += ['', '联合修正静态检查：通用接受单位合同；初案达标无需修正RunId/比较与独立不足案例；七项UT文字表与JSON需求集合逐项一致。仅证明设计同步，不是Agent或仿真执行结果。']
    lines += ['', 'S02-02静态规格检查：21项启动/完整验收前置及产物理由、输入/输出/步骤/验证与真实task投影一致；依赖无环/无后置，P5覆盖其他20项；IX-01至03有可达生产者与验收所有者。逻辑波次不代表实际排期，运行结果仍空。', '', '[任务拆解与依赖](任务拆解与依赖.md) · [首版任务JSON](../00_项目基线/首版任务选择.json)']
    lines += ['', 'S02-03六窗口计划检查：用户负责全项目；21首版任务各有唯一计划主责，六窗口基础写入范围无重叠，共同类型/配置归W00；任务投影及正文一致，7后置无窗口。S02-03信用不变；S02-04/05均独立文档复审通过，信用各1/0.5保持，S02文档100%/证据70%；设计阶段出口按当前实际接受记录；产品运行未完成、产品编码未派发；本轮设计复审派发按窗口计划独立记录，交接/提速尚无实测。', '', '[本机窗口计划](本机并行窗口计划.md) · [机器计划](本机并行窗口计划.json)']
    pdf_state = '已实际交付并检查' if pdf_delivery['status'] == 'delivered' else ('冻结后实际生成检查中' if design_frozen else '待设计冻结后生成')
    figma_batch = design_gate['frontend_supervision'].get('current_figma_batch')
    figma_state = 'Figma当前66状态批次已授权并完成，实际覆盖取S04记录' if figma_batch and figma_batch.get('status') == 'completed_current_Figma_clickable_design_prototype' else 'Figma当前批次取执行状态记录'
    if backend_dispatched:
        phase = '实现及复审通过' if backend_reviewed else '实施中'
        execution_scope = ('S08按原完整任务包持续推进，共同底座与各整包实施中；'
                           if backend['dispatch'].get('dispatch_mode') == 'whole_task_package'
                           else f'S08当前{backend["task_id"]}原步骤{backend["within_original_step"]}切片{phase}；')
        coding_state = (f'S07准备通过，{execution_scope}'
                        f'{len(dispatched_packages)}项任务已按明确范围派发、其余{21-len(dispatched_packages)}包draft_not_dispatchable；'
                        f'已接收Host限定范围可用操作{len(available_operations)}/18，完整业务能力未验收')
    else:
        coding_state = 'S07实际准备已通过；P0-01后端切片ready_for_dispatch，其余20包draft_not_dispatchable' if preparation_accepted else 'S07实际准备尚未通过；21包draft_not_dispatchable'
    lines += ['', f'完整设计关口为S01—S06全部30项/S06-05，尚需{len(pending_design_ids)}项。6份启动提示词/21任务包范围与主责一致，设计认可取实际冻结记录；{coding_state}。{figma_state}；当前报告PDF{pdf_state}，文件/QA取current_report_delivery_ref。产品前端构建、S10独立用户及外部提交各留原关口。', '', '[设计与执行准入](完整设计与执行准入.md) · [编码规则](编码任务包与AI实现审查.md) · [启动提示词](窗口启动提示词/README.md) · [任务包](编码任务包/README.md)']
    lines += ['', f'八项基线与现行任务框架检查：人/AI窗口/参赛成员分开，S01-01至S02-03消费者一致；21任务预算及共同工作加总、按窗口归集和13成果前置/完整任务覆盖正确，没有新任务依赖循环或每日日程。当前{human_total[0] + shared_total[0]}—{human_total[1] + shared_total[1]}人小时/{window_total[0]}—{window_total[1]}产品窗活动分别估计，未实测/不保证按期；正式过项{len(passed_ids)}，材料准备仍10月30日前。', '', '[八项联合复核](S01-01至S02-03_联合复核.md) · [任务工量/里程碑](任务框架与工量里程碑.md)']
    lines += ['', 'S02-04规范静态检查：21包/五共同工作估量口径与输入依据、13节点交付/接收/证据责任、4材料准备项与1外部手续、实际校准及剩余处理、四例有限工作流程推演仍保持；只证明规范可执行，未测工时或运行产品。', '', '[S02-04独立复审](S02-04_工量与成果里程碑复审.md)']
    lines += ['', 'S02-05独立复审静态检查：12生命周期与6工作族输入/输出/退出、21任务/17SF/3IX映射及原两种前置、预期业务结果/测试结论、失败与变更复核、候选测试/材料/维护边界保持。设计出口读取当前实际接受记录，产品阶段未退出。', '', '[阶段规范](阶段实现计划.md) · [S02-05独立复审](S02-05_阶段闭环与退出条件复审.md)']
    lines += ['', f'S03-01静态功能交接检查：七UT人工/Agent输入与动作、成功/未完成及下游交接字段齐全；17SF文字/映射一致，21所选任务及18计划操作引用覆盖。三例功能推演由独立复审记录；本工具验证引用/结构与原生成分支，未运行产品或验证目标使用者。当前下一项{maturity["checkpoint_review"]["next_checkpoint"]}。', '', '[S03-01独立复审](S03-01_用户任务与功能需求复审.md) · [七UT映射](../00_项目基线/用户任务与需求映射.json)']
    lines += ['', f'S03-02静态业务复核：9类现有业务对象、独立状态轴/预留与9条异常出口、派工/部分接受/取消及专业首版/后置范围一致；6边界与6既有案例有限复核通过，冻结预期未改写。当前下一项{maturity["checkpoint_review"]["next_checkpoint"]}，不代表产品执行。', '', '[S03-02独立复审](S03-02_业务对象规则与异常路径复审.md) · [业务JSON](../../packages/domain/business-objects.design.json)']
    lines += ['', 'S03-04非功能规范静态检查：11条规范/10个已确认选择及责任/消费者一致，11例有限数学与判定、38必需组、120位置映射通过。48帧率窗口/1800编辑/40存取仅固定基准，真实重叠及其他子检查另做。04/05本机前置、同机工具/Renderer及新进程打开时钟明确；两机兼容、失败优先及四状态汇总保持。30FPS/200ms/3秒目标未改，30秒体验单列，无产品实测。', '', '[S03-04独立复审](S03-04_可测量非功能需求复审.md) · [非功能规范](../01_业务需求/非功能需求与测量验收.md) · [独立检查结果](非功能规格检查结果.json)']
    lines += ['', f'S03-05完整追踪：17SF/11NFR/17AC/21任务/18操作/3IX及{trace_verification["checked_input_expected_and_result_references"]}处具体输入/预期/结果节点、原任务投影和生成消费者通过；六条业务边界进入验收，SF-15在MS-12收口。S03文档100%/支撑70%，下一项{maturity["checkpoint_review"]["next_checkpoint"]}。四PREP的实际候选/输入适配按后续产品用途执行，产品验收0；当前设计冻结状态见完整准入记录。', '', '[需求追踪](首版需求验收追踪.md) · [S03-05独立复审](S03-05_首版需求与验收追踪复审.md)']
    report_path.write_text('\n'.join(lines) + '\n', encoding='utf-8')
    print(json.dumps({'files': len(files), 'matrix_passed': sum(x['status'] == '设计同步通过' for x in results), 'references': checked_links, 'errors': errors, 'actions': output['actions']}, ensure_ascii=False))
    if errors:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
