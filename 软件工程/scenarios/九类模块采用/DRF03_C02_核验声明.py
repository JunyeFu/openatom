"""核对实际拥有者包装定义的声明、指针和显式实例选择；不调用Host。"""
from copy import deepcopy
from pathlib import Path
import json
import sys

sys.dont_write_bytecode = True
BASE = Path(__file__).resolve().parent
ROOT = BASE.parents[1]
sys.path.insert(0, str(ROOT/'tools/type-contract-proof/python-libs'))
from jsonschema import Draft202012Validator


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def check(definition, config):
    required = {'packaging_job_id': 'string', 'packaging_job_revision': 'integer'}
    for key, kind in required.items():
        declarations = [p for p in definition['parameters'] if p['id'] == key]
        if not declarations:
            return 'missing_declaration'
        if len(declarations) != 1:
            return 'duplicate_declaration'
        param = declarations[0]
        if param['value_type'] != kind:
            return 'wrong_declared_type'
        if param['binding'] != f'/payload/data/configuration/{key}/value':
            return 'wrong_binding'
    for key, kind in required.items():
        if key not in config:
            return 'missing_explicit_selection'
        value = config[key]
        if value['kind'] != kind:
            return 'wrong_selected_type'
    if not config['packaging_job_id']['value']:
        return 'missing_explicit_selection'
    return 'explicit_declared_selection_only_not_binding_acceptance'


def main():
    source = read(ROOT/'packages/packaging/consumer-device.design.json')
    Draft202012Validator(read(ROOT/'tools/type-contract-proof/output/schemas/module.schema.json')).validate(source)
    definition = source['data']['definition']
    assert definition['id'] == 'community.parameterized-pack' and definition['version'] == '0.2.0'
    assert definition['payload']['data']['configuration'] == {}
    request = read(BASE/'DRF03_C02_显式工位作业选择.request.json')
    Draft202012Validator(read(ROOT/'tools/type-contract-proof/output/api-schemas/apply_commands.request.schema.json')).validate(request)
    selected = request['params']['commands'][0]['data']['data']['configuration']
    results = []
    for name, mutation, expected in [
        ('declared_selected', None, 'explicit_declared_selection_only_not_binding_acceptance'),
        ('unselected', 'unselected', 'missing_explicit_selection'),
        ('key_without_declaration', 'missing', 'missing_declaration'),
        ('duplicate', 'duplicate', 'duplicate_declaration'),
        ('wrong_declared_kind', 'type', 'wrong_declared_type'),
        ('wrong_pointer', 'binding', 'wrong_binding'),
        ('wrong_selected_kind', 'selected_type', 'wrong_selected_type'),
    ]:
        draft, config = deepcopy(definition), deepcopy(selected)
        parameter = next(p for p in draft['parameters'] if p['id'] == 'packaging_job_id')
        if mutation == 'unselected':
            config = {}
        elif mutation == 'missing':
            draft['parameters'].remove(parameter)
        elif mutation == 'duplicate':
            draft['parameters'].append(deepcopy(parameter))
        elif mutation == 'type':
            parameter['value_type'] = 'number'
        elif mutation == 'binding':
            parameter['binding'] = '/id'
        elif mutation == 'selected_type':
            config['packaging_job_id'] = {'kind': 'integer', 'value': 1}
        actual = check(draft, config)
        results.append({'id': name, 'expected': expected, 'actual_reference': actual, 'matched': actual == expected})
    assert all(c['matched'] for c in results)
    registry = read(ROOT/'packages/module-kit/category-contracts.design.json')
    binding = read(ROOT/'packages/domain/professional-artifact-binding.design.json')
    assert registry['packaging_consumer_device']['canonical_definition_ref'] == 'packages/packaging/consumer-device.design.json'
    assert binding['consumer_definition_contract_ref'] == registry['packaging_consumer_device']['canonical_definition_ref']
    declarations = [p for p in definition['parameters'] if p['id'].startswith('packaging_job_')]
    assert registry['packaging_consumer_device']['actual_parameter_declarations'] == declarations
    assert binding['required_declared_selection_parameters'] == declarations
    original = read(ROOT/'modules/devices/parameterized-pack/manifest.json')
    assert original['version'] == '0.1.0' and len(read(ROOT/'modules/catalog.json')['modules']) == 21
    result = {'document_kind': 'actual_owned_consumer_definition_static_and_finite_checks_not_Host', 'cases': results,
              'case_count': len(results), 'all_matched': True, 'module_schema': 'passed', 'selection_request_schema': 'passed',
              'actual_declaration_source': 'packages/packaging/consumer-device.design.json', 'implicit_default_Job_created': False,
              'registered_or_existing_versions_changed': False, 'actual_Host': None, 'actual_binding_or_product_result': None}
    (BASE/'DRF03_C02_声明核验结果.json').write_text(json.dumps(result, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
    print(json.dumps({key: result[key] for key in ['case_count', 'all_matched', 'module_schema', 'selection_request_schema', 'implicit_default_Job_created', 'actual_Host']}, ensure_ascii=False))


if __name__ == '__main__':
    main()
