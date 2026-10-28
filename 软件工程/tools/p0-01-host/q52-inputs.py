"""Prepare bounded Q52 copies; compare every input byte after actual Native opens."""
import argparse
import copy
import json
import shutil
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--verify', action='store_true')
args = parser.parse_args()
root = Path(__file__).resolve().parents[2]
out = root / '.work/single-window-20261017/p001-q52'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))

if args.verify:
    results = []
    for case in read(out / 'expectations.json')['cases']:
        actual = out / 'inputs' / case['name']
        expected = out / 'expected' / case['name']
        files = {p.relative_to(expected) for p in expected.rglob('*') if p.is_file()}
        assert files == {p.relative_to(actual) for p in actual.rglob('*') if p.is_file()}, case['name']
        assert all((actual / p).read_bytes() == (expected / p).read_bytes() for p in files), case['name']
        results.append({'name': case['name'], 'files_compared': len(files), 'unchanged': True})
    (out / 'input-preservation.json').write_text(json.dumps(results, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(results, ensure_ascii=False))
else:
    assert not out.exists(), 'Existing validation inputs are retained; choose a new run before preparing again.'
    source = root / '.work/whole-packages/p001-20261017/complete-input'
    positive = out / 'inputs/asset-complete'
    shutil.copytree(source, positive)
    manifest = read(positive / 'project.json')
    media = read(positive / 'module-media.json')
    asset_source = root / 'modules/media/p302-controlled/installable/image'
    asset = {'id': 'p001.q52.fixed.png', 'path': 'assets/fixed.png', 'kind': 'png'}
    rights = copy.deepcopy(read(asset_source / 'module.json')['data']['definition']['contribution']['assets'][0])
    rights['asset_ref'] = asset['id']
    media['data']['assets'] = [asset]
    media['data']['rights'] = [rights]
    media['data']['files'] = [{'path': asset['path'], 'kind': 'asset'}]
    media['data']['definition']['contribution']['assets'] = [rights]
    media['data']['definition']['payload']['data']['nodes'][0]['material']['texture'] = asset
    manifest['data']['assets'].append(rights)
    manifest['data']['documents'].append({'role': 'asset', 'module_ref': None, 'object_ref': {'kind': 'asset', 'id': asset['id']}, 'revision': None, 'file_ref': {'path': asset['path'], 'kind': 'asset'}, 'required': True})
    (positive / 'assets').mkdir()
    shutil.copyfile(asset_source / 'station.png', positive / asset['path'])
    for name, value in [('project.json', manifest), ('module-media.json', media)]:
        (positive / name).write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    cases = [
        {'name': 'asset-complete', 'expected': '解析成功并显示明确采用核对；取消后仍是活动C/r8，素材与全部输入不改写。'},
        {'name': 'unknown-schema', 'expected': 'UnsupportedSchema，指出project.json格式未知；当前C/r8及原文件保留。'},
        {'name': 'missing-module', 'expected': 'MissingReference，指出缺失的必需module-devices.json；当前C/r8及剩余原文件保留。'},
        {'name': 'missing-asset', 'expected': 'MissingReference，指出缺失的必需assets/fixed.png；当前C/r8及剩余原文件保留。'},
    ]
    for case in cases[1:]:
        target = out / 'inputs' / case['name']
        shutil.copytree(positive, target)
        if case['name'] == 'unknown-schema':
            value = read(target / 'project.json'); value['schema_version'] = '99.0'
            (target / 'project.json').write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
        else:
            omitted = 'module-devices.json' if case['name'] == 'missing-module' else asset['path']
            omission = (target / omitted).resolve()
            assert omission.is_relative_to((out / 'inputs').resolve())
            omission.unlink()
    for case in cases:
        shutil.copytree(out / 'inputs' / case['name'], out / 'expected' / case['name'])
    (out / 'expectations.json').write_text(json.dumps({'source_ref': str(source.relative_to(root)), 'fixed_asset_source': str((asset_source / 'station.png').relative_to(root)), 'active_project_precondition': 'P1/r8, D1 title Native 搬移重开验证 D1-C, saved and clean', 'cases': cases, 'actual_result': None}, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(str(out))
