"""rustc-only link against the explicit P001-approved Commands frontier."""
import argparse
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser()
parser.add_argument('--frontier', type=Path, required=True)
parser.add_argument('--output-name', default='current-core')
parser.add_argument('--optimize', action='store_true')
parser.add_argument('--step-probe', action='store_true')
args = parser.parse_args()
frontier = json.loads(args.frontier.read_text(encoding='utf-8-sig'))
core = Path(frontier['sf_commands'])
dependencies = Path(frontier['dependency_directory'])
out = ROOT / '.work/p201-whole-20261017'
command = ['C:/Users/fujunye/.cargo/bin/rustc.exe', '--edition', '2024', str(ROOT / 'tests/planner/p201-whole/core_consumer.rs'), '-L', 'dependency=' + str(dependencies)]
if args.optimize:
    command += ['-O']
for crate in ('sf_commands', 'sf_domain', 'serde_json'):
    command += ['--extern', crate + '=' + frontier[crate]]
if args.step_probe:
    command += ['--cfg', 'p201_step_probe', '--extern', 'sf_simulation=' + frontier['sf_simulation']]
command += ['-o', str(out / (args.output_name + '.exe'))]
result = subprocess.run(command, capture_output=True, text=True, encoding='utf-8')
(out / (args.output_name + '-build.stdout.txt')).write_text(result.stdout, encoding='utf-8')
(out / (args.output_name + '-build.stderr.txt')).write_text(result.stderr, encoding='utf-8')
(out / (args.output_name + '-build.json')).write_text(json.dumps({'command': command, 'exit_code': result.returncode, 'commands_mtime': core.stat().st_mtime, 'Cargo_used': False, 'frontier': frontier, 'frontier_ref': str(args.frontier)}, ensure_ascii=False, indent=2), encoding='utf-8')
print(json.dumps({'exit_code': result.returncode, 'executable': str(out / (args.output_name + '.exe'))}, ensure_ascii=False))
raise SystemExit(result.returncode)
