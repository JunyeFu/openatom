"""Finite canonical IPC shapes and C02 declaration readback; no Host handles/Run."""
from pathlib import Path
import json
import sys

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT / 'python-libs'))
from jsonschema import Draft202012Validator

def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))

def main():
    schema = read(ROOT / 'output/schemas/api-all-types.schema.json')
    cases = read(ROOT / 'ipc-managed-asset-cases.design.json')['cases']
    native = read(ROOT / 'output/ipc-managed-asset-proof-result.json')['cases']
    results = []
    for case, actual in zip(cases, native, strict=True):
        field = 'ipc_task_response' if case['carrier'] == 'response' else 'ipc_request'
        target = dict(schema['properties'][field], **{'$defs': schema['$defs']})
        accepted = not list(Draft202012Validator(target).iter_errors(case['input']))
        assert accepted == case['expected_shape_accept'] == actual['shape_accept'], case['id']
        assert case['id'] == actual['id']
        results.append({'id': case['id'], 'shape_accept': accepted})
    proof = read(ROOT / 'output/c02-consumer-proof-result.json')
    original = read(ROOT.parent.parent / proof['definition_ref'])
    canonical = proof['definition_canonical']
    assert original['data']['definition']['parameters'] == canonical['data']['definition']['parameters']
    assert original['data']['definition']['payload']['data']['configuration'] == canonical['data']['definition']['payload']['data']['configuration']
    normalizations = []
    def check(a, b, path=''):
        if isinstance(a, dict):
            added = b.keys() - a.keys()
            if added:
                assert added == {'declared_output_quality'} and b['declared_output_quality'] is None, path
                assert {'role', 'quantity', 'port', 'required_quality'} <= a.keys(), path
                normalizations.append({'path': path + '/declared_output_quality', 'before': 'missing', 'after': None})
            assert not a.keys() - b.keys(), path
            for k in a:
                check(a[k], b[k], path + '/' + k)
        elif isinstance(a, list):
            assert len(a) == len(b), path
            for i, (x, y) in enumerate(zip(a, b, strict=True)):
                check(x, y, path + '/' + str(i))
        elif a != b:
            # Only observed cosmetic RGB f64 JSON parse/re-encode differences.
            assert '/material/color_rgba/' in path, path
            assert isinstance(a, (int, float)) and isinstance(b, (int, float))
            assert abs(a - b) <= 1e-15, (path, a, b)
            normalizations.append({'path': path, 'before': a, 'after': b})
    check(original, canonical)
    result = {'status': 'passed_canonical_native_and_standard_schema_handoff_shapes',
              'IPC_cases': results, 'C02_parameter_and_configuration_exact': True,
              'C02_cosmetic_float_normalizations': normalizations,
              'actual_product_runtime': None, 'actual_Host_handle_transfer': None}
    (ROOT / 'output/shared-handoff-check-result.json').write_text(
        json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({k: v for k, v in result.items() if k != 'C02_cosmetic_float_normalizations'}, ensure_ascii=False))

if __name__ == '__main__':
    main()

