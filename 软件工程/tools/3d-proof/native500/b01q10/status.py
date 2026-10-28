from pathlib import Path
import json,re
from datetime import datetime
root=Path.cwd();home=root/'modules/media/native-equipment-500';folder=home/'reviews/B01-Q10'
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
write=lambda p,v:p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
receipt=read(root/'.work/native500-quality-20261025/public-registration-checks.json');registry=read(home/'accepted-registration.json');scope=read(home/'paper-packaging-review-scope.json')
assert receipt['passed'] and receipt['device_types']==registry['device_count']==27 and receipt['exact_registered_modules']==81 and len(receipt['withdrawn_exact_versions_absent'])==6
write(folder/'startup-registration-checks.json',receipt)
v=read(folder/'review-scope.json');v['pending']=[];v['status']='accepted_by_AI';write(folder/'review-scope.json',v)
v=read(home/'delivery-state.json');v.update(public_Native_startup_registration_applied=True,startup_registration_evidence='modules/media/native-equipment-500/reviews/B01-Q10/startup-registration-checks.json');write(home/'delivery-state.json',v)
summary='纸包装27/92种通过、65种待审；本批喷码、上光、UV固化3种及真实上光→固化顺序链复审通过。全目录27/500通过、473待审。'
p=root/'README.md';lines=p.read_text(encoding='utf-8').splitlines();i=next(i for i,s in enumerate(lines)if s.startswith(('设备扩展库已逐批通过','当前纸包装目标','纸包装')));lines[i]=summary+' [逐设备进度](modules/media/native-equipment-500/quality-review-progress.json)单独记录；完成纸包装范围后停止，不代表整包完成。';p.write_text('\n'.join(lines)+'\n',encoding='utf-8')
p=home/'README.md';t=p.read_text(encoding='utf-8');t=re.sub(r'\*\*纸包装[^\n]+\*\*','**'+summary+'**',t,count=1)
line='- B01-Q10@1.1.0：在线喷码、辊式上光、UV固化3种通过；47字点/0.52875µL墨剂、4mL/20µm湿膜、必需灯回路、同纸身份顺序链；18工况/23作业、48 Native视图和两进程保存重开通过。'
if line not in t:t=t.replace('[纸包装范围]',line+'\n\n[纸包装范围]',1)
t=re.sub(r'\[本批复审记录\]\(reviews/[^)]+\)','[本批复审记录](reviews/B01-Q10/review-notes.md)',t);p.write_text(t,encoding='utf-8')
p=root/'docs/04_开发与验收/剩余任务Todo.md';lines=p.read_text(encoding='utf-8').splitlines();lines[0]='> '+summary+' 见[范围](../../modules/media/native-equipment-500/paper-packaging-review-scope.json)和[进度](../../modules/media/native-equipment-500/quality-review-progress.json)；完成纸包装范围后停止，不关闭P3-02或正式NFR。';p.write_text('\n'.join(lines)+'\n',encoding='utf-8')
p=root/'docs/04_开发与验收/剩余任务Todo.json';v=read(p);b=v['bounded_native500_model_review'];b.update(accepted_device_types=27,pending_device_types=473);b['active_paper_packaging_scope'].update(accepted=27,pending=65);b['review_refs']=[f'modules/media/native-equipment-500/reviews/{batch}/quality-review.json'for batch in registry['review_batches']];b['accepted_revisions']=[batch+'@'+read(home/'reviews'/batch/'quality-review.json')['version']for batch in registry['review_batches']];write(p,v)
p=root/'docs/04_开发与验收/整包派发/P3-02_整包执行.json';v=read(p);reuse=v['sequential_closeout']['completed_reuse'];refs=[f'modules/media/native-equipment-500/reviews/B01-Q10/{f}'for f in ['quality-review.json','release-catalog.json','Native-render-checks.json','startup-registration-checks.json']]
if not any(refs[0]in e.get('evidence_refs',[])for e in reuse):reuse.append({'within_original_step':1,'scope':'B01-Q10：喷码、上光、UV固化3种按原设备身份1.1.0接收；18工况/23作业、真实同纸上光→固化顺序链、48 Native取景、两普通进程各51时点及菜单保存新进程重开、27定义/54媒体精确首启登记通过。纸包装27/92，65待审；不关闭P3-02或正式NFR。','evidence_refs':refs})
write(p,v)
(folder/'README.md').write_text('''# B01-Q10 · 喷码、上光与 UV 固化接收

在线喷码机、辊式上光机、UV 固化通道三个原生设备定义 1.1.0，保持原 sf.native50 身份。6 个设备 GLB、6 个物品 GLB 均可由包内源码独立重建；78 张模型/机构/工件图、18 工况及真实上光→固化同纸顺序链共 23 作业、48 个 Native 业务取景通过。透明薄膜条纹已修正后重验。

两普通 Native 进程各 51 个时点，持有/等待冻结、菜单保存后新进程重开、完整 Run 不变及正常关闭通过。当前普通 Native 首启登记为 27 定义/54 媒体，共 81 个精确模块；此前撤回的六个旧印刷模块版本仍未登记。

当前纸包装 27/92、65 待审；全目录 27/500、473 待审。参见[复审记录](review-notes.md)、[业务范围](review-scope.json)、[验收输入](acceptance-input.json)及[首启回执](startup-registration-checks.json)。仅为本批 AI 接收，不关闭整个 P3-02、正式 NFR 或人工验收。
''',encoding='utf-8')
print(summary)
