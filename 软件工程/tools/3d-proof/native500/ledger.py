"""Merge only this delivery's new/revised GLB observations into the existing MQ ledger."""
from pathlib import Path
import importlib.util,json
from datetime import datetime
ROOT=Path(__file__).resolve().parents[3]
spec=importlib.util.spec_from_file_location('mq',ROOT/'tools/check_model_quality.py');mq=importlib.util.module_from_spec(spec);spec.loader.exec_module(mq)
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
home=ROOT/'modules/media/native-equipment-500';out=ROOT/'scenarios/媒体表现/native500-20261024'
inventory=read(home/'inventory.json');ledger=read(mq.LEDGER);rows={r['model_path']:r for r in ledger['models']}
proof=read(ROOT/'tools/3d-proof/native500/source-rebuild-checks.json');assert proof['models']==910
done={(r['id'],r['lod']) for r in proof['results'] if r['same_export_bytes']}
render={}
for b in inventory['batches']:
 for r in read(out/('render-'+b['id'].lower()+'.json'))['checks']:render[r['id'],r['lod']]=r
reasons={
 'MQ-01':'七向实际画面与米/Y-up声明已提供；逐设备合理比例、端口/工装同尺度的完整认可待完成。',
 'MQ-02':'封闭机架、接地与背底图已提供；逐设备缺面、悬空及运动穿插的完整目视接收待完成。',
 'MQ-03':'每个工艺具名机构和可编辑源已提供；不能由共享生成器或工艺名称推定关键机构符合实际类别，仍待逐设备精审。',
 'MQ-04':'连接件、面板、导轨驱动和服务通道已制作；逐部件支撑、走线连续及随动认可待完成。',
 'MQ-05':'金属/喷涂/橡胶材质与原创2K表面已实际重开；逐设备等效光照/尺度参照比较待完成。',
 'MQ-06':'实际产品解码绑定、所有阶段正反向seek与唯一动画通道技术检查通过；逐部件动作随动/脱离审查待完成。',
 'MQ-07':'未逐设备执行实际物品holder、支承、夹取和移交验收；预览没有生成业务物品。',
 'MQ-08':'未逐设备执行Native缺料、资源等待、服务中断、输出阻塞、暂停回看与保存重开联合验收。'}
updated=0
for e in inventory['entries']:
 if e['batch']=='B01' and not e.get('revised_from'):continue
 assert (e['id'],e['suffix']) in done
 check=render[e['id'],e['suffix']];assert check['bytes']==e['bytes'] and check['nodes']==e['nodes']
 path=(home/e['directory']/e['asset']['path']).resolve();name=mq.relative(path);observed=mq.observation(path)
 prefix=mq.relative(out/e['id']);view_names={'isometric':'iso','front':'front','back':'back','left':'left','right':'right','top':'top','bottom':'bottom'}
 views={k:prefix+'/'+e['suffix']+'-'+v+'.png' for k,v in view_names.items()};assert all((ROOT/v).is_file() for v in views.values())
 evidence=['tools/3d-proof/native500/package-checks.json','tools/3d-proof/native500/source-rebuild-checks.json',mq.relative(out/('render-'+e['batch'].lower()+'.json'))]
 review=dict(reviewed_file=name,file_revision={k:observed[k] for k in ['bytes','modified_ns']},reviewer='Codex AI technical checks and batch overview; full individual visual acceptance pending',reviewed_at=datetime.now().astimezone().isoformat(),human_acceptance='pending',appearance_review='batch_overview_and_selected_closeups_only',open_defects=[],scope='candidate construction and technical checks; not formal visual or Native acceptance',
  reference_comparison=dict(reference_files=['scenarios/媒体表现/native50-20261021/screw-assembly/detailed-iso.png'],evidence=[views['isometric']],conclusion='相同产品呈现链路、同源材质基准；尚未完成每个设备与对应参照的等效观察尺度精审。'),
  gates=dict(source_export=dict(result='pass',evidence=evidence[:2]),visual=dict(result='pending',reason='七向图和关键姿态已保存，完整逐设备画面接收尚未完成。',evidence=[views['isometric']]),platform_reopen=dict(result='pending',reason='Host登记/查询通过；尚未执行每个设备Native实际Run、等待和保存重开。',evidence=evidence[:1])),
  dimensions={k:dict(result='pending',reason=v,evidence=evidence) for k,v in reasons.items()},views=views,closeups=[prefix+'/action-'+str(u)+'.png' for u in [0,50,100]] if e['suffix']=='detailed' else [],motion_evidence=evidence)
 rows[name]=dict(model_path=name,category=mq.category(path),asset_quality='pending',observed=observed,review=review);updated+=1
ledger['models']=sorted(rows.values(),key=lambda r:r['model_path']);mq.LEDGER.write_text(json.dumps(ledger,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps(dict(new_or_revised_models=updated,formal_acceptance='pending',unrelated_records='preserved')))
