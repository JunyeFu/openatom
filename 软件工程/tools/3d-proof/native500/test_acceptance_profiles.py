"""Review-policy unit tests; these fixtures are not Native acceptance evidence."""
import copy
import unittest

from acceptance_profiles import required_conditions, native_conditions, select_native_plan, validate_efficient_batch


def fixture():
    device = {'id': 'sample', 'auxiliary': 'dose'}
    conditions = required_conditions(device)
    behavior = {'passed': True, 'run_id': 'unit-test-run', 'checks': [
        {'id': 'sample', 'conditions': {c: {'passed': True} for c in conditions}}]}
    persistence = {'passed': True, 'ordinary_UI_open_save_new_process_reopen': True,
                   'processes': [{'host_instance_id': p} for p in ('p1', 'p2')],
                   'current_media_checks': [{'module_ref': {'id': 'selected.media', 'version': '1'},
                                             'exact_current_media_preserved': True}],
                   'checks': [{'device': 'sample', 'condition': c, 'process': p,
                               'passed': True, 'holders_verified': True}
                              for c in conditions for p in ('p1', 'p2')]}
    cases = [{'name': 'sample--' + c + ' / state', 'passed': True,
              'visible_objects': [{'kind': 'device', 'id': 'sample--' + c}],
              'scene': {'evidence_origin': 'actual_software',
                        'view': {'run_id': 'unit-test-run'}}} for c in conditions]
    native = {'passed': True, 'source_unchanged': True, 'driver': {'cases': cases}}
    views = {'passed': True, 'checks': [{'name': c['name'], 'passed': True} for c in cases]}
    contacts = {'passed': True, 'critical_layout_checks': [{'id': 'sample', 'passed': True,
                'stages': {s: 'pass' for s in ('input', 'processing', 'output_wait', 'delivered')}}]}
    return [[device], behavior, persistence, views, native, contacts, [('selected.media', '1')]]


class EfficientAcceptanceTests(unittest.TestCase):
    def family_fixture(self):
        args = fixture()
        other = copy.deepcopy(args)
        other[0][0]['id'] = 'variant'
        other[1]['checks'][0]['id'] = 'variant'
        other[5]['critical_layout_checks'][0]['id'] = 'variant'
        for row in other[2]['checks']: row['device'] = 'variant'
        for row in other[3]['checks']: row['name'] = row['name'].replace('sample--', 'variant--')
        for row in other[4]['driver']['cases']:
            row['name'] = row['name'].replace('sample--', 'variant--')
            row['visible_objects'][0]['id'] = row['visible_objects'][0]['id'].replace('sample--', 'variant--')
        args[0] += other[0]
        for device in args[0]: device['rule'] = 'transform'
        for index, key in [(1, 'checks'), (2, 'checks'), (3, 'checks'), (5, 'critical_layout_checks')]:
            args[index][key] += other[index][key]
        args[4]['driver']['cases'] += other[4]['driver']['cases']
        reuse = {'variant': {'representative': 'sample', 'basis': 'shared transform and phase/holder media bindings'}}
        required = native_conditions(args[0], reuse)
        args[4]['driver'] = select_native_plan(args[0], args[4]['driver'], reuse)
        names = {r['name'] for r in args[4]['driver']['cases']}
        args[3]['checks'] = [r for r in args[3]['checks'] if r['name'] in names]
        args[2]['checks'] = [r for r in args[2]['checks'] if (r['device'], r['condition']) in required]
        return args, reuse

    def test_family_reuses_three_waits_but_keeps_own_auxiliary(self):
        args, reuse = self.family_fixture()
        self.assertEqual(len(args[4]['driver']['cases']), 9) # 12 - three shared waits
        validate_efficient_batch(*args, wait_reuse=reuse)
        self.assertIn(('variant', 'missing-auxiliary'), native_conditions(args[0], reuse))

    def test_family_cannot_skip_member_host_or_own_native_output(self):
        for missing in ('host', 'native'):
            args, reuse = self.family_fixture()
            if missing == 'host': del args[1]['checks'][1]['conditions']['interrupted']
            else: args[4]['driver']['cases'] = [r for r in args[4]['driver']['cases'] if not r['name'].startswith('variant--output-wait /')]
            with self.assertRaises(AssertionError): validate_efficient_batch(*args, wait_reuse=reuse)

    def test_family_still_needs_representative_wait_and_new_process_readback(self):
        for missing in ('representative', 'reopen'):
            args, reuse = self.family_fixture()
            if missing == 'representative': args[4]['driver']['cases'] = [r for r in args[4]['driver']['cases'] if not r['name'].startswith('sample--interrupted /')]
            else: args[2]['checks'] = [r for r in args[2]['checks'] if not (r['device'] == 'variant' and r['condition'] == 'normal' and r['process'] == 'p2')]
            with self.assertRaises(AssertionError): validate_efficient_batch(*args, wait_reuse=reuse)

    def test_family_cannot_reuse_another_identity_rule(self):
        args, reuse = self.family_fixture()
        args[0][1]['rule'] = 'preserve'
        with self.assertRaises(AssertionError): validate_efficient_batch(*args, wait_reuse=reuse)

    def test_semantic_coverage_does_not_require_28_pictures(self):
        args = fixture()
        self.assertEqual(len(args[4]['driver']['cases']), 6)
        validate_efficient_batch(*args)

    def test_different_picture_count_is_also_accepted(self):
        args = fixture()
        extra = copy.deepcopy(args[4]['driver']['cases'][0])
        extra['name'] += ' close-up'
        args[4]['driver']['cases'].append(extra)
        args[3]['checks'].append({'name': extra['name'], 'passed': True})
        validate_efficient_batch(*args)

    def test_missing_auxiliary_wait_is_rejected(self):
        args = fixture()
        args[4]['driver']['cases'] = [c for c in args[4]['driver']['cases']
                                     if '--missing-auxiliary /' not in c['name']]
        with self.assertRaises(AssertionError):
            validate_efficient_batch(*args)

    def test_failed_output_layout_is_rejected(self):
        args = fixture()
        args[5]['critical_layout_checks'][0]['stages']['output_wait'] = 'fail'
        with self.assertRaises(AssertionError):
            validate_efficient_batch(*args)

    def test_other_run_picture_is_rejected(self):
        args = fixture()
        args[4]['driver']['cases'][0]['scene']['view']['run_id'] = 'another-run'
        with self.assertRaises(AssertionError):
            validate_efficient_batch(*args)

    def test_other_device_picture_is_rejected(self):
        args = fixture()
        args[4]['driver']['cases'][0]['visible_objects'][0]['id'] = 'another-device'
        with self.assertRaises(AssertionError):
            validate_efficient_batch(*args)

    def test_missing_holder_readback_is_rejected(self):
        args = fixture()
        args[2]['checks'][0]['holders_verified'] = False
        with self.assertRaises(AssertionError):
            validate_efficient_batch(*args)

    def test_unchecked_saved_asset_is_rejected(self):
        args = fixture()
        args[6].append(('another-selected.media', '1'))
        with self.assertRaises(AssertionError):
            validate_efficient_batch(*args)

    def test_same_process_reopen_is_rejected(self):
        args = fixture()
        args[2]['processes'] = [{'host_instance_id': 'p1'}] * 2
        with self.assertRaises(AssertionError):
            validate_efficient_batch(*args)


if __name__ == '__main__':
    unittest.main()
