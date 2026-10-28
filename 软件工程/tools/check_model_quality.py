"""Inventory repository GLBs and check explicitly selected model deliveries.

This does not judge appearance or approve models. Review evidence is supplied by
the model reviewer after inspecting actual exports and product captures.
"""
import argparse
import json
import struct
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LEDGER = ROOT / 'modules/media/model-quality-ledger.json'
SPEC = ROOT / 'docs/02_交互设计/设备建模精度与表现验收规范.json'
VIEWS = ('isometric', 'front', 'back', 'left', 'right', 'top', 'bottom')
GATES = ('source_export', 'visual', 'platform_reopen')


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def relative(path):
    return path.resolve().relative_to(ROOT).as_posix()


def scoped_path(value):
    path = (ROOT / value).resolve()
    path.relative_to(ROOT)
    return path


def model_files():
    return sorted([*ROOT.joinpath('modules').rglob('*.glb'),
                   *ROOT.joinpath('tools/3d-proof/output').glob('*.glb')])


def observation(path):
    stat = path.stat()
    with path.open('rb') as source:
        header = source.read(20)
        if len(header) != 20 or header[:4] != b'glTF':
            raise ValueError(f'Cannot inventory GLB: {relative(path)}')
        size = struct.unpack_from('<I', header, 12)[0]
        document = json.loads(source.read(size))
    return dict(bytes=stat.st_size, modified_ns=stat.st_mtime_ns,
                generator=document.get('asset', {}).get('generator'),
                nodes=len(document.get('nodes', [])),
                meshes=len(document.get('meshes', [])),
                textures=len(document.get('textures', [])))


def category(path):
    value = relative(path)
    if value.startswith('tools/3d-proof/'):
        return 'supervised_reference'
    if '/p302-controlled/' in value or '/p303-owned/' in value:
        return 'test_fixture'
    if '-fit/' in value:
        return 'diagnostic_only'
    if '-simple/' in value or path.stem.endswith('-simple'):
        return 'simplified_candidate'
    return 'detailed_candidate'


def refresh():
    ledger = read(LEDGER) if LEDGER.exists() else dict(
        document_kind='per_model_quality_ledger', schema_version='1.0',
        policy_ref=relative(SPEC), inventory_roots=['modules', 'tools/3d-proof/output'],
        excluded='Historical scenarios, builds and independent working copies are not new models.',
        acceptance_note='Inventory and geometry counts do not approve visual quality or Native delivery.',
        models=[])
    old = {row['model_path']: row for row in ledger['models']}
    rows = []
    for path in model_files():
        name, observed = relative(path), observation(path)
        row = old.get(name)
        if row is None:
            row = dict(model_path=name, category=category(path),
                       asset_quality='pending', observed=observed, review=None)
        elif row['observed'] != observed:
            row = dict(row, observed=observed, asset_quality='pending')
        rows.append(row)
    # Retain records for removed files: a refresh must not erase an old review.
    for name, row in old.items():
        if not scoped_path(name).exists():
            rows.append(dict(row, asset_quality='missing'))
    ledger['models'] = sorted(rows, key=lambda row: row['model_path'])
    LEDGER.write_text(json.dumps(ledger, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    return ledger


def evidence_exists(values):
    return (isinstance(values, list) and bool(values)
            and all(isinstance(value, str) and scoped_path(value).is_file() for value in values))


def capture_exists(value):
    return (isinstance(value, str) and Path(value).suffix.lower() in ('.png', '.jpg', '.jpeg', '.webp')
            and scoped_path(value).is_file())


def release_errors(row, dimension_ids):
    problems = []
    if row['category'] in ('test_fixture', 'diagnostic_only'):
        problems.append('test/diagnostic asset cannot be an official model delivery')
    if row.get('asset_quality') != 'accepted' or not isinstance(row.get('review'), dict):
        return problems + ['per-model acceptance pending']
    review = row['review']
    expected = {key: row['observed'][key] for key in ('bytes', 'modified_ns')}
    if review.get('reviewed_file') != row['model_path'] or review.get('file_revision') != expected:
        problems.append('review does not identify this observed file revision')
    if not review.get('reviewer') or not review.get('reviewed_at'):
        problems.append('reviewer/date missing')
    if review.get('open_defects') != []:
        problems.append('open defect list is missing or nonempty')
    comparison = review.get('reference_comparison', {})
    if (not evidence_exists(comparison.get('reference_files'))
            or not evidence_exists(comparison.get('evidence')) or not comparison.get('conclusion')):
        problems.append('same-grade reference comparison missing')
    for name in GATES:
        gate = review.get('gates', {}).get(name, {})
        if gate.get('result') != 'pass' or not evidence_exists(gate.get('evidence')):
            problems.append(f'{name}: actual evidence/pass missing')
    dimensions = review.get('dimensions', {})
    for name in dimension_ids:
        check = dimensions.get(name, {})
        if (check.get('result') not in ('pass', 'not_applicable')
                or not check.get('reason') or not evidence_exists(check.get('evidence'))):
            problems.append(f'{name}: review/reason/evidence missing')
    for name in VIEWS:
        if not capture_exists(review.get('views', {}).get(name)):
            problems.append(f'{name}: actual exported-model view missing')
    view_paths = [review.get('views', {}).get(name) for name in VIEWS]
    if len(set(value for value in view_paths if isinstance(value, str))) != len(VIEWS):
        problems.append('required views must use distinct captures, not one repeated image')
    if not review.get('closeups') or not all(capture_exists(value) for value in review['closeups']):
        problems.append('critical mechanism/material close-ups missing')
    if dimensions.get('MQ-06', {}).get('result') != 'not_applicable':
        if not evidence_exists(review.get('motion_evidence')):
            problems.append('applicable motion/wait evidence missing')
    return problems


def selected_models(catalog):
    paths = set()
    for entry in read(scoped_path(catalog))['register_order']:
        directory = scoped_path(entry['directory'])
        document = read(directory / 'module.json')
        package = document.get('data', document)
        identity = package['definition']
        if (identity['id'], identity['version']) != (entry['id'], entry['version']):
            raise ValueError(f'Catalog/module identity mismatch: {entry["directory"]}')
        for asset in package['assets']:
            if asset['path'].lower().endswith('.glb'):
                paths.add(relative(directory / asset['path']))
    return sorted(paths)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--refresh', action='store_true')
    mode.add_argument('--check', action='store_true')
    mode.add_argument('--release-catalog', metavar='JSON')
    args = parser.parse_args()
    ledger = refresh() if args.refresh else read(LEDGER)
    rows = {row['model_path']: row for row in ledger['models']}
    errors = []
    if len(rows) != len(ledger['models']):
        errors.append('Duplicate model records')
    current = {relative(path) for path in model_files()}
    for name in sorted(current - rows.keys()):
        errors.append(f'{name}: not inventoried')
    for name in sorted(current & rows.keys()):
        if observation(scoped_path(name)) != rows[name]['observed']:
            errors.append(f'{name}: changed; refresh and review again')
    for name in sorted(rows.keys() - current):
        errors.append(f'{name}: file missing')
    selected = []
    if args.release_catalog:
        selected = selected_models(args.release_catalog)
        if not selected:
            errors.append('Catalog selects no GLB files; no model acceptance established')
        dimension_ids = [item['id'] for item in read(SPEC)['requirements']]
        for name in selected:
            if name not in rows:
                errors.append(f'{name}: not inventoried')
            else:
                errors.extend(f'{name}: {problem}' for problem in release_errors(rows[name], dimension_ids))
    print(json.dumps(dict(mode='refresh' if args.refresh else 'release' if args.release_catalog else 'inventory',
                          inventory_files=len(current), selected_files=len(selected),
                          categories=dict(Counter(row['category'] for row in rows.values())),
                          accepted_records=sum(row['asset_quality'] == 'accepted' for row in rows.values()),
                          passed=not errors, errors=errors), ensure_ascii=False, indent=2))
    return int(bool(errors))


if __name__ == '__main__':
    raise SystemExit(main())
