export interface BusinessSection { id: string; title: string; owner: 'project' | 'scenario'; path: string[]; type: string; note: string; detail?: string }
export const sections: BusinessSection[] = [
  {id:'orders',title:'任务与订单',owner:'scenario',path:['orders'],type:'OrderLine',note:'设置订单的产品、数量、交期和接收位置。',detail:'优先级数值越大越先安排。订单与生产任务分别设置优先级；填写交期后可计算准时率。'},
  {id:'tasks',title:'生产任务与分配',owner:'scenario',path:['production_tasks'],type:'ProductionTask',note:'设置计划产量，并将生产任务分配给订单。预留数量与实际接收数量分别记录。'},
  {id:'products',title:'产品与版本',owner:'project',path:['products'],type:'ProductRevision',note:'选择产品对应的物品、用料清单和工艺路线版本。已有运行记录保留当次输入。'},
  {id:'boms',title:'BOM用料',owner:'project',path:['boms'],type:'BomDefinition',note:'逐项设置用料及其消耗工序。'},
  {id:'routes',title:'工艺路线',owner:'project',path:['routes'],type:'RouteDefinition',note:'设置各工序的顺序、执行设备和工艺版本。'},
  {id:'arrivals',title:'后续到料',owner:'scenario',path:['arrivals'],type:'ArrivalPlan',note:'设置到料时间和数量，未到料的物品不能提前领用。'},
  {id:'initial',title:'初始物品输入',owner:'scenario',path:['initial_conditions','items'],type:'InitialItem',note:'设置方案开始时的物品、数量和来源，不改变当前生产库存。'},
  {id:'allocations',title:'显式物料预留',owner:'scenario',path:['initial_conditions','material_allocations'],type:'MaterialAllocation',note:'按物品编号和数量预留已有物料。'},
  {id:'quality',title:'质量结果预设',owner:'scenario',path:['quality_presets'],type:'InspectionPreset',note:'按检验顺序设置结果，未知结果保持待判。'},
  {id:'decisions',title:'声明质量决定',owner:'scenario',path:['initial_conditions','declared_quality_decisions'],type:'QualityDecision',note:'设置新方案中的放行、复检、返工、隔离或报废决定，以及生效时间和依据。'},
  {id:'calendars',title:'班次与日历',owner:'project',path:['calendars'],type:'Calendar',note:'设置可工作时段，包含开始时间、不包含结束时间。'},
  {id:'resources',title:'人员与共享资源',owner:'project',path:['resources'],type:'ResourceInstance',note:'设置人员和共享资源的角色、资质、数量、班次与地点。'},
  {id:'device_bindings',title:'设备角色与工艺选择',owner:'project',path:['devices'],type:'DeviceInstance',note:'选择设备使用的资源和工艺版本。设备参数在“设备阶段与参数”中修改。'},
  {id:'locations',title:'库位与接受能力',owner:'project',path:['locations'],type:'StockLocation',note:'设置库位用途、接收类型和容量。容量未填写时表示未知。'},
  {id:'items',title:'物品类型',owner:'project',path:['item_types'],type:'ItemType',note:'设置物品类型、版本与计量单位。库存数量在方案中填写。'},
  {id:'setup',title:'家族与换单',owner:'project',path:['setup_policies'],type:'DeviceSetupPolicy',note:'设置不同产品家族之间的换单时间、准备方式和首件要求。'},
  {id:'supplies',title:'供应与运输输入',owner:'scenario',path:['supplies'],type:'SupplyRelation',note:'设置供料目标、发运时段和运输方式。实际发出、在途、接收及返库数量分别记录。'},
  {id:'overrides',title:'方案参数覆盖',owner:'scenario',path:['parameter_overrides'],type:'ParameterOverride',note:'此页仅供查看。修改参数请进入“设备阶段与参数”；清除不支持的覆盖后，请保存并重新运行。'},
  {id:'costs',title:'成本与计价来源',owner:'scenario',path:['cost_inputs'],type:'CostInput',note:'填写单价、币种和计费依据。未填写的成本保持未知，不影响时间模拟；同一项工作请勿重复计费。'},
  {id:'sources',title:'来源记录',owner:'project',path:['sources'],type:'SourceRecord',note:'记录数据来源。来源信息与专业用途认可分别管理。'}
];
