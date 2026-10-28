"""Map the unchanged real factory operations to external-execution implementation needs."""
import argparse
from collections import Counter
from pathlib import Path
from prepare_full_factory import ROOT, read, write


def prepare(source):
    p=read(source/'actual-frozen-project.json');s=read(source/'actual-scenario.json');run=read(source/'actual-connected-run.json')
    definitions={(m['definition']['id'],m['definition']['version']):m['definition'] for m in read(source/'actual-frozen-modules.json')}
    devices={d['id']:d for d in p['devices']};tasks={t['id']:t for t in s['production_tasks']}
    product_routes={x['routing_ref']['id'] for x in p['products']}
    jobs=[j for j in run['jobs'] if j['execution_ref']['kind']=='project_device']
    groups={}
    for job in jobs:
        e=job['execution_ref']['data'];ref=job['operation_ref'];key=(job['task_ref'],e['route_step_id'],e['device_id'],ref['id'],ref['version'])
        groups.setdefault(key,[]).append(job)
    labels={'multiphase':'多阶段回执','pause_resume':'暂停/续作','family':'工艺族及换型衔接','material_flow':'按阶段耗料与交付','quality':'质量判定及分流','outputs':'多输出或非单位产出'}
    rows=[]
    for (task,step,device,identity,version),actual in sorted(groups.items()):
        definition=definitions[(identity,version)];payload=definition['payload']
        if payload['category']=='devices':
            behavior=payload['data']['behavior']
            if behavior['selected']=='inline':op=behavior['inline']
            else:
                ref=behavior['operation_ref'];op=definitions[(ref['id'],ref['version'])]['payload']['data']
        else:assert payload['category']=='operations';op=payload['data']
        phases=op['phases'];reasons=[]
        if len(phases)!=1:reasons.append('multiphase')
        if any(x['interruption_policy']=='pause_resume' for x in phases):reasons.append('pause_resume')
        if op.get('family'):reasons.append('family')
        if op.get('material_flow'):reasons.append('material_flow')
        if op.get('quality_and_routes'):reasons.append('quality')
        outputs=op['outputs']
        if len(outputs)!=1 or outputs[0]['quantity']!={'value':{'coefficient':1,'decimal_places':0},'unit':'piece'}:reasons.append('outputs')
        connected=tasks[task]['routing_ref']['id'] in product_routes
        rows.append({'task_id':task,'step_id':step,'device_id':device,'device_title':devices[device]['title'],
            'operation_ref':{'id':identity,'version':version},'connected_product_route':connected,'local_completed_jobs':len(actual),
            'phases':[{'id':x['id'],'duration':x['duration'],'resources':x['phase_resources']} for x in phases],
            'identity_policy':op['identity_policy'],'inputs':op['inputs'],'outputs':outputs,'material_flow':op.get('material_flow'),
            'static_required_extensions':reasons,'external_verified':False})
    connected=[r for r in rows if r['connected_product_route']]
    counts=Counter(reason for r in connected for reason in r['static_required_extensions'])
    bindings={(r['device_id'],r['operation_ref']['id'],r['operation_ref']['version'],phase['id']) for r in connected for phase in r['phases']}
    matrix={'scope':'Actual template1.6.0 frozen input and completed local Run','classification_kind':'Static operation contract mapping; not per-operation external execution proof',
            'source':str(source),'devices':len(devices),'connected_devices':len({r['device_id'] for r in connected}),
            'local_manufacturing_jobs':len(jobs),'local_transport_trips':len(run['transport']),
            'connected_route_steps':len(connected),'connected_steps_requiring_extensions':sum(bool(r['static_required_extensions']) for r in connected),
            'connected_reason_step_counts':dict(counts),'rows':rows,'whole_factory_external_verified':False}
    matrix['required_exact_device_operation_phase_bindings']=len(bindings)
    target=ROOT/'docs/06_v2.0设计与开发控制';write(target/'任务8_整厂外部执行能力表.json',matrix)
    lines=['# 任务8：整厂外部执行能力与实施顺序','',
        '基线：已验证模板1.6.0。下面使用真实冻结定义与本地Run，区分本地生产结果和外部设备反馈；静态分类不代表云端已执行。',
        f"本地已完成{len(jobs)}次制造、{len(run['transport'])}趟运输；31台联线、61个独立单元。已联线工艺包含{len(connected)}个路线步骤，当前均需扩展外部执行契约。",'',
        '## 已联线工序的具体缺口','',
        '| 设备 | 路线步骤 | 本地次数 | 阶段 | 必须补齐 |','| --- | --- | ---: | --- | --- |']
    for r in connected:lines.append('| '+r['device_title']+' | '+r['step_id']+' | '+str(r['local_completed_jobs'])+' | '+' → '.join(x['id'] for x in r['phases'])+' | '+'、'.join(labels[x] for x in r['static_required_extensions'])+' |')
    lines += ['', '运输单独建模：119趟包含装载、去程、卸载和返程。不得只发布制造命令而将本地预测运输计成云执行完成。完整JSON另保留61个独立单元，未从总目标删除。', '',
        '## 开发顺序与精确落点','',
        '1. `packages/simulation/src/external.rs`：候选按原工序的阶段冻结；下一阶段仅在前阶段真实完成后允许派发。已开始工序保持原设备，禁止阶段间改派造成物料瞬移。每阶段完成只推进当前阶段，最终产出仍由原Engine提交。',
        '2. `packages/commands/src/execution_session.rs:1051,1080` 与 `host_execution_preparation.rs`：当前明确限制每设备仅一个绑定，DeviceSlot也只保存一个绑定，须改为同一物理设备持有多个精确工序/阶段绑定，共享同一boot/观测/忙闲状态。调度时按当前工序阶段选绑定；不能把三个阶段伪装成三台设备。`execution_session.rs:1721`和`external.rs:995`队列区分未开始工序与等待下一阶段的在制工序；截止时停止新工序、允许已接纳工序完成。',
        '3. `packages/simulation/src/external.rs`、`engine.rs` 与原Stock：保留阶段预留、耗料、质量、产出、等待输出空间、父子容器身份和共享资源；不得将多阶段合成一个虚拟单阶段绕过边界。按实际输出/订单验收定义目标，移除“末端作业数等于产品数量”对整厂的错误假设。',
        '4. `integrations/devices/virtual_device.py` 和现有云绑定：沿用phase_id/token，注册完整阶段集合；运输需真实运输反馈契约，不能沿用process_unit假装返程完成。联验区分SIL反馈和真实硬件。',
        '5. 验证逐级覆盖：load/auto/unload不提前产出；重复/乱序回执不重复扣料；阶段切换资源正确；故障保留在制品；完整三箱装托耗膜与嵌套身份；最后是普通Native到现有Sealos应用的实际回执闭环。','',
        '当前已实现：不支持工艺的错误指出设备、作业、精确工序版本、阶段及原因。当前未实现：整厂多阶段外部执行、阶段耗料、运输反馈及整厂云端联验。1.6.0独立包仍是上一轮已验包，本轮源码诊断修改尚未重新打包。','']
    (target/'任务8_整厂外部执行能力与实施顺序.md').write_text('\n'.join(lines),encoding='utf-8')
    print({k:v for k,v in matrix.items() if k!='rows'})


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('source',type=Path)
    prepare(parser.parse_args().source)
