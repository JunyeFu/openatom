"""Validate actually generated draft2020-12 schemas/fixtures; not product acceptance."""
from pathlib import Path
from copy import deepcopy
import json
import re
import sys

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT / 'python-libs'))
from jsonschema import Draft202012Validator


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def main():
    schemas = {path.name.removesuffix('.schema.json'): read(path)
               for path in (ROOT / 'output/schemas').glob('*.schema.json')}
    for name, schema in schemas.items():
        Draft202012Validator.check_schema(schema)
    positives = []
    for path in sorted((ROOT / 'fixtures').glob('*.json')):
        name = path.stem
        schema_name = 'module' if name.startswith('module-') else name
        validator = Draft202012Validator(schemas[schema_name])
        errors = list(validator.iter_errors(read(path)))
        assert not errors, (name, [e.message for e in errors])
        assert read(path) == read(ROOT / f'output/{name}.roundtrip.json'), name
        positives.append(name)
    definitions = schemas['all-types']['$defs']

    def for_type(name):
        return Draft202012Validator({'$ref': '#/$defs/' + name, '$defs': definitions})

    negatives = []
    cases = [
        ('SafeUint', 9007199254740992), ('SafeUint', -1), ('SafeUint', 1.5),
        ('DecimalPlaces', 10), ('ProjectId', ''), ('ProjectId', 'a\x00b'),
        ('RelativeFilePath', '../outside.json'), ('RelativeFilePath', '/root.json'),
        ('RelativeFilePath', 'C:/root.json'), ('RelativeFilePath', 'a\\b.json'),
        ('RelativeFilePath', 'a//b.json'), ('RelativeFilePath', 'a/./b.json'),
        ('VersionId', '01.2.3'), ('VersionId', '1.2'),
        ('VersionId', '1.2.3-01'), ('VersionId', '1.2.3+'),
    ]
    for type_name, value in cases:
        assert list(for_type(type_name).iter_errors(value)), (type_name, value)
        negatives.append({'type': type_name, 'input': value, 'status': 'rejected'})
    for type_name, value in [('SafeUint', 9007199254740991),
                             ('RelativeFilePath', 'modules/m0001/definition.json'),
                             ('VersionId', '1.2.3-rc.1+build'), ('ProjectId', '厂房-P1')]:
        assert not list(for_type(type_name).iter_errors(value)), (type_name, value)

    # Readonly provider capability cannot be smuggled into a definition.
    malformed = deepcopy(read(ROOT / 'fixtures/module-professional.json'))
    malformed['data']['definition']['payload']['data']['available_modes'] = ['calculated_nominal']
    assert list(Draft202012Validator(schemas['module']).iter_errors(malformed))
    negatives.append({'type': 'ModulePackage', 'input': 'definition.available_modes', 'status': 'rejected'})
    malformed = deepcopy(read(ROOT / 'fixtures/module-media.json'))
    malformed['data']['definition']['payload']['data']['production_effect'] = 'creates_stock'
    assert list(Draft202012Validator(schemas['module']).iter_errors(malformed))
    negatives.append({'type': 'ModulePackage', 'input': 'media.creates_stock', 'status': 'rejected'})

    # Check local schema references and actual TS imports, rather than only file counts.
    refs = []
    def walk(node, root):
        if isinstance(node, dict):
            if '$ref' in node:
                ref = node['$ref']
                assert ref.startswith('#/'), ('nonlocal_schema_ref', ref)
                value = root
                for token in ref[2:].split('/'):
                    value = value[token.replace('~1', '/').replace('~0', '~')]
                refs.append(ref)
            for value in node.values():
                walk(value, root)
        elif isinstance(node, list):
            for value in node:
                walk(value, root)
    for schema in schemas.values():
        walk(schema, schema)
    imports = 0
    for path in (ROOT / 'output/typescript').glob('*.ts'):
        text = path.read_text(encoding='utf-8-sig')
        for module in re.findall(r'from "(\./[^"]+)"', text):
            assert path.with_name(module[2:] + '.ts').is_file(), (path.name, module)
            imports += 1
    result = {'status': 'passed_actual_draft2020_12_schema_and_fixture_validation',
              'validator': 'python-jsonschema4.26.0 isolated tool directory',
              'schema_documents': len(schemas), 'positive_fixture_documents': positives,
              'negative_cases': negatives, 'positive_boundary_cases': 4,
              'schema_references_resolved': len(refs), 'TS_imports_resolved': imports,
        'TS_client_compiled_by_this_check': False,
              'actual_product_runtime': None, 'errors': []}
    (ROOT / 'output/schema-validation-result.json').write_text(
        json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({k: result[k] for k in ['status', 'schema_documents',
                     'schema_references_resolved', 'TS_imports_resolved', 'errors']}, ensure_ascii=False))


if __name__ == '__main__':
    main()
