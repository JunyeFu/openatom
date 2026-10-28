"""S06-02 actual type/codegen proofs and field bindings; not product acceptance."""
from pathlib import Path
import json
import re
import shutil
import subprocess

S = Path(__file__).resolve().parent.parent
A = S / 'docs/04_开发与验收'
LAB = S / 'tools/type-contract-proof'


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def verify():
    spec = read(S / 'docs/03_架构与契约/全层类型与工程格式.json')
    source = (S / spec['rust_type_source_ref']).read_text(encoding='utf-8')
    assert 'serde_json::Value' not in source
    records = re.findall(r'^pub struct (\w+)', source, re.M)
    enums = re.findall(r'^pub enum (\w+)', source, re.M)
    ids = re.findall(r'string_id!\s*\((\w+)\);', source)
    assert len(ids) == 13
    fields = {}
    for name, body in re.findall(r'pub struct (\w+)(?:<[^>]+>)?\s*\{(.*?)^\}', source, re.S | re.M):
        fields[name] = re.findall(r'^\s+pub (\w+)\s*:', body, re.M)
    inventory = {'document_kind': 'S06_02_name_and_field_navigation_not_independent_type_source',
                 'source_ref': spec['rust_type_source_ref'], 'record_names': records,
                 'enum_names': enums, 'id_names': ids, 'fields_by_record': fields,
                 'field_source': 'canonical Rust source only; inventory is derived navigation',
                 'actual_product_runtime': None}
    (S / spec['inventory_ref']).write_text(json.dumps(inventory, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    schema = read(S / spec['derived_schema_ref'])
    defs = schema['$defs']
    ts = {p.stem for p in (LAB / 'output/typescript').glob('*.ts')}
    assert set(records + enums + ids) <= ts
    assert defs['SafeUint']['maximum'] == 9007199254740991
    assert defs['DecimalPlaces']['minimum'] == 0 and defs['DecimalPlaces']['maximum'] == 9
    assert set(defs['CameraState']['properties']) >= {'unit', 'coordinate_frame', 'eye', 'target', 'zoom'}
    assert 'role' in defs['DocumentEntry']['required']
    assert 'assets' in defs['ModulePackage']['required']
    assert set(defs['MeasurementStatus']['enum']) == {'not_executed', 'not_evaluable', 'pass', 'fail'}
    assert 'available_modes' not in defs['ProfessionalDefinition']['properties']
    assert 'available_modes' in defs['ProviderCapability']['properties']
    bindings = read(S / spec['module_field_bindings_ref'])
    logical = read(S / spec['module_protocol_ref'])
    assert [x['id'] for x in bindings['categories']] == [x['id'] for x in logical['categories']]
    assert len(bindings['common_bindings']) == len(logical['common_field_contracts']) == 10
    checked_fields = 0
    all_bindings = list(bindings['common_bindings'])
    for actual, expected in zip(bindings['categories'], logical['categories']):
        assert [x['logical_path'] for x in actual['bindings']] == [x['path'] for x in expected['field_contracts']]
        assert [x['required_when'] for x in actual['bindings']] == [x['required_when'] for x in expected['field_contracts']]
        all_bindings += actual['bindings']
    for binding in all_bindings:
        assert binding['carriers']
        for carrier in binding['carriers']:
            node = defs[carrier['rust_type']]
            if 'discriminator' in carrier:
                actual = [v['properties'][carrier['discriminator']]['const'] for v in node['oneOf']]
                assert actual == carrier['values']
            else:
                assert set(carrier['fields']) <= set(node['properties'])
                assert set(carrier['fields']) <= set(fields[carrier['rust_type']])
                checked_fields += len(carrier['fields'])
    category_fields = sum(len(c['bindings']) for c in bindings['categories'])
    assert category_fields == 61
    nfr = read(S / spec['measurement_records']['source_ref'])
    measurement = spec['measurement_records']['field_bindings']
    assert set(measurement['group']) == set(nfr['test_execution_contract']['measurement_record_contract']['group_fields'])
    assert set(measurement['attempt']) == set(nfr['test_execution_contract']['measurement_record_contract']['attempt_fields'])
    for group in measurement.values():
        for entries in group.values():
            for entry in entries:
                assert entry['field'] in defs[entry['type']]['properties']
    native = read(LAB / 'output/type-proof-result.json')
    schemas = read(LAB / 'output/schema-validation-result.json')
    persistence = read(LAB / 'output/persistence-proof-result.json')
    assert native['status'] == 'passed_actual_native_type_compile_and_codegen'
    assert native['typescript_files'] == len(ts)
    assert native['fixture_roundtrips'] == len(list((LAB / 'fixtures').glob('*.json'))) == 20
    required_domain_schemas = {'all-types', 'manifest', 'project', 'scenario', 'module',
        'template', 'generation-profile', 'packaging-job', 'packaging-result',
        'artifact', 'run', 'evaluation', 'measurement'}
    actual_schema_names = {p.name.removesuffix('.schema.json')
                          for p in (LAB / 'output/schemas').glob('*.schema.json')}
    assert required_domain_schemas <= actual_schema_names
    assert native['domain_schema_documents'] == len(required_domain_schemas) == 13
    assert native['schema_documents'] == schemas['schema_documents'] == len(actual_schema_names)
    assert len(schemas['positive_fixture_documents']) == 20 and schemas['errors'] == []
    assert len(persistence['cases']) == 12 and persistence['selected_document_count'] == 20
    assert persistence['errors'] == [] and persistence['actual_product_runtime'] is None
    package = read(LAB / 'node_modules/typescript/package.json')
    assert package['version'] == '7.0.2'
    node = shutil.which('node')
    assert node is not None
    command = [node, str(LAB / 'node_modules/typescript/bin/tsc'), '-p', str(LAB / 'tsconfig.json')]
    compiled = subprocess.run(command, capture_output=True, text=True, encoding='utf-8')
    ts_result = {'status': 'passed_actual_generated_DTO_type_check' if compiled.returncode == 0 else 'failed',
                 'compiler': 'typescript7.0.2', 'command': command, 'returncode': compiled.returncode,
                 'stdout': compiled.stdout, 'stderr': compiled.stderr, 'generated_files_checked': len(ts),
                 'frontend_build_or_product_runtime': None}
    (LAB / 'output/typescript-check-result.json').write_text(json.dumps(ts_result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    assert compiled.returncode == 0, (compiled.stdout, compiled.stderr)
    selection = read(S / spec['selected_task_source_ref'])
    assert spec['consumer_tasks'] == [t['id'] for t in selection['selected']]
    assert len(selection['selected']) == 21 and len(selection['deferred']) == 7
    assert spec['actual_product_runtime'] is None and spec['product_API_operations_implemented'] == 0
    assert spec['storage']['restart_policy'].startswith('from_common_initial_input')
    assert spec['storage']['single_active_project_document'] is True
    result = {'status': 'passed_actual_types_codegen_standard_schema_and_finite_storage_proof',
              'checkpoint': 'S06-02', 'record_types': len(records), 'enum_types': len(enums),
              'opaque_ID_types': len(ids), 'generated_TS_files': len(ts),
              'schema_documents': 13, 'fixture_roundtrips': 20, 'category_fields_bound': 61,
              'aggregate_schema_documents': len(actual_schema_names),
              'common_fields_bound': 10, 'concrete_carrier_fields_checked': checked_fields,
              'measurement_fields_bound': sum(len(v) for v in measurement.values()),
              'schema_refs_resolved': schemas['schema_references_resolved'],
              'actual_TS_compiler_status': ts_result['status'], 'persistence_cases': 12,
              'consumer_tasks': 21, 'actual_product_runtime': None,
              'product_operations_implemented': 0, 'actual_frontend_or_Figma': None,
              'spec_ref': 'docs/03_架构与契约/全层类型与工程格式.json', 'errors': []}
    (A / '类型格式检查结果.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    return result


if __name__ == '__main__':
    print(json.dumps(verify(), ensure_ascii=False))
