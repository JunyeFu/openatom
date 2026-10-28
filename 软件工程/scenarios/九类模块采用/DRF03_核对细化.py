"""DRF03现行声明/Schema与有限业务参考；只写本目录结果，非产品加载器或Host。"""
from copy import deepcopy
from fractions import Fraction
from pathlib import Path
import json
import sys

sys.dont_write_bytecode = True
BASE = Path(__file__).resolve().parent
ROOT = BASE.parents[1]
sys.path.insert(0, str(ROOT / 'tools/type-contract-proof/python-libs'))
from jsonschema import Draft202012Validator


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def aggregate(required, checks):
    if any(checks.get(purpose) == 'Fail' for purpose in required):
        return 'Fail'
    if not required or any(checks.get(purpose) != 'Pass' for purpose in required):
        return 'Undetermined'
    return 'Pass'


def binding_gate(ctx):
    if ctx['mode'] not in ctx['provider_modes']:
        return 'blocked_unsupported_mode'
    if ctx['artifact_origin'] != 'actual_software':
        return 'blocked_non_actual_origin'
    if ctx['current_subject'] != ctx['artifact_subject']:
        return 'blocked_subject_revision'
    if ctx['current_thickness_mm'] != ctx['artifact_thickness_mm']:
        return 'blocked_input_conditions'
    checks = deepcopy(ctx['checks'])
    if ctx['glue_connections'] and not ctx['has_actual_glue_region']:
        checks['manufacturing'] = 'Undetermined'
    outcome = aggregate(ctx['required'], checks)
    if outcome != 'Pass':
        return 'blocked_' + outcome
    if not set(ctx['required']) <= set(ctx['artifact_scope']) & set(ctx['ack_scope']):
        return 'blocked_scope'
    if not set(ctx['required_payload']) <= set(ctx['payload']):
        return 'blocked_Undetermined'
    if ctx['ack_artifact'] != ctx['artifact_id']:
        return 'blocked_ack_artifact'
    if ctx['ack_subject'] != ctx['current_subject']:
        return 'blocked_ack_subject'
    return 'binding_eligible_finite_model_only'


def reference(case, defaults):
    kind, inp = case['kind'], case['input']
    if kind == 'binding':
        ctx = deepcopy(defaults)
        ctx.update(inp)
        return binding_gate(ctx)
    if kind == 'acknowledgment':
        return 'PermissionDenied' if inp['caller'] == 'external_agent' and inp['creates_human_statement'] else 'human_channel_required'
    if kind == 'display_version':
        old_run = {'media_version': inp['old_run_media_version']}
        current = {'media_version': inp['new_media_version']}
        return {'old_run_media_version': old_run['media_version'], 'current_media_version': current['media_version'], 'stock': inp['stock'], 'professional_recompute': False}
    if kind == 'professional_change':
        assert inp['change'] == 'material_thickness'
        return {'new_job_revision': inp['old_job_revision'] + 1, 'new_artifact_required': True, 'copy_acknowledgment': False, 'old_job_revision': inp['old_job_revision']}
    if kind == 'adoption':
        state = {'revision': inp['current_revision'], 'instances': []}
        before = deepcopy(state)
        if inp['base_revision'] != state['revision']:
            outcome = 'RevisionConflict'
        elif inp['source_unit'] != inp['target_unit']:
            outcome = 'InvalidInput'
        else:
            raise AssertionError('This finite adoption fixture covers two rejected atomic attempts only.')
        assert state == before
        return {'result': outcome, 'project_revision': state['revision'], 'created_instances': len(state['instances'])}
    if kind == 'template_refs':
        mapping = {key: inp['namespace'] + ':' + key for key in inp['ids']}
        return {'ids': [mapping[key] for key in inp['ids']], 'links': [[mapping[a], mapping[b]] for a, b in inp['links']], 'external_resource': inp['external_resource'], 'stock_created': 0}
    if kind == 'export_rights':
        missing = [asset['path'] for asset in inp['assets'] if not asset['redistribution_confirmed']]
        return {'redistributable_export_ready': not missing, 'missing_rights': missing}
    if kind == 'duration':
        value = Fraction(inp['value_seconds']) * 1000
        return int(value) if value.denominator == 1 and 0 <= value <= 2**53 - 1 else 'InvalidInput_non_integer_ms'
    if kind == 'media_transform':
        x, y, z = inp['source_m_y_up']
        return {'job_mm_z_up': [1000*x, -1000*z, 1000*y], 'physical_width_mm': inp['physical_width_mm']}
    if kind == 'timeline_progress':
        work = sum(max(0, min(end, inp['run_at_ms']) - start) for start, end in inp['working_intervals'])
        return {'run_at_ms': inp['run_at_ms'], 'effective_work_ms': work, 'phase_progress': str(Fraction(work, inp['required_work_ms'])), 'packaging_duration_hint_ms': inp['packaging_duration_hint_ms'], 'factory_duration_ms': inp['required_work_ms']}
    if kind == 'process_identity':
        return 'unmapped_no_index_guess' if not inp['named_pose_has_step_id'] or not inp['named_pose_has_panel_id'] else 'explicit_mapping_required'
    if kind == 'aggregate':
        return aggregate(inp['required'], inp['checks'])
    raise AssertionError(kind)


def main():
    registry = read(ROOT / 'packages/module-kit/category-contracts.design.json')
    binding = read(ROOT / 'packages/domain/module-field-bindings.design.json')
    schema = read(ROOT / 'tools/type-contract-proof/output/schemas/all-types.schema.json')
    definitions = schema['$defs']
    checked_fields = 0
    categories = {cat['id']: cat for cat in registry['categories']}
    required = {'canonical_carriers', 'unit_rule', 'validation_rule', 'reference_rule', 'instantiation_rule', 'sample_ref', 'canonical_shape_fixture_ref', 'version_rule'}
    for cat in categories.values():
        actual_bindings = {field['logical_path']: field['carriers'] for field in next(row for row in binding['categories'] if row['id'] == cat['id'])['bindings']}
        for field in cat['field_contracts']:
            assert required <= field.keys() and all(field[key] for key in required)
            assert field['canonical_carriers'] == actual_bindings[field['path']]
            for carrier in field['canonical_carriers']:
                properties = definitions[carrier['rust_type']].get('properties', {})
                for name in carrier.get('fields', []):
                    assert name in properties, (cat['id'], field['path'], carrier, name)
            assert (ROOT / field['sample_ref']).exists()
            assert (ROOT / field['canonical_shape_fixture_ref']).exists()
            checked_fields += 1
    assert checked_fields == 61 and len(registry['common_field_contracts']) == 10
    for field in registry['common_field_contracts']:
        assert field['canonical_carriers'] and field['unit_rule'] and field['validation_rule']
        for carrier in field['canonical_carriers']:
            for name in carrier.get('fields', []):
                assert name in definitions[carrier['rust_type']]['properties']
    module_schema = read(ROOT / 'tools/type-contract-proof/output/schemas/module.schema.json')
    validator = Draft202012Validator(module_schema)
    shape_results = []
    for cat in categories.values():
        typed = read(ROOT / f'tools/type-contract-proof/fixtures/module-{cat["id"]}.json')
        validator.validate(typed)
        assert typed['data']['definition']['payload']['category'] == cat['id']
        legacy = read(ROOT / 'modules' / cat['example_manifest'])
        legacy_rejected = bool(list(validator.iter_errors(legacy)))
        assert legacy_rejected  # This does not prove an adapter was implemented.
        shape_results.append({'category': cat['id'], 'canonical_fixture_shape': 'admitted', 'legacy_flat_design_direct_admission': 'rejected_adapter_required'})
    old = read(BASE / 'expected-category-adoption.design.json')
    assert len(old['cases']) == 15 and all(case['actual_result'] is None for case in old['cases'])
    catalog = read(ROOT / 'modules/catalog.json')
    assert len(catalog['modules']) == 21 and {row['category'] for row in catalog['modules']} == set(categories)
    fixtures = read(BASE / 'DRF03_有限独立预期.json')
    results = []
    for case in fixtures['cases']:
        actual = reference(case, fixtures['binding_model_defaults'])
        assert case['actual_product_result'] is None
        results.append({'id': case['id'], 'expected': case['expected'], 'actual_finite_reference': actual, 'matched': actual == case['expected'], 'actual_product_result': None})
    previous = read(ROOT / 'tools/packaging-algorithm-proof/output/proof-result.json')
    hold = read(ROOT / 'tools/packaging-algorithm-proof/output/fixed-pose-result.json')
    assert previous['case_count'] == 32 and previous['all_expected_matched']
    assert hold['case_count'] == 6 and hold['all_expected_matched']
    named_pose_fields = list(definitions['NamedPose']['properties'])
    process_carrier = definitions['PackagingResult']['properties']['process_poses']['items']['$ref'].rsplit('/', 1)[-1]
    process_identity_gap = process_carrier == 'NamedPose'
    result = {'document_kind': 'DRF03_actual_static_and_isolated_reference_checks_not_Host_or_product',
              'status': 'passed_owned_static_and_finite_reference_with_public_handoffs_pending',
              'category_count': len(categories), 'payload_fields_checked_against_actual_generated_schema': checked_fields,
              'common_fields_checked': 10, 'category_shape_cases': shape_results, 'shape_checks': len(shape_results)*2,
              'finite_reference_cases': results, 'finite_reference_count': len(results), 'all_independent_expectations_matched': all(case['matched'] for case in results),
              'preserved_catalog_entries': len(catalog['modules']), 'preserved_original_expectations': len(old['cases']),
              'S06_04_existing_outputs_read_only': {'geometry_cases': previous['case_count'], 'held_pose_cases': hold['case_count'], 'outputs_rerun_or_rewritten': False},
              'observed_NamedPose_fields': named_pose_fields, 'current_process_carrier': process_carrier,
              'process_identity_gap_still_observed': process_identity_gap,
              'legacy_R19_meaning': 'Old NamedPose remains unmapped and must be rejected as a process sample; it is not the current canonical process carrier after C03 merge.',
              'shared_schema_source': 'tools/type-contract-proof/output/schemas/all-types.schema.json',
              'actual_Host': None, 'actual_product_runtime': None, 'actual_prototype': None, 'human_walkthrough': None,
              'actual_result': None, 'dependencies_installed': False, 'shared_files_written': False}
    (BASE / 'DRF03_静态与有限参考结果.json').write_text(json.dumps(result, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
    print(json.dumps({key: result[key] for key in ['category_count', 'payload_fields_checked_against_actual_generated_schema', 'common_fields_checked', 'shape_checks', 'finite_reference_count', 'all_independent_expectations_matched', 'process_identity_gap_still_observed', 'actual_product_runtime']}, ensure_ascii=False))
    if not result['all_independent_expectations_matched']:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
