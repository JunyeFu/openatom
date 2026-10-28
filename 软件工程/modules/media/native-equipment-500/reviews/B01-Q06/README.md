# B01-Q06 · 单张纸板分切机接收

原 sf.native50.device.sheet-slitter@1.1.0；双LOD两个设备GLB、原纸/半板/阻塞夹具三个Item GLB可由包内源码独立重建。最终26图、6工况/9作业、15 Native业务取景通过；一张原板真实生成两件独立半板，来源、数量、厚度和总面积均核对。

输出区剩一格时先交付一件；完全占满时Job持有两件，逐格腾空后依次交付。两个普通Native进程各17时点、等待/暂停、菜单保存新进程重开、完整Run不变及正常关闭通过。普通Native首启登记29定义/58媒体，共87模块；六个旧印刷模块版本仍未登记。

当前纸包装29/92，63待审；全目录29/500，471待审。见[复审记录](review-notes.md)、[范围](review-scope.json)、[验收输入](acceptance-input.json)、[首启回执](startup-registration-checks.json)。本批为固定配方纵向直切的AI复审，不关闭整个P3-02、正式NFR或人工验收。
