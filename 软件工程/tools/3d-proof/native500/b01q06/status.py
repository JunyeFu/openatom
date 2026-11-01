from pathlib import Path
import json,re
root=Path.cwd();home=root/'modules/media/native-equipment-500';folder=home/'reviews/B01-Q06'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
receipt=read(root/'.work/native500-quality-20261025/public-registration-checks.json');registry=read(home/'accepted-registration.json');scope=read(home/'paper-packaging-review-scope.json')
assert receipt['passed'] and receipt['device_types']==registry['device_count']==29 and receipt['exact_registered_modules']==87 and len(receipt['withdrawn_exact_versions_absent'])==6
write(folder/'startup-registration-checks.json',receipt)
v=read(folder/'review-scope.json');v['remaining']=[];v['formal_supported_total']=29;v['status']='accepted_by_AI';write(folder/'review-scope.json',v)
v=read(home/'delivery-state.json');v.update(public_Native_startup_registration_applied=True,startup_registration_evidence='modules/media/native-equipment-500/reviews/B01-Q06/startup-registration-checks.json');write(home/'delivery-state.json',v)
summary='纸包装29/92种通过、63种待审；本批分切机的邻接刀盘、真实双输出及部分/完全出料等待及Native保存重开复审通过。全目录29/500通过、471待审。'
p=root/'README.md';lines=p.read_text(encoding='utf8').splitlines();i=next(i for i,s in enumerate(lines)if s.startswith(('设备扩展库已逐批通过','当前纸包装目标','纸包装')));lines[i]=summary+' [逐设备进度](modules/media/native-equipment-500/quality-review-progress.json)单独记录；完成纸包装范围后停止，不代表整包完成。';p.write_text('\n'.join(lines)+'\n',encoding='utf8')
p=home/'README.md';t=p.read_text(encoding='utf8');t=re.sub(r'\*\*纸包装[^\n]+\*\*','**'+summary+'**',t,count=1);line='- B01-Q06@1.1.0：分切机1种通过；邻接剪刀盘/独立承托/出料分离，1原板→2独立半板；6工况/9作业、部分和完全出料堵塞、15 Native视图及两进程保存重开通过。'
if line not in t:t=t.replace('[纸包装范围]',line+'\n\n[纸包装范围]',1)
t=re.sub(r'\[本批复审记录\]\(reviews/[^)]+\)','[本批复审记录](reviews/B01-Q06/review-notes.md)',t);p.write_text(t,encoding='utf8')
p=root/'docs/04_开发与验收/剩余任务Todo.md';lines=p.read_text(encoding='utf8').splitlines();lines[0]='> '+summary+' 见[范围](../../modules/media/native-equipment-500/paper-packaging-review-scope.json)和[进度](../../modules/media/native-equipment-500/quality-review-progress.json)；完成纸包装范围后停止，不关闭P3-02或正式NFR。';p.write_text('\n'.join(lines)+'\n',encoding='utf8')
p=root/'docs/04_开发与验收/剩余任务Todo.json';v=read(p);b=v['bounded_native500_model_review'];b.update(accepted_device_types=29,pending_device_types=471);b['active_paper_packaging_scope'].update(accepted=29,pending=63);b['review_refs']=[f'modules/media/native-equipment-500/reviews/{batch}/quality-review.json'for batch in registry['review_batches']];b['accepted_revisions']=[batch+'@'+read(home/'reviews'/batch/'quality-review.json')['version']for batch in registry['review_batches']];write(p,v)
p=root/'docs/04_开发与验收/整包派发/P3-02_整包执行.json';v=read(p);reuse=v['sequential_closeout']['completed_reuse'];refs=[f'modules/media/native-equipment-500/reviews/B01-Q06/{f}'for f in ['quality-review.json','release-catalog.json','Native-render-checks.json','startup-registration-checks.json']]
if not any(refs[0]in e.get('evidence_refs',[])for e in reuse):reuse.append({'within_original_step':1,'scope':'B01-Q06：分切机沿用原身份1.1.0接收；1张原板变为2张独立半板，来源/数量/厚度/总面积保持；6工况/9作业、部分与完全出料阻塞逐件交付、15 Native取景、两普通进程各17时点和菜单保存新进程重开、29定义/58媒体首启精确登记通过。纸包装29/92，63待审；不关闭P3-02或正式NFR。','evidence_refs':refs})
write(p,v)
(folder/'README.md').write_text('''# B01-Q06 · 单张纸板分切机接收

原 sf.native50.device.sheet-slitter@1.1.0；双LOD两个设备GLB、原纸/半板/阻塞夹具三个Item GLB可由包内源码独立重建。最终26图、6工况/9作业、15 Native业务取景通过；一张原板真实生成两件独立半板，来源、数量、厚度和总面积均核对。

输出区剩一格时先交付一件；完全占满时Job持有两件，逐格腾空后依次交付。两个普通Native进程各17时点、等待/暂停、菜单保存新进程重开、完整Run不变及正常关闭通过。普通Native首启登记29定义/58媒体，共87模块；六个旧印刷模块版本仍未登记。

当前纸包装29/92，63待审；全目录29/500，471待审。见[复审记录](review-notes.md)、[范围](review-scope.json)、[验收输入](acceptance-input.json)、[首启回执](startup-registration-checks.json)。本批为固定配方纵向直切的AI复审，不关闭整个P3-02、正式NFR或人工验收。
''',encoding='utf8')
print(summary)
