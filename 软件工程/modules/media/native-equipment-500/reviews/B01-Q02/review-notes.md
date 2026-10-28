# B01-Q02 · 双色柔版印刷机复审

`sf.native50.device.flexo-printer@1.1.0`沿用原独立身份，旧1.0.0保留。本批1种设备、双LOD2个设备GLB；原纸、蓝墨、红墨和阻塞夹具共4个物品GLB，夹具与实例不增加设备计数。

## 可据其运行的业务定义

一张0.50×0.40×0.0008m纸板，面积0.20m²，状态flat→two_color_printed。纸张经过两套网纹辊、凸版滚筒和压印辊印组；保持同一Item，数量和面积不变。蓝墨`auxiliary`、红墨`auxiliary_red`分别是独立batch ItemType、独立运行态Item、端口和持有槽位。每色数量1 piece明确代表0.2mL计量剂量；各自形成0.10×0.10m、20µm印层，独立体积守恒。

25s周期：load5s、auto15s、unload5s；实际装卸资源与自动阶段资源/日历驱动。两种油墨在auto结束真实消耗，原纸枚举更新。耗材来源由Job.input_refs和transformed生命周期证明；preserve不会向原纸composition_refs偷偷追加耗材。declared_blue_layer_volume_m3/declared_red_layer_volume_m3是配方常量，不表示初始纸已印刷。缺任一墨剂都不能启动，缺料时不显示虚构耗材。

工序关系参照[BOBST柔版印刷过程](https://www.bobst.com/cuen/products/flexo-printing-rotary-die-cutting/process/)中的网纹辊→凸版→承印物/压印辊。模型为原创明确示例：微量重力滴供、单刮刀、两个固定色块；不声明工业循环墨路、光栅/RIP、网点增大、粘度、干燥或流体/接触力求解。尺寸与工时不采用制造商额定参数。下游应显式消费two_color_printed状态，任意图案/产品须另行绑定定义。

## 修正与验收证据

修正两色定义缺失、压印辊穿入机床面、薄印层共面及审阅器深度条纹，见open-defects.json。工作面0.92m、压印辊最低点0.80m，三段闭合输送带让出辊组间隙；脚垫/台腿/台面及墨杯支撑核对。源码、微表面和铭牌原创CC0，所有可安装包自带独立重建源码。

2个设备GLB和4个物品GLB独立包内重建一致。29张实际GLB图：双LOD七向14、详细机构近景3、支脚2、蓝/红真实纸墨组合10。实际产品解码正反向16时点/模型核对纸材位移及每色材料守恒；最终重拍已消除审阅相机近裁面造成的纸/带深度条纹。产品Native相机未修改。

最终Host：`.work/native500-quality-20261025/b01q02/attempt-03`，8任务含1个明确清出料缓存夹具，7设备工况：正常、缺纸、缺蓝墨、缺红墨、阶段资源等待、中断、出料阻塞；全部恢复完成。完整Run与文件在新Host重开通过。

普通Native：`b01q02-ordinary-01`与`b01q02-ordinary-02`，各20个历史时点、实际菜单保存到`b01q02/native-saved-r1/saved`再由新进程重开，14条逐工况检查通过，原纸/两色Item身份及持有明确，暂停有效进度不变。最终16视图`b01q02-render-01`核对实际纸、蓝墨、红墨的出现/消失与恢复；观看保持Run不变。不以整幅PNG相等替代暂停判定。

逐机MQ八项AI记录见visual-review.json；首启精确登记另存startup-registration-checks.json。本批不关闭整个P3-02、正式NFR或人工验收。历史素材、失败尝试和Run保留。
