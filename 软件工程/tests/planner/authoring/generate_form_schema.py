"""Project the existing frozen schema into the nine-category form consumer."""
import json
import re
from pathlib import Path

root = Path(__file__).resolve().parents[3]
source = root / "packages/commands/generated/schemas/apply_commands.request.schema.json"
all_definitions = json.loads(source.read_text(encoding="utf-8"))["$defs"]
pending = ["ModuleDefinition", "ModuleAdoption", "TemplateSubset"]
selected = {}
while pending:
    name = pending.pop()
    if name in selected:
        continue
    selected[name] = all_definitions[name]
    pending.extend(re.findall(r'"\$ref": "#/\$defs/([^\"]+)"', json.dumps(selected[name])))
destination = root / "apps/planner/src/features/modules/formSchema.ts"
destination.parent.mkdir(parents=True, exist_ok=True)
header = '''// Derived from the existing apply_commands schema; no new public fields.
export interface FormSchema {
  [keyword: string]: unknown;
  $ref?: string; type?: string | string[];
  properties?: Record<string, FormSchema>; required?: string[];
  oneOf?: FormSchema[]; anyOf?: FormSchema[]; enum?: (string | number | boolean | null)[];
  const?: string | number | boolean; items?: FormSchema; prefixItems?: FormSchema[];
  additionalProperties?: boolean | FormSchema;
  minimum?: number; maximum?: number; minItems?: number; maxItems?: number;
  minLength?: number;
}
export const formSchemas: Record<string, FormSchema> = '''
destination.write_bytes((header + json.dumps(selected, ensure_ascii=False, separators=(",", ":")) + ";\n").encode("utf-8"))
print(f"Projected {len(selected)} canonical types for definition/adoption forms")

# Declared, editable field examples; these are never labeled actual execution.
examples = {}
for category in ["items", "devices", "resources", "tasks", "operations", "transport", "templates", "professional", "media"]:
    path = root / "tools/type-contract-proof/fixtures" / f"module-{category}.json"
    examples[category] = json.loads(path.read_text(encoding="utf-8"))["data"]
sample_destination = root / "apps/planner/src/features/modules/fieldExamples.ts"
sample_destination.write_bytes(("// Canonical editable type examples only, not imported assets or runtime evidence.\nimport type { ModulePackage } from '../../../../../packages/commands/generated/typescript/ModulePackage';\nimport type { Category } from './model';\nexport const fieldExamples: Record<Category, ModulePackage> = " + json.dumps(examples, ensure_ascii=False, separators=(",", ":")) + ";\n").encode("utf-8"))
print("Projected nine explicit field examples without Runtime/stock/occupancy state")
