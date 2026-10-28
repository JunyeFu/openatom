"""Current generated-schema admission; this does not execute the Engine."""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/type-contract-proof/python-libs'))
from jsonschema import Draft202012Validator

schema = json.loads((ROOT / 'packages/commands/generated/schemas/get_object.response.schema.json').read_text(encoding='utf-8-sig'))
results = []
fixture_directory = ROOT / (sys.argv[2] if len(sys.argv) > 2 else 'tests/planner/p201-whole/fixtures/b60')
for variant in ('balanced', 'dense'):
    value = json.loads((fixture_directory / f'{variant}.input.json').read_text(encoding='utf-8'))
    for key, definition, objects in [('project', 'ProjectState', [value['project']]), ('scenario', 'ScenarioInput', [value['scenario']]), ('packages', 'ModulePackage', value['packages'])]:
        validator = Draft202012Validator({'$schema': schema['$schema'], '$defs': schema['$defs'], '$ref': '#/$defs/' + definition})
        for index, obj in enumerate(objects):
            errors = list(validator.iter_errors(obj))
            results.append({'variant': variant, 'object': key, 'index': index, 'errors': [{'path': list(error.absolute_path), 'message': error.message} for error in errors]})
output = ROOT / '.work/p201-whole-20261017' / (sys.argv[1] if len(sys.argv) > 1 else 'b60-schema.json')
output.write_text(json.dumps({'schema': 'packages/commands/generated/schemas/get_object.response.schema.json', 'fixture_directory': str(fixture_directory), 'checks': results, 'actual_execution': None}, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
failed = [value for value in results if value['errors']]
print(json.dumps({'objects': len(results), 'failed': failed}, ensure_ascii=False))
raise SystemExit(bool(failed))
