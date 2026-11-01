from pathlib import Path
import json,re
root=Path.cwd();home=root/'modules/media/native-equipment-500';folder=home/'reviews/B01-Q05'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
receipt=read(root/'.work/native500-quality-20261025/public-registration-checks.json');registry=read(home/'accepted-registration.json');scope=read(home/'paper-packaging-review-scope.json')
assert receipt['passed'] and receipt['device_types']==registry['device_count']==28 and receipt['exact_registered_modules']==84 and len(receipt['withdrawn_exact_versions_absent'])==6
write(folder/'startup-registration-checks.json',receipt)
v=read(folder/'review-scope.json');v['pending']=[];v['status']='accepted_by_AI';write(folder/'review-scope.json',v)
v=read(home/'delivery-state.json');v.update(public_Native_startup_registration_applied=True,startup_registration_evidence='modules/media/native-equipment-500/reviews/B01-Q05/startup-registration-checks.json');write(home/'delivery-state.json',v)
summary='纸包装28/92种通过、64种待审；本批滚轮压痕机的独立轮组、纵向压痕及Native保存重开复审通过。全目录28/500通过、472待审。'
p=root/'README.md';lines=p.read_text(encoding='utf8').splitlines();i=next(i for i,s in enumerate(lines)if s.startswith(('设备扩展库已逐批通过','当前纸包装目标','纸包装')));lines[i]=summary+' [逐设备进度](modules/media/native-equipment-500/quality-review-progress.json)单独记录；完成纸包装范围后停止，不代表整包完成。';p.write_text('\n'.join(lines)+'\n',encoding='utf8')
p=home/'README.md';t=p.read_text(encoding='utf8');t=re.sub(r'\*\*纸包装[^\n]+\*\*','**'+summary+'**',t,count=1);line='- B01-Q05@1.1.0：滚轮压痕1种通过；独立双压痕轮/带槽下辊、同向切点、实际0.6mm纵向压痕；5工况/6作业、12 Native视图和两进程保存重开通过。'
if line not in t:t=t.replace('[纸包装范围]',line+'\n\n[纸包装范围]',1)
t=re.sub(r'\[本批复审记录\]\(reviews/[^)]+\)','[本批复审记录](reviews/B01-Q05/review-notes.md)',t);p.write_text(t,encoding='utf8')
p=root/'docs/04_开发与验收/剩余任务Todo.md';lines=p.read_text(encoding='utf8').splitlines();lines[0]='> '+summary+' 见[范围](../../modules/media/native-equipment-500/paper-packaging-review-scope.json)和[进度](../../modules/media/native-equipment-500/quality-review-progress.json)；完成纸包装范围后停止，不关闭P3-02或正式NFR。';p.write_text('\n'.join(lines)+'\n',encoding='utf8')
p=root/'docs/04_开发与验收/剩余任务Todo.json';v=read(p);b=v['bounded_native500_model_review'];b.update(accepted_device_types=28,pending_device_types=472);b['active_paper_packaging_scope'].update(accepted=28,pending=64);b['review_refs']=[f'modules/media/native-equipment-500/reviews/{batch}/quality-review.json'for batch in registry['review_batches']];b['accepted_revisions']=[batch+'@'+read(home/'reviews'/batch/'quality-review.json')['version']for batch in registry['review_batches']];write(p,v)
p=root/'docs/04_开发与验收/整包派发/P3-02_整包执行.json';v=read(p);reuse=v['sequential_closeout']['completed_reuse'];refs=[f'modules/media/native-equipment-500/reviews/B01-Q05/{f}'for f in ['quality-review.json','release-catalog.json','Native-render-checks.json','startup-registration-checks.json']]
if not any(refs[0]in e.get('evidence_refs',[])for e in reuse):reuse.append({'within_original_step':1,'scope':'B01-Q05：滚轮压痕机沿用原设备身份1.1.0接收；两独立压痕轮/带槽承托辊、实际0.6mm纵向压痕、同纸身份/数量/面积，5工况/6作业、12 Native取景、两普通进程各14时点及菜单保存后新进程重开、28定义/56媒体首启精确登记通过。纸包装28/92，64待审；不关闭P3-02或正式NFR。','evidence_refs':refs})
write(p,v)
(folder/'README.md').write_text('''# B01-Q05 · 滚轮压痕机接收

滚轮压痕机 sf.native50.device.rotary-scorer@1.1.0，双LOD两个设备GLB和原纸/阻塞夹具两个Item GLB均可由包内源码独立重建。实际26图、5工况/6作业、12个Native业务取景通过；两只独立压痕轮、带槽下辊、实际0.6mm纵向压痕与同向切点已核对。

两个普通Native进程各14个时点、实际持有/等待冻结、菜单保存后新进程重开、完整Run不变和正常关闭通过。普通Native首启精确登记28定义/56媒体，共84个模块；六个旧印刷模块版本仍未登记。

纸包装28/92，64待审；全目录28/500，472待审。详见[复审记录](review-notes.md)、[业务范围](review-scope.json)、[验收输入](acceptance-input.json)及[首启回执](startup-registration-checks.json)。固定配方的运动学加工展示不代表接触力/楞芯压溃求解；本批AI接收不关闭整个P3-02、正式NFR或人工验收。
''',encoding='utf8')
print(summary)
