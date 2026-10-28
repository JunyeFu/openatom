# H04-01 全部15分支授权合入差异

本轮唯一公共写入者是复审窗口1/W00。窗口1已合入并原生编译的私有Rust片段留档见[delta](DRF04_Host授权合入.delta.rs)，逐字段和15分支表/独立正反例见[JSON](DRF04_Host授权合入.json)。本文件与片段都不是已实现Host，也不产生新wire字段。

HostCallContext只增加edit_grants；每份grant绑定真实会话与Workspace或Edit Scope，包含分支、现有对象的合法叶字段/删除许可、新建类别/累计数量/来源版本、明确可复用资源及已创建ID。数组叶字段还钉住成员ID，索引变化不能把授权转给另一到料/订单。此最小上下文覆盖本批Q47/48要求，既有类型/单位/Schema、文件用途、人工认可规则保留。

共同Commands先验证请求形状、真实连接/Scope及base_revision，再用当前Host状态展开候选，归一成创建/叶变更/删除/资源复用与所用版本。15分支均检查完整候选的所有效果；无效整对象写入没有变更效果，但请求本身仍需有分支许可。单批可先新增再连线，权限与引用检查完成后只提交一次。出现任何PermissionDenied，工程、修订、历史与grant创建数量全部不变。

新建数量按grant累计不同ID计，分批不重置；删除/Undo不退还数量，Redo恢复本grant同ID仍要复核当前范围。新模板/模块实例取实际来源版本的基线，并检查传入覆盖；不把caller的source标签当可信参数。新建资源、改复用人员容量/日历和播种Scenario输入均需明确许可，设备数许可不包含这些效果。

CreateProject和Workspace打开采用依原契约仅实际UI来源；RecordArtifactAcknowledgment也仅实际UI。Agent声明origin/grant/认可一律不成为权限。CommitDefinitionVersion只冻结本地版本；Undo/Redo从Host真实历史展开，不能回滚认可、生产、文件或借旧宽权限修改新窄范围。

核验使用手工独立预期的候选效果模型，结果见[授权核验](DRF04_授权差异核验.json)。这验证授权判定/累计数量/原子拒绝的设计，不执行公共15分支实现；实际Host候选编译与Commands接线仍由窗口1合入。原生stdio/管道子集和真实产品/客户端证据另记，不相互替代。
