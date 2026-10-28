"""S04-01导航结构与有限LOD/坐标推导，不创建或运行UI。"""
from pathlib import Path
from statistics import median
import json
import sys

S = Path(__file__).resolve().parent.parent
A = S / 'docs/04_开发与验收'


def read(ref):
    return json.loads((S / ref).read_text(encoding='utf-8-sig'))


def choose_lod(current, projected_sizes, thresholds, lock=None):
    if lock is not None or not projected_sizes:
        return lock or current
    p = median(projected_sizes)
    while True:
        previous = current
        if current == 'detailed' and p < thresholds['detailed_to_simplified_lt']:
            current = 'simplified'
        elif current == 'simplified':
            if p < thresholds['simplified_to_region_lt']:
                current = 'region'
            elif p >= thresholds['simplified_to_detailed_gte']:
                current = 'detailed'
        elif current == 'region' and p >= thresholds['region_to_simplified_gte']:
            current = 'simplified'
        if current == previous:
            return current


def main():
    nav = read('docs/02_交互设计/交互流程与视图导航.json')
    mapping = read('docs/00_项目基线/用户任务与需求映射.json')
    ops = read('packages/commands/command-catalog.design.json')['operation_catalog']
    nodes = {r['id']: r for r in nav['page_tree']}
    assert len(nodes) == len(nav['page_tree'])
    assert 'timeline_description_panel' in nodes
    assert [r['id'] for r in nav['page_tree'] if r['kind'] == 'canvas'] == nav['normal_canvases'] == ['line_canvas', 'multi_hall_canvas']
    assert nav['zoom_opens_detail'] is False and nav['zoom_switches_canvas'] is False
    assert nav['overview_is_line_canvas_lod'] is True
    assert all(child in nodes for row in nodes.values() for child in row['children'])
    reachable, queue = set(), ['project_entry']
    while queue:
        node = queue.pop()
        if node not in reachable:
            reachable.add(node)
            queue.extend(nodes[node]['children'])
    assert reachable == set(nodes)
    routes = nav['task_routes']
    assert [r['user_task'] for r in routes] == [r['user_task'] for r in mapping['rows']]
    assert {sf for r in routes for sf in r['SF_ids']} == {f'SF-{i:02}' for i in range(1, 18)}
    assert {op for r in routes for op in r['planned_operation_refs']} == {r['operation'] for r in ops}
    for r in routes:
        assert all(p in nodes for p in r['panel_sequence'])
        assert r['exit_and_handoff'] and r['back_behavior'] and r['actual_result'] is None
    # 设计授权不等于已有产品运行；不要求撤销现行用户授权才能通过静态检查。
    assert nav['runtime_available'] is False and nav['actual_result'] is None
    assert all((S / ref).exists() for ref in nav['source_refs'])
    t = nav['lod_policy']['thresholds_css_px']
    current = 'detailed'
    sequence = [(64, 'detailed'), (63, 'simplified'), (79, 'simplified'), (80, 'detailed'),
                (23, 'region'), (31, 'region'), (32, 'simplified'), (80, 'detailed')]
    for p, expected in sequence:
        current = choose_lod(current, [p, p], t)
        assert current == expected
    assert choose_lod('detailed', [1], t, 'detailed') == 'detailed'
    assert choose_lod('region', [], t) == 'region'
    assert nav['pointer_policy']['gesture_priority'].index('explicit_handle') < nav['pointer_policy']['gesture_priority'].index('object_hit')
    assert 'pointercancel' in nav['pointer_policy']['move'] and '背/底面' in nav['pointer_policy']['orbit']
    origin, delta, local = (10, 20, 0), (3, 4, 0), (1, 2, 0)
    moved_root = tuple(x + d for x, d in zip(origin, delta))
    moved_member = tuple(x + l for x, l in zip(moved_root, local))
    assert moved_root == (13, 24, 0) and moved_member == (14, 26, 0) and local == (1, 2, 0)
    assert len(nav['editing_policy']['close_dirty_form']) == 3
    assert nav['confirmed_choices_ref'] == 'docs/00_项目基线/全量设计编码确认选择.json'
    assert '手动' in nav['lod_policy']['default']
    assert '新ID' in nav['editing_policy']['group_actions']['supported'][-1]
    assert '不物理删除' in nav['editing_policy']['undo']
    for field in ['active_canvas', 'active_hall_id_or_null', 'camera_pose', 'lod_value', 'run_id_or_null', 'view_time', 'selected_object_ref_or_null']:
        assert field in nav['return_context_fields']
    assert '只读' in nav['editing_policy']['run_edit']
    assert len(nav['deferred_ui']) == 6 and '不作为已可执行' in nav['first_release_deferred_visibility']
    policy = read('packages/domain/factory-generation.design.json')['delivery_policy']['initial_target_met']
    assert all(policy[k] is False for k in ['requires_repair', 'requires_revised_run', 'requires_comparison'])
    forms = read('docs/02_交互设计/首版表单与反馈.json')
    assert forms['checkpoint'] == 'S04-03' and forms['navigation_ref'] == 'docs/02_交互设计/交互流程与视图导航.json'
    assert len(forms['forms']) == 28
    categories = read('packages/module-kit/category-contracts.design.json')['categories']
    assert {f['category'] for f in forms['forms'] if f['category'] not in [None, 'common']} == {c['id'] for c in categories}
    assert {op for f in forms['forms'] for op in f['planned_operations']} == {op['operation'] for op in ops}
    field_ids = [x['ui_field_id'] for f in forms['forms'] for x in f['fields']]
    assert len(field_ids) == len(set(field_ids))
    field_bindings = forms['field_contract_bindings']
    assert len(field_bindings) == 71
    assert len({b['contract_ref'] for b in field_bindings}) == 71
    for binding in field_bindings:
        assert set(binding['ui_field_ids']) <= set(field_ids)
        file_ref, pointer = binding['contract_ref'].split('#', 1)
        contract = read(file_ref)
        for part in pointer.lstrip('/').split('/'):
            contract = contract[int(part)] if isinstance(contract, list) else contract[part]
        assert binding['logical_field'] == contract.get('path', contract.get('field'))
        assert contract['unit_rule'] and contract['validation_rule'] and contract['canonical_carriers']
    timeline_form = next(f for f in forms['forms'] if f['id'] == 'F16')
    timeline_fields = {f['ui_field_id']: f for f in timeline_form['fields']}
    assert set(timeline_fields) == {f'F16-{i:02}' for i in range(1, 10)}
    for ident in ['F16-04', 'F16-05', 'F16-06', 'F16-07']:
        assert timeline_fields[ident]['mode'] == 'view_selection'
    for ident in ['F16-03', 'F16-08', 'F16-09']:
        assert timeline_fields[ident]['mode'] == 'read_only'
    detail = read(nav['timeline_complete_definition_ref'])
    assert forms['timeline_complete_definition_ref'] == nav['timeline_complete_definition_ref']
    assert [c['command'] for c in nav['timeline_window_menu']] == ['window.timeline', 'window.timeline_description']
    assert 'timeline_view_menu' not in nav
    layout = nav['timeline_projection_policy']['layout']
    assert layout['description_side'] == 'left' and layout['timeline_side'] == 'right'
    assert layout['hidden_description_on_selection'] == 'keep_hidden_update_shared_selected_event_ref'
    assert [s['id'] for s in detail['sections']] == [f'TL{i:02}' for i in range(1, 13)]
    assert len(detail['private_view_records']) == 5
    cases = {c['id']: c for c in detail['independent_design_cases']}
    c = cases['DTC01']
    data, expected = c['input'], c['expected']
    for axis in ['main', 'child']:
        start, end = data[axis + '_window']
        x = data[axis + '_left'] + (data['time_ms'] - start) * data[axis + '_width'] / (end - start)
        assert x == expected[axis + '_x']
        recovered = start + (x - data[axis + '_left']) * (end - start) / data[axis + '_width']
        assert recovered == expected['recovered_time_ms']
    c = cases['DTC02']
    original, window = c['input']['true_interval'], c['input']['child_window']
    assert [max(original[0], window[0]), min(original[1], window[1])] == c['expected']['visible_interval']
    assert original == c['expected']['true_interval'] and c['input']['job_id'] == c['expected']['job_id']
    assert (original[0] < window[0]) == c['expected']['continues_left']
    assert (original[1] > window[1]) == c['expected']['continues_right']
    c = cases['DTC03']
    start, end = c['input']['child_window']
    boundary = c['input']['computed_until_ms']
    assert [start, boundary] == c['expected']['known_window']
    assert [boundary, end] == c['expected']['unknown_window']
    assert c['expected']['phase_completed_claim'] is False
    assert all(c['actual_product_result'] is None for c in cases.values())
    for form in forms['forms']:
        assert form['navigation_node'] in nodes and (S / form['source_ref']).exists()
        assert form['apply_and_effect'] and form['actual_result'] is None
        for field in form['fields']:
            assert field['value_kind'] in forms['value_kind_controls']
            assert field['label'] and field['required_for'] and field['unit_or_reference']
            if field['value_kind'] == 'result':
                assert field['mode'] == 'read_only'
    feedback = {r['id']: r for r in forms['feedback_states']}
    assert len(feedback) == 12 and all(r['trigger'] and r['visible'] and r['next'] for r in feedback.values())
    assert '真实终态' in feedback['cancel_requested']['next']
    assert '不能清空' in feedback['cancel_requested']['visible']
    assert '不自动覆盖' in feedback['revision_conflict']['next']
    assert forms['actual_Figma'] is None and forms['actual_frontend'] is None and forms['actual_runtime'] is None
    result = {'check_kind': 'navigation_specification_and_finite_math_not_ui_execution',
              'date': '2026-10-14 Asia/Shanghai', 'status': 'passed_static_navigation_specification',
              'nodes': len(nodes), 'normal_canvases': 2, 'task_routes': len(routes), 'SF_coverage': 17,
              'planned_operation_refs': len(ops),
              'finite_checks': ['LOD八步迟滞/跨级稳定', '锁档与空厅', '两画布与节点可达性',
                                '七UT/17SF/18操作覆盖', '手柄与点击优先序', '线组根变换及局部坐标',
                                '返回/脏草稿/只读归属字段', '初案达标与后置导航边界'],
              'actual_frontend_or_Figma': None, 'actual_product_runtime': None, 'errors': []}
    result['form_spec_checks'] = {'checkpoint': 'S04-03', 'status': 'passed_static_form_and_feedback_specification',
                                 'forms': 28, 'field_groups': len(field_ids), 'category_coverage': 9, 'feedback_states': 12,
                                 'actual_UI_or_product_execution': None}
    result['timeline_detail_checks'] = {'definition_sections': 12, 'private_view_records': 5,
                                       'finite_math_cases_passed': ['DTC01', 'DTC02', 'DTC03'],
                                       'behavior_cases_require_actual_prototype': [c for c in cases if c not in ['DTC01', 'DTC02', 'DTC03']],
                                       'actual_UI_or_product_execution': None}
    if '--read-only' not in sys.argv[1:]:
        (A / '交互导航检查结果.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(result, ensure_ascii=False))


if __name__ == '__main__':
    main()
