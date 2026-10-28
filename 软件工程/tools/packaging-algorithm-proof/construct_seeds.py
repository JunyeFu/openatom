"""三机制的有限声明种子编译试做；不计算完整几何或生成认可/库存。"""
from copy import deepcopy
from pathlib import Path
import json
import math
import sys
sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent;ROOT=HERE.parent.parent;OUT=HERE/'output'
sys.path.insert(0,str(ROOT/'tools/type-contract-proof/python-libs'))
from jsonschema import Draft202012Validator

def pose(x=0,y=0,z=0,flip=False):
    return {'translation':{'x':x,'y':y,'z':z},'rotation':{'x':1 if flip else 0,'y':0,'z':0,'w':0 if flip else 1},
            'scale':{'x':1,'y':1,'z':1},'unit':'millimeter','coordinate_frame':'right_handed_z_up'}
def v3(x,y,z=0):return {'x':x,'y':y,'z':z}
def rect(l,w):return [{'x':0,'y':0},{'x':l,'y':0},{'x':l,'y':w},{'x':0,'y':w}]

def blank(prefix,l,w,h,t,f,placement):
    # 明确输入f>t，给板实体分离空间；不是纸厚/压痕的材料定律。
    assert l>0 and w>0 and h>0 and 0<t<f
    root=prefix+'_BASE';part=prefix+'_PART';mat=prefix+'_MAT'
    layouts=[('BASE',l,w,0,0),('FRONT',l,h,0,-f-h),('BACK',l,h,0,w+f),('LEFT',h,w,-f-h,0),('RIGHT',h,w,l+f,0)]
    panels=[{'id':prefix+'_'+name,'part_id':part,'polygon_mm':rect(a,b),'thickness_mm':t,'material_ref':mat,'flat_pose':pose(x,y)} for name,a,b,x,y in layouts]
    # 原始axis在父BASE局部，旋向与受限板平面一致。
    specs=[('FRONT',v3(0,-f/2),v3(l,-f/2),-math.pi/2),('BACK',v3(0,w+f/2),v3(l,w+f/2),math.pi/2),
           ('LEFT',v3(-f/2,0),v3(-f/2,w),math.pi/2),('RIGHT',v3(l+f/2,0),v3(l+f/2,w),-math.pi/2)]
    bands=[];steps=[]
    for name,start,end,target in specs:
        ident=prefix+'_FOLD_'+name;step=prefix+'_FORM_'+name
        bands.append({'id':ident,'panel_a':root,'panel_b':prefix+'_'+name,'axis_start_mm':start,'axis_end_mm':end,
                      'width_mm':f,'target_angle_rad':target,'allowed_range_rad':[min(0,target),max(0,target)]})
        steps.append({'id':step,'purpose':'forming','duration_hint_ms':None,'motion':{'kind':'fold','data':{'fold_band':ident,'from_angle_rad':0,'to_angle_rad':target}},
                      'prerequisites':[] if not steps else [steps[-1]['id']]})
    parts=[{'id':part,'title':'self-built restricted planar blank','panel_ids':[p['id'] for p in panels],'root_panel':root,'placement':placement}]
    materials=[{'id':mat,'title':'self-built material input, not industrial approval','thickness_mm':t,'basis':'template_declared_example','source_ref':None}]
    return {'panels':panels,'fold_bands':bands,'parts':parts,'materials':materials,'steps':steps}

def envelope(l,w,h,x,y,z):
    return {'subject':{'id':'PRODUCT1','revision':'r1'},'vertices_mm':[v3(a,b,c) for a in [-l/2,l/2] for b in [-w/2,w/2] for c in [-h/2,h/2]],
            'allowed_poses':[pose(x,y,z)],'clearance_mm':0.5}
def base_job(mechanism):
    job=deepcopy(json.loads((ROOT/'tools/type-contract-proof/fixtures/packaging-job.json').read_text(encoding='utf-8'))['data'])
    job.update({'id':mechanism+'_SEED','mechanism':mechanism,'variables':[], 'dimension_constraints':[], 'changes':[],
        'panels':[],'fold_bands':[],'parts':[],'materials':[],'steps':[], 'connections':[],
        'tolerance':{'distance_mm':1e-6,'angle_rad':1e-8,'constraint_mm':4e-6,'basis':'explicit finite seed model profile, not calibrated material data'},
        'budget':{'candidate_limit':64,'interval_query_limit':50000,'subdivision_limit':8192,'basis':'finite declaration seed, not actual geometry or 30-second performance'}})
    job['conditions']={'net_space_required':True,'forming_required':True,'opening_required':True,'extraction_required':True,'allowed_contacts':[],
        'product_envelope':None,'extraction':None}
    return job
def append_blank(job,data):
    if job['steps'] and data['steps']:data['steps'][0]['prerequisites']=[job['steps'][-1]['id']]
    for key,rows in data.items():job[key].extend(rows)
def translation(job,ident,part,start,delta,purpose):
    job['steps'].append({'id':ident,'purpose':purpose,'duration_hint_ms':None,'motion':{'kind':'rigid_translation','data':{'part_id':part,'start_pose':start,'displacement_mm':delta}},'prerequisites':[job['steps'][-1]['id']]})
def open_front(job,prefix):
    job['steps'].append({'id':prefix+'_OPEN_FRONT','purpose':'opening','duration_hint_ms':None,
        'motion':{'kind':'fold','data':{'fold_band':prefix+'_FOLD_FRONT','from_angle_rad':-math.pi/2,'to_angle_rad':0}},'prerequisites':[job['steps'][-1]['id']]})
def extracted(job,part,position,direction,distance):
    job['conditions']['extraction']={'part_ref':part,'product_pose':position,'direction':direction,'distance_mm':distance}

def make_container(l=60,t=0.5):
    job=base_job('restricted_folded_container');append_blank(job,blank('BOX',l,40,20,t,2,pose()));open_front(job,'BOX')
    job['conditions']['product_envelope']=envelope(l-20,20,12,l/2,20,8)
    extracted(job,'BOX_PART',pose(l/2,20,8),v3(0,-1),80)
    return job
def make_outer_liner():
    job=base_job('outer_box_and_inner_liner')
    append_blank(job,blank('OUTER',60,40,20,0.5,2,pose()))
    append_blank(job,blank('LINER',40,25,15,0.5,2,pose(10,7.5,40)))
    translation(job,'INSERT_LINER','LINER_PART',pose(10,7.5,40),v3(0,0,-38.5),'forming')
    open_front(job,'OUTER');open_front(job,'LINER')
    job['conditions']['product_envelope']=envelope(24,12,8,30,20,8)
    extracted(job,'OUTER_PART',pose(30,20,8),v3(0,-1),80)
    return job
def make_cover():
    job=base_job('cover_or_sliding_sleeve')
    append_blank(job,blank('TRAY',60,40,4,0.5,2,pose()))
    append_blank(job,blank('COVER',64,44,20,0.5,2,pose(-2,42,23,True)))
    translation(job,'LIFT_COVER','COVER_PART',pose(-2,42,23,True),v3(0,0,27),'opening')
    translation(job,'MOVE_COVER_ASIDE','COVER_PART',pose(-2,42,50,True),v3(102,0,0),'opening')
    job['conditions']['product_envelope']=envelope(40,20,10,30,20,10)
    extracted(job,'TRAY_PART',pose(30,20,10),v3(0,0,1),60)
    return job

def main():
    schema=json.loads((ROOT/'tools/type-contract-proof/output/schemas/packaging-job.schema.json').read_text(encoding='utf-8'))
    validator=Draft202012Validator(schema);OUT.mkdir(exist_ok=True)
    jobs=[make_container(),make_outer_liner(),make_cover()]
    # 独立计数来自三个手工定义的结构：5/10/10板，4/8/8折带，1/2/2parts。
    rows=[]
    for job,expected in zip(jobs,[(5,4,1),(10,8,2),(10,8,2)]):
        value={'schema_version':'1.0','data':job};validator.validate(value)
        counts=(len(job['panels']),len(job['fold_bands']),len(job['parts']));assert counts==expected
        path=OUT/(job['mechanism']+'-seed.json');path.write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
        rows.append({'mechanism':job['mechanism'],'expected_counts':list(expected),'actual_counts':list(counts),'schema':'passed','geometry':'not_executed'})
    changed_product=make_container(80);changed_product['changes']=['product_function'];changed_product['subject_ref']['revision']='r2'
    assert max(v['x'] for v in changed_product['panels'][0]['polygon_mm'])==80
    changed_material=make_container(t=0.8);changed_material['changes']=['material_approved'];assert all(p['thickness_mm']==0.8 for p in changed_material['panels'])
    repeated=make_container(); extra=blank('REPEAT',60,40,20,0.5,2,pose(200,0,0));append_blank(repeated,extra);repeated['changes']=['part_relations_or_repeat_order']
    assert len(repeated['parts'])==2 and len({p['id'] for p in repeated['panels']})==10
    for name,job in [('product-change',changed_product),('material-change',changed_material),('explicit-repeat',repeated)]:
        value={'schema_version':'1.0','data':job};validator.validate(value);(OUT/(name+'-seed.json')).write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    result={'evidence_level':'actual_isolated_structure_program_expansion_and_current_schema_not_full_geometry',
        'three_mechanisms':rows,'three_change_classes_generated':True,'blank_entry':'uses same explicit operations, no hidden default seed',
        'sliding_sleeve_variant':'explicit supplied strip/seam topology must close under interval method; this finite sample uses cover branch',
        'complete_geometry_checks':None,'actual_product_runtime':None,'actual_dependencies_installed':False}
    (OUT/'structure-expansion-result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({'mechanisms':3,'schema_checked_jobs':6,'three_change_classes_generated':True,'complete_geometry_checks':None},ensure_ascii=False))

if __name__=='__main__':main()
