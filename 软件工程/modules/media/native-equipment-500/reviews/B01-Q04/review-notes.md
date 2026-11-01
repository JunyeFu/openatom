# B01-Q04 · 在线喷码、辊式上光与 UV 固化

三个独立设备定义分别为 `sf.native50.device.inkjet-marker`、`sf.native50.device.varnish-coater` 和 `sf.native50.device.uv-curer`，新修订 1.1.0。双 LOD 共 6 个设备 GLB；3 个原纸类型、2 个 batch 剂量类型及 1 个阻塞夹具共 6 个物品 GLB，不另计设备数量。结构、铭牌、表面和包内可编辑源码沿用原创 CC0 体系。

共同纸材为 0.50×0.40×0.0008m，面积 0.20m²；每次作业 1 张，保持同一 Item ID、数量和面积。设备媒体为米制 Y 向上，工程为 Z 向上；纸路承托面高 0.92m。load/auto/unload 分别占明确周期的 20%/60%/20%，装卸绑定 operator，自动阶段绑定 auto_fixture；这些尺寸与周期是项目示例，不表示厂商额定能力。

| 独立设备 | 业务输入与输出 | 机构与动作 | 消耗及真实约束 |
|---|---|---|---|
| 在线喷码机 | flat→coded_A01；保持原纸 | 固定 DOD 喷头、7 喷孔、3mm 离纸间隙、光电支架及输送带；字点随纸跨过 x=0 的喷印位置逐列出现 | 1 piece 墨剂代表固定 A01 的 47 个 1.5×1.5mm、5µm 厚字点，总体积 0.52875µL；8s 周期 |
| 辊式上光机 | flat→varnished_wet；保持原纸 | 上涂布辊/下压印辊/计量辊、刮刀和支架、供剂杯及管、透明侧护罩；两个接触面的切向速度均与纸张同为 +2m/归一化 auto | 1 piece 上光剂明确代表 4mL，生成覆盖 0.20m² 的 20µm 湿膜；20s 周期 |
| UV 固化通道 | varnished_wet→varnished_cured；保持原纸 | 罩体、检查窗、反射罩/石英灯管/安装件、散热风扇和输送带；蓝灯仅表示固化阶段 | 自动阶段另需 lamp_circuit，fixture/slot=1；不额外消耗上光剂；18s 周期 |

喷码按实际字点数扣减剂量，字点不通过缩放伪造墨量。上光按通过辊隙的纸长增加湿膜覆盖和体积，供剂同步减少。`declared_code_ink_volume_m3` / `declared_coating_volume_m3` 是配方常量；实际 `presentation_state` 才决定是否已有喷码/涂层。耗材由 Job.input_refs 与 transformed 生命周期证明消耗；preserve 不追加虚构 composition_refs。

UV 单机输入类型默认 wet，代表已上光来料；顺序线同时将两个设备显式绑定到同一个 `varnish-coater-input` 类型，起始 flat，绝不靠换 ItemType 冒充加工。真实两步 route 使用 coat→cure 前置关系：同一张纸 0–20s 上光、20–38s 固化，最终保持原 ID 到达固化出料位置，只消耗一次 4mL 剂量。连接按当前工程物料连接规则交接，不声明传送距离的物流时间求解。

固化仅在实际 auto 完成后改变状态，等待与暂停保持真实 Job 有效进度；灯回路表示已就绪可执行的服务，不提供 UV 光谱、曝光剂量、温升或固化化学求解。喷码仅支持固定 A01 示例，不声明任意字体、可变文本、RIP 或连续喷墨回收系统。参考 [Videojet 非接触喷印](https://www.videojet.co.uk/uk/homepage/products/continuous-inkjet-printers/printing-without-contact.html)及 [Nordson UV 固化设备](https://www.nordson.com/en/products/industries/uv-curing-equipment)核对设备关系，不使用厂商 CAD。

复审修正了原泛化机构、未定义的加工状态/剂量、上光→固化类型绑定和过早固化表达。近景又发现透明薄膜叠加的深度条纹，最终保留真实 20µm 膜几何，表面采用表达已涂纸张颜色及湿/固化粗糙度的单层不透明 PBR；这不是上光液自身不透明的物性声明。修正后实际解码和 Native 重新验收，未通过增厚纸或薄膜消除条纹。

最终证据以 acceptance-input.json 为准。6 个设备与 6 个物品 GLB 独立包内源码重建一致；设备双 LOD 七向与正反 seek、支腿接地、喷头间隙、47 字点及墨量、上光辊方向/20µm 膜量、固化前后状态经实际产品解码核对。Host 共 23 个作业：18 个单机工况、3 个缓冲清空作业和顺序线 2 个作业；覆盖正常、缺料、阶段等待、中断、输出堵塞、缺耗材和固化灯回路不可用。Native 共 48 个真实业务取景，普通 Native 两个独立进程各核对 51 个时点，菜单保存后新进程重开，完整 Run 不变和正常关闭。

只有质量记录、两进程持有/等待/保存重开及首启精确登记通过后，才将这 3 种计入正式数量。仅声明 AI 复审，不关闭 P3-02、正式 NFR 或人工验收。
