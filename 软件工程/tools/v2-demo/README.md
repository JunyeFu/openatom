# Task8实际演示工程生成入口

物品可视布局的完整升级入口：

```powershell
$env:TEMP='D:/SuperFactory-v2-temp/01a1731e/os-temp'
$env:TMP=$env:TEMP
python -X utf8 -B tools/v2-demo/geometry.py <全新bundle目录> --evidence <本批私有证据目录>
```

从原保存r2真实导入，生成新模块精确版本、源/汇六槽及设备命名锚点槽，同步四条PortRef、路由与实例版本；显式采用简化LOD依赖，避免“源包有模块、冻结工程未采用”。随后实际Host创建新预测Run、保存、另起新Host重开，保留原Run/冻结字节，调用真实External Engine/ExecutionProjector并解析实际Host读取的素材。独立缓存位于上述D盘TEMP目录下的`sf-v2-demo-geometry-20261027/target`，无Root target/Native/真实云；原C盘缓存已保留在`D:/SuperFactory-v2-temp/01a1731e/from-c/sf-v2-demo-geometry-20261027/target`。

当前正式保存示例为`resources/v2-demo/geometry-v4/project/`r4（三个Run）；本轮完整重现从原r2直接生成新r3，RunId由实际新进程生成，不要求与已保存示例相同。实际四步退出码均0，见`.work/v2-implementation-20261026/demo-geometry/reproduction-evidence/generation-commands.json`。旧生成命令及下文r2说明继续有效。

从软件工程根运行。输出必须是新bundle父目录中的新project目录；已有工程、映射及旧Run拒绝覆盖。输入只依赖随包resources/v2-demo/source，无需旧.work源工程，不重建或改变模型。

```powershell
python -X utf8 -B tools/v2-demo/generate.py <新的bundle父目录>/project --evidence <本批私有证据目录>
```

入口用上述D盘TEMP目录中的专用target编译sf-commands/sf-simulation普通库，实际执行：原Host公开import_package(open_project)→adopt_imported_draft→set_declared_parameter→run_scenario/Task/get_run→save_project；随后另起新Host进程重开，读取原Run/冻结输入/模块与read_media_asset素材，并调用真实Engine::new_external。最后复用生产presentation的GLB/场景解码器和已安装Pillow像素适配器，无Native/端口/云/GPU。所有步骤exit0才签收。

输出project/是实际ProjectArchive；父目录另含candidate-bindings.json与profile-bindings.json，均为非秘密映射。源模块/GLB由writer原样保留，10/15秒仅在实例与真实parameter_sources上由Host修改；预测仍按v1固定A运行。

已有包可只检查：

```powershell
python -X utf8 -B tools/v2-demo/audit-package.py resources/v2-demo <本批私有审计回执.json>
```

本批已实际重跑到.work/v2-implementation-20261026/demo-package/reproduction/project，四步命令均exit0；未覆盖随包示例。新输出中的RunId由新Host实际生成，不承诺与已保存示例RunId相同。

## 92设备工厂的当前开发入口

原控制项为V2-05-G01和V2-07-G01。`prepare_full_factory_project.py`从当前接收源准备92台正常工况；`full-factory-build`由真实Host采用、运行和保存。`connect_full_factory_routes.py`生成盒坯送料→压痕→折糊、装箱→封箱→贴标→捆扎→最终检验两条路线，`full-factory-connect`实际运行并检查108次设备作业、27趟运输、每条订单3件卸货放行成品，以及FAIL/待检分流和旧Run保留。

运输显示路线及装载槽保存于SupplyRelation.presentation_path，只控制画面；库存、运输阶段、占用和接受仍来自Engine。最终检验采用显式Operations版本，接受已获工序放行的来料，但仍执行独立检验；原设备模型不改。检验示例以纸箱原身份匹配，内装物不冒充纸箱祖先。

完成Host输出后可运行`node tools/v2-demo/full_factory_preview.mjs <本次Host输出目录> 8922`，打开`/factory-proof`检查同一FactoryScene的归档渲染和时点切换。此入口读取真实冻结工程、模块、观察帧及资产，不提供生产命令，也未接入端口计数。它只证明渲染，不能替代普通Native完整工厂验收。

当前临时环境：在各命令自身的环境设置TEMP/TMP为`D:/SuperFactory-v2-temp/01a1731e/os-temp`；根工作区编译缓存位于`F:/SuperFactory-v2-temp/01a1731e/retained-root-target`，隔离工具原C缓存已迁到`D:/SuperFactory-v2-temp/01a1731e/from-c/sf-v2-root-01a1731e`。完整C临时路径映射见同任务D目录的`c-temp-relocation.json`。不要按历史C路径重新创建缓存，也不要重写旧Run中的历史记录。


纸管/标签整厂联线：`connect_factory_families.py --source <flow12实际Host输出目录> --output <新暂存目录>`，再用工具`full-factory-families <原保存工程> <暂存目录> <新回执目录>`，最后运行`check_factory_family_materials.py <回执目录>`。独立预期为125次制造、54趟运输、30件成品；纸管2段/托、标签6张/托。当前实际证据在F盘`full-factory/families-host03`，负例01/02保留。模板构建器已生成1.1.0并保留1.0.0源；`full-factory-template-families <软件工程根> <新回执目录>`验证公共Host采用/运行/保存重开，实际回执`families-template-host01`。本轮217项引擎回归与最终相关9项通过，普通Native及新物料画面仍未验。后续改模板内容必须使用新的精确版本，不覆盖旧版本或旧Run。


纸袋主线与提手预制：先`prepare_bag_factory_design.py`更新原draw.io第6页和明确物料规格；`bag_factory_media.mjs <新素材目录>`与`check_bag_model_geometry.mjs <素材目录>`生成/采样配置夹具和物料。`connect_factory_bags.py --source <families-host03目录> --media <素材目录> --output <新暂存目录>`再由工具`full-factory-bags <原保存工程> <暂存目录> <新回执目录>`实际执行；`check_factory_bag_materials.py <回执目录> <原来源目录>`核对物料与共用资源。独立预期145次制造/78趟运输/33件成品，实际证据`F:/SuperFactory-v2-temp/01a1731e/full-factory/bags-host03`；模板1.2.0由`full-factory-template-bags <软件工程根> <新回执目录>`实际采用/保存重开，证据`bags-template-host01`。新物料画布检查`check_factory_bag_render.mjs`消费同一真实归档，证据`bags-render02`；当前无Native1.2.0通过。保留所有旧源/模板/Run/失败尝试，后续从原V2-07场景支撑与可读性继续。


当前工厂画面检查：`check_factory_environment.mjs <bags-host03目录> <新输出目录>`核对实际厂房/出货台/线组坐标及五类成品名称；`check_factory_view.mjs <bags-host03目录> <新输出目录> 8922`验证总览及同一纸箱 6 次正反跟随。只读 Vite 关闭目录监听，修改源代码后应停止并重新启动再验，不能把刷新后的旧转换缓存当作新实现。

普通 Native 模板检查脚本当前对应 1.2.0：`check_factory_template_native.mjs <独立候选与启动脚本父目录> <新Native尝试名>`、`reopen_factory_template_native.mjs <同父目录> <另一新Native尝试名> <前次尝试名>`。已实测证据位于 F 盘 `factory-native-1_2`，包括五类订单、纸袋跟随/切时点、真实OS目录保存及新进程原Run完整重开。该候选使用源资源联接；完整独立交付包、活动工序采样和云阶段仍按原控制项完成。


活动阶段采样（原 Run 只读）：`prepare_factory_phase_samples.py <实际 Host 输出目录> <新计划文件>` 选择实际加工/等待时点；工具 `full-factory-observe <原保存工程> <计划文件> <新输出目录>` 从原 Host 读取，不重新模拟或保存来源。`check_factory_phases.mjs <观察输出目录> <新渲染输出目录> 8922` 核对实际阶段和聚焦。当前 `phase-render04` 已验证 92 台设备、103 个加工点、4 个等待点、55 次正反 seek；当前普通 Native 证据为 `factory-native-phase`。实际云阶段、73 个独立单元及独立包仍未验。


执行观察可读性普通 Native 检查：`check_execution_readability_native.mjs <已暂存普通候选与OS选择器父目录> <保留的多工序completion/project-final父目录> <新Native尝试名>`。读取真实其他会话/历史/HTTPS云存档，再按原12件冻结方案准备且停止一条未开工会话，另存独立目录；不用生产IPC替身。当前证据 `factory-native-wording/native-03/readability/receipt.json` 覆盖四种来源、秒级回看、1920/1366布局与恢复，正常关闭见 `native-03/close.json`。`resume-current-owner` 只用于已核对终端失败、无义务的原目标错误续作；`finish-save` 只用于已停止且有实际未保存归档的同一原进程。源链接候选不等于独立交付包。
