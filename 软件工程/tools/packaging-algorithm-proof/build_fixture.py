"""创建非空公共Job试例；只读现行类型Schema，输出只在本隔离目录。"""
from pathlib import Path
from copy import deepcopy
import json
import sys

sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent
ROOT=HERE.parent.parent
sys.path.insert(0,str(ROOT/'tools/type-contract-proof/python-libs'))
from jsonschema import Draft202012Validator

def pose(x=0,y=0,z=0):
    return {'translation':{'x':x,'y':y,'z':z},'rotation':{'x':0,'y':0,'z':0,'w':1},
            'scale':{'x':1,'y':1,'z':1},'unit':'millimeter','coordinate_frame':'right_handed_z_up'}
def rectangle(x0,y0,x1,y1):return [{'x':x0,'y':y0},{'x':x1,'y':y0},{'x':x1,'y':y1},{'x':x0,'y':y1}]

def main():
    source=json.loads((ROOT/'tools/type-contract-proof/fixtures/packaging-job.json').read_text(encoding='utf-8'))
    job=source['data']
    job.update({'id':'LOCAL_CONTACT_JOB','mechanism':'outer_box_and_inner_liner','changes':['part_relations_or_repeat_order']})
    first=job['panels'][0];first.update({'id':'A','part_id':'PART_A','polygon_mm':rectangle(0,0,100,60)})
    second=deepcopy(first);second.update({'id':'B','part_id':'PART_B','polygon_mm':rectangle(0,0,10,10)})
    job['panels']=[first,second]
    job['parts']=[{'id':'PART_A','title':'local joint base, not complete outer box','panel_ids':['A'],'root_panel':'A','placement':pose()},
                  {'id':'PART_B','title':'local joint insert, not complete inner liner','panel_ids':['B'],'root_panel':'B','placement':pose(20,20,2)}]
    job['conditions'].update({'net_space_required':False,'forming_required':True,'opening_required':False,'extraction_required':False,
        'allowed_contacts':[{'id':'L1','panel_a':'A','panel_b':'B','face_a':'front','face_b':'back',
            'polygon_a_mm':rectangle(19,19,35,35),'polygon_b_mm':rectangle(0,0,10,10),'normal_overlap_limit_mm':0.15,
            'active_step_ids':['APPROACH'],'basis':'self-built finite joint example; declared 0.15mm geometric overlap, not industrial adhesive evidence'}]})
    job['steps']=[{'id':'APPROACH','purpose':'forming','duration_hint_ms':None,
        'motion':{'kind':'rigid_translation','data':{'part_id':'PART_B','start_pose':pose(20,20,2),'displacement_mm':{'x':0,'y':0,'z':-1.1}}},'prerequisites':[]}]
    job['connections']=[{'id':'GLUE1','panel_a':'A','panel_b':'B','frame_a':pose(25,25,0.45),
        'frame_b':pose(5,5,-0.45),'kind':'glue','allow_contact':True,'contact_allowance_refs':['L1'],'engage_after_step_id':'APPROACH'}]
    job['tolerance']={'distance_mm':1e-6,'angle_rad':1e-8,'constraint_mm':4e-6,'basis':'L=100mm; scale-aware model comparison profile, not paper material tolerance'}
    job['budget']={'candidate_limit':64,'interval_query_limit':50000,'subdivision_limit':8192,'basis':'explicit isolated finite method trial budget; no wall-clock guarantee'}
    schema=json.loads((ROOT/'tools/type-contract-proof/output/schemas/packaging-job.schema.json').read_text(encoding='utf-8'))
    Draft202012Validator.check_schema(schema);Draft202012Validator(schema).validate(source)
    api=json.loads((ROOT/'tools/type-contract-proof/api-fixtures/compute_packaging.request.json').read_text(encoding='utf-8'))
    api['request_id']='LOCAL-CONTACT-METHOD-TRIAL';api['params']['packaging_job']=deepcopy(job)
    api['params']['computation_budget']=deepcopy(job['budget'])
    api_schema=json.loads((ROOT/'tools/type-contract-proof/output/api-schemas/compute_packaging.request.schema.json').read_text(encoding='utf-8'))
    Draft202012Validator(api_schema).validate(api)
    OUT=HERE/'output';OUT.mkdir(exist_ok=True)
    for name,data in [('local-contact-job.json',source),('local-contact-api-request.json',api)]:
        (OUT/name).write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    # 老ID对不再是新载体，独立负例应由Schema拒绝；不是自动补全成整面许可。
    old_pair=deepcopy(source);old_pair['data']['conditions']['allowed_contacts']=[['A','B']]
    errors=list(Draft202012Validator(schema).iter_errors(old_pair))
    assert errors
    result={'evidence_level':'current_public_JSON_Schema_admission_only','nonempty_panels':2,'nonempty_contact_allowances':1,
        'nonempty_connections':1,'nonempty_steps':1,'job_schema':'passed','compute_packaging_request_schema':'passed',
        'legacy_pair_rejected':True,'complete_mechanism_or_geometry_proved_by_schema':False,'shared_files_written':False,'dependencies_installed':False}
    (OUT/'fixture-schema-result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(result,ensure_ascii=False))

if __name__=='__main__':main()
