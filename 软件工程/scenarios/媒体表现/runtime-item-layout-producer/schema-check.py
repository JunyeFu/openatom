from pathlib import Path
import json,copy,sys
root=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(root/'tools/type-contract-proof/python-libs'))
from jsonschema import Draft202012Validator
out=Path(__file__).resolve().parent
project=json.loads((out/'actual-host-reception.json').read_text(encoding='utf8'))['source_Project']
rows=[]
for family,folder in [('canonical','tools/type-contract-proof/output/schemas'),('runtime','packages/commands/generated/schemas')]:
 schema=json.loads((root/folder/'project.schema.json').read_text(encoding='utf8'))
 validator=Draft202012Validator(schema)
 value={'schema_version':'1.0','data':project}
 cases=[('three-owner-exact-layouts',value,True)]
 for label,change in [('missing-assignment',lambda p:p['devices'][0]['item_layout'].pop('assignment')),('missing-Pose',lambda p:p['devices'][0]['item_layout']['slots'][0].pop('offset')),('unknown-assignment',lambda p:p['devices'][0]['item_layout'].__setitem__('assignment','automatic_guess')),('unknown-slot-field',lambda p:p['devices'][0]['item_layout']['slots'][0].__setitem__('capacity',100))]:
  v=copy.deepcopy(value);change(v['data']);cases.append((label,v,False))
 v=copy.deepcopy(value)
 for coll,key,index in [('devices','item_layout',0),('locations','item_layout',0),('item_types','contents_layout',4)]:v['data'][coll][index][key]=None
 cases.append(('explicit-null-is-unset',v,True))
 for label,value,expected in cases:
  actual=validator.is_valid(value);assert actual==expected,(family,label,list(validator.iter_errors(value))[:1]);rows.append({'family':family,'case':label,'accepted':actual})
 for typename in ['ItemSlotLayout','ItemDisplaySlot','ItemSlotAssignment']:
  assert typename in schema['$defs'],(family,typename)
result={'status':'passed_actual_generated_canonical_and_runtime_schema','cases':rows,'new_TS_contracts':3,'Native_or_Renderer':False}
(out/'schema-check.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
print(json.dumps({'schema_cases':len(rows),'status':'passed'}))
