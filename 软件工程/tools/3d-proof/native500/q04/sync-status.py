from pathlib import Path
import json
root=Path.cwd();home=root/'modules/media/native-equipment-500';review=home/'reviews/B02-Q04';read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'));write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
receipt=read(root/'.work/native500-quality-20261025/public-registration-checks.json');assert receipt['passed'] and receipt['device_types']==20 and receipt['exact_registered_modules']==60
write(review/'startup-registration-checks.json',receipt)
delivery=read(home/'delivery-state.json');delivery.update(public_Native_startup_registration_applied=True,startup_registration_evidence='modules/media/native-equipment-500/reviews/B02-Q04/startup-registration-checks.json');write(home/'delivery-state.json',delivery)
(review/'README.md').write_text('''# B02-Q04 · 五种纸包装整饰设备复审通过

平压烫金机、纸面凹凸压印机、局部UV上光机、平面丝网印刷机、移印机@1.1.0已通过AI逐机MQ复审。本批新增5种，纸包装范围20/92通过、72待审；全目录20/500通过、480待审。独立LOD、实例与物品夹具不增加设备数量。

见[逐机记录](review-notes.md)、[工艺范围](review-scope.json)、[修正清单](open-defects.json)、[当前验收输入](acceptance-input.json)。双LOD设备10个GLB、实际物品12个GLB均可由包内源码重建；130张实际GLB图、36任务30工况、70个实际Native视图及两个普通Native进程各85时点、菜单保存后新进程重开通过。Native首启实际登记20设备定义/40媒体、共60个精确模块。

成品/废PET、原纸身份、实际batch耗材、面积与材料体积均按声明配方核对；不声明任意工件物理加工。该批结果不关闭整个P3-02、正式NFR或人工验收。原候选、Run与失败试次保留。
''',encoding='utf8')
p=root/'README.md';s=p.read_text(encoding='utf8');s=s.replace('设备扩展库已逐批通过 **15/500种**，余485种待审；','设备扩展库已逐批通过 **20/500种**；当前纸包装目标 **20/92种**，余72种继续复审，完成该范围后停止。');p.write_text(s,encoding='utf8')
p=home/'README.md';s=p.read_text(encoding='utf8').replace('**2026-10-25：正式通过15/500种，485种待审。**','**当前纸包装目标20/92种通过、72种待审；完成纸包装完整范围后停止。全目录正式通过20/500种、480种待审。**').replace('[本批复审记录](reviews/B02-Q03/review-notes.md)','- B02-Q04@1.1.0：烫金、压凸、局部UV、丝印和移印5种通过；真实耗材/PET、30工况、70个Native视图和保存重开。\n\n[纸包装范围](paper-packaging-review-scope.json) · [本批复审记录](reviews/B02-Q04/review-notes.md)');p.write_text(s,encoding='utf8')
p=root/'docs/04_开发与验收/剩余任务Todo.md';lines=p.read_text(encoding='utf8').splitlines();lines[0]='> 当前纸包装设备目标20/92通过、72待审；完成本范围后停止。B02-Q01-R1、Q02、Q03、Q04累计正式20/500，余480未计数。Q04补齐整饰工件/耗材/PET、逐机MQ及普通Native保存重开；旧Q01@1.1.0仍撤回。见[范围](../../modules/media/native-equipment-500/paper-packaging-review-scope.json)和[进度](../../modules/media/native-equipment-500/quality-review-progress.json)。';p.write_text('\n'.join(lines)+'\n',encoding='utf8')
p=root/'docs/04_开发与验收/剩余任务Todo.json';v=read(p);b=v['bounded_native500_model_review'];b.update(accepted_device_types=20,pending_device_types=480,active_paper_packaging_scope={'target':92,'accepted':20,'pending':72,'stop_when_complete':True,'ref':'modules/media/native-equipment-500/paper-packaging-review-scope.json'});ref='modules/media/native-equipment-500/reviews/B02-Q04/quality-review.json'
if ref not in b['review_refs']:b['review_refs'].append(ref)
if 'B02-Q04@1.1.0' not in b['accepted_revisions']:b['accepted_revisions'].append('B02-Q04@1.1.0')
write(p,v)
p=root/'docs/04_开发与验收/整包派发/P3-02_整包执行.json';v=read(p);reuse=v['sequential_closeout']['completed_reuse'];refs=['modules/media/native-equipment-500/reviews/B02-Q04/'+f for f in ['quality-review.json','release-catalog.json','Native-render-checks.json','startup-registration-checks.json']]+['.work/native500-quality-20261025/q04/Native-persistence-r3-checks.json']
if not any(refs[0] in e.get('evidence_refs',[]) for e in reuse):reuse.append({'within_original_step':1,'scope':'B02-Q04五种整饰设备@1.1.0：逐机MQ、源重建、130张实际GLB图、真实纸材/计量耗材/PET、30工况36任务、70个Native视图及两普通进程保存重开通过。累计20/500；当前纸包装20/92、72待审，完成纸包装范围后停止。首启登记20设备40媒体，不关闭整个P3-02或正式NFR。','evidence_refs':refs})
write(p,v)
print('Q04 startup20/40/60 and paper scope20/92 synchronized.')
