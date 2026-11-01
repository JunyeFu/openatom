"""Internal batch review checks; never creates a Native or visual result."""


def required_conditions(equipment):
    conditions = {'normal', 'missing', 'phase-wait', 'interrupted', 'output-wait'}
    if equipment.get('auxiliary'):
        conditions.add('missing-auxiliary')
    return conditions | set(equipment.get('acceptance_conditions', []))


def native_conditions(equipment, wait_reuse=None):
    """Keep every device's normal/output and special cases; reuse three shared waits."""
    devices = {d['id']: d for d in equipment}
    reuse = wait_reuse or {}
    shared_waits = {'missing', 'phase-wait', 'interrupted'}
    for member, row in reuse.items():
        representative = row['representative']
        assert member in devices and representative in devices, 'unknown Native representative/member'
        assert member != representative and representative not in reuse, 'Native reuse cannot chain'
        assert isinstance(row.get('basis'), str) and row['basis'].strip(), 'name the shared behavior and phase/anchor bindings'
        assert devices[member]['rule'] == devices[representative]['rule'], 'different identity rules need their own Native waits'
    return {(d['id'], c) for d in equipment for c in required_conditions(d)
            if d['id'] not in reuse or c not in shared_waits}


def select_native_plan(equipment, plan, wait_reuse=None):
    required = native_conditions(equipment, wait_reuse)
    def pair(case):
        return tuple(case['name'].split(' / ')[0].rsplit('--', 1))
    cases = [c for c in plan['cases'] if pair(c) in required]
    assert required <= {pair(c) for c in cases}, 'Native plan is missing required device/conditions'
    return {**plan, 'cases': cases}


def validate_efficient_batch(equipment, behavior, persistence, render_checks,
                             native, contacts, expected_media, wait_reuse=None):
    native_required = native_conditions(equipment, wait_reuse)
    for report in (behavior, persistence, render_checks, native, contacts):
        assert report['passed'], 'required batch result failed or pending'
    assert native['source_unchanged'], 'viewing changed the production source'
    assert persistence['ordinary_UI_open_save_new_process_reopen']
    processes = {p['host_instance_id'] for p in persistence['processes']}
    assert len(processes) == 2, 'save/reopen needs two distinct real processes'
    saved = persistence['current_media_checks']
    assert saved and all(c['exact_current_media_preserved'] for c in saved)
    saved_refs = {(c['module_ref']['id'], c['module_ref']['version']) for c in saved}
    assert set(expected_media) <= saved_refs, 'a selected current asset was not checked'

    host_rows = {row['id']: row for row in behavior['checks']}
    layout_rows = {row['id']: row for row in contacts['critical_layout_checks']}
    verified_views = {row['name'] for row in render_checks['checks'] if row['passed']}
    views = native['driver']['cases']
    for view in views:
        assert view['passed'] and view['name'] in verified_views
        assert view['scene']['evidence_origin'] == 'actual_software'
        assert view['scene']['view']['run_id'] == behavior['run_id']
        instance = view['name'].split(' / ')[0]
        assert any(o['kind'] == 'device' and o['id'] == instance
                   for o in view['visible_objects']), 'capture shows another device'
    held = {(c['device'], c['condition'], c['process'])
            for c in persistence['checks'] if c['passed'] and c['holders_verified']}

    for device in equipment:
        key = device['id']
        conditions = required_conditions(device)
        assert conditions <= host_rows[key]['conditions'].keys(), key + ': Host coverage missing'
        assert all(host_rows[key]['conditions'][c]['passed'] for c in conditions)
        for condition in conditions:
            if (key, condition) not in native_required:
                continue # Host still checks this device; Native uses its declared representative.
            prefix = key + '--' + condition + ' / '
            assert any(v['name'].startswith(prefix) for v in views), prefix + 'Native state missing'
            assert all((key, condition, process) in held for process in processes), prefix + 'holder/recovery readback missing'
        layout = layout_rows[key]
        assert layout['passed'], key + ': critical layout failed'
        for stage in ('input', 'processing', 'output_wait', 'delivered'):
            result = layout['stages'][stage]
            assert result == 'pass' or (isinstance(result, dict)
                   and result.get('result') == 'not_applicable' and result.get('reason')), key + ': ' + stage


if __name__ == '__main__':
    # Select from a real Host-generated plan; do not change the Run or full plan.
    from pathlib import Path
    import json
    import sys
    root = Path(__file__).resolve().parents[3]
    review = root / 'modules/media/native-equipment-500/reviews' / sys.argv[1]
    read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
    config = read(review / 'acceptance-input.json')
    host = root / config['host']
    plan = select_native_plan(read(review / 'catalog.json')['equipment'],
                              read(host / 'native-plan.json'), config.get('native_wait_reuse'))
    target = host / 'native-plan-selected.json'
    assert not target.exists(), 'use a new selected plan after a revision; preserve old evidence'
    target.write_text(json.dumps(plan, ensure_ascii=False, indent=2) + '\n', encoding='utf8')
    print(f'{len(plan["cases"])} Native views selected; full Host coverage unchanged: {target}')
