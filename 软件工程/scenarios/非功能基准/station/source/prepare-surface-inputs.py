"""Create separate B60 1.0.4 projects from preserved 1.0.3 input directories."""
import argparse
import json
from pathlib import Path
from shutil import copyfile

def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))

def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')

def replace(value, old_id, new_id):
    if isinstance(value, dict):
        if value.get('id') in ['p201.b60.station', 'p201.b60.simple'] and value.get('version') == '1.0.3':
            value['version'] = '1.0.4'
        for key, child in value.items():
            if child == old_id:
                value[key] = new_id
            else:
                replace(child, old_id, new_id)
    elif isinstance(value, list):
        for i, child in enumerate(value):
            if child == old_id:
                value[i] = new_id
            else:
                replace(child, old_id, new_id)

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--input-root', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
candidate = Path(__file__).resolve().parent.parent / 'candidate-1.0.4'
records = []
for variant in ['balanced', 'dense']:
    source, target = args.input_root / variant, args.output / variant
    target.mkdir(parents=True, exist_ok=False)
    manifest = read(source / 'project.json')
    active = read(source / 'data/g000001/project-state.json')
    scenario = read(source / 'data/g000001/scenario-0001.json')
    old_id = active['data']['id']
    new_id = old_id.removesuffix('-MQ-v3') + '-MQ-v4'
    for value in [manifest, active, scenario]:
        replace(value, old_id, new_id)
    active['data'].update(revision=5, title=f'NFR-B60 · {variant} · MQ-v4')
    scenario['data'].update(revision=5, project_revision=5)
    manifest['data'].update(project_revision=5, title=active['data']['title'])
    for entry in manifest['data']['documents']:
        if entry['role'] in ['active_project', 'scenario_input']:
            entry['revision'] = 5
    detailed = read(candidate / 'station.module-document.json')
    simplified = read(candidate / 'simple.module-document.json')
    for i, module in enumerate([detailed, simplified, read(source / 'module-2.json'), read(source / 'module-3.json')]):
        write(target / f'module-{i}.json', module)
    asset = detailed['data']['assets'][0]
    manifest['data']['documents'] = [entry for entry in manifest['data']['documents'] if entry['role'] != 'asset']
    manifest['data']['documents'].append(dict(role='asset', module_ref=None,
        object_ref=dict(kind='asset', id=asset['id']), revision=None,
        file_ref=dict(path=asset['path'], kind='asset'), required=True))
    manifest['data']['assets'] = detailed['data']['rights']
    asset_target = target / asset['path']
    asset_target.parent.mkdir(parents=True, exist_ok=True)
    copyfile(candidate / asset['path'], asset_target)
    write(target / 'project.json', manifest)
    write(target / 'data/g000001/project-state.json', active)
    write(target / 'data/g000001/scenario-0001.json', scenario)
    assert len(active['data']['devices']) == 60
    assert len(scenario['data']['initial_conditions']['items']) == 3000
    assert not any(e['role'] == 'run_result' for e in manifest['data']['documents'])
    assert all((target / e['file_ref']['path']).is_file() for e in manifest['data']['documents'])
    records.append(dict(variant=variant, directory=str(target.resolve()), project_id=new_id,
        revision=5, devices=60, initial_items=3000, files=8, included_GLb_bytes=asset_target.stat().st_size,
        asset_rights=manifest['data']['assets'], old_Run_or_project_modified=False,
        Native_opened=False, new_Run_created=False, benchmark_ready=False))
write(args.output / 'preparation.json', dict(inputs=records, whole_task_done=False, formal_NFR_acceptance=False))
print(json.dumps(records, ensure_ascii=False))
