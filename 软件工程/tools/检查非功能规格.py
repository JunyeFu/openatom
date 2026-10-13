"""S03-04独立规范、负载及合成数学检查；不启动产品/测量性能。"""
from pathlib import Path
from statistics import median
from math import ceil
import json

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
    assert spec['actual_runtime'] is None and spec['actual_measurements'] == []
    assert len(bench['halls']) == 3 and len(bench['devices']) == 60 and len(bench['lines']) == 6
    assert len({d['id'] for d in bench['devices']}) == 60
    assert len(bench['internal_links']) == 6 * 9
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
    assert bench['asset_freeze']['actual_manifest_ref'] is None
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
        else:
            raise AssertionError(('unknown_synthetic_case', case['id']))
        assert verdict == case['expected_verdict']
        math_results.append({'id': case['id'], 'result': result, 'verdict': verdict})
    assert len(math_results) == 6 and examples['actual_product_samples'] == [] and examples['actual_runtime'] is None
    result = {'check_kind': 'S03_04_static_specification_and_synthetic_math_not_performance',
              'date': '2026-10-13 Asia/Shanghai', 'status': 'passed_static_specification_and_finite_math',
              'NFR_rows': 11, 'task_owners_and_SF_refs': 'passed', 'inherited_numeric_targets': 'unchanged',
              'load_profile': {'halls': 3, 'devices': 60, 'tracked_identities': 3000, 'variants': 2},
              'synthetic_math_results': math_results, 'machine_read': 'inventory_only',
              'actual_product_runtime': None, 'actual_performance_result': None, 'errors': []}
    (A / '非功能规格检查结果.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({k: result[k] for k in ['status', 'NFR_rows', 'load_profile', 'actual_performance_result', 'errors']}, ensure_ascii=False))


if __name__ == '__main__':
    main()
