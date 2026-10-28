# 纸包装结构与过程模块

`module.json`是现行`Versioned<ModulePackage>`，专业类别只声明输入/成果与支持范围。实际可用模式必须来自Host登记的Rust提供者，不能用这份文件自行宣布已安装或可算。当前为`runtime_candidate`；作者/许可尚待项目统一确认，不标发布或可再分发。

## 可编辑输入

`templates/`中的每份文件均为同一公共`Versioned<PackageStructureJob>`：

- `blank.job.json`：空白受限结构。先明确材料/板片/构件/树/过程/条件及数值档，空结构不能计算为Pass。
- `restricted-folded-container.job.json`：底、四壁、四个真实折带，先形成再打开前壁，固定姿态直线取物。
- `outer-box-inner-liner.job.json`：两份独立blank，分别成型、内衬实际放入、两前壁打开、保留所有相关障碍。
- `cover.job.json`：托盘与罩，形成后实际上移再侧移，为取物留出路径。
- `sliding-sleeve-explicit-candidate.job.json`：另一个明确改动后的滑套候选，末两带2.875mm，局部下搭0.125mm，全部ID/几何/连接框架写在Job中。历史2.9mm候选未被自动吸附或覆盖。

模板编译后的结果包含实际板片多边形、厚度、flat_pose、折带轴/宽度、连接/局部许可、显式part及步骤，全部可以继续修改。编辑沿现有Host的`PutPackagingJob`路径提交一个明确新修订；没有另建模板专用的不可编辑几何格式。

新增板/折带/连接/part直接修改相应公共列表并维护ID引用；多次生产不改变专业材料数量，重复件须真的复制part、板/折带/内部连接/许可/过程ID及引用。改变认可材料时一并改对应所有板厚。产品改版同时更新`subject_ref`和包络的`subject`，旧成果的认可不复制到新版。

## 实际生产者接口

`sf_packaging::compute_packaging(&job, &explicit_budget, context, &cancelled, &mut progress)`返回实际选定Job、`PackagingResult`、`ArtifactRecord`、制造文件bytes、提供者能力及取消状态。内核不写文件。Host保存实际冻结输入/结果/bytes，人工认可、明确绑定、生产Run消费分别处理。

过程回放读取结果中的`step_id / progress / panel_id / Pose`，单位mm、Z向上；这是专业几何参数轴。生产工时仍读取工厂阶段资源/工作区间，不把这段预览或`duration_hint_ms`当成工时或库存变化。

Pass要求全部请求用途和制造实际完成。严格内部交点或闭合/保持约束反例产生Fail；缺输入、未实现的几何范围、数值边界、预算/取消保持Undetermined。每次完整Job共用一次预算。实际材料面积从blank生成，胶层面积不混入纸材面积。

## 当前验证与接收

内核测试位于`packages/packaging/tests/kernel_units.rs`，独立消费者验收由窗口5维护`integration_acceptance.rs`。模板与Rust计算通过不等于Host保存/采用/消费者/报告/前端IX-03已经接收；整包状态以`docs/04_开发与验收/整包派发/P4-02_整包执行.json`为准。

许可证和素材没有确认前仍保留未知；本模块不夹带第三方商业模型或包装专利适用性结论。
