from pathlib import Path
import json,re
root=Path.cwd();home=root/'modules/media/native-equipment-500';review=home/'reviews/B01-Q03';read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'));write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
receipt=read(root/'.work/native500-quality-20261025/public-registration-checks.json');assert receipt['passed']and receipt['device_types']==22 and receipt['exact_registered_modules']==66
write(review/'startup-registration-checks.json',receipt);delivery=read(home/'delivery-state.json');delivery.update(public_Native_startup_registration_applied=True,startup_registration_evidence='modules/media/native-equipment-500/reviews/B01-Q03/startup-registration-checks.json');write(home/'delivery-state.json',delivery)
(review/'README.md').write_text('''# B01-Q03 · 单张胶印机复审通过

`sf.native50.device.offset-printer@1.1.0`完成三滚筒/供墨/润版建模与AI逐机MQ验收。源码及实际GLB切点速度同向核对、7工况8任务、29张模型图、16个Native视图、两普通进程各20时点及保存后新进程重开通过。首启实际22设备/44媒体、共66精确模块。

本批新增1种，同时旧双色柔版、印刷开槽1.1.0因滚筒方向错误撤回2种；目前纸包装22/92通过、70待审，全目录22/500通过、478待审。下一步先按1.1.1修复重验两旧修订；不沿用它们的历史通过标签。

详见[完整业务与验收](review-notes.md)、[资源和物料定义](review-scope.json)、[实际验收输入](acceptance-input.json)。原纸保身份、0.2mL实际墨剂消耗；润版循环明确为必需资源服务，不伪造水库存或水耗。源码/历史Run保留；不关闭P3-02、正式NFR或人工验收。
''',encoding='utf8')
summary='当前纸包装目标22/92通过、70待审；单张胶印新通过1种，双色柔版及印刷开槽旧1.1.0因方向错误撤回2种，先发1.1.1重验。全目录22/500通过、478待审。'
p=root/'README.md';lines=p.read_text(encoding='utf8').splitlines();i=next(i for i,s in enumerate(lines)if s.startswith(('设备扩展库已逐批通过','当前纸包装目标')));lines[i]=summary+' [逐设备进度](modules/media/native-equipment-500/quality-review-progress.json)单独记录；完成纸包装范围后停止，不代表整包完成。';p.write_text('\n'.join(lines)+'\n',encoding='utf8')
p=home/'README.md';s=p.read_text(encoding='utf8');s=re.sub(r'\*\*当前纸包装目标[^\n]+\*\*', '**'+summary+'**',s,count=1);line='- B01-Q03@1.1.0：单张胶印1种通过；三滚筒/供墨润版、实际切点同向、7工况及Native保存重开。'
if line not in s:s=s.replace('[纸包装范围]',line+'\n\n[纸包装范围]',1)
s=re.sub(r'\[本批复审记录\]\(reviews/[^)]+\)', '[本批复审记录](reviews/B01-Q03/review-notes.md)',s);p.write_text(s,encoding='utf8')
p=root/'docs/04_开发与验收/剩余任务Todo.md';lines=p.read_text(encoding='utf8').splitlines();lines[0]='> '+summary+' 见[范围](../../modules/media/native-equipment-500/paper-packaging-review-scope.json)和[进度](../../modules/media/native-equipment-500/quality-review-progress.json)，不关闭P3-02或正式NFR。';p.write_text('\n'.join(lines)+'\n',encoding='utf8')
p=root/'docs/04_开发与验收/剩余任务Todo.json';v=read(p);b=v['bounded_native500_model_review'];registry=read(home/'accepted-registration.json');b.update(accepted_device_types=22,pending_device_types=478,active_paper_packaging_scope={'target':92,'accepted':22,'pending':70,'stop_when_complete':True,'ref':'modules/media/native-equipment-500/paper-packaging-review-scope.json'});b['review_refs']=['modules/media/native-equipment-500/reviews/'+batch+'/quality-review.json'for batch in registry['review_batches']];b['accepted_revisions']=[batch+'@'+read(home/'reviews'/batch/'quality-review.json')['version']for batch in registry['review_batches']];b['pending_rolling_direction_repairs']=[{'id':id,'withdrawn':'1.1.0','required_new':'1.1.1','ref':'modules/media/native-equipment-500/reviews/'+batch+'/repair-required.json'}for batch,id in [('B01-Q02','flexo-printer'),('B02-Q03','flexo-slotter')]];write(p,v)
p=root/'docs/04_开发与验收/整包派发/P3-02_整包执行.json';v=read(p);reuse=v['sequential_closeout']['completed_reuse'];refs=['modules/media/native-equipment-500/reviews/B01-Q03/'+f for f in ['quality-review.json','release-catalog.json','Native-render-checks.json','startup-registration-checks.json']]+['.work/native500-quality-20261025/b01q03/Native-persistence-r2-checks.json']
if not any(refs[0]in e.get('evidence_refs',[])for e in reuse):reuse.append({'within_original_step':1,'scope':'B01-Q03单张胶印1.1.0：三滚筒/供墨/润版资源、实际切点同向、7工况8任务、29模型图、16 Native视图及两普通进程保存重开通过。方向复核撤回旧双色柔版及印刷开槽1.1.0两种，当前纸包装22/92、70待审，先1.1.1修复重验；首启22设备44媒体，不关闭P3-02或正式NFR。','evidence_refs':refs})
v['native500_rolling_direction_withdrawals']={'evidence':'.work/native500-quality-20261025/rolling-direction-audit/known-reversed-accepted.json','device_ids':['flexo-printer','flexo-slotter'],'status':'withdrawn_pending_new_revision_reacceptance'};write(p,v);print('Current22/92 and two required rolling-direction repairs synchronized.')
