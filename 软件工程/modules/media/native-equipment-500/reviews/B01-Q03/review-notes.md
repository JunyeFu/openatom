# B01-Q03 · 单张胶印机逐机复审

原独立身份`sf.native50.device.offset-printer`修订1.1.0。双LOD共2个设备GLB，原纸/墨剂/阻塞夹具3个物品GLB；夹具、LOD和实例不增加设备数。原创CC0结构/表面/铭牌及包内源码，尺寸工时为本项目明确示例。

一张0.50×0.40×0.0008m纸板，面积0.20m²，保持同一Item、数量1，flat→offset_printed。印版先转印到橡皮布，橡皮布向纸材转印，压印辊承托；供墨和润版为分别可辨认的辊组。参考[Heidelberg润版说明](https://www.heidelberg.com/global/media/en/global_media/company___publications/heidelberg_news/tips_tricks/hn_271_tips_tricks.pdf)与[IS&T胶印过程](https://www.imaging.org/IST/IST/Resources/Tutorials/Printing_Press.aspx)核对关系，不导入厂商CAD或额定参数。

油墨为独立batch ItemType，1 piece明确代表0.2mL剂量，形成0.10×0.10m×20µm蓝色层。declared_blue_layer_volume_m3是配方常量，不表示初始已涂墨。实际Job.input_refs和油墨transformed生命周期证明消耗；preserve不向原纸composition_refs追加耗材。

润版循环为auto必需的`dampening_circuit`资源，fixture/slot数量1，表示已经供液并可运行的循环服务。现场不可用通过实际日历/占用阻止印刷，纸/墨仍持有并在恢复后继续。该资源不是一份虚构水Item，不计算有限水存量、液位或水耗；若需要水耗平衡，应建立对应物料工序，不能从这份资源可用性结果推断。外观有封闭循环罐、泵和供回管，不烘焙液体库存。

周期28s，load5.6s/auto16.8s/unload5.6s；装卸operator和自动auto_fixture加dampening_circuit均为真实绑定。默认固定单蓝色块；未声明任意印刷稿/RIP、油水平衡、干燥、网点增大或流体/接触力求解。下游应显式绑定offset_printed状态和对应ItemType。

修正原三上辊泛化、印版夹进入幅面、以及滚筒旋向错误。新增了真实切点速度检查：橡皮布下表面与压印辊上表面沿纸路均为+2m/归一化auto，既有源码重建和实际产品GLB解码分别通过；不以看起来在转代替同向滚动。该复核同时发现旧双色柔版和印刷开槽1.1.0有同类缺陷，已独立撤回计数，等待新修订重验；本批不掩盖它们的待修状态。

2设备GLB与3物品GLB独立源码重建一致；29张实际GLB图：双LOD七向14、机构近景3、支脚2、实际纸墨组合10。三滚筒/供墨润版相切、床面及夹条净空通过。最终Host`.work/native500-quality-20261025/b01q03/attempt-03`：8任务含1清缓存夹具，7设备工况覆盖正常、缺纸、缺墨、阶段等待、中断、出料阻塞和润版服务等待，均恢复完成；同纸身份/数量/面积和墨剂体积核对，新Host保存重开通过。

最终普通Native`b01q03-ordinary-03`、`b01q03-ordinary-04`各20个历史时点，菜单保存至`b01q03/native-saved-r2/saved`后新进程重开；14条逐工况持有检查、完整Run不变及正常关闭通过。最终16视图`b01q03-render-02`核对实际物品出现/缺料消失、等待和润版恢复。源、旧试次和Run保留。准入后首启精确登记另存startup-registration-checks.json；本批不关闭P3-02、正式NFR或人工验收。
