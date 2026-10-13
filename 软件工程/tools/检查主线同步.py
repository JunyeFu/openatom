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

S = Path(__file__).resolve().parent.parent
P = S.parent
D = S / 'docs/04_开发与验收'
W = P.parents[2]


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


class Links(HTMLParser):
    def __init__(self):
        super().__init__()
        self.links = []

    def handle_starttag(self, tag, attrs):
        if tag in ['a', 'img']:
            self.links += [v for k, v in attrs if k in ['href', 'src'] and v]


def main():
    errors = []
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
    selection = read_json(S / 'docs/00_项目基线/首版任务选择.json')
    mapping_doc = read_json(S / 'docs/00_项目基线/用户任务与需求映射.json')
    mappings = mapping_doc['rows']
    maturity = read_json(S / 'docs/04_开发与验收/软件开发流程与当前成熟度评估.json')
    ids = ['items', 'devices', 'resources', 'tasks', 'operations', 'transport', 'templates', 'professional', 'media']
    assert [x['id'] for x in catalog['categories']] == ids == [x['id'] for x in contracts['categories']]
    assert len(catalog['modules']) == 21
    assert len(commands) == 18 == len({x['operation'] for x in commands})
    assert all(x['runtime_available'] is False and x['implementation_status'] == 'planned' for x in commands)
    assert len(tasks) == 28 and len(selection['selected']) == 21 and len(selection['deferred']) == 7
    assert all(x['status'] == 'planned' and x['actual_result'] is None for x in tasks + selection['selected'])
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
    generation = read_json(S / 'packages/domain/factory-generation.design.json')
    policy = generation['delivery_policy']['initial_target_met']
    assert 'accepted_unit' in generation['goal_fields'] and 'acceptance_scope' in generation['goal_fields']
    assert all(policy[k] is False for k in ['requires_repair', 'requires_revised_run', 'requires_comparison'])
    scenarios = read_json(S / 'scenarios/目标驱动建厂/acceptance.design.json')
    assert 'compare_runs_or_explain_unrepairable' not in scenarios['required_steps']
    cases = {c['id']: c for c in scenarios['cases']}
    assert 'no_required_change_revised_run_or_comparison' in cases['GF-07']['expectation']
    assert 'new_run_comparison_or_inability_reason' in cases['GF-05']['expectation']
    assert {e['accepted_unit'] for e in scenarios['generic_unit_examples']} == {'carton', 'piece'}
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
    assert all(r['actual_thread_id'] is None and r['dispatch_status'] == 'planned' for r in roles.values())
    assert window_plan['actual_execution']['windows_dispatched_by_this_plan'] == 0
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
        assert assignment['status'] == 'planned' and assignment['actual_result'] is None
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
    assert design_gate['design_freeze']['status'] == 'not_ready'
    assert design_gate['execution_entry']['last_checkpoint_before_product_coding'] == 'S07-05'
    assert design_gate['execution_entry']['first_product_coding_phase'] == 'S08'
    assert design_gate['final_topic_pdf']['generated_this_turn'] is False
    assert design_gate['final_topic_pdf']['actual_output'] is None
    assert design_gate['frontend_supervision']['software'] == 'Figma'
    for key in ['design_start_authorization', 'frontend_build_authorization', 'approved_figma_artifact']:
        assert design_gate['frontend_supervision'][key] is None
    assert window_plan['frontend_supervision'] == design_gate['frontend_supervision']
    assert maturity['phases'][3]['checkpoints'][4]['checkpoint'] == '负责人实际原型走查'
    assert maturity['phases'][3]['checkpoints'][4]['review']['status'] == 'pending'
    prompt_index = read_json(D / '窗口启动提示词/启动提示词索引.json')
    assert {row['window_id'] for row in prompt_index['prompts']} == set(roles)
    assert len(prompt_index['prompts']) == 6
    for row in prompt_index['prompts']:
        assert row['primary_tasks'] == roles[row['window_id']]['primary_tasks']
        assert row['launch_authorized'] is False
        text = (S / row['prompt_ref']).read_text(encoding='utf-8-sig')
        assert all(token in text for token in ['S06-05', 'S07-05', 'Figma', 'explicit approval', 'module views', 'read-only review'])
        assert '阶段实现计划.json' in text and 'P work families do not introduce whole-family start barriers' in text
    package_index = read_json(S / selection['coding_package_index_ref'])
    package_by_id = {r['task_id']: r for r in package_index['packages']}
    assert len(package_index['packages']) == len(package_by_id) == 21 and set(package_by_id) == chosen
    for row in selection['selected']:
        package = package_by_id[row['id']]
        assert package['primary_window'] == row['planned_window_id']
        assert package['status'] == 'draft_not_dispatchable' and package['design_accepted'] is False
        assert package['actual_dispatch'] is None and package['actual_commands'] is None
        assert package['package_ref'] == row['coding_package_ref'] == task_by_id[row['id']]['first_release_coding_package_ref']
        assert row['coding_protocol_ref'] == task_by_id[row['id']]['first_release_coding_protocol_ref'] == design_gate['coding_protocol_ref']
        text = (S / package['package_ref']).read_text(encoding='utf-8-sig')
        assert row['scope'] in text and row['release_acceptance'] in text and 'draft_not_dispatchable' in text
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
        if milestone['id'] != 'MS-00':
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
        assert lifecycle['actual_exit_result'] is None
        expected_review_state = 'definitions_reviewed' if all(row['review']['status'] == 'passed' for row in phase['checkpoints']) else 'pending_reviews'
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
        assert family['status'] == 'planned' and family['actual_result'] is None
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
        assert row['verification_milestone'] == 'MS-11' and row['material_closure_milestone'] == 'MS-12'
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
    assert all(stage_plan[key] is None for key in ['actual_product_execution', 'actual_lifecycle_exit_results', 'actual_product_family_exits', 'actual_sf_execution', 'actual_ix_execution'])
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
    assert maturity['observations']['planned_operations'] == 18 and maturity['observations']['runtime_available_operations'] == 0
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
    files = sorted(f for f in P.rglob('*') if f.is_file())
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
            for target in links:
                target = unquote(target.strip('<>').split('#')[0])
                if not target or re.match(r'(?:[a-zA-Z]+:|//)', target) and not re.match(r'^[A-Za-z]:[/\\]', target):
                    continue
                checked_links += 1
                if not (path.parent / target).exists():
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
    output = {'check_kind': 'all_project_file_design_sync_not_runtime_acceptance', 'date': '2026-10-13', 'files': len(files), 'actions': dict(Counter(x['action'] for x in inventory)), 'matrix': results, 'checked_local_references': checked_links, 'errors': errors, 'runtime_available_operations': 0, 'completed_product_tasks': 0, 'maturity_percentages_unchanged': True, 'source_records': 87, 'professional_research_directions': 41, 'selected_professional_kernels': 1, 'existing_design_images': 22, 'preserved_assets_are_not_new_UI_coverage': True, 'joint_correction_checks': {'F01_generic_quantity_contract': 'passed', 'F02_initial_success_and_conditional_repair': 'passed', 'F03_all_seven_UT_SF_sets': 'passed'}}
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
    output['maturity_percentages_unchanged'] = True
    output['maturity_credit_change_this_revision'] = {
        'checkpoint': 'S03-02_independent_document_review', 'completion': [1, 1], 'evidence': [1, 1],
        'S03_completion': maturity['phases'][2]['completion_percent'],
        'S03_strength': maturity['phases'][2]['strength_percent'], 'changed_by_checker': False,
    }
    output['parallel_window_plan_checks'] = {
        'checkpoint': 'S02-03', 'status': 'passed_static_plan', 'accountable_owner': 'user',
        'planned_window_roles': len(roles), 'assigned_selected_tasks': len(assigned),
        'declared_write_ranges': len(write_ranges), 'overlapping_base_write_ranges': [],
        'shared_contract_and_build_configuration_owner': 'W00',
        'task_projection_and_document_mapping': 'passed',
        'deferred_tasks_assigned_windows': [], 'actual_dispatched_by_this_plan': 0,
        'actual_parallel_handoff': None, 'speedup_measurement': None,
    }
    output['design_and_coding_entry_checks'] = {
        'status': 'policy_and_drafts_synced_not_design_freeze',
        'design_review_gate': 'S06-05', 'required_design_reviews': len(design_checkpoints),
        'pending_design_reviews': len(pending_design_ids), 'execution_preparation_gate': 'S07-05',
        'startup_prompts': 6, 'coding_package_drafts': 21,
        'frontend_software': 'Figma', 'frontend_authorized_or_started': False,
        'module_views_technology_interfaces_before_coding': True,
        'final_topic_pdf_deferred': True,
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
    result_path.write_text(json.dumps(output, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    lines = ['# 全项目主线同步检查', '', f'2026-10-13。范围：超级工厂整个目录，以及工作区/大作业/开放原子三个阅读入口。检查的是文件主线一致与结构，不是软件功能验收；当前文档复审{len(passed_ids)}项通过/其余{len(checkpoints)-len(passed_ids)}项待复审。成熟度信用值与比例另按真实检查项记录，本工具不增加通过项。', '', '现行主线：九类自由创作与社区扩展的Windows工业模拟平台；外部Agent一句话产品/日产/场地→原生或导入社区行业模板/九类库→可编辑布局/连接/排期→静态检查/真实模拟→初案达标直接交付，不足时有限有据修正/说明→四格式评估和可编辑工程。', '', '|编号|逐项要求|更新检查|消费者数|', '|---|---|---|---|']
    lines += [f"|{x['id']}|{x['requirement']}|{x['status']}|{len(x['consumers'])}|" for x in results]
    lines += ['', f'逐文件清单共{len(files)}项，已检查{checked_links}处本地引用。处理方式：' + '；'.join(f'{k} {v}项' for k, v in output['actions'].items()) + '。', '', '[逐文件CSV](逐文件主线同步检查.csv) · [机器结果及消费者](主线同步检查结果.json) · [现行主线](../00_项目基线/项目主线.json) · [目标生成验收](../../scenarios/目标驱动建厂/README.md)', '', '已直接消解旧正文把目标生成列为后置、API/需求/清单旧计数、入口只列部分九类、生成源覆盖新字段、专业候选版与平台首版混称及报告使用改名前路径等问题。', '', '原厂原文、历史判断、旧图像像素和生成记录保留，不能改成新要求的证据。已有22张AI图未完整展示新增目标生成面板，其阅读说明和计划已明确；本轮没有生成新图或产品运行截图。', '', '首版17项需求、18项计划操作、21项选择/7项后置；三通用离散模板＋纸箱行业配置，只选纸包装真实专业内核。实际Agent/软件调用、产能达标和用户效果仍待实现及验证。']
    if errors:
        lines += ['', '## 尚待修正', ''] + ['- ' + json.dumps(e, ensure_ascii=False) for e in errors]
    else:
        lines += ['', '上述12项设计同步检查通过，无未消解的本地引用/结构问题。历史复审中的15/16计数只对应当时记录，现行入口按17/18解释。']
    lines += ['', '联合修正静态检查：通用接受单位合同；初案达标无需修正RunId/比较与独立不足案例；七项UT文字表与JSON需求集合逐项一致。仅证明设计同步，不是Agent或仿真执行结果。']
    lines += ['', 'S02-02静态规格检查：21项启动/完整验收前置及产物理由、输入/输出/步骤/验证与真实task投影一致；依赖无环/无后置，P5覆盖其他20项；IX-01至03有可达生产者与验收所有者。逻辑波次不代表实际排期，运行结果仍空。', '', '[任务拆解与依赖](任务拆解与依赖.md) · [首版任务JSON](../00_项目基线/首版任务选择.json)']
    lines += ['', 'S02-03六窗口计划检查：用户负责全项目；21首版任务各有唯一计划主责，六窗口基础写入范围无重叠，共同类型/配置归W00；任务投影及正文一致，7后置无窗口。S02-03信用不变；S02-04/05均独立文档复审通过，信用各1/0.5保持，S02文档100%/证据70%；实际阶段/运行未完成。没有派发或交接演练。', '', '[本机窗口计划](本机并行窗口计划.md) · [机器计划](本机并行窗口计划.json)']
    lines += ['', f'完整设计关口为S01—S06全部30项/S06-05，尚需{len(pending_design_ids)}项；S07-05后进S08。6份启动提示词/21份任务包草案与主责/范围一致；模块视图、技术/实现/接口认可后方可派发。Figma已选择，但设计/前端构建须用户监督与明确授权，尚未开始；最终选题PDF按用户要求延后。规则同步不把后续项标通过。', '', '[设计与执行准入](完整设计与执行准入.md) · [编码规则](编码任务包与AI实现审查.md) · [启动提示词](窗口启动提示词/README.md) · [任务包](编码任务包/README.md)']
    lines += ['', f'八项基线与现行任务框架检查：人/AI窗口/参赛成员分开，S01-01至S02-03消费者一致；21任务预算及共同工作加总、按窗口归集和13成果前置/完整任务覆盖正确，没有新任务依赖循环或每日日程。当前136—225人小时/251—438产品窗活动分别估计，未实测/不保证按期；正式过项{len(passed_ids)}，材料准备仍10月30日前。', '', '[八项联合复核](S01-01至S02-03_联合复核.md) · [任务工量/里程碑](任务框架与工量里程碑.md)']
    lines += ['', 'S02-04规范静态检查：21包/五共同工作估量口径与输入依据、13节点交付/接收/证据责任、4材料准备项与1外部手续、实际校准及剩余处理、四例有限工作流程推演仍保持；只证明规范可执行，未测工时或运行产品。', '', '[S02-04独立复审](S02-04_工量与成果里程碑复审.md)']
    lines += ['', 'S02-05独立复审静态检查：12生命周期与6工作族输入/输出/退出、21任务/17SF/3IX映射及原两种前置、预期业务结果/测试结论、失败与变更复核、候选测试/材料/维护边界保持。实际阶段和产品未退出。', '', '[阶段规范](阶段实现计划.md) · [S02-05独立复审](S02-05_阶段闭环与退出条件复审.md)']
    lines += ['', f'S03-01静态功能交接检查：七UT人工/Agent输入与动作、成功/未完成及下游交接字段齐全；17SF文字/映射一致，21所选任务及18计划操作引用覆盖。三例功能推演由独立复审记录；本工具验证引用/结构与原生成分支，未运行产品或验证目标使用者。当前下一项{maturity["checkpoint_review"]["next_checkpoint"]}。', '', '[S03-01独立复审](S03-01_用户任务与功能需求复审.md) · [七UT映射](../00_项目基线/用户任务与需求映射.json)']
    lines += ['', f'S03-02静态业务复核：9类现有业务对象、独立状态轴/预留与9条异常出口、派工/部分接受/取消及专业首版/后置范围一致；6边界与6既有案例有限复核通过，冻结预期未改写。当前下一项{maturity["checkpoint_review"]["next_checkpoint"]}，不代表产品执行。', '', '[S03-02独立复审](S03-02_业务对象规则与异常路径复审.md) · [业务JSON](../../packages/domain/business-objects.design.json)']
    report_path.write_text('\n'.join(lines) + '\n', encoding='utf-8')
    print(json.dumps({'files': len(files), 'matrix_passed': sum(x['status'] == '设计同步通过' for x in results), 'references': checked_links, 'errors': errors, 'actions': output['actions']}, ensure_ascii=False))
    if errors:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
