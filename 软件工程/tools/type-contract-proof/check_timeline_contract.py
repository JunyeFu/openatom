"""TC01/TC02 canonical standard Schema/API agreement; no Engine or Host calls."""
from pathlib import Path
from copy import deepcopy
import json
import sys

D = Path(__file__).resolve().parent
sys.path.insert(0, str(D / 'python-libs'))
from jsonschema import Draft202012Validator

def read(path): return json.loads(path.read_text(encoding='utf-8-sig'))

cases = read(D / 'timeline-contract-cases.design.json')['cases']
native = read(D / 'output/timeline-contract-proof-result.json')
run_validator = Draft202012Validator(read(D / 'output/schemas/run.schema.json'))
api_validator = Draft202012Validator(read(D / 'output/api-schemas/get_run.response.schema.json'))
template = read(D / 'api-fixtures/get_run.response.json')
rows = []
for case, result in zip(cases, native['cases'], strict=True):
    domain_ok = not list(run_validator.iter_errors({'schema_version': '1.0', 'data': case['run']}))
    response = deepcopy(template)
    response['outcome']['data'] = {'kind': 'full', 'data': case['run']}
    api_ok = not list(api_validator.iter_errors(response))
    assert result['id'] == case['id']
    assert domain_ok == api_ok == result['shape_accepted'] == result['full_result_API_shape_accepted'] == case['expected_shape_accepted'], case['id']
    assert result['finite_semantics_accepted'] == case['expected_semantics_accepted']
    rows.append({'id': case['id'], 'canonical_run_shape': domain_ok, 'get_run_full_shape': api_ok,
                 'finite_consumer_semantics': result['finite_semantics_accepted']})
result = {'status': 'passed_standard_schema_native_and_FullResult_agreement', 'cases': rows,
          'public_operations_added': 0, 'actual_product_runtime': None, 'errors': []}
(D / 'output/timeline-schema-proof-result.json').write_text(json.dumps(result, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
print(json.dumps({'status':result['status'], 'cases':len(rows), 'errors':[]}, ensure_ascii=False))
