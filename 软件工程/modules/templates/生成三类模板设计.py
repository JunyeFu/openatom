"""生成可检查的模板设计与匹配模块声明，不生成运行引擎或实际专业结果。"""
import copy,json,os
from pathlib import Path

BASE=Path(__file__).resolve().parent
PROJECT=BASE.parent.parent
def read(path):
    return json.loads(path.read_text(encoding='utf-8'))
def save(path,value):
    path.parent.mkdir(parents=True,exist_ok=True)
    path.write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')

# 旧样例保留；新配方使用新定义，不能把两输入模块偷偷当三输入模块。
assembly=read(PROJECT/'modules/devices/manual-assembly/manifest.json')
control=copy.deepcopy(assembly)
control.update(id='community.control-box-assembly',title='壳体、PCB与螺钉齐套装配')
control['ports'].insert(2,dict(id='fastener',kind='material',direction='in',item='fastener',unit='piece'))
control['behavior']['inputs'].append(dict(port='fastener',quantity=4))
control['behavior']['duration']['value']=40

board=copy.deepcopy(assembly)
board.update(id='community.declared-board-assembly',title='电子板件组装流转（声明过程）')
board['ports']=[
    dict(id='bare_board',kind='material',direction='in',item='bare_pcb',unit='piece'),
    dict(id='component',kind='material',direction='in',item='component',unit='piece'),
    dict(id='mounted',kind='material',direction='out',item='mounted_pcb',unit='piece')]
board['behavior']['inputs']=[dict(port='bare_board',quantity=1),dict(port='component',quantity=8)]
board['behavior']['outputs']=[dict(port='mounted',quantity=1)]
board['behavior']['duration']['value']=30
board['behavior']['resources']=[]
board['behavior']['start']='declared_preparation_ready_and_inputs_available'
board['professional_contract']=dict(artifact_type='placement_job',
    fields=['board_revision','coordinate_frame','component_library_ref','placements','fiducials','feeders','nozzles','machine_profile','native_job_ref','phase_plan'],
    declared_scope='aggregate_material_flow_only_not_actual_placement_or_solder_quality',
    tool_result_required_for=['calculated_nominal','software_in_loop'],provider_status='not_implemented')
board['design_notes']='声明过程一次领用一板八部件；实际专业模式由工厂逐事件提交领用，不能再叠加声明过程领用。'

pack=read(PROJECT/'modules/devices/carton-pack/manifest.json')
pack.update(id='community.parameterized-pack',title='指定物品的单件装盒')
pack['parameters'].insert(0,dict(id='product_item',type='item_type',binding='configuration.product_item'))
for port in pack['ports']:
    if port['id']=='product':
        del port['item'];port['item_parameter']='product_item'

inspect=read(PROJECT/'modules/devices/inspect-route/manifest.json')
inspect.update(id='community.parameterized-inspect',title='指定物品的检验与处置')
inspect['parameters'].insert(0,dict(id='product_item',type='item_type',binding='configuration.product_item'))
for port in inspect['ports']:
    del port['item'];port['item_parameter']='product_item'

definitions={'control-box-assembly':control,'declared-board-assembly':board,
             'parameterized-pack':pack,'parameterized-inspect':inspect}
for folder,manifest in definitions.items():
    manifest['definition_state']='design'
    target=PROJECT/'modules/devices'/folder
    # 表现来自本项目原生声明。模板不借用没有授权的设备照片。
    scene=copy.deepcopy(read(PROJECT/'modules/devices/manual-assembly/presentation/native-scene.json'))
    scene['id']=folder+'-scene';scene['port_anchors']=[]
    for i,port in enumerate(manifest['ports']):
        scene['port_anchors'].append(dict(port=port['id'],node='table',local_position=[-.5 if port['direction']=='in' else .5,.08,(i%3-1)*.2]))
    manifest['presentation']['media']=dict(kind='native_scene',path='presentation/native-scene.json')
    manifest['presentation'].pop('timeline',None)
    manifest['contribution']=dict(author='待填写',license='待声明',assets=[],
        status='design',source_basis='本项目已有声明规则和自建原生几何，未发布运行模块')
    save(target/'manifest.json',manifest);save(target/'presentation/native-scene.json',scene)
    (target/'README.md').write_text('# '+manifest['title']+'\n\n状态：匹配配方/端口的设计定义，不是可运行模块。输入输出按manifest端口；行为、参数与资源按实例明确绑定。原生部件尺寸只是本项目表现假设，没有设备照片或现场工艺参数。作者和许可发布前填写，actual_result未取得。\n\n'+
        ('电子板仅声明一板八部件的物料流。准备、实际Job/板位/供料器/吸嘴、后续处理和放行分别有条件；不以30秒动作替代完整专业执行或电气质量。\n' if folder=='declared-board-assembly' else '')+
        '模板与使用边界见[三类工厂](../../templates/README.md)，旧两输入/固定物品样例保持原定义。\n',encoding='utf-8')

def module_ref(module_id,project_file):
    folder={'community.fixed-transform':'fixed-transform',**{v['id']:k for k,v in definitions.items()}}[module_id]
    return dict(id=module_id,version='0.1.0',manifest=Path(os.path.relpath(PROJECT/'modules/devices'/folder/'manifest.json',project_file.parent)).as_posix(),status='declaration_design')

def device(device_id,module_id,params,bindings,resources,phases=None):
    d=dict(id=device_id,kind='fixed',module=module_id,process_mode='declared_process',
        parameters=params,port_bindings=bindings,job_resources=[device_id]+resources)
    role={'community.control-box-assembly':'assembler','community.parameterized-inspect':'inspector','community.parameterized-pack':'packer'}.get(module_id)
    if role:d['resource_bindings']={role:resources[0]}
    if phases is not None:d['phases']=phases
    return d

def select_packaging_branch(packing_device,mode,selected_job=None):
    """Pure selection: no writes/registration/Job lookup, no default latest Job."""
    candidate=copy.deepcopy(packing_device)
    if mode=='declared_process':
        return candidate,None
    if mode!='calculated_nominal':
        raise ValueError('Only declared_process/calculated_nominal are selected here')
    definition_file=PROJECT/'packages/packaging/consumer-device.design.json'
    definition=read(definition_file)['data']['definition']
    if (definition['id'],definition['version'])!=('community.parameterized-pack','0.2.0'):
        raise ValueError('Explicit consumer-device 0.2.0 required; do not choose latest')
    declarations={p['id']:p for p in definition['parameters']}
    parameter_ids=('packaging_job_id','packaging_job_revision')
    selected={} if selected_job is None else selected_job
    missing=[key for key in parameter_ids if selected.get(key) is None or selected.get(key)=='']
    candidate['process_mode']=mode
    candidate['module_ref']=dict(id=definition['id'],version=definition['version'])
    candidate['configuration']={}
    if not missing:
        job_id=selected['packaging_job_id'];revision=selected['packaging_job_revision']
        if not isinstance(job_id,str) or not job_id:
            raise ValueError('Explicit Job ID must be a nonempty string')
        if type(revision) is not int or not 0<=revision<=9007199254740991:
            raise ValueError('Explicit Job revision must be a SafeUint integer')
        candidate['configuration']={
            'packaging_job_id':dict(kind='string',value=job_id),
            'packaging_job_revision':dict(kind='integer',value=revision)}
    selection=dict(definition_ref='packages/packaging/consumer-device.design.json',
        module_ref=candidate['module_ref'],selected_profile='paper_packaging_complete_v1',
        parameter_declarations=[copy.deepcopy(declarations[key]) for key in parameter_ids],
        missing_selection=missing,actual_result=None,
        execution_gate='unregistered design version; requires actual selected Job/result/profile/acceptance/binding; no inline-time fallback',
        legacy_parameters_scope='existing cycle_seconds/product_item retained as old declared input, not a calculated-nominal result')
    return candidate,selection

def build(folder,title,kind,packing_mode='declared_process',packaging_job_selection=None):
    target=BASE/folder;project_file=target/'project.design.json'
    item_ids=['blank','shell','carton','package'] if kind=='machining' else ['shell','pcb','fastener','assembly','carton','package']
    if kind=='parallel':item_ids=['blank','shell','bare_pcb','component','mounted_pcb','pcb','fastener','assembly','carton','package']
    stock={'blank':2,'carton':2} if kind=='machining' else {'shell':2,'pcb':2,'fastener':8,'carton':2}
    if kind=='parallel':stock={'blank':2,'bare_pcb':2,'component':16,'fastener':8,'carton':2}
    workers=['W-B']+(['W-C'] if kind!='purchased' else [])+(['W-A'] if kind!='machining' else [])+(['W-E'] if kind=='parallel' else [])
    devices=[];connections=[];artifacts=[];preparation=[]
    if kind!='purchased':
        devices.append(device('CNC01','community.fixed-transform',{'input_item':'blank','output_item':'shell','cycle_seconds':60},
            {'input':'store-blank','output':'store-shell'},[],[
             dict(id='load',duration_ms=10000,phase_resources=['W-C']),
             dict(id='auto',duration_ms=45000,phase_resources=[]),
             dict(id='unload',duration_ms=5000,phase_resources=['W-C'])]))
        artifacts.append(dict(id='machining-job',artifact_type='machining_job',accepted_scope='declared_process_only',
            required_fields=['product_revision','stock','tools','fixture','nc_dialect','nominal_geometry','phase_plan'],producer_result=None))
        devices[-1]['artifact_ref']='machining-job'
    if kind=='parallel':
        preparation.append(dict(id='board-preparation',target='Board01',duration_ms=30000,resources=['W-E'],basis='自建准备任务，仅做一次'))
        devices.append(device('Board01','community.declared-board-assembly',{'cycle_seconds':30},
            {'bare_board':'store-bare_pcb','component':'store-component','mounted':'store-mounted_pcb'},[],[
             dict(id='load',duration_ms=5000,phase_resources=['W-E']),
             dict(id='placement',duration_ms=20000,phase_resources=[]),
             dict(id='unload',duration_ms=5000,phase_resources=['W-E'])]))
        devices[-1]['ready_after']='board-preparation';devices[-1]['artifact_ref']='placement-job'
        devices.append(device('BoardFinish01','community.fixed-transform',
            {'input_item':'mounted_pcb','output_item':'pcb','cycle_seconds':30},
            {'input':'store-mounted_pcb','output':'store-pcb'},[]))
        devices[-1]['scope']='用户明确声明的后续处理/放行；不推导焊接、电气测试或真实工艺质量'
        placements=[dict(id='placement-'+str(i+1),component='component',x_mm=10+(i%4)*20,y_mm=10+(i//4)*20,rotation_deg=0) for i in range(8)]
        artifacts.append(dict(id='placement-job',artifact_type='placement_job',accepted_scope='declared_process_only',
            board_revision='self-board-r1',coordinate_frame='board_local_mm',board_size_mm=[100,60],
            placements=placements,fiducials_mm=[[5,5],[95,55]],feeders=[dict(id='F1',component_store='store-component')],
            nozzles=['N1'],machine_profile='self-declared-profile',component_library_ref=None,native_job_ref=None,
            missing_actual_professional_inputs=['actual_component_library','native_job','supported_machine_profile'],
            phase_plan=[5,20,5],producer_result=None))
    if kind!='machining':
        devices.append(device('Assembly01','community.control-box-assembly',{'cycle_seconds':40},
            {'shell':'store-shell','pcb':'store-pcb','fastener':'store-fastener','assembled':'store-assembly'},['W-A']))
    product='shell' if kind=='machining' else 'assembly'
    devices.append(device('Inspection01','community.parameterized-inspect',{'product_item':product,'cycle_seconds':10,'max_reworks':0},
        {'input':'store-'+product,'passed':'store-released','rework':'store-isolate','quarantine':'store-isolate'},['W-B']))
    devices[-1]['quality_source']='preset_PASS_for_nominal_demo_not_field_inspection'
    devices.append(device('Packing01','community.parameterized-pack',{'product_item':product,'cycle_seconds':20},
        {'product':'store-released','carton':'store-carton','packed':'store-package'},['W-B']))
    devices[-1]['input_quality']={product:'released'}
    devices[-1],packaging_selection=select_packaging_branch(devices[-1],packing_mode,packaging_job_selection)
    stores=[dict(id='store-'+i,hall='H1',item_type=i,capacity=16 if i in ['component','fastener'] else 4,unit='piece') for i in item_ids]
    stores += [dict(id='store-released',hall='H1',item_type=product,capacity=2,unit='piece'),
               dict(id='store-isolate',hall='H1',item_type=product,capacity=2,unit='piece')]
    layout={'CNC01':[4,8],'Board01':[4,4],'BoardFinish01':[8,4],'Assembly01':[12,8],'Inspection01':[15,8],'Packing01':[18,8]}
    for d in devices:d['layout_m']=layout[d['id']]
    for i,s in enumerate(stores):s['layout_m']=[2+(i%6)*3,2+(i//6)*2]
    for d in devices:
        if d['id']=='BoardFinish01':d['output_quality']=dict(pcb='released',source='declared_preset_for_flow_only')
    refs=[module_ref(i,project_file) for i in dict.fromkeys(d['module'] for d in devices)]
    if packaging_selection is not None:
        for ref in refs:
            if ref['id']==packaging_selection['module_ref']['id']:
                ref.update(version='0.2.0',status='unregistered_design_candidate',
                    manifest=Path(os.path.relpath(PROJECT/'packages/packaging/consumer-device.design.json',project_file.parent)).as_posix(),
                    definition_format='canonical_ModulePackage_1.0')
    lines=[dict(id='L1',hall='H1',members=[d['id'] for d in devices])]
    if kind=='parallel':
        lines=[dict(id='L-shell',hall='H1',members=['CNC01']),
               dict(id='L-board',hall='H1',members=['Board01','BoardFinish01']),
               dict(id='L-final',hall='H1',members=['Assembly01','Inspection01','Packing01'])]
    for d in devices:
        manifest=next((m for m in definitions.values() if m['id']==d['module']),None)
        if manifest is None:manifest=read(PROJECT/'modules/devices/fixed-transform/manifest.json')
        for port in manifest['ports']:
            item=port.get('item') or d['parameters'][port['item_parameter']]
            endpoint=d['id']+'.'+port['id'];store=d['port_bindings'][port['id']]
            connections.append(dict(kind='material',item_type=item,
                **({'from':store,'to':endpoint} if port['direction']=='in' else {'from':endpoint,'to':store})))
    save(project_file,dict(document_kind='factory_template_project_design_not_runtime',schema_revision='draft-3-template',
        id='template-'+folder,title=title,status='design',actual_result=None,
        instantiation_contract_ref='modules/templates/instantiation.design.json',
        canonical_file_conversion='explicit design-to-domain mapping required; this design JSON is not Project file format 1.0',
        adoption_preview=dict(exact_module_versions=True,new_object_ids_and_references=True,explicit_existing_resource_bindings=True,supply_and_order_choices_separate=True,default_explicit_new_scenario_seed=None),
        unresolved_geometry_rule='absent height/footprint/clearance stays unresolved; existing XY plan alone is not site Pass',
        assumptions=['工时、尺寸、容量、物料与质量预设均为自行设计输入','工位间移交在本例为瞬时，不计算未声明走动/运输',
                     '专业完整输入与实际结果分开；当前仅声明流转，不计算名义几何/贴装/焊接或现场质量'],
        time=dict(unit='millisecond',origin='scenario_start',observation_end=300000),
        demand=dict(item_type='package',quantity=2,unit='piece',destination='store-package',acceptance_scope='factory_store_not_customer'),
        item_types=[dict(id=i,unit='piece',spec_ref='self-'+i+'-r1') for i in item_ids],
        halls=[dict(id='H1',width_m=24,depth_m=16,height_m=None)],layout_scope='self_declared_plan_positions_not_clearance_or_safety_validation',
        resources=[dict(id=w,kind='worker',capacity=1,calendar_ms=[[0,300000]]) for w in workers],
        stores=stores,initial_stock=[dict(store='store-'+i,item_type=i,quantity=q,unit='piece') for i,q in stock.items()],
        module_refs=refs,devices=devices,preparation_tasks=preparation,professional_artifacts=artifacts,
        lines=lines,
        connections=connections,quality_inputs=['PASS','PASS'],quality_source='preset',
        import_policy=dict(new_project='use_explicit_demo_seed',into_existing_project='layout_and_definitions_only_stock_and_demand_zero_until_chosen',
            object_ids='new_namespace_and_rewrite_references',item_types='explicit_match_of_id_unit_spec_not_display_name'),
        entry_exit=dict(input_types=list(stock),output_type='package',track_start='any_named_material_endpoint',track_end='declared_legal_store_or_operation'),
        professional_modes=dict(selected='declared_process',calculated_nominal='requires_actual_supported_producer_result',
            software_in_loop='requires_actual_software_target_and_event_contract',silent_downgrade=False),
        **({'selected_packaging_branch':packaging_selection} if packaging_selection is not None else {})))
    save(target/'manifest.json',dict(document_kind='factory_template_design_not_runtime_module',id='community.template-'+folder,
        version='0.1.0',category='templates',title=title,project='project.design.json',status='design',definition_state='design',
        instantiation_contract_ref='modules/templates/instantiation.design.json',
        user_decisions=['产品/规格与BOM','专业模式和认可输入','设备/人员/日历','布局与运输模式','物料供给与质量','观察窗口与接受位置'],
        contribution=dict(author='待填写',license='待声明',assets=[])))
    (target/'README.md').write_text('# '+title+'\n\n[工程声明](project.design.json) · [模板清单](manifest.json)。这是独立设计实例，不是运行插件。选择新工程时明确采用演示初始物料；插入已有工程只加入布局/定义，物料与需求默认不复制。\n\n专业模式当前declared_process；切换名义计算或软件在环必须有对应工具/目标的实际结果，不静默退回工时动画。质量是预设，库位接受不是客户履约。\n\n'+
        ('完整电子专业输入保留板件版本、坐标/位置表、基准、供料器、吸嘴、机器与准备/阶段；声明后续处理不证明焊接、电气质量。采购完成PCB是明确的替代路线，不把它计成已运行电子装联支线。\n\n' if kind=='parallel' else '')+
        '采用ID/资源/种子规则见[实例化契约](../instantiation.design.json)。\n\n见[三类模板业务与边界](../../../docs/01_业务需求/三类工厂模板与接手.md)。本文件是可编辑通用静态模板；[目标生成](../../../docs/03_架构与契约/目标驱动厂房生成.md)另用可选行业配置填充九类角色，纸箱制造配置独立保留。当前actual_result为空，时间预期由独立设计推导说明。\n',encoding='utf-8')

build('machining-delivery','机加工检验与包装交付','machining')
build('purchased-assembly','外购件齐套总装与包装','purchased')
build('parallel-electromechanical','机电支线并行制造与总装','parallel')

catalog_path=PROJECT/'modules/catalog.json';catalog=read(catalog_path)
new_ids={m['id'] for m in definitions.values()}|{'community.template-'+s for s in ['machining-delivery','purchased-assembly','parallel-electromechanical']}
catalog['modules']=[m for m in catalog['modules'] if m['id'] not in new_ids]
for folder,m in definitions.items():
    catalog['modules'].append(dict(id=m['id'],category='devices',manifest='devices/'+folder+'/manifest.json',
        readme='devices/'+folder+'/README.md',status='design_example',design_capabilities=['explicit_recipe_ports','native_scene']))
for folder in ['machining-delivery','purchased-assembly','parallel-electromechanical']:
    catalog['modules'].append(dict(id='community.template-'+folder,category='templates',manifest='templates/'+folder+'/manifest.json',
        readme='templates/'+folder+'/README.md',status='design_example',design_capabilities=['independent_project_design','explicit_import_policy']))
save(catalog_path,catalog)
print('3 independent template designs and 4 recipe-matched device definitions; no runtime or professional execution result.')
