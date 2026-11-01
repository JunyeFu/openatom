from pathlib import Path
import json
root=Path.cwd();home=root/'modules/media/native-equipment-500';review=home/'reviews/B01-Q02';read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'));write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
receipt=read(root/'.work/native500-quality-20261025/public-registration-checks.json');assert receipt['passed'] and receipt['device_types']==23 and receipt['exact_registered_modules']==69
write(review/'startup-registration-checks.json',receipt)
delivery=read(home/'delivery-state.json');delivery.update(public_Native_startup_registration_applied=True,startup_registration_evidence='modules/media/native-equipment-500/reviews/B01-Q02/startup-registration-checks.json');write(home/'delivery-state.json',delivery)
(review/'README.md').write_text('''# B01-Q02 · 双色柔版印刷机复审通过

`sf.native50.device.flexo-printer@1.1.0`通过AI逐机MQ及实际Native验收。本批新增1种，纸包装23/92通过、69待审；全目录23/500通过、477待审。双LOD和物品夹具不增加设备计数。

独立蓝、红两种batch计量墨剂，各0.2mL；两印组逐色转印，保持同一纸张身份、数量和面积。见[完整定义与验收](review-notes.md)、[机器可读范围](review-scope.json)、[修正清单](open-defects.json)、[当前验收输入](acceptance-input.json)。两设备GLB及四物品GLB均可独立源码重建；29张实际GLB图、8任务7工况、16个Native业务视图、两普通进程各20时点、保存后新进程重开通过。首启23设备/46媒体、共69精确模块已核对。

具体双固定色块示例，不声明任意印刷稿件/RIP、干燥或流体物理；该批不关闭整个P3-02、正式NFR或人工验收。旧版本、Run和失败尝试均保留。
''',encoding='utf8')
p=root/'README.md';lines=p.read_text(encoding='utf8').splitlines();index=next(i for i,s in enumerate(lines)if s.startswith('设备扩展库已逐批通过'));lines[index]='设备扩展库已逐批通过 **23/500种**；当前纸包装目标 **23/92种**，余69种继续复审，完成该范围后停止。[逐设备进度](modules/media/native-equipment-500/quality-review-progress.json)与正式登记单独记录，不代表整包开发完成。';p.write_text('\n'.join(lines)+'\n',encoding='utf8')
p=home/'README.md';s=p.read_text(encoding='utf8').replace('22/92种通过、70种待审','23/92种通过、69种待审').replace('22/500种、478种待审','23/500种、477种待审');line='- B01-Q02@1.1.0：双色柔版1种通过；两色独立墨剂、7工况、16个Native视图、保存重开及精确登记。'
if line not in s:s=s.replace('[纸包装范围]',line+'\n\n[纸包装范围]',1)
s=s.replace('[本批复审记录](reviews/B01-Q01/review-notes.md)','[本批复审记录](reviews/B01-Q02/review-notes.md)');p.write_text(s,encoding='utf8')
p=root/'docs/04_开发与验收/剩余任务Todo.md';lines=p.read_text(encoding='utf8').splitlines();lines[0]='> 当前纸包装目标23/92通过、69待审，完成本范围后停止；全目录23/500通过、477未计数。最新B01-Q02双色柔版补齐两种独立墨剂、分别缺墨及持有/等待、普通Native保存重开和首启精确登记。见[范围](../../modules/media/native-equipment-500/paper-packaging-review-scope.json)和[进度](../../modules/media/native-equipment-500/quality-review-progress.json)；不关闭P3-02或正式NFR。';p.write_text('\n'.join(lines)+'\n',encoding='utf8')
p=root/'docs/04_开发与验收/剩余任务Todo.json';v=read(p);b=v['bounded_native500_model_review'];registry=read(home/'accepted-registration.json');b.update(accepted_device_types=23,pending_device_types=477,active_paper_packaging_scope={'target':92,'accepted':23,'pending':69,'stop_when_complete':True,'ref':'modules/media/native-equipment-500/paper-packaging-review-scope.json'});b['review_refs']=['modules/media/native-equipment-500/reviews/'+batch+'/quality-review.json'for batch in registry['review_batches']];b['accepted_revisions']=[batch+'@'+read(home/'reviews'/batch/'quality-review.json')['version']for batch in registry['review_batches']];write(p,v)
p=root/'docs/04_开发与验收/整包派发/P3-02_整包执行.json';v=read(p);reuse=v['sequential_closeout']['completed_reuse'];refs=['modules/media/native-equipment-500/reviews/B01-Q02/'+f for f in ['quality-review.json','release-catalog.json','Native-render-checks.json','startup-registration-checks.json']]+['.work/native500-quality-20261025/b01q02/Native-persistence-r1-checks.json']
if not any(refs[0]in e.get('evidence_refs',[])for e in reuse):reuse.append({'within_original_step':1,'scope':'B01-Q02双色柔版1.1.0：独立蓝/红batch墨剂及两个印组、原纸身份/数量/面积和两色体积、7工况8任务、29张实际GLB图、16个Native视图、两普通进程保存重开通过。纸包装23/92、69待审，全目录23/500；首启23设备46媒体，不关闭P3-02或正式NFR。','evidence_refs':refs})
write(p,v);print('B01-Q02 startup23/46/69 and paper23/92 synchronized.')
