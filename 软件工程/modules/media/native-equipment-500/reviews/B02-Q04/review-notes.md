# B02-Q04 · 纸包装整饰设备逐机复审

本批为平压烫金机、纸面凹凸压印机、局部UV上光机、平面丝网印刷机、移印机@1.1.0，共5种独立设备、10个双LOD设备GLB；12个物品GLB为业务夹具，不增加设备计数。

## 已实现的业务与边界

逐机尺寸、周期、ItemType/Operation/StockLocation和工艺来源见review-scope.json、catalog.json及包内source。预切PET载体烫金消耗纸与贴片，产生新纸Item和独立废PET，两者保留两个输入来源；不是卷筒烫金。压凸保持原纸身份和0.20m²名义面积，flat→embossed、1.2mm凸起，无废料。UV、丝印各消耗0.2mL batch剂量，形成0.10×0.10m×20µm层；移印消耗0.08mL batch剂量，形成0.04×0.04m×50µm层。batch数量1表示这一明确计量剂量，不表示1mL。

保身份工序只修改已声明的presentation_state枚举；declared_finish_layer_volume_m3、declared_emboss_height_m为配方常量，不表示初始flat纸已涂覆或凸起。保身份耗材消耗由实际Job.input_refs及耗材transformed生命周期证明，当前契约不会向原纸composition_refs追加辅料；烫金新产物则按transform_with_origin保留来源。全部使用实际Item持有、Job进度与工程显式槽位，不在设备模型内放置假库存。

UV为涂布→抬网→滑台→定时固化的原子组合操作，不输出可流转湿半成品；蓝灯只表示固化阶段。不声明光学、热学、软体或制造公差仿真。默认配方针对所列工件和状态，其他产品/串联工艺应显式绑定对应ItemType和Operation，不隐式兼容任意原料。

## 修正与实际验证

缺陷逐条见open-defects.json：修正batch耗材准入、接触关键帧间隙、PET夹持穿纸、上下压凸模具、薄层共面、UV观察遮挡、液压管路断接、压凸纸面平滑法线和成品近景遮挡。原候选与失败试次保留，当前设备GLB均由独立包内源码重导出一致。

最终Host输入与36任务/30工况：`.work/native500-quality-20261025/q04/attempt-06`。包含正常、缺纸、阶段资源等待、途中中断、成品阻塞，以及适用缺耗材与废PET阻塞。数量、原纸身份/变换来源、面积、涂层/载体体积、等待与恢复完成时间通过。6个显式清缓存任务用于解除实际阻塞。

实际产品解码130张图：70张双LOD七向、15张机构近景、10张支脚、30张真实工件/耗材组合、5张完成近景。源码接触与真实GLB联合检查覆盖模具、纸/PET、网版/刮刀、凹版/胶垫和转移层。实际Native最终画面：`q04-render-03`，70个Run视图，精确Item refs及缺料时不存在的Item均核对。

普通Native两个独立进程`q04-ordinary-05`、`q04-ordinary-06`各85个历史时点，实际菜单保存到`q04/native-saved-r3/saved`后新进程重开；60条逐工况检查通过，完整Run相同，暂停时有效进度一致。最终回执`q04/Native-persistence-r3-checks.json`。不以整幅PNG逐像素相等替代机构暂停判定。

逐机MQ八项AI复审见visual-review.json。正式计数仅在accept-paper.py核对上述当前文件后更新accepted-registration.json；Native新版本首启登记另存startup-registration-checks.json。该批不关闭整个P3-02、正式NFR或人工验收。
