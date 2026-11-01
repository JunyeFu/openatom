"""Additive current-schema cases; existing schema/fixture cases remain unchanged."""
from copy import deepcopy
import json
from pathlib import Path
import sys

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'tools/type-contract-proof/python-libs'))
from jsonschema import Draft202012Validator

schemas=list((ROOT/'packages/commands/generated/schemas').glob('*.schema.json'))
assert len(schemas)==40,len(schemas)
for path in schemas:Draft202012Validator.check_schema(json.loads(path.read_text(encoding='utf-8-sig')))
project=json.loads((ROOT/'packages/commands/generated/schemas/project.schema.json').read_text(encoding='utf-8-sig'))
validator=Draft202012Validator({'$ref':'#/$defs/FactoryGenerationGoal','$defs':project['$defs']})
source=json.loads((ROOT/'packages/simulation/tests/fixtures/multi_sink_goal.json').read_text(encoding='utf-8'))['scenario']['acceptance_goal']
cases=[]
def check(name,value,accepted):
    errors=list(validator.iter_errors(value))
    assert bool(errors) != accepted,(name,[error.message for error in errors])
    cases.append(dict(name=name,accepted=accepted,actual_errors=[error.message for error in errors]))
check('six_explicit_scopes',source,True)
old=deepcopy(source);del old['additional_acceptance_scopes'];check('legacy_missing_list',old,True)
empty=deepcopy(source);empty['additional_acceptance_scopes']=[];check('explicit_empty_list',empty,True)
for name,value in [('null',None),('object',{}),('null_element',[None]),('incomplete_scope',[{}])]:
    candidate=deepcopy(source);candidate['additional_acceptance_scopes']=value;check(name,candidate,False)
unknown=deepcopy(source);unknown['additional_acceptance_scopes'][0]['invented']=True;check('unknown_scope_field',unknown,False)
missing=deepcopy(source);missing['additional_acceptance_scopes'][0]['destination']='';check('empty_actual_destination',missing,False)
result=dict(kind='actual_40_current_schemas_and_additive_goal_cases_not_runtime_acceptance',schema_count=len(schemas),passed=len(cases),failed=0,cases=cases)
(HERE/'goal-schema.result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print('40 current schemas checked;9 additive goal cases passed; original fixtures untouched.')
