# B60 固定工位的建模修正

2026-10-23 当前结论：1.0.4 的四类实际等待、持有/接触/交接与保存搬移新进程重开已有独立 Native 证据。新 1.0.5 已修复立柱、灯塔、传感器线缆及固定夹具四处连接，普通 Native 七视角/修复近景、声明正反向动作和新版本缺料实际 Run/保存搬移重开通过有限验证；完整MQ仍开放。原始L03材质扩展加载拒绝已最小修复，C11普通Native同光照/同尺度对照后MQ-05限定通过，原GLB与旧Run不变；其余新版本等待Native矩阵及完整MQ/NFR仍分别核对。见[1.0.5状态](surface-105-repair.json)及[Native结果](../../../.work/cu-closeout-20261023/root-b60-105-native/README.md)。

MQ-08 的四个生产 Host 实际 Run（缺料、执行资源占用、日历连续窗口不足/关闭、输出空间阻塞）已全部完成 C09 普通 Native 所选 Job、物品/资源持有及双向回看核对；C09 文件菜单保存r5、正常关闭、搬移13文件一致、C10新进程打开原Run后1500/4000ms读回通过。时间轴新增明确所选作业过程入口；短底栏长等待详情改为内部滚动，固定播放栏及读点经 C10 复验。见 [本轮Native结果](../../../.work/cu-closeout-20261023/root-waits-native/README.md) / [JSON](../../../.work/cu-closeout-20261023/root-waits-native/receipt.json)。本精确定义为单个非抢占cycle、无辅助阶段资源，途中暂停及跨阶段等待不适用；平台完整MQ-08、MQ资产认可和正式NFR仍开放。

1.0.2 是修正候选，尚未通过完整 MQ、Native 联合接收或 NFR。原负载仍为三厂房、六条十工位产线、3000 个真实物品身份，每工位处理两秒；不得用本次几何检查代替详细质量认可或正式性能结果。

2026-10-22 在普通 Native 打开原 `p201.b60.station@1.0.1` 并检查斜视、背面、底部后，确认执行头沿 Y 偏离工件锚点 0.5m、导轨轴承未随滑台移动，以及面板、把手、百叶和驱动安装处缺少连续支撑。1.0.2 保留原行为、端口、工件支承点、展示槽偏移和 2×2×3m 外包络；补齐实际悬臂、随动轴承、与同阶段进度绑定的伸缩杆及安装支撑。旧版本与已保存 Run 保留，不改写旧运行的冻结模型。

`source/` 保存精确旧输入与可重建源；`candidate-1.0.2/` 为生成的详细/简化两个精确版本。`check.ts` 调用当前生产 `bindScene`，检查六个正向/反向时点的执行头对齐、轴承随动、伸缩杆连续接触、工件接触、落地、外包络，以及所修面板/电机/把手/百叶的实际几何连接；同时核对所有行为、端口和配置未改变。检查不创建 DOM 或渲染器。

从软件工程根运行，使用项目已有 Python、Node、Vite、Three.js，不安装依赖；将输出目录指向有足够空间的位置：

```powershell
python scenarios/非功能基准/station/source/generate.py
node scenarios/非功能基准/station/source/check-build.mjs C:/Temp/sf-b60-quality-check
node C:/Temp/sf-b60-quality-check/check.mjs scenarios/非功能基准/station C:/Temp/sf-b60-quality-check/result.json
```

当前实际结果及待验收项见 [repair.json](repair.json)。1.0.2 已完成普通 Native 的真实1200秒计算、独立保存和新进程读回，3000物品身份与原定接受时刻核对通过；完整七视角、机构/材质近景与 L03 对照、真实 holder/等待/回看仍须按 [MQ 规范](../../../docs/02_交互设计/设备建模精度与表现验收规范.json)完成；资产冻结前不得填 `benchmark_ready=true`。简化档保持原轻量用途，不用其替代详细档质量。

## 1.0.3 材质候选（2026-10-22）

增加圆角、型材真实槽孔、金属/涂层/橡胶材质、内嵌微表面和静态铭牌/面板。当前仍是待Native验收候选，不替换1.0.2已保存Run或冻结基准。生产GLB读取、6幅嵌入纹理解码和6个正反向动作采样通过，保留原2秒节拍、行为/端口/配置与2×2×3m包络。源生成曾出现matrix/TRS重建失位，已改为显式TRS导出并验证。

复现（Python须有已安装Pillow）：

```powershell
python -B scenarios/非功能基准/station/source/create-surfaces.py
node scenarios/非功能基准/station/source/build-surfaces.mjs .work/b60-surface-build
node scenarios/非功能基准/station/source/check-surfaces.mjs .work/b60-surface-check
```

检查使用现有Pillow像素适配器，不等价GPU贴图/视觉/MQ/NFR通过。后续先在实际Native明确采用新版本，完成七视角、近景、机构/工件和保存重开，再冻结性能输入。结果见surface-repair.json。

2026-10-22 Native更新：C07已完成1.0.3普通保存和新进程重开，C08补齐斜视及六面入口并完成同版本七视图。正面近景发现控制屏文字上下颠倒，故1.0.3完整MQ仍未通过；新1.0.4源/GLB已修正贴图映射，真实生产解码、UV方向与六个正反动作采样通过，普通Native待验。当前下一素材候选与剩余要求见[surface-104-repair.json](surface-104-repair.json)，旧Run/1.0.3不改写。


2026-10-23 普通 Native 有限验证：1.0.4在真实Run `B60-surface-8752:run:1:2` 上核对L1-D01第三个Job：3000ms资源忙等待、0%且无实际持有；5000ms cycle 50%、ITEM-0003绑定本Job且近景接触一致；6000ms旧Job完成、资源释放、物品交接D02，不将下一件物品挂回历史动作。通过文件菜单保存r5，正常关闭Host43100后搬移本轮副本，再由新Host51084打开并重选原Run，读回1.0.4、155994事件/30000作业/3000接受记录。冻结工程、冻结方案、Run结果和GLB字节保持。证据见[Native回执](../../../.work/cu-closeout-20261022/root-b60-native/C08-104-native-receipt.json)。完整MQ及正式NFR仍未通过。

2026-10-23 续接：已把额度停止前的简化LOD 5000→3000→5000ms反向定位、resource_busy等待无持有及恢复同一作业姿态的四帧证据归档；区域档明确line_whole且保留作业/阶段/资源身份。本轮新Host41288/window131366经文件菜单打开原已搬移r5目录，重新选择同一Run；337对象、4精确定义、1.0.4和155994事件/30000作业/3000接受记录读回一致，无工程编辑。本轮未发现新产品缺陷。缺料、阶段等资源、途中中断、输出阻塞的完整姿态矩阵及原MQ/NFR/IX仍开放；P3-03保持in_progress。 证据：`.work/cu-closeout-20261022/root-b60-native/104-lod-reverse/receipt.json`。
