export interface BusinessSection { id: string; title: string; owner: 'project' | 'scenario'; path: string[]; type: string; note: string }
export const sections: BusinessSection[] = [
  {id:'orders',title:'任务与订单',owner:'scenario',path:['orders'],type:'OrderLine',note:'优先序越大越先；订单与任务的显式优先序分别展示；释放不是实际开工，交期留空时不计算准时率。'},
  {id:'tasks',title:'生产任务与分配',owner:'scenario',path:['production_tasks'],type:'ProductionTask',note:'计划数量与订单分配分开；显式预留不代表实际接受。'},
  {id:'products',title:'产品与版本',owner:'project',path:['products'],type:'ProductRevision',note:'明确物品、BOM及路线版本；已有Run仍使用原冻结输入。'},
  {id:'boms',title:'BOM用料',owner:'project',path:['boms'],type:'BomDefinition',note:'逐行指定消耗工序；不将全BOM自动绑定首工序。'},
  {id:'routes',title:'工艺路线',owner:'project',path:['routes'],type:'RouteDefinition',note:'明确前置、执行设备、工艺版本及角色；不按画面位置推断路径。'},
  {id:'arrivals',title:'后续到料',owner:'scenario',path:['arrivals'],type:'ArrivalPlan',note:'预计到达与真实接受分开；未来到料不能提前领用。'},
  {id:'initial',title:'初始物品输入',owner:'scenario',path:['initial_conditions','items'],type:'InitialItem',note:'仅定义下一方案起点，不编辑当前实际库存；数量和来源必须明确。'},
  {id:'allocations',title:'显式物料预留',owner:'scenario',path:['initial_conditions','material_allocations'],type:'MaterialAllocation',note:'预留物品ID与数量；不能由目标数量制造库存。'},
  {id:'quality',title:'质量结果预设',owner:'scenario',path:['quality_presets'],type:'InspectionPreset',note:'按实际检查尝试顺序消费；未知保持待判，不自动合格。'},
  {id:'decisions',title:'声明质量决定',owner:'scenario',path:['initial_conditions','declared_quality_decisions'],type:'QualityDecision',note:'配置新方案的明确事件；放行/复检/返工/隔离/报废保留来源和生效时间，不修改运行内实物。'},
  {id:'calendars',title:'班次与日历',owner:'project',path:['calendars'],type:'Calendar',note:'可用区间为[开始,结束)，人员日历不等于机器持有时间。'},
  {id:'resources',title:'人员与共享资源',owner:'project',path:['resources'],type:'ResourceInstance',note:'声明角色、资格、单位、容量、日历与地点；动画不增加工时。'},
  {id:'device_bindings',title:'设备角色与工艺选择',owner:'project',path:['devices'],type:'DeviceInstance',note:'绑定已存在资源和精确工艺版本；布局与实例参数由原字段保留，实例参数在阶段面板提交。'},
  {id:'locations',title:'库位与接受能力',owner:'project',path:['locations'],type:'StockLocation',note:'待判、隔离、输出与普通领用分开；不从空白容量推断无限支持。'},
  {id:'items',title:'物品类型',owner:'project',path:['item_types'],type:'ItemType',note:'类型定义不是库存；引用版本及声明单位须匹配。'},
  {id:'setup',title:'家族与换单',owner:'project',path:['setup_policies'],type:'DeviceSetupPolicy',note:'序列相关换单、内部/外部准备及首件门槛；不把换单算成每件加工时间。'},
  {id:'supplies',title:'供应与运输输入',owner:'scenario',path:['supplies'],type:'SupplyRelation',note:'目标与实际发出/在途/接受/返库分开；时间来自明确精确版本及声明，不按屏幕距离计算。'},
  {id:'overrides',title:'方案参数覆盖',owner:'scenario',path:['parameter_overrides'],type:'ParameterOverride',note:'本版运行尚不支持独立方案覆盖，此处仅查看已有内容。修改设备实例参数请用“设备阶段与参数”；清除不受支持的覆盖后，须保存并明确重跑。'},
  {id:'costs',title:'成本与计价来源',owner:'scenario',path:['cost_inputs'],type:'CostInput',note:'未知价格/币种保持未知，成本缺项不阻止时间彩排；同一劳动不得重复计价。'},
  {id:'sources',title:'来源记录',owner:'project',path:['sources'],type:'SourceRecord',note:'企业声明、人工填写、模块声明和模板示例分开；来源记录不代表实际认可。'}
];
