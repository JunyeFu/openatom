"""Build the actual cloud crate without including unrelated workspaces or secrets."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[3]


def toml_value(value):
    if isinstance(value, dict):
        return '{ ' + ', '.join(f'{key} = {toml_value(item)}' for key, item in value.items()) + ' }'
    return json.dumps(value, ensure_ascii=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True, help='new task directory outside C: on Windows')
    parser.add_argument('--tag', default='sf-cloud-service:sealos-20261027')
    args = parser.parse_args()
    output = args.output.resolve()
    if __import__('os').name == 'nt' and output.drive.upper() == 'C:':
        parser.error('use the project temporary directory on D: for new contexts/cache')
    output.mkdir(parents=True, exist_ok=True)
    context = output / 'context'
    context.mkdir(exist_ok=False)
    root_manifest = tomllib.loads((ROOT / 'Cargo.toml').read_text(encoding='utf-8'))
    domain_manifest = tomllib.loads((ROOT / 'packages/domain/Cargo.toml').read_text(encoding='utf-8'))
    lines = ['[workspace]', 'resolver = "3"', 'members = ["packages/domain"]', '', '[workspace.package]']
    for key, value in root_manifest['workspace']['package'].items():
        lines.append(f'{key} = {toml_value(value)}')
    lines += ['', '[workspace.dependencies]']
    for key, value in domain_manifest['dependencies'].items():
        if value.get('workspace'):
            lines.append(f'{key} = {toml_value(root_manifest["workspace"]["dependencies"][key])}')
    (context / 'Cargo.toml').write_text('\n'.join(lines) + '\n', encoding='utf-8')
    copied = []
    for relative in ['packages/domain/Cargo.toml', 'packages/domain/src',
                     'apps/cloud-service/Cargo.toml', 'apps/cloud-service/Cargo.lock',
                     'apps/cloud-service/src', 'apps/cloud-service/migrations',
                     'apps/cloud-service/Dockerfile']:
        source, destination = ROOT / relative, context / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        if source.is_dir():
            shutil.copytree(source, destination)
        else:
            shutil.copyfile(source, destination)
        copied.append(relative)
    files = sorted(p.relative_to(context).as_posix() for p in context.rglob('*') if p.is_file())
    (output / 'context-inputs.json').write_text(json.dumps({
        'source_root': str(ROOT), 'copied': copied, 'files': files,
        'bytes': sum(p.stat().st_size for p in context.rglob('*') if p.is_file()),
        'workspace': 'generated minimal root: domain inherits exact current package/dependency values; cloud manifest and lock unchanged',
        'cache_policy': 'reuse the existing builder cache; do not export a second unconsumed local cache',
    }, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    command = ['docker', 'buildx', 'build', '--platform', 'linux/amd64', '--load',
               '--progress', 'plain',
               '--tag', args.tag, '--file', str(context / 'apps/cloud-service/Dockerfile'), str(context)]
    (output / 'build-command.json').write_text(json.dumps(command, indent=2) + '\n', encoding='utf-8')
    with (output / 'build.log').open('w', encoding='utf-8') as log:
        result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT)
    print(json.dumps({'exit_code': result.returncode, 'output': str(output), 'log': str(output / 'build.log')}))
    raise SystemExit(result.returncode)


if __name__ == '__main__':
    main()
