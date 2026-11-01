# S06-04隔离结构、过程和制造方法试做

实际输出在[output](output)，代码只写本目录；不修改公共类型/Commands、原S05试例或产品进度，不安装依赖。

- [32个独立预期](finite-expectations.json)覆盖非空局部区域、板面/深度/步骤/双面ID、转动过程、祖先关节传播、固定姿态取物、连接激活/后续保持、制造、有限搜索和数值边界。
- [实际方法结果](output/proof-result.json)使用Fraction精确有理区间、带余项的sin/cos；矩形单元先投影顶点后取范围，禁止单元碰撞须严格内部点证据。未定静态阈值、预算耗尽保持Undetermined。
- [公共Job](output/local-contact-job.json)含2实际板、1非空LocalContactAllowance、1真实连接和1动作；PUBLIC01直接从这些字段建立全approach区间局部检查，**只证明该接触子问题**。其[API请求](output/local-contact-api-request.json)和Job经过现行Schema验证，旧ID对负例被拒绝；不调用Host。
- [实际结构展开结果](output/structure-expansion-result.json)生成三个机制的5/10/10板、4/8/8折带和1/2/2构件，并生成产品/材料/明确复制三变化。六个Job通过现行Schema；未运行这六个Job完整几何。cover/sliding_sleeve有限样例选cover分支，滑套的实际闭缝/局部连接仍须所选输入与过程核对。
- [SVG独立回读](output/svg-readback-result.json)读实际文件核对刀线64mm、折线10mm、实际共同胶投影100mm²、mm图层及连接/区域/步骤来源。缺胶区不造图层，重叠展开不当合法并集。

在软件工程根执行：

```powershell
& 'C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe' -B -X utf8 tools/packaging-algorithm-proof/build_fixture.py
& 'C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe' -B -X utf8 tools/packaging-algorithm-proof/construct_seeds.py
& 'C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe' -B -X utf8 tools/packaging-algorithm-proof/prove.py
& 'C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe' -B -X utf8 tools/packaging-algorithm-proof/inspect_outputs.py
```

Schema校验复用只读`tools/type-contract-proof/python-libs`，其rpds为CPython3.12二进制，须用已配置3.12.14 runtime。CPython3.14不可加载它，未为此安装/覆盖依赖；`-B`避免写共享bytecode。几何证明脚本本身只需标准库，也已在3.14.7实际执行。

这里没有通用多边形裁剪产品实现、完整Rust f64向外舍入内核、全部三机制运行集成或工业材料证据。保留原两Cuboid TOI输入/预期/结果，不能用本目录文件数量宣布S06冻结或SF通过。正式规格见[过程算法](../../docs/03_架构与契约/纸包装结构与过程算法.md)、[数值制造](../../docs/03_架构与契约/纸包装数值与制造规则.md)与[公共交接](../../docs/04_开发与验收/S06-04_公共契约交接.md)。

## 实际Rust数值原语

```powershell
rustc --edition 2024 tools/packaging-algorithm-proof/float_interval.rs -o tools/packaging-algorithm-proof/output/float_interval.exe
& 'C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe' -B -X utf8 tools/packaging-algorithm-proof/check_float_intervals.py
```

[16组实际输出与独立精确参考](output/float-interval-result.json)支持加/乘/除/严重消去及六种参数区间的sin/cos原语包围；不证明完整矩阵、裁剪或纸包装Rust内核。真实输出保留原始端点；宽消去区间不能作高精度肯定判定。

## FixedPose共享合入后的独立复核

[六个独立预期](fixed-pose-expectations.json)检查明确保持B、未知目标、禁止瞬移、保持期间其他构件/解析产品碰撞和已激活连接。参考方法只覆盖声明的恒定单位姿态与矩形板；不调用Host，不声称产品计算内核已实现。另有缺part_id、旧bare Pose、缺pose三种错误形状，分别由Job、compute_packaging和apply_commands三份现行Schema检查。

先由W00合入并重新生成公共Schema，再执行：

```powershell
& 'C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe' -B -X utf8 tools/packaging-algorithm-proof/check_fixed_pose.py
```

本脚本读取当前公共生成物；旧Schema拒绝新目标载体时直接失败，不能把待合入预期写成已通过。原有32项预期保持不变。

## RR 原17—22的实际复查

`prove.py`的公共Job局部适配器原先把真实多边形换成包围矩形，三角形许可外的明确穿透竟返回Pass。现由`polygon_cells.py`作精确有理简单多边形剖分、凹口保留及同ID深度层标记；严格内部碰撞点仍经过原区间逆变换独立核验。没有完成的通用制造明确Undetermined，不再用包围矩形造胶层。修前/修后和12项面积、凹口、跨凹槽区域、错误ID、预算/制造预期在`../design-quality-audit/report-linked/window3/`；原32项独立预期未改。

`check_sleeve.py`实际生成`output/sliding-sleeve-job.json`，四壁和接缝片共5板/4折带，真实闭缝frame/许可区；10项预期覆盖已折端点、闭缝错误、全区间滑移与中途障碍、刚体保持/单独转动、净空间/取物、材料和产品改版。初次恰好贴许可边界的输入返回未定，原始Job/结果保留在本轮window3目录；明确选择5mm的A侧许可区与4mm实际接缝片后才有正分离余量，不更改数值死区或原32项预期。

这些函数是明确fixture的隔离方法，不是任意Job调度器。四折进一步实际复核检出上搭缝第四折u=.875的穿底板反例；原Job及修前结果保留。下搭缝候选修订2的四折已实际Pass，242查询/27细分；负例回归仍Fail。`../design-quality-audit/report-linked/window3/sliding-sleeve-combined.json`逐case→用途→整Job统一稳定结论：成型、净空间、闭缝保持、开合和开后取物取得该输入的有限证据，整套制造仍未证，整个滑套Job为Undetermined。完整三机制/三变化、一般空间姿态、多许可/多步骤全Job调度、Rust数值/取消/进度和Host→认可→绑定→Engine，继续由原P4-02及P0-03/P1接点实施。工业加工资料/样件另需真实接手方，不能从有限几何补造。

```powershell
python -B -X utf8 tools/design-quality-audit/report-linked/window3/check_polygon_contact.py --after
python -B -X utf8 tools/design-quality-audit/report-linked/window3/check_polygon_cells.py
python -B -X utf8 tools/packaging-algorithm-proof/check_sleeve.py
```

## AQ-04 成本与预算

执行`python -B -X utf8 tools/packaging-algorithm-proof/measure_cost.py`得到output/aq04-cost-result.json：五类既有独立预期的1/2/4组复制及三预算边界，共18组实际Fraction参考成本。旧32项/六保持姿态预期不改；新建议档与显式探索档消费algorithm.design.json，不能把参考毫秒当完整Rust/三机制产品保证。

本轮完整复现入口：`python -B -X utf8 tools/design-quality-audit/report-linked/window3/run_review.py`。原始stdout/退出码保留在同目录`final-execution.json`；`sleeve-initial-boundary*.json`只属初轮历史，其旧硬编码用途汇总不可作为通过依据。
