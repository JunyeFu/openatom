# 14类模型逐项返修 · 0.4.2

本目录是2026-10-22制作、2026-10-26完成逐资产技术接收的实际GLB：装配、返工、装箱、机器人、输送、检验、封箱、托盘、控制盒、纸箱、壳体、PCB、螺钉、胶带卷，共14类，详细/简化两档28包。旧0.4.0/0.4.1、监督基准、现有工程和冻结Run保留，未自动升级采用。

[完整实际画面图册](../../../scenarios/媒体表现/model-quality-20261022/模型审阅图册.html) · [逐模型审查](../../../scenarios/媒体表现/model-quality-20261022/逐模型审查结果.md) · [精确登记清单](registration.json) · [模型/阶段/包络](inventory.json)

图册逐个提供1920×1080七向及近景、12秒实际GLB动作/观察MP4、起中止/反向关键帧、关键机构专用机位、对应监督基准对照和GLB下载。物品/托盘没有主动机构时，视频明确为观察转台。画面来自实际导出文件，由产品共享`decodeGlb`和`bindScene`重开渲染，不是AI生图。

## 本轮实际修改

- 装配/返工/检验/装箱台补齐连接、支承和接口。装配台补丝杠支承、夹头和暂存盘；返工软管接入过滤单元；检验台补调焦结构。
- 装箱推杆按同一推板行程连续伸缩，固定夹座补底板和垫高支承，折板加强筋随原铰轴运动。
- 输送补回程托辊、张紧调节与供线；机器人补底座压紧与末端气路细节；封箱补有支承的切刀护罩。
- 壳体/控制盒/PCB的安装孔切为实际贯通孔，接头为空心；PCB焊盘贴板，并有孔壁镀层。螺纹重建为V形连续螺旋截面。
- 保留托盘完整背底、木板接触、木纹；纸箱折痕附着各自盖片；胶带两面有卷绕层次和空心纸芯。

本轮使用现有Three.js参数化建模与GLTFExporter，含可编辑曲面、挤出和螺旋几何；未调用或假称调用Blender。尺寸是本项目声明示例，不是厂家实测或工业标准件认证。原创建模/表面依原CC0-1.0声明，Three.js为MIT依赖。

## 重建与查看

从软件工程根运行，使用现有锁文件中的Three.js，无新依赖安装：

```powershell
node modules/media/equipment-quality-v042/generate.mjs
node tools/model-quality-review/v042/server.mjs
```

打开[实际GLB审阅器](http://127.0.0.1:19289/tools/model-quality-review/v042/index.html)，可切14类、两档、七向和机构机位，拖动声明阶段或播放动作。目录中的静态图册可直接打开，已有视频不依赖服务。

每包`source/`均带完整生成源、`revision.mjs`、参数和原创表面。例：从软件工程根执行`node modules/media/equipment-quality-v042/installable/packing/source/generate.mjs`重建该包；`source/parameters.json`指定它自己的型号/LOD。包内`module.json`保留精确节点、阶段、锚点、依赖与文件清单。

28份源码均在独立副本重开并导出相同GLB，生产解码/场景绑定、反向seek、真实孔洞及推杆连续接触检查见[辅助检查](../../../scenarios/媒体表现/model-quality-20261022/batch/source-and-motion-checks.json)。这些结果不能代替画面审查。

## 当前接收范围

28个精确文件在[质量清单](../model-quality-ledger.json)均为`accepted`，已接收源与导出、实际画面、适用MQ八维度及Native保存重开；简化档仍只按LOD身份接收。设备等待补验见[2026-10-26回执](../../../output/playwright/20261026-MQ-equipment-waits/receipt.json)，静态物品及封箱/纸箱按清单中各自的原回执接收，未借用别的资产证明自身通过。

真人认可仍为`pending`；逐资产接收不表示工业精度认证或旧压力矩阵通过。2026-10-27仅复看已有实图、核对28文件当前登记并同步图册/审阅器，没有改GLB或冒充本轮新跑Native。`registration.json`是精确目录，本次不修改Host启动登记或已采用版本。
