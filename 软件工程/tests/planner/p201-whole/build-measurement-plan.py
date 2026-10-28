"""Freeze intended attempts, not measured outcomes or performance values."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
def read(path):
    return json.loads((ROOT / path).read_text(encoding='utf-8-sig'))
profile = read('scenarios/非功能基准/benchmark.design.json')
spec = read('docs/01_业务需求/非功能需求与测量验收.json')
contract = spec['test_execution_contract']
ordered = [(identifier, line) for line in profile['lines'] for identifier in line['device_ids']]
fps, editing, directories = [], [], []
for variant in ('balanced', 'dense'):
    for round_index in range(1, 4):
        for canvas, lods in [('line_canvas', ['detailed', 'simplified', 'region']), ('multi_hall_canvas', ['region'])]:
            for lod in lods:
                for motion in ('completed_run_playback_10x', 'paused_camera_motion'):
                    fps.append({'id': f'{variant}/{canvas}/{lod}/{motion}/{round_index}', 'load_variant': variant, 'canvas': canvas, 'lod': lod, 'motion': motion, 'round': round_index,
                                'warmup_seconds': 10, 'window_seconds': 60, 'view_start_ms': 0 if motion == 'completed_run_playback_10x' else 600000,
                                'view_end_ms': 600000, 'clock_source': 'actual_visible_candidate_WebView2_Performance_Frames', 'candidate_reference': None, 'actual_result': None})
        for action in ('select_device', 'open_device_detail', 'edit_existing_scalar_parameter'):
            attempts = []
            for index in range(100):
                identifier, line = ordered[index % 60]
                if action == 'edit_existing_scalar_parameter':
                    expected = {'device_id': 'L1-D01', 'parameter_id': 'processing_sim_seconds', 'virtual_seconds': [3, 2][index % 2], 'canonical_milliseconds': [3000, 2000][index % 2], 'new_revision_confirmed_and_presented': True, 'old_Run_unchanged': True}
                else:
                    expected = {'device_id': identifier, 'setup_hall_before_start': line[variant + '_hall'], 'first_target_visit_in_this_group': index < 60, 'actual_correct_selection_or_detail_frame': True}
                attempts.append({'attempt_index': index + 1, 'expected_outcome': expected, 'actual_outcome': None, 'elapsed_ms': None})
            editing.append({'id': f'{variant}/{action}/{round_index}', 'load_variant': variant, 'action': action, 'round': round_index, 'planned_attempt_count': 100, 'attempts': attempts, 'candidate_reference': None, 'group_status': 'planned_not_executed'})
    for action in ('save_new_directory', 'save_update_directory', 'open_new_app_process', 'reopen_same_process'):
        for index in range(1, 6):
            directories.append({'id': f'{variant}/{action}/{index}', 'load_variant': variant, 'action': action, 'attempt_index': index, 'frozen_processing_ms': 2000, 'actual_outcome': None, 'elapsed_ms': None, 'candidate_reference': None})
assert len(fps) == 48 and sum(len(group['attempts']) for group in editing) == 1800 and len(directories) == 40
usability = next(value['usability_checks'] for value in spec['requirements'] if value['id'] == 'NFR-03')
value = {'document_kind': 'concrete_frozen_attempt_plan_not_actual_measurement', 'spec_ref': 'docs/01_业务需求/非功能需求与测量验收.json', 'candidate_reference': None,
         'machine_reference': '.work/p201-whole-20261017/current-machine.json', 'fps_windows': fps, 'editor_groups': editing, 'directory_attempts': directories,
         'usability_checks': usability, 'failed_attempt_rule': contract['failed_attempt_rule'], 'clock_rule': contract['measurement_record_contract']['clock_rule'], 'actual_execution': None}
(ROOT / 'tests/planner/p201-whole/fixtures/b60/measurement-plan.json').write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(json.dumps({'fps_windows': len(fps), 'editor_attempts': 1800, 'directory_attempts': len(directories), 'text_scales': usability['text_scale_percent_test_cases'], 'actual_execution': None}))
