"""S03-04独立规范、负载及合成数学检查；不启动产品/测量性能。"""
from pathlib import Path
from statistics import median
from math import ceil
import json
import re
from datetime import datetime, timedelta, timezone

S = Path(__file__).resolve().parent.parent
A = S / 'docs/04_开发与验收'


def read(rel):
    return json.loads((S / rel).read_text(encoding='utf-8-sig'))


def main():
    spec = read('docs/01_业务需求/非功能需求与测量验收.json')
    bench = read(spec['benchmark_ref'])
    examples = read(spec['sample_examples_ref'])
    machine = read(spec['machine_ref'])
    selection = read('docs/00_项目基线/首版任务选择.json')
    windows = read('docs/04_开发与验收/本机并行窗口计划.json')
    task_ids = {r['id'] for r in selection['selected']}
    role_ids = {r['id'] for r in windows['window_roles']}
    targets = spec['numeric_targets']
    assert targets == {'halls': 3, 'devices': 60, 'tracked_items': 3000,
                       'canvas_css_px': [1920, 1080], 'median_fps_gte': 30,
                       'p95_editor_visible_ms_lte': 200, 'save_each_ms_lte': 3000,
                       'open_each_ms_lte': 3000}
    # 对照只读功能规范中的既有目标，不改写其正文。
    inherited = (S / spec['inherited_quality_source_ref']).read_text(encoding='utf-8-sig')
    quality_section = inherited.split('## 性能与质量目标', 1)[1]
    for token in ['60固定设备', '3000件', '30FPS', '200ms', '3秒']:
        assert token in quality_section, ('inherited_target_missing', token)
    rows = spec['requirements']
    assert [r['id'] for r in rows] == [f'NFR-{i:02}' for i in range(1, 12)]
    for row in rows:
        assert set(row['delivery_owners']) <= role_ids
        assert set(row['implementation_tasks']) <= task_ids
        assert set(row['related_SF']) <= set(selection['scope_freeze']['included_requirement_ids'])
        assert row['preconditions'] and row['procedure'] and row['measurement'] and row['acceptance']
        assert row['runtime_status'] == 'not_executed' and row['actual_result'] is None
    row_map = {row['id']: row for row in rows}
    coverage = spec['confirmation_coverage']
    assert set(coverage) == set(spec['confirmed_design_choices']['question_ids'])
    for question, ids in coverage.items():
        assert ids and all(question in row_map[ident]['confirmation_ids'] for ident in ids)
    protocol = spec['test_execution_contract']
    assert protocol['result_precedence'] == ['fail', 'not_evaluable', 'pass']
    readiness = protocol['readiness_contract']['NFR-01']
    assert readiness == row_map['NFR-01']['readiness_contract']
    assert set(readiness) == {'benchmark_ready', 'compatibility_ready', 'partial_results'}
    assert 'benchmark_ready' in row_map['NFR-02']['preconditions'][0]
    assert 'benchmark_ready' in row_map['NFR-03']['preconditions'][0]
    classification = protocol['runtime_classification']
    assert classification['aggregation_order'] == [
        'known_failure', 'no_checks_started', 'required_evidence_missing', 'all_required_checks_pass']
    record = protocol['measurement_record_contract']
    assert {'clock_source', 'group_status', 'subcheck_id'} <= set(record['group_fields'])
    assert {'attempt_index', 'actual_outcome', 'attempt_status'} <= set(record['attempt_fields'])
    required_groups = protocol['required_check_groups']
    assert set(required_groups) == set(row_map)
    assert sum(len(groups) for groups in required_groups.values()) == 38
    assert all(groups and all(key and description for key, description in groups.items())
               for groups in required_groups.values())
    assert set(protocol['long_task_fixture_requirements']) == {'simulation', 'packaging', 'media', 'report'}
    for fixture in protocol['long_task_fixture_requirements'].values():
        assert fixture['input'] and fixture['preparation']
        assert set(fixture['producer_tasks']) <= task_ids
        assert (S / fixture['source_ref']).is_file()
    assert 'benchmark_ready' in row_map['NFR-04']['preconditions'][0]
    assert 'benchmark_ready' in row_map['NFR-05']['preconditions'][0]
    assert '同机客户端' in row_map['NFR-04']['measurement']['external_caller_clock']
    assert '不预先加载' in row_map['NFR-05']['measurement']['new_process_boundary']
    matrix = protocol['matrix']
    assert matrix['fps_conditions_per_round'] == 2 * (3 * 2 + 2) == 16
    assert matrix['fps_observation_windows'] == 16 * matrix['fps_rounds'] == 48
    assert matrix['editor_attempts_total'] == 2 * 3 * 3 * 100 == 1800
    assert matrix['save_open_attempts_total'] == 2 * 4 * 5 == 40
    assert len(row_map['NFR-03']['usability_checks']['keyboard_tasks']) == 8
    assert row_map['NFR-03']['usability_checks']['text_scale_percent_test_cases'] == [100, 150, 200]
    assert 'Host确认' in row_map['NFR-03']['measurement']['end']
    assert set(row_map['NFR-04']['delivery_owners']) >= {'W00', 'W02', 'W03', 'W04', 'W05'}
    assert len(row_map['NFR-04']['measurement']['groups']) == 4
    assert '另一台' in row_map['NFR-01']['procedure'][-1]
    display = row_map['NFR-09']['display_semantics_checks']
    for key in ['staff_input', 'staff_expected']:
        ref = display[key]
        node = read(ref['path'])
        for token in ref['json_pointer'].strip('/').split('/'):
            node = node[int(token)] if isinstance(node, list) else node[token]
        assert node['id'] == ref['expected_identity']
    assert node['expected_worker_occupied_ms'] == 10000
    assert node['expected_wait_ms'] == 5000 and node['expected_completion_ms'] == 45000
    trial = row_map['NFR-10']['experience_trial']
    assert trial['target_ms_lte'] == 30000 and trial['actual_result'] is None
    assert trial['role'] == 'candidate_experience_target_not_hard_timeout_or_runtime_claim'
    assert spec['actual_runtime'] is None and spec['actual_measurements'] == []
    assert len(bench['halls']) == 3 and len(bench['devices']) == 60 and len(bench['lines']) == 6
    assert len({d['id'] for d in bench['devices']}) == 60
    assert len(bench['internal_links']) == 6 * 9
    coordinates = bench['coordinate_contract']
    assert coordinates['source_tuple_slots'] == ['width_x', 'height_z', 'depth_y']
    assert coordinates['factory_axes'] == {'X': 'width_m', 'Y': 'depth_m', 'Z': 'height_m'}
    assert coordinates['visual_envelope_slots'] == ['width_x', 'depth_y', 'height_z']
    generation_axes = read('packages/domain/factory-generation.design.json')['site_binding']
    assert generation_axes == {'length': 'hall.width_m/X', 'width': 'hall.depth_m/Y', 'height': 'hall.height_m/Z'}
    rotation = coordinates['factory_to_renderer_rotation_matrix']
    # 有限坐标推导：保持右手朝向的正交旋转，而非仅交换轴产生镜像。
    for i in range(3):
        for j in range(3):
            assert sum(rotation[i][k] * rotation[j][k] for k in range(3)) == int(i == j)
    determinant = (rotation[0][0] * (rotation[1][1]*rotation[2][2] - rotation[1][2]*rotation[2][1])
                   - rotation[0][1] * (rotation[1][0]*rotation[2][2] - rotation[1][2]*rotation[2][0])
                   + rotation[0][2] * (rotation[1][0]*rotation[2][1] - rotation[1][1]*rotation[2][0]))
    assert determinant == 1
    positions_checked = 0
    for device in bench['devices']:
        for variant in ['balanced', 'dense']:
            a, b, c = device[variant + '_position_m']
            factory = [a, c, b]
            renderer = [sum(rotation[i][j] * factory[j] for j in range(3)) for i in range(3)]
            assert renderer == [a, b, -c]
            assert [renderer[0], -renderer[2], renderer[1]] == factory
            assert 0 <= factory[0] <= 60 and 0 <= factory[1] <= 80 and 0 <= factory[2] <= 8
            positions_checked += 1
    all_line_devices = [ident for line in bench['lines'] for ident in line['device_ids']]
    assert len(all_line_devices) == len(set(all_line_devices)) == 60
    assert set(all_line_devices) == {d['id'] for d in bench['devices']}
    assert sum(line['initial_tracked_item_count'] for line in bench['lines']) == 3000
    identities = [f"{line['id']}-ITEM-{i:04}" for line in bench['lines'] for i in range(1, 501)]
    assert len(identities) == len(set(identities)) == 3000
    hall_ids = [h['id'] for h in bench['halls']]
    for variant in bench['variants']:
        key = 'balanced_hall' if variant['id'] == 'balanced' else 'dense_hall'
        counts = [sum(len(line['device_ids']) for line in bench['lines'] if line[key] == h) for h in hall_ids]
        assert counts == variant['device_count_by_hall'] and sum(counts) == 60
        assert all(0 <= p[0] <= 60 and 0 <= p[2] <= 80 for p in
                   [d[variant['id'] + '_position_m'] for d in bench['devices']])
    assert bench['actual_project_ref'] is None and bench['actual_execution'] is None
    assert 'online_observation_speed_virtual_s_per_wall_s' not in bench['execution_intent']
    assert bench['execution_intent']['result_playback_virtual_s_per_wall_s'] == 10
    assert 'without changing Engine/inventory' in bench['execution_intent']['real_engine_before_playback']
    assert bench['asset_freeze']['actual_manifest_ref'] is None
    assert bench['measurement_matrix'] == matrix
    # 当前S03-04的可执行采样计划与S06-02真实字段消费者。
    plan = protocol['editor_attempt_plan']
    assert plan['source_ref'] == spec['benchmark_ref']
    assert plan['attempts_per_action_per_round'] == 100
    assert plan['actual_execution'] is None
    assert set(plan['actions']) == set(row_map['NFR-03']['measurement']['actions'])
    targets = [all_line_devices[i % 60] for i in range(100)]
    assert set(targets) == set(all_line_devices)
    assert all(a != b for a, b in zip(targets, targets[1:]))
    edit = plan['actions']['edit_existing_scalar_parameter']['target']
    assert edit['device_id'] == all_line_devices[0]
    assert edit['declared_parameter'] == 'processing_sim_seconds'
    assert edit['values_in_virtual_seconds'] == [3, 2]
    before = bench['execution_intent']['processing_sim_seconds']
    for i in range(100):
        after = edit['values_in_virtual_seconds'][i % 2]
        assert before != after
        before = after
    assert before == bench['execution_intent']['processing_sim_seconds']
    assert row_map['NFR-03']['measurement']['attempt_plan_ref'] == '/test_execution_contract/editor_attempt_plan'
    binding_ref = record['type_binding_ref']
    binding = read(binding_ref['path'])['measurement_records']
    assert binding_ref['json_pointer'] == '/measurement_records'
    assert binding['source_ref'] == spec['spec_md_ref'].replace('.md', '.json')
    assert binding['schema_ref'] == record['schema_ref'] and (S / record['schema_ref']).is_file()
    assert binding['actual_samples'] == []
    type_source = (S / read(binding_ref['path'])['rust_type_source_ref']).read_text(encoding='utf-8')
    fields = {name: set(re.findall(r'pub (\w+):', body))
              for name, body in re.findall(r'pub struct (\w+)[^{]*\{(.*?)\n\}', type_source, re.S)}
    bound_count = 0
    for kind in ['group', 'attempt']:
        assert set(binding['field_bindings'][kind]) == set(record[kind + '_fields'])
        for bound in binding['field_bindings'][kind].values():
            for carrier in bound:
                assert carrier['field'] in fields[carrier['type']]
                bound_count += 1
    assert machine['actual_performance_result'] is None and machine['actual_rendering_adapter'] is None
    math_results = []
    for case in examples['cases']:
        if case['kind'] == 'nearest_rank_p95':
            values = sorted(value['value'] for value in case['compressed_samples_ms'] for _ in range(value['count']))
            assert len(values) == case['expected_n'] == 100
            result = values[ceil(0.95 * len(values)) - 1]
            assert result == case['expected_p95_ms']
            verdict = 'pass_for_synthetic_math_only' if result <= case['limit_ms'] else 'fail_for_synthetic_math_only'
        elif case['kind'] == 'fps_median':
            result = median(1000 / delta for delta in case['frame_intervals_ms'])
            assert result == case['expected_median_instantaneous_fps']
            verdict = 'pass_for_synthetic_math_only' if result >= case['limit_fps'] else 'fail_for_synthetic_math_only'
        elif case['kind'] == 'insufficient_samples':
            assert case['n'] < case['required_n']
            result, verdict = None, 'not_evaluable'
        elif case['kind'] == 'all_individual_save_times':
            result = max(case['samples_ms'])
            verdict = 'pass_for_synthetic_math_only' if all(v <= case['limit_ms'] for v in case['samples_ms']) else 'fail_for_synthetic_math_only'
        elif case['kind'] == 'deterministic_serial_lines':
            first = case['stations_per_line'] * case['processing_sim_s']
            last = (case['stations_per_line'] + case['items_per_line'] - 1) * case['processing_sim_s']
            accepted = lambda t: case['line_count'] * min(case['items_per_line'], max(0, (t - first) // case['processing_sim_s'] + 1))
            assert first == case['expected_first_accept_s'] == bench['execution_intent']['expected_first_accept_s']
            assert last == case['expected_last_accept_s'] == bench['execution_intent']['expected_last_accept_s']
            assert accepted(600) == case['accepted_at_600_s'] == bench['execution_intent']['expected_accepted_count_at_600_s']
            assert accepted(1200) == case['accepted_at_1200_s'] == bench['execution_intent']['expected_accepted_count_at_1200_s']
            result, verdict = {'first_s': first, 'last_s': last, 'accepted_1200': accepted(1200)}, 'passed_finite_design_math_only'
        elif case['kind'] == 'attempt_outcome_precedence':
            values = sorted(case['latencies_ms'])
            p95 = values[ceil(0.95 * len(values)) - 1] if len(values) >= 100 else None
            assert p95 == case['expected_p95_ms']
            if case['failed_attempts']:
                verdict = 'fail_for_synthetic_math_only'
            elif case['missing_evidence_attempts'] or len(values) < case['attempts']:
                verdict = 'not_evaluable'
            else:
                verdict = 'pass_for_synthetic_math_only'
            result = {'attempts': case['attempts'], 'p95_ms': p95,
                      'failed_attempts': case['failed_attempts'],
                      'missing_evidence_attempts': case['missing_evidence_attempts']}
        elif case['kind'] == 'professional_experience_classification':
            result = []
            for sample in case['samples']:
                if not sample['complete'] or not sample['matches_expected']:
                    experience = 'not_completed'
                else:
                    experience = 'target_met' if sample['elapsed_ms'] <= case['target_ms'] else 'target_not_met'
                assert experience == sample['expected_experience']
                result.append({'professional_outcome': sample['professional_outcome'], 'experience': experience})
            verdict = 'passed_finite_design_math_only'
        elif case['kind'] == 'environment_failure_precedence':
            result = []
            for sample in case['samples']:
                status = ('fail' if sample['known_product_failure'] else
                          'not_evaluable' if not sample['environment_valid'] else 'pass')
                assert status == sample['expected_status']
                result.append(status)
            verdict = 'passed_finite_design_math_only'
        elif case['kind'] == 'required_check_aggregation':
            result = []
            for sample in case['samples']:
                children = sample['child_statuses']
                assert children and set(children) <= {'fail', 'not_executed', 'not_evaluable', 'pass'}
                if 'fail' in children:
                    status = 'fail'
                elif all(child == 'not_executed' for child in children):
                    status = 'not_executed'
                elif any(child != 'pass' for child in children):
                    status = 'not_evaluable'
                else:
                    status = 'pass'
                assert status == sample['expected_status']
                result.append(status)
            verdict = 'passed_finite_design_math_only'
        else:
            raise AssertionError(('unknown_synthetic_case', case['id']))
        assert verdict == case['expected_verdict']
        math_results.append({'id': case['id'], 'result': result, 'verdict': verdict})
    assert len(math_results) == 11 and examples['actual_product_samples'] == [] and examples['actual_runtime'] is None
    result = {'check_kind': 'S03_04_static_specification_and_synthetic_math_not_performance',
              'date': datetime.now(timezone(timedelta(hours=8))).date().isoformat() + ' Asia/Shanghai', 'status': 'passed_static_specification_and_finite_math',
              'NFR_rows': 11, 'task_owners_and_SF_refs': 'passed', 'inherited_numeric_targets': 'unchanged',
              'load_profile': {'halls': 3, 'devices': 60, 'tracked_identities': 3000, 'variants': 2},
              'synthetic_math_results': math_results, 'machine_read': 'inventory_only',
              'confirmed_choices_covered': coverage, 'measurement_matrix': matrix,
              'confirmed_choice_and_timing_checks': 'passed_static_specification',
              'readiness_and_result_classification_checks': 'passed_static_specification_and_finite_cases',
              'minimal_measurement_record_checks': 'passed_static_specification',
              'required_check_group_count': sum(len(groups) for groups in required_groups.values()),
              'required_check_groups': required_groups,
              'coordinate_mapping_checks': {'status': 'passed_finite_coordinate_reasoning',
                                            'positions': positions_checked, 'rotation_determinant': determinant,
                                            'actual_renderer_execution': None},
              'readiness_and_clock_boundaries': 'NFR02_to05_benchmark_ready_only_local_tool_clock_and_new_process_open_explicit',
              'existing_staff_expected_values_preserved': True,
              'editor_attempt_plan_checks': {'status': 'passed_finite_plan_not_runtime',
                                            'distinct_device_targets': len(set(targets)),
                                            'attempts_per_action_per_round': len(targets),
                                            'baseline_attempts_unchanged': matrix['editor_attempts_total'],
                                            'actual_execution': None},
              'measurement_type_binding_checks': {'status': 'passed_static_field_mapping',
                                                 'type_contract_ref': binding_ref['path'],
                                                 'logical_fields': len(record['group_fields']) + len(record['attempt_fields']),
                                                 'carrier_bindings': bound_count},
              'actual_product_runtime': None, 'actual_performance_result': None, 'errors': []}
    (A / '非功能规格检查结果.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({k: result[k] for k in ['status', 'NFR_rows', 'load_profile', 'actual_performance_result', 'errors']}, ensure_ascii=False))


if __name__ == '__main__':
    main()
