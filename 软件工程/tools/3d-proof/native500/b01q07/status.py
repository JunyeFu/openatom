from pathlib import Path
import json,re
root=Path.cwd();home=root/'modules/media/native-equipment-500';folder=home/'reviews/B01-Q07'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
receipt=read(root/'.work/native500-quality-20261025/public-registration-checks.json');registry=read(home/'accepted-registration.json');scope=read(home/'paper-packaging-review-scope.json')
assert receipt['passed'] and receipt['device_types']==registry['device_count']==30 and receipt['exact_registered_modules']==90 and len(receipt['withdrawn_exact_versions_absent'])==6
write(folder/'startup-registration-checks.json',receipt)
v=read(folder/'host-checks.json');v['Native']='passed';write(folder/'host-checks.json',v)
v=read(folder/'review-scope.json');v['remaining']=[];v['formal_supported_total']=30;v['status']='accepted_by_AI';write(folder/'review-scope.json',v)
v=read(home/'delivery-state.json');v.update(public_Native_startup_registration_applied=True,startup_registration_evidence='modules/media/native-equipment-500/reviews/B01-Q07/startup-registration-checks.json');write(home/'delivery-state.json',v)
summary='纸包装30/92种通过、62种待审；本批平压平模切机的刀模/四角清废、真实持有/等待及保存重开复审通过。全目录30/500通过、470待审。'
p=root/'README.md';lines=p.read_text(encoding='utf8').splitlines();i=next(i for i,s in enumerate(lines)if s.startswith(('设备扩展库已逐批通过','当前纸包装目标','纸包装')));lines[i]=summary+' [逐设备进度](modules/media/native-equipment-500/quality-review-progress.json)单独记录；完成纸包装范围后停止，不代表整包完成。';p.write_text('\n'.join(lines)+'\n',encoding='utf8')
p=home/'README.md';t=p.read_text(encoding='utf8');t=re.sub(r'\*\*纸包装[^\n]+\*\*','**'+summary+'**',t,count=1);line='- B01-Q07@1.1.0：平压平模切1种通过；实际十字刀模/砧床/压垫与四角导轨清废，1坯+4角料面积保持；7工况/13作业、17 Native视图及两进程保存重开通过。'
if line not in t:t=t.replace('[纸包装范围]',line+'\n\n[纸包装范围]',1)
t=re.sub(r'\[本批复审记录\]\(reviews/[^)]+\)','[本批复审记录](reviews/B01-Q07/review-notes.md)',t);p.write_text(t,encoding='utf8')
p=root/'docs/04_开发与验收/剩余任务Todo.md';lines=p.read_text(encoding='utf8').splitlines();lines[0]='> '+summary+' 见[范围](../../modules/media/native-equipment-500/paper-packaging-review-scope.json)和[进度](../../modules/media/native-equipment-500/quality-review-progress.json)；完成纸包装范围后停止，不关闭P3-02或正式NFR。';p.write_text('\n'.join(lines)+'\n',encoding='utf8')
p=root/'docs/04_开发与验收/剩余任务Todo.json';v=read(p);b=v['bounded_native500_model_review'];b.update(accepted_device_types=30,pending_device_types=470);b['active_paper_packaging_scope'].update(accepted=30,pending=62);b['review_refs']=[f'modules/media/native-equipment-500/reviews/{batch}/quality-review.json'for batch in registry['review_batches']];b['accepted_revisions']=[batch+'@'+read(home/'reviews'/batch/'quality-review.json')['version']for batch in registry['review_batches']];write(p,v)
p=root/'docs/04_开发与验收/整包派发/P3-02_整包执行.json';v=read(p);reuse=v['sequential_closeout']['completed_reuse'];refs=[f'modules/media/native-equipment-500/reviews/B01-Q07/{f}'for f in ['quality-review.json','release-catalog.json','Native-render-checks.json','startup-registration-checks.json']]
if not any(refs[0]in e.get('evidence_refs',[])for e in reuse):reuse.append({'within_original_step':1,'scope':'B01-Q07：平压平模切机沿用原身份1.1.0接收；真实十字刀模/砧床/活塞杆及四角导轨清废，1原板→1坯+4角料来源/数量/面积/厚度保持；7工况/13作业、成品与部分/完全废料堵塞恢复，17 Native取景、两普通进程各20时点及保存新进程重开、30定义/60媒体首启精确登记通过。纸包装30/92，62待审；不关闭P3-02或正式NFR。','evidence_refs':refs})
write(p,v)
(folder/'README.md').write_text('''# B01-Q07 · 平压平模切机接收

原sf.native50.device.platen-die-cutter@1.1.0；双LOD两个设备GLB与四个Item GLB包内独立源码重建通过。固定十字轮廓刀模、砧床、退料压垫、连接活塞杆及四角导轨清废已复审；实际28图，输入0.20m²、坯形0.16m²和四角各0.01m²网格及真实输出面积保持。

13作业/7工况、17 Native取景、两个普通进程各20时点、真实holder/等待冻结及菜单保存新进程重开通过。普通Native首启精确登记30定义/60媒体，共90模块；六个旧印刷模块版本仍未登记。

当前纸包装30/92，62待审；全目录30/500，470待审。见[复审记录](review-notes.md)、[范围](review-scope.json)、[验收输入](acceptance-input.json)、[首启回执](startup-registration-checks.json)。固定模切配方不包含压痕线或任意CAD刀模导入；本批AI接收不关闭整个P3-02、正式NFR或人工验收。
''',encoding='utf8')
print(summary)
