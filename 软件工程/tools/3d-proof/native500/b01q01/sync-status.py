from pathlib import Path
import json
root=Path.cwd();home=root/'modules/media/native-equipment-500';review=home/'reviews/B01-Q01';read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'));write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
receipt=read(root/'.work/native500-quality-20261025/public-registration-checks.json');assert receipt['passed'] and receipt['device_types']==22 and receipt['exact_registered_modules']==66
write(review/'startup-registration-checks.json',receipt)
delivery=read(home/'delivery-state.json');delivery.update(public_Native_startup_registration_applied=True,startup_registration_evidence='modules/media/native-equipment-500/reviews/B01-Q01/startup-registration-checks.json');write(home/'delivery-state.json',delivery)
(review/'README.md').write_text('''# B01-Q01 · 送纸与举升两设备复审通过

真空送纸机、纸堆升降上料台沿用sf.native50独立身份，修订1.1.0已通过AI逐机MQ及实际Native验收。本批新增2种，纸包装22/92种通过、70待审；全目录22/500通过、478待审。LOD、物品夹具和实例不增加设备计数。

见[逐机记录](review-notes.md)、[工艺和接料边界](review-scope.json)、[缺陷修正](open-defects.json)、[精确验收输入](acceptance-input.json)。4个设备GLB与3个物品GLB独立包内源码重建通过；74张实际GLB图、12个任务10个设备工况、24个Native业务视图、两普通Native进程各28时点和菜单保存后新进程重开通过。首启实际核对22设备/44媒体、共66个精确模块。

单张纸与聚合纸垛保留不同ItemType：举升不自动拆垛，送纸不创建库存；跨类型工序需要显式定义。该批不关闭整个P3-02、正式NFR或人工验收；旧版本、Run与失败试次全部保留。
''',encoding='utf8')
p=root/'README.md';s=p.read_text(encoding='utf8').replace('**20/500种**','**22/500种**').replace('**20/92种**，余72种','**22/92种**，余70种');p.write_text(s,encoding='utf8')
p=home/'README.md';s=p.read_text(encoding='utf8').replace('20/92种通过、72种待审','22/92种通过、70种待审').replace('20/500种、480种待审','22/500种、478种待审');s=s.replace('[纸包装范围]', '- B01-Q01@1.1.0：真空送纸和整垛举升2种通过；原sf.native50身份、实际纸材持有/等待、24个Native视图及保存重开。\n\n[纸包装范围]',0 if 'B01-Q01@1.1.0' in s else 1).replace('[本批复审记录](reviews/B02-Q04/review-notes.md)','[本批复审记录](reviews/B01-Q01/review-notes.md)');p.write_text(s,encoding='utf8')
p=root/'docs/04_开发与验收/剩余任务Todo.md';lines=p.read_text(encoding='utf8').splitlines();lines[0]='> 当前纸包装目标22/92通过、70待审，完成本范围后停止；全目录22/500通过、478未计数。最新B01-Q01送纸与举升两设备通过逐机MQ、实际持有/等待、普通Native保存重开及首启精确登记。旧Q01@1.1.0仍撤回；不关闭P3-02或正式NFR。见[范围](../../modules/media/native-equipment-500/paper-packaging-review-scope.json)和[进度](../../modules/media/native-equipment-500/quality-review-progress.json)。';p.write_text('\n'.join(lines)+'\n',encoding='utf8')
p=root/'docs/04_开发与验收/剩余任务Todo.json';v=read(p);b=v['bounded_native500_model_review'];b.update(accepted_device_types=22,pending_device_types=478);b['active_paper_packaging_scope']={'target':92,'accepted':22,'pending':70,'stop_when_complete':True,'ref':'modules/media/native-equipment-500/paper-packaging-review-scope.json'};ref='modules/media/native-equipment-500/reviews/B01-Q01/quality-review.json'
if ref not in b['review_refs']:b['review_refs'].append(ref)
if 'B01-Q01@1.1.0' not in b['accepted_revisions']:b['accepted_revisions'].append('B01-Q01@1.1.0')
write(p,v)
p=root/'docs/04_开发与验收/整包派发/P3-02_整包执行.json';v=read(p);reuse=v['sequential_closeout']['completed_reuse'];refs=['modules/media/native-equipment-500/reviews/B01-Q01/'+f for f in ['quality-review.json','release-catalog.json','Native-render-checks.json','startup-registration-checks.json']]+['.work/native500-quality-20261025/b01q01/Native-persistence-r2-checks.json']
if not any(refs[0] in e.get('evidence_refs',[]) for e in reuse):reuse.append({'within_original_step':1,'scope':'B01-Q01送纸和举升两设备1.1.0通过：4设备GLB独立源重建、74张实际GLB图、真实单张/聚合纸垛身份与数量/面积、10工况12任务、24个Native视图、两普通进程保存重开。纸包装22/92、70待审，全目录22/500；首启22设备44媒体，不关闭P3-02或正式NFR。聚合垛不隐式拆成单张。','evidence_refs':refs})
write(p,v);print('B01-Q01 startup22/44/66 and paper22/92 synchronized.')
