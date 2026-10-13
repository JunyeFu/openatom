"""生成九类设计登记与缺失类别样例；不注册API或创建运行库存。"""
import copy,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
BASE=Path(__file__).resolve().parent
def save(path,value):
    path.parent.mkdir(parents=True,exist_ok=True)
    path.write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
def manifest(module_id,category,title,field,payload):
    return dict(document_kind='typed_module_declaration_design_not_runtime',id=module_id,version='0.1.0',
        category=category,title=title,definition_state='design',
        contribution=dict(author='待填写',license='待声明',assets=[],status='design'),
        **{field:payload})
specs=[
 ('items','shell-spec','物品规格：壳体','item_definition',dict(unit='piece',tracking='individual',
     spec_ref='self-shell-r1',attributes={'envelope_mm':[100,60,30]},quantity_is_instance_state=True)),
 ('resources','operator-role','可共享操作人员角色','resource_definition',dict(kind='worker',role='assembler',
     capacity={'value':1,'unit':'person'},default_calendar_ms=[[0,300000]],current_occupation_is_instance_state=True)),
 ('resources','power-source','声明供电服务源','resource_definition',dict(kind='service_source',service='electric_active_power',
     capacity={'value':14,'unit':'kW'},background={'value':0,'unit':'kW'},reference_conditions='自建有功容量，不推导电网/视在容量',
     consumption_is_runtime_state=True)),
 ('tasks','board-preparation','板件作业的一次准备','task_definition',dict(kind='preparation',target_role='board_station',
     prerequisites=['selected_board_job_and_configuration'],resource_roles=[{'role':'setter','quantity':1}],
     duration={'value':30,'unit':'second','basis':'自建输入'},equipment_access='exclusive',
     completion_effect={'readiness_flag':'prepared_for_selected_board_job'},produces_material=False)),
 ('operations','control-box-rule','壳体/PCB/螺钉齐套规则','operation_definition',dict(builtin='assemble',
     item_roles=['shell','pcb','fastener','assembly'],inputs=[{'role':'shell','quantity':1,'unit':'piece'},{'role':'pcb','quantity':1,'unit':'piece'},{'role':'fastener','quantity':4,'unit':'piece'}],
     outputs=[{'role':'assembly','quantity':1,'unit':'piece'}],resource_roles=[{'role':'assembler','quantity':1}],
     duration={'value':40,'unit':'second','basis':'自建输入'},identity_policy='new_output_preserves_input_composition',
     changes_stock_only_through_factory_engine=True)),
 ('professional','machining-producer','名义数控工程提供者（计划）','professional_contract',dict(
     input_fields=['product_revision','stock_geometry','tool_definitions','fixture','coordinate_frame','nc_dialect','process_choices'],
     output_artifact_type='machining_job',output_fields=['nc_program','nominal_geometry','phase_plan','supported_scope'],
     unit='mm',coordinate_frame='explicit_machine_local',
     intended_modes=['calculated_nominal','software_in_loop'],available_modes=[],
     producer_status='not_implemented',material_commit_owner='factory_engine',
     scope='沿用原完整三轴2.5D工程目标，清单不表示已实际计算')),
 ('media','editable-bench','可编辑台面与锚点表现','presentation',dict(
     kind='native_scene',path='assets/native-scene.json',unit='meter',
     anchor_roles=['input','output'],binding_owner='device_or_item_instance',
     layout_footprint_effect='only_after_explicit_design_adoption',production_effect='none'))
]
for category,folder,title,key,payload in specs:
    target=ROOT/'modules'/category/folder
    m=manifest('community.'+folder,category,title,key,payload)
    if category=='media':
        scene=copy.deepcopy(json.loads((ROOT/'modules/devices/manual-assembly/presentation/native-scene.json').read_text(encoding='utf-8')))
        scene['id']='shared-bench-scene';scene['port_anchors']=[]
        scene['anchor_roles']=[dict(id='input',node='table',local_position=[-.5,.08,0]),dict(id='output',node='table',local_position=[.5,.08,0])]
        save(target/'assets/native-scene.json',scene)
        m['contribution']['assets']=[dict(path='assets/native-scene.json',source='本项目自建原生数据',license='待声明',redistribution_confirmed=False)]
    save(target/'manifest.json',m)
    save(target/'example.design.json',dict(document_kind='typed_definition_adoption_example_not_runtime',
        definition_ref={'id':m['id'],'version':'0.1.0'},adoption='explicit_into_project',
        initial_stock_created=0,actual_result=None))
    (target/'README.md').write_text('# '+title+'\n\n类别'+category+'的独立声明样例。清单定义能力/规格，实例与运行状态由工程持有；导入不自动创建现有库存。\n\n'+
        '所有数字和原生几何是自建输入，未注册或运行。作者/许可发布前填写，专业提供者available_modes为空；素材不产出物品。\n\n'+
        '[清单](manifest.json) · [采用说明](example.design.json) · [共同协议](../../../docs/03_架构与契约/九类模块清单与消费者.md)。\n',encoding='utf-8')

categories=[
 ('items','物品',['item_definition'],'item_type','domain/planner','specification_not_stock','items/shell-spec/manifest.json'),
 ('devices','固定设备',['parameters','ports','behavior','presentation'],'device_instance','module-kit/simulation/presentation','one_selected_behavior_source','devices/control-box-assembly/manifest.json'),
 ('resources','资源',['resource_definition'],'resource_instance','domain/simulation','capacity_not_current_occupation','resources/operator-role/manifest.json'),
 ('tasks','任务（人工、准备与维护）',['task_definition'],'task_instance','simulation/planner','work_or_readiness_not_material_output','tasks/board-preparation/manifest.json'),
 ('operations','工艺规则',['operation_definition'],'operation_binding','simulation','only_factory_commits_material','operations/control-box-rule/manifest.json'),
 ('transport','物流与供应',['parameters','ports','behavior'],'transport_binding','simulation/planner','one_vehicle_and_one_cargo_holder','transport/batch-transport/manifest.json'),
 ('templates','厂房与组合',['project'],'project_object_group','planner/module-kit','new_ids_without_runtime_stock_copy','templates/parallel-electromechanical/manifest.json'),
 ('professional','专业计算',['professional_contract'],'producer_binding','professional_tool_and_factory_consumer','artifact_not_automatic_stock_or_acceptance','professional/machining-producer/manifest.json'),
 ('media','表现素材',['presentation'],'presentation_binding','presentation','visual_only','media/editable-bench/manifest.json')
]
registry=dict(document_kind='category_contract_design_not_registered_loader',revision='draft-1',
    common_fields=['id','version','category','title','definition_state','contribution'],
    definition_state_values=['design','editable_example','runtime_candidate','released','unmaintained'],
    runtime_state_owner='project_instances_and_results_not_definitions',
    categories=[dict(id=c,title=t,payload_fields=f,instantiate_as=i,consumer=o,invariant=v,example_manifest=e)
                for c,t,f,i,o,v,e in categories],
    implementation_tasks=['P0-01','P3-01','P4-01','P5-01'],actual_result=None)
for category in registry['categories']:
    if category['id']=='devices':
        category['payload_fields']=['parameters','ports','presentation']
        category['behavior_sources']=['inline_behavior','operation_ref']
        category['behavior_source_selection']='exactly_one_explicit_source'
# Keep the current authoring/goal-generation consumer fields when regenerating examples.
previous=json.loads((BASE/'category-contracts.design.json').read_text(encoding='utf-8'))
previous_categories={c['id']:c for c in previous['categories']}
for category in registry['categories']:
    old=previous_categories[category['id']]
    for key in ['authoring_actions','agent_edit_operation','business_scope','optional_payload_fields','generation_profile_scope']:
        if key in old:category[key]=old[key]
save(BASE/'category-contracts.design.json',registry)

catalog_path=ROOT/'modules/catalog.json';catalog=json.loads(catalog_path.read_text(encoding='utf-8'))
ids={'community.'+folder for _,folder,_,_,_ in specs}
catalog['modules']=[x for x in catalog['modules'] if x['id'] not in ids]
for category,folder,title,key,payload in specs:
    catalog['modules'].append(dict(id='community.'+folder,category=category,title=title,
        manifest=category+'/'+folder+'/manifest.json',readme=category+'/'+folder+'/README.md',status='design_example'))
for c in catalog['categories']:
    if c['id']=='tasks':c['title']='任务（人工、准备与维护）'
save(catalog_path,catalog)
# 共同状态补入已有清单，原参数/端口/行为/素材字段不改。
for ref in catalog['modules']:
    path=ROOT/'modules'/ref['manifest'];m=json.loads(path.read_text(encoding='utf-8'))
    m['definition_state']='design';save(path,m)
print(f"9 category contracts and 7 category examples; {len(catalog['modules'])} declaration entries, no registered runtime plugins.")
