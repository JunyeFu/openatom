from pathlib import Path
import json,re

root = Path.cwd()
home = root / 'modules/media/native-equipment-500'
def read(p): return json.loads(p.read_text(encoding='utf-8-sig'))
def write(p,v): p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf8')

receipt = read(root / '.work/native500-quality-20261025/public-registration-checks.json')
assert receipt['passed'] and receipt['device_types'] == 24 and receipt['exact_registered_modules'] == 72
assert len(receipt['withdrawn_exact_versions_absent']) == 6
pairs = [('B01-Q02','B01-Q02-R1','flexo-printer'),('B02-Q03','B02-Q03-R1','flexo-slotter')]
for old,batch,device in pairs:
    folder = home/'reviews'/batch
    write(folder/'startup-registration-checks.json',receipt)
    quality = read(folder/'quality-review.json')
    (folder/'README.md').write_text(f'''# {batch} · {device}@1.1.1修复接收

印辊、承托辊、网纹辊旋向与凸版初相位已修正，源码及实际GLB切点同向同速检查通过。双LOD2设备GLB独立源码重建、{quality['actual_GLB_views']}张实际模型图、7工况、16个Native业务视图、两普通进程各20时点及保存后新进程重开通过。首启实际24设备/48媒体、共72精确模块；两个旧1.1.0的6个设备/媒体引用均未登记。

本修订恢复一个独立设备类型；全库纸包装24/92通过、68待审，全目录24/500通过、476待审。旧1.1.0继续撤回，未覆盖其源码/模型/Run。见[完整复审](review-notes.md)、[实际验收输入](acceptance-input.json)及[业务范围](review-scope.json)。不关闭P3-02、正式NFR或人工验收。
''',encoding='utf8')
    p = home/'reviews'/old/'repair-required.json'
    v = read(p);v.update(replacement_accepted={'batch':batch,'version':'1.1.1','ref':f'modules/media/native-equipment-500/reviews/{batch}/quality-review.json'},old_revision_still_withdrawn=True);write(p,v)
    p = home/'reviews'/old/'quality-review.json';v=read(p)
    for w in v['currently_withdrawn_device_revisions']:w['replacement_accepted']={'batch':batch,'version':'1.1.1'}
    write(p,v)
    p = home/'reviews'/old/'README.md';s=p.read_text(encoding='utf8');s=re.sub(r'待独立1\.1\.1修正重验。',f'替代1.1.1已在[{batch}](../{batch}/README.md)修正重验接收。',s,count=1);p.write_text(s,encoding='utf8')

p=home/'quality-review-progress.json';v=read(p)
for w in v['withdrawn_rolling_directions']:
    w['replacement_accepted']={'batch':next(batch for old,batch,id in pairs if id==w['id']),'version':'1.1.1'}
write(p,v)
p=home/'delivery-state.json';v=read(p);v.update(public_Native_startup_registration_applied=True,startup_registration_evidence='modules/media/native-equipment-500/reviews/B02-Q03-R1/startup-registration-checks.json');write(p,v)
summary='纸包装24/92种通过、68种待审；双色柔版和印刷开槽1.1.1修复重验完成，两旧1.1.0继续撤回。全目录24/500通过、476待审。'
p=root/'README.md';lines=p.read_text(encoding='utf8').splitlines();i=next(i for i,s in enumerate(lines)if s.startswith(('设备扩展库已逐批通过','当前纸包装目标','纸包装')));lines[i]=summary+' [逐设备进度](modules/media/native-equipment-500/quality-review-progress.json)单独记录；完成纸包装范围后停止，不代表整包完成。';p.write_text('\n'.join(lines)+'\n',encoding='utf8')
p=home/'README.md';s=p.read_text(encoding='utf8');s=re.sub(r'\*\*当前纸包装目标[^\n]+\*\*','**'+summary+'**',s,count=1);s=s.replace('已撤回，需1.1.1重验','已撤回；其独立1.1.1修复重验已通过')
for _,batch,id in pairs:
    line=f'- {batch}@1.1.1：{id}切点同向及凸版相位修复；源/实际GLB/7工况/Native保存重开重新验收通过。'
    if line not in s:s=s.replace('[纸包装范围]',line+'\n\n[纸包装范围]',1)
s=re.sub(r'\[本批复审记录\]\(reviews/[^)]+\)','[本批复审记录](reviews/B02-Q03-R1/review-notes.md)',s);p.write_text(s,encoding='utf8')
p=root/'docs/04_开发与验收/剩余任务Todo.md';lines=p.read_text(encoding='utf8').splitlines();lines[0]='> '+summary+' 见[范围](../../modules/media/native-equipment-500/paper-packaging-review-scope.json)和[进度](../../modules/media/native-equipment-500/quality-review-progress.json)；完成纸包装范围后停止，不关闭P3-02或正式NFR。';p.write_text('\n'.join(lines)+'\n',encoding='utf8')
p=root/'docs/04_开发与验收/剩余任务Todo.json';v=read(p);b=v['bounded_native500_model_review'];registry=read(home/'accepted-registration.json');b.update(accepted_device_types=24,pending_device_types=476,pending_rolling_direction_repairs=[]);b['active_paper_packaging_scope'].update(accepted=24,pending=68);b['review_refs']=[f'modules/media/native-equipment-500/reviews/{batch}/quality-review.json'for batch in registry['review_batches']];b['accepted_revisions']=[batch+'@'+read(home/'reviews'/batch/'quality-review.json')['version']for batch in registry['review_batches']];write(p,v)
p=root/'docs/04_开发与验收/整包派发/P3-02_整包执行.json';v=read(p);reuse=v['sequential_closeout']['completed_reuse']
for old,batch,id in pairs:
    refs=[f'modules/media/native-equipment-500/reviews/{batch}/{f}'for f in ['quality-review.json','release-catalog.json','Native-render-checks.json','startup-registration-checks.json']]
    if not any(refs[0]in e.get('evidence_refs',[])for e in reuse):reuse.append({'within_original_step':1,'scope':f'{batch}@1.1.1完成{id}印辊方向及凸版相位修复；源和实际GLB切点同向、7工况、16 Native业务视图、两普通进程保存重开重新通过，恢复一个类型。当前纸包装24/92、68待审；旧1.1.0仍撤回，不关闭P3-02或正式NFR。','evidence_refs':refs})
v['native500_rolling_direction_withdrawals'].update(status='old_revisions_withdrawn_replacements_1.1.1_accepted',replacement_refs=[f'modules/media/native-equipment-500/reviews/{batch}/quality-review.json'for _,batch,_ in pairs]);write(p,v)
print('Two repairs restored only at1.1.1; current paper24/92, Native24/48 and old-version absence synchronized.')
