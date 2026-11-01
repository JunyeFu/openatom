"""Project actual canonical input schemas and preserve the four declaration seeds."""
import json
from pathlib import Path

root = Path(__file__).resolve().parents[3]
feature = root / 'apps/planner/src/features/packaging'
request = json.loads((root / 'packages/commands/generated/schemas/compute_packaging.request.schema.json').read_text(encoding='utf-8-sig'))
apply = json.loads((root / 'packages/commands/generated/schemas/apply_commands.request.schema.json').read_text(encoding='utf-8-sig'))
defs = {**request['$defs'], **apply['$defs']}

def expand(node):
    if isinstance(node, list):
        return [expand(value) for value in node]
    if not isinstance(node, dict):
        return node
    if '$ref' in node:
        return expand(defs[node['$ref'].split('/')[-1]])
    return {key: ({name: expand(field) for name, field in value.items()} if key == 'properties' else expand(value))
            for key, value in node.items() if key not in ('$schema', 'title', 'description')}

schemas = {name: expand(defs[name]) for name in ('PackageStructureJob', 'ProductRevision', 'CalculationBudget')}
(feature / 'schemas.ts').write_text("// Projection of actual canonical compute/apply input schemas; no added fields.\nimport type { FormSchema } from '../modules/formSchema';\nexport const packagingSchemas: Record<string, FormSchema> = " + json.dumps(schemas, ensure_ascii=False, separators=(',', ':')) + ';\n', encoding='utf-8')
examples = root / 'modules/professional/paper-packaging-ui'
examples.mkdir(parents=True, exist_ok=True)
for name in ('restricted-folded-container', 'outer-box-inner-liner', 'sliding-sleeve-explicit-candidate', 'blank'):
    raw = (root / f'modules/professional/paper-packaging/templates/{name}.job.json').read_text(encoding='utf-8-sig')
    (examples / f'{name}.job.json').write_text(raw, encoding='utf-8')
