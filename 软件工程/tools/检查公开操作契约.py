"""S06-03 typed API/schema/finite protocol proof; never product Host/API execution."""
from pathlib import Path
import json
import re
import sys

S = Path(__file__).resolve().parent.parent
LAB = S / 'tools/type-contract-proof'
A = S / 'docs/04_开发与验收'
sys.path.insert(0, str(LAB / 'python-libs'))
from jsonschema import Draft202012Validator


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def verify():
    spec = read(S / 'docs/03_架构与契约/公开操作与宿主调用契约.json')
    catalog = read(S / spec['catalog_ref'])
    operations = spec['operations']
    names = [r['operation'] for r in operations]
    assert names == [r['operation'] for r in catalog['operation_catalog']]
    assert len(set(names)) == 18
    available = [r['operation'] for r in operations if r['runtime_available']]
    source = (S / spec['rust_API_source_ref']).read_text(encoding='utf-8')
    assert 'serde_json::Value' not in source
    trait = source.split('pub trait CommandsService {', 1)[1].split('\n}', 1)[0]
    assert re.findall(r'fn (\w+)\(', trait) == names
    for row in operations:
        directory_row = next(r for r in catalog['operation_catalog'] if r['operation'] == row['operation'])
        assert directory_row['output_type'] == row['admission_output']
        assert directory_row['completed_task_output_type'] == row['completed_task_output']
        method = re.search(r'fn ' + row['operation'] + r'\((.*?)\)\s*->\s*Result<([^,]+), ApiError>', trait, re.S)
        assert method and '&HostCallContext' in method[1] and 'ResolvedCallScope' in method[1]
        assert 'params: ' + row['params_type'] in method[1]
        assert method[2] == row['admission_output']
        assert row['runtime_available'] == directory_row['runtime_available']
        assert row['implementation_status'] == directory_row['implementation_status']
        if row['runtime_available']:
            assert row['implementation_status'] == 'implemented_minimal_scope'
            assert row['implementation_ref'] == directory_row['implementation_ref']
            execution = read(S / row['implementation_ref'])
            assert execution['status'] == 'implemented_and_reviewed'
            assert row['operation'] in execution['actual_result']['implemented_operations']
            assert execution['actual_result']['tests']['failed'] == 0
        else:
            assert row['implementation_status'] == 'planned'
    schemas = {}
    fixtures = []
    refs = 0
    for row in operations:
        for kind in ['request', 'response']:
            name = row['operation'] + '.' + kind
            schema = read(S / row[kind + '_schema_ref'])
            Draft202012Validator.check_schema(schema)
            assert schema['type'] == 'object'
            if kind == 'request':
                assert set(schema['required']) == {'api_version', 'request_id', 'project_id', 'scope', 'operation', 'params'}
                operation_tag = schema['properties']['operation']
                if '$ref' in operation_tag:
                    operation_tag = schema['$defs'][operation_tag['$ref'].split('/')[-1]]
                assert operation_tag['enum'] == [row['operation']]
            schemas[name] = schema
            fixture = read(LAB / 'api-fixtures' / (name + '.json'))
            errors = list(Draft202012Validator(schema).iter_errors(fixture))
            assert not errors, (name, [e.message for e in errors])
            assert fixture == read(LAB / 'output/api-roundtrips' / (name + '.json'))
            fixtures.append(name)
            def walk(n):
                nonlocal refs
                if isinstance(n, dict):
                    if '$ref' in n:
                        ref = n['$ref']
                        assert ref.startswith('#/')
                        target = schema
                        for token in ref[2:].split('/'):
                            target = target[token.replace('~1', '/').replace('~0', '~')]
                        refs += 1
                    for value in n.values():
                        walk(value)
                elif isinstance(n, list):
                    for value in n:
                        walk(value)
            walk(schema)
    assert len(schemas) == len(fixtures) == 36
    union = read(LAB / 'output/schemas/api-request.schema.json')
    Draft202012Validator.check_schema(union)
    assert len(union['anyOf']) == 18
    parse_cases = read(LAB / 'api-parse-cases.design.json')['cases']
    native_parse = read(LAB / 'output/api-parse-proof-result.json')
    assert native_parse['status'] == 'passed_actual_native_request_parse_cases'
    assert len(parse_cases) == len(native_parse['cases']) == 28
    shape_results = []
    for expected, native in zip(parse_cases, native_parse['cases']):
        accepted = not list(Draft202012Validator(union).iter_errors(expected['request']))
        assert native['id'] == expected['id']
        assert accepted == native['accepted'] == expected['expected_accepted'], expected['id']
        shape_results.append({'id': expected['id'], 'native_and_schema_accepted': accepted})
    policy_expected = read(LAB / 'api-admission-cases.design.json')['cases']
    policy = read(LAB / 'output/api-policy-proof-result.json')
    assert len(policy_expected) == len(policy['policy_cases']) == 26
    for expected, actual in zip(policy_expected, policy['policy_cases']):
        assert expected['id'] == actual['id'] and expected['expected'] == actual['expected'] == actual['actual_model_outcome']
    assert policy['byte_framing_cases'] == 5
    assert policy['real_Windows_peer_or_Host_authorization'] is False
    domain = read(S / spec['type_format_spec_ref'])
    assert spec['domain_source_ref'] == domain['rust_type_source_ref']
    source_types = (S / spec['domain_source_ref']).read_text(encoding='utf-8')
    assert 'pub enum TaskScope' in source_types and 'pub selected_scope: Option<EditScope>' in source_types
    assert spec['edit_contract']['command_variants'] == 15
    assert spec['edit_contract']['editable_project_object_variants'] == 12
    assert spec['api_version'] == catalog['api_design_version'] == '0.1'
    assert spec['project_file_version'] == '1.0' and spec['IPC_wire_version'] == '1'
    assert spec['implemented_public_operations'] == len(available)
    if available:
        assert spec['actual_product_API_execution']['implemented_operations'] == available
    else:
        assert spec['actual_product_API_execution'] is None
    selection = read(S / 'docs/00_项目基线/首版任务选择.json')
    assert [r['task_id'] for r in spec['consumer_handoffs']] == [r['id'] for r in selection['selected']]
    for consumer in spec['consumer_handoffs']:
        assert set(consumer['operation_names']) <= set(names)
        task = next(r for r in selection['selected'] if r['id'] == consumer['task_id'])
        assert consumer['window_id'] == task['planned_window_id']
        assert selection['api_contract_spec_ref'] in task['execution_plan']['input_refs']
    result = {'status': 'passed_actual_typed_API_schema_and_finite_protocol_proof',
              'checkpoint': 'S06-03', 'operations': 18, 'service_signatures': 18,
              'operation_schema_documents': len(schemas), 'typed_fixture_roundtrips': len(fixtures),
              'native_and_schema_shape_cases': shape_results, 'policy_model_cases': 26,
              'byte_framing_cases': 5, 'resolved_schema_refs': refs,
              'actual_product_API_execution': None, 'real_Host_authorization': False,
              'product_operations_implemented': 0, 'frontend_or_Figma': None, 'errors': []}
    # The S06 proof remains historical typed evidence, separate from current Host execution.
    result['observed_current_product_operations'] = available
    result['current_product_execution_ref'] = (spec['actual_product_API_execution']['result_ref']
                                               if available else None)
    (A / '公开操作契约检查结果.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    return result


if __name__ == '__main__':
    result = verify()
    print(json.dumps({k: v for k, v in result.items() if k != 'native_and_schema_shape_cases'}, ensure_ascii=False))
