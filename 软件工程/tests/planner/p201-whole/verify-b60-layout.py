"""Independent production expectations for the explicitly laid out B60 copies."""
from collections import Counter
from pathlib import Path
import json
import sys

root = Path(sys.argv[1]).resolve()
read = lambda path: json.loads(path.read_text(encoding='utf-8-sig'))
results = []
for name in ('balanced', 'dense'):
    saved = root / 'saved' / name
    manifest = read(saved / 'project.json')['data']
    documents = {d['role']: d for d in manifest['documents']}
    load = lambda role: read(saved / documents[role]['file_ref']['path'])['data']
    project, scenario, run = map(load, ('active_project', 'scenario_input', 'run_result'))
    assert project['revision'] == scenario['revision'] == scenario['project_revision'] == 7
    assert len(project['devices']) == 60 and len(project['halls']) == 3
    assert Counter(d['hall_id'] for d in project['devices']) == ({'H1': 20, 'H2': 20, 'H3': 20} if name == 'balanced' else {'H1': 60})
    assert len(project['lines']) == 6 and len(project['connections']) == 120
    assert all(len(l['item_layout']['slots']) == (1 if l['purpose'] == 'buffer' else 500) for l in project['locations'])
    assert run['computed_until_ms'] == 1200000 and run['whole_window_computed']
    assert run['computation_status'] == 'completed'
    initial = {item['id'] for item in scenario['initial_conditions']['items']}
    assert len(initial) == 3000 and {item['id'] for item in run['final_items']} == initial
    accepted = {}
    for event in run['acceptance_events']:
        item_id, = event['item_refs']
        assert item_id not in accepted
        line, serial = item_id.split('-ITEM-')
        assert event['at_ms'] == 20000 + (int(serial) - 1) * 2000
        assert event['destination'] == line + '-SINK'
        accepted[item_id] = event['at_ms']
    assert set(accepted) == initial
    by_time = {str(t): sum(at <= t for at in accepted.values()) for t in (0, 600000, 1200000)}
    assert by_time == {'0': 0, '600000': 1746, '1200000': 3000}
    assert len(run['jobs']) == 30000 and len(run['events']) == 155994
    assert all(j['finished_at_ms'] - j['started_at_ms'] == 2000 for j in run['jobs'])
    assert all(j['operation_ref']['version'] == '1.0.5' for j in run['jobs'])
    assert all(item['holder'] == {'kind': 'location', 'id': item['id'].split('-ITEM-')[0] + '-SINK'} for item in run['final_items'])
    hosted, reopened = (read(root / f'{name}-{suffix}.json') for suffix in ('host', 'reopen'))
    assert hosted['pid'] != reopened['pid'] and reopened['run_id'] == run['id']
    assert reopened['all_location_layouts_retained']
    assert all((saved / d['file_ref']['path']).is_file() for d in manifest['documents'])
    for document in manifest['documents']:
        if document['role'] in ('module_definition', 'asset'):
            relative = document['file_ref']['path']
            assert (saved / relative).read_bytes() == (root / 'inputs' / name / relative).read_bytes()
    results.append({'variant': name, 'run_id': run['id'], 'revision': 7,
        'all3000_input_final_and_accepted_ids_equal': True,
        'all3000_acceptance_times_match_independent_pipeline_formula': True,
        'accepted_by_time_ms': by_time, 'jobs': 30000, 'events': 155994,
        'all_jobs_2000ms': True, 'all_final_holders_match_original_sink': True,
        'all66_location_layouts_retained': True, 'saved_and_new_process_reopened': True,
        'fixed_modules_and_assets_byte_equal_after_save': True,
        'Native_render_acceptance': False, 'formal_NFR': False})
(root / 'verification.json').write_text(json.dumps(results, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(json.dumps(results, ensure_ascii=False))
