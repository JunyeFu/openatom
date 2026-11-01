"""Freeze one real Native UI-versus-stdio concurrency acceptance case."""
import argparse
import json
from pathlib import Path
import subprocess
import sys

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--bridge', type=Path, required=True)
parser.add_argument('--stage', type=Path, required=True)
args = parser.parse_args()
subprocess.run([sys.executable, '-X', 'utf8', str(Path(__file__).with_name('prepare_codex_readonly.py')),
    '--bridge', str(args.bridge), '--stage', str(args.stage)], check=True)
file = args.stage / 'case.json'
case = json.loads(file.read_text(encoding='utf-8'))
case['client'] = 'protocol'
case['authority']['operations'] += ['apply_commands', 'run_scenario', 'get_run', 'get_task', 'save_project']
case['authority']['edit'] = [{'scope': {'kind': 'edit', 'data': {'kind': 'project', 'data': 'P1'}},
    'branches': ['put_object'], 'objects': [{'target': {'kind': 'device', 'id': 'ASM-01'},
    'fields': [{'pointer': '/pose/translation/x', 'element_id': None}], 'can_remove': False}],
    'creates': [], 'source_versions': [], 'reusable_resources': []}]
case['deadline_seconds'] = 300
file.write_text(json.dumps(case, ensure_ascii=False, indent=2), encoding='utf-8')
