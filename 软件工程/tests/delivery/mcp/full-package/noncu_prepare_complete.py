"""A separate declared-region fixture; preserve the original finite case."""
import copy,json,shutil
from pathlib import Path
ROOT=Path(__file__).resolve().parents[4];OUT=ROOT/'.work/delivery'
stage=OUT/'cases/complete'
value=json.loads((OUT/'cases/success/input.json').read_text(encoding='utf-8'))
before=copy.deepcopy(value)
site=value['scenario']['acceptance_goal']['site_envelope']
site['regions']=[{'id':'explicit-whole-floor-fixture','kind':'usable','polygon_m':[
    {'x':0,'y':0},{'x':site['length_m'],'y':0},{'x':site['length_m'],'y':site['width_m']},{'x':0,'y':site['width_m']}],'clearance_m':None}]
shutil.copytree(OUT/'cases/success/seed',stage/'seed')
(stage/'seed/scenario.json').write_text(json.dumps({'schema_version':'1.0','data':value['scenario']},ensure_ascii=False,indent=2),encoding='utf-8')
case=json.loads((OUT/'cases/success/case.json').read_text(encoding='utf-8'))
case.update(mode='complete',seed=str(stage/'seed'),output=str(stage/'actual'),drafts=[])
for name,data in [('input.json',value),('case.json',case),('source.json',{
    'frozen_original':'packages/commands/tests/fixtures/production/p1-legacy-goal.json',
    'controlled_transformation':'Only an explicit usable rectangle covering the already declared 20x10m site; no reserved regions or static lanes in this controlled fixture.',
    'original_case_retained':'../success/input.json','production_parameters_and_target_quantity_changed':False,
    'scenario_acceptance_goal_site_regions_changed':True,
    'expected_before_execution':{'all_static_checks':'pass','accepted_quantity':3,'target_quantity':1,
        'actual_runs':1,'formats':['pdf','html','csv','json'],'no_apply_or_compare':True},
    'real_factory_measured_layout_or_rendered_image':False})]:
    (stage/name).write_text(json.dumps(data,ensure_ascii=False,indent=2),encoding='utf-8')
assert value['project']==before['project'] and value['definitions']==before['definitions']
print('Prepared separate complete-region fixture; original case and goal quantity retained')
