"""沿用41份当前专业定义，补真实消费者与最低成果字段；不生成运行插件。"""
from pathlib import Path
import json,csv,sys
sys.stdout.reconfigure(encoding='utf-8')
B=Path(__file__).parent

# id、工程角色、成果类型、业务字段、实际消费者。范围取原定义，不自行扩成通用求解器。
SPECS=[
 ('R17-CAM','工艺准备/运行','machining_job','product_revision stock_ref tools_ref fixture_ref nc_dialect nc_ref nominal_geometry_ref phase_plan','加工工位；装配条件'),
 ('R20-PACKAGING','产品/工艺准备','package_structure_job','product_revision panels folds joints forming_sequence use_sequence geometric_conditions manufacturing_layers','包装规格与装盒条件'),
 ('N21','工艺准备/运行','placement_campaign','board_revision job_ref board_locations feeder_assignments nozzle_assignments preparation_stages first_article_gate','PCB支线作业与总装齐套'),
 ('R18-PANEL','设备建设/投用','cabinet_installation_job','connections terminals cabinet_geometry wire_list material_list installation_tasks acceptance_conditions','控制柜安装与设备投用'),
 ('R17-ROBOT_OLP','工艺准备/运行','robot_operation_job','workstation_ref tcp_ref approved_curve motion_ref axis_limits process_events transfer_conditions','机器人加工/搬运工位'),
 ('N20','联调/运行','device_behavior_program','states operations signal_types feedback_conditions execution_target clock_boundary','设备行为；软件目标反馈'),
 ('N22','布局/物流','storage_operation_plan','locations capacity_rules layout_ref handling_tasks route_refs item_compatibility','库位、接收与搬运'),
 ('R19-TOLERANCE','产品/质量条件','assembly_acceptance_conditions','features functional_limits deviation_model approved_variants measurement_conditions','装配前置；检查判据'),
 ('N09','质量/运行','vision_inspection_recipe','product_revision regions expected_sources model_ref preprocessing outputs acceptance_rules license_refs','视觉检查、放行与返工分流'),
 ('R13','质量分析','quality_analysis_result','observation_refs analysis_scope model_assumptions findings proposed_changes','人工处置或工艺变更评审'),
 ('R14-PRINT','包材工艺/运行','print_preparation_job','document_revision material_ref imposition_ref machine_conditions quantity_units approved_job','标签/包材供应支线'),
 ('N19','后续材料类型','textile_cutting_job','material_rolls ply_plan cut_parts bundles remaining_material_ref quantity_relations','卷料到裁片/套件的新工序'),
 ('N14','计划','production_plan','orders allocations routes dispatch_order resource_calendars acceptance_locations','选定生产任务与订单比较'),
 ('R09-CARGO','发运/物流','loading_plan','packages contents vehicle_constraints loading_sequence unloading_order destination_acceptance','装卸任务与客户接受'),
 ('R11','交付','engineering_handover','selected_project_revision artifact_refs approvals input_assumptions result_refs license_scope','接手工程、操作资料与贡献包'),
 ('N16','公用服务','electric_thermal_service_plan','source_nodes capacity_profiles load_profiles quantity_units operating_conditions price_basis','电/热/冷服务绑定与条件成本'),
 ('R23-AIR','公用服务','compressed_air_service_plan','source_nodes connection_points flow_profiles reference_conditions pressure_basis quality_conditions supported_solver_scope','气服务容量/品质与设备需求'),
 ('R22-WATER','公用服务/后续流体','water_service_plan','source_nodes flow_units composition quality_conditions supply_return_boundary residual_streams','用水需求与后续组分流'),
 ('R12-PROCESS','后续材料类型','process_stream_job','stream_refs composition material_energy_balance unit_operations residuals batch_discrete_boundary','新流体/批次工序与包装边界'),
 ('R10','后续材料类型','furnace_batch_recipe','charge_materials composition mass_units approved_recipe batch_properties downstream_conversion','炉次到材料/单件的新工序'),
 ('N17','研发/工艺认可','approved_material_recipe','candidate_refs experiment_conditions evidence_scope approved_recipe process_conditions','认可配方与生产准备'),
 ('R21-OPTICS','产品/设备准备','optical_configuration','optical_components geometry material_properties imaging_conditions alignment_constraints','成像设备与视觉检验条件'),
 ('R24-THERMAL','产品/设备准备','thermal_operating_conditions','geometry_ref load_cases material_refs cooling_boundary model_scope accepted_conditions','设备工况与冷却服务边界'),
 ('N13','产品变更','electronics_revision_package','circuit_revision firmware_constraints bom_revision placement_coordinates functional_evidence','电子装联输入与变更评审'),
 ('R22-MOTOR','设备准备/服务','drive_operating_conditions','drive_ref load_cases control_conditions power_profile thermal_limits supported_scope','设备驱动条件与电服务需求'),
 ('R23-GEAR','设备建设','transmission_configuration','geometry_ref ratio material_refs load_cases accepted_operating_conditions','设备建设与机器人专业输入'),
 ('R21-MOLDING','后续专业工艺','molding_operation_job','part_revision mold_ref material_ref filling_conditions cycle_scope nominal_result_ref','后续成型工位与塑料件'),
 ('R24-STRUCTURE','产品/设备建设','structural_conditions','geometry_ref material_refs supports load_cases accepted_scope','加工/装配准备与设备安装条件'),
 ('R21-NVH','产品研发/质量','nvh_acceptance_conditions','product_revision excitation_cases measurement_positions spectral_results supported_scope','产品声振条件与试验任务'),
 ('R20-RELIABILITY','研发/质量','test_plan','product_revision failure_assumptions specimens resources durations acceptance_criteria','试验准备与认可决定'),
 ('R20-SITE','场地建设','site_readiness_plan','terrain_ref coordinate_frame grading_plan earthwork_balance facility_positions access_conditions','厂区建设与目标位置'),
 ('N18','建设费用','construction_quantity_cost_plan','building_inputs quantities unit_rates rate_sources inclusions exclusions changes','建设方案费用与改造评审'),
 ('R18-LIGHTING','厂房环境','lighting_conditions','space_geometry photometric_assets placement_ref operating_conditions calculation_scope accepted_conditions','厂房环境准备与投用条件'),
 ('R18-ACOUSTICS','厂房环境','building_acoustic_conditions','space_geometry materials sources receiver_positions calculation_scope accepted_conditions','厂房/配套空间环境条件'),
 ('R23-HVAC','公用服务/厂房环境','hvac_service_plan','zones demand_profiles equipment_conditions supply_return_boundary electricity_scope accepted_conditions','暖通投用及冷热/电服务绑定'),
 ('R-MAINTENANCE','可用性/准备','maintenance_window_plan','asset_ref unavailable_windows task_resources completion_conditions requalification_scope','设备日历与重新投用'),
 ('R-REMAN','后续逆向工艺','remanufacturing_job','original_item_ref teardown_findings replacement_parts restoration_tasks accepted_scope','旧件/新件关系与修复工序'),
 ('R-NESTING','后续材料类型','nesting_job','stock_geometry part_geometry material_ref cutting_plan remnant_ref quantity_relations','板/卷到多件转换与余料库存'),
 ('R-DATA','离线输入','engineering_import_mapping','source_refs id_mapping units revisions missing_fields permitted_scope','工程对象导入与参数来源'),
 ('R-COSIM','专业协同','solver_coupling_plan','tool_refs input_output_bindings units time_basis boundary_events requested_scope','离线成果/软件步进与工厂事件'),
 ('R17-COLOR','包材质量','color_measurement_conditions','print_revision instruments reference_conditions target_values measurement_refs acceptance_scope','印刷准备与色彩检查条件')]

def main():
 topics=json.loads((B/'模块目录.json').read_text(encoding='utf-8'))
 byid={t['id']:t for t in topics};assert {x[0] for x in SPECS}==set(byid)
 rows=[]
 for ident,role,kind,fields,consumer in SPECS:
  t=byid[ident]
  rows.append(dict(topic_id=ident,name=t['name'],category=t['category'],integration_role=role,
                   artifact_type=kind,required_payload_fields=fields.split(),consumer=consumer,
                   subject_and_scope='声明对应产品/工艺/设备/场地版本与认可用途',
                   acceptance_owner=t['user'],existing_interface=t['interface'],scope_limits=t['boundary'],
                   research_sources=t['source_files'],current_provider_status='planned',
                   first_release_role=t['first_release_role'],mainline_integration=t['mainline_integration'],
                   actual_execution_result=None))
 document=dict(document_kind='professional_integration_design_not_plugin_registry',date='2026-10-13',
               common_fields=['artifact_id','artifact_type','schema_version','subject_ref','subject_revision',
                              'input_conditions','unit','coordinate_frame','supported_scope','content_ref',
                              'acceptance_ref','producer_result_ref'],modules=rows,
               note='41研究方向的接入设计；原专业范围继续保留，不表示全部已实现或纳入当前首版；只选纸包装真实专业闭环，目标生成调用受支持能力。')
 (B/'专业接入目录.json').write_text(json.dumps(document,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
 with (B/'专业接入目录.csv').open('w',encoding='utf-8-sig',newline='') as f:
  cols=['name','category','integration_role','artifact_type','required_payload_fields','consumer','current_provider_status','first_release_role']
  writer=csv.DictWriter(f,fieldnames=cols);writer.writeheader()
  for row in rows:writer.writerow({k:'; '.join(row[k]) if isinstance(row[k],list) else row[k] for k in cols})
 text=['# 专业成果怎样接到超级工厂', '首版仅纸包装实际专业闭环；外部Agent按行业目标/场地模板填九类角色后使用受支持提供者，其他40方向是研究储备，不是可调用内核。完整链见[目标生成规格](../软件工程/docs/03_架构与契约/目标驱动厂房生成.md)。','沿用41份专业定义，明确消费者和业务字段；没有把目录升级为已运行插件。',
       '[接入JSON](专业接入目录.json) · [接入CSV](专业接入目录.csv) · [完整业务契约](../软件工程/docs/03_架构与契约/专业成果接入与软件在环.md)',
       '共同字段用于说明成果及认可用途。具体专业字段保留原有几何/工艺计算边界；生产时钟、库存和资源仍归共同引擎。']
 for cat in dict.fromkeys(r['category'] for r in rows):
  text += ['## '+cat.split('_',1)[1], '|模块|工程角色|交给谁|业务字段|','|---|---|---|---|']
  for r in rows:
   if r['category']==cat:text.append('|'+r['name']+'|'+r['integration_role']+'|'+r['consumer']+'|'+'、'.join(r['required_payload_fields'])+'|')
 (B/'专业接入说明.md').write_text('\n\n'.join(text)+'\n',encoding='utf-8')
 print(json.dumps(dict(integration_designs=len(rows),implemented_providers=0),ensure_ascii=False))

if __name__=='__main__':main()
