from pathlib import Path
import json

home = Path('modules/media/native-equipment-500/reviews')
def read(p): return json.loads(p.read_text(encoding='utf8'))
def write(p, v): p.write_text(json.dumps(v, ensure_ascii=False, indent=2) + '\n', encoding='utf8')

for old, batch, device, proof, attempt, render, views, jobs in [
    ('B01-Q02', 'B01-Q02-R1', 'flexo-printer', 'b01q02r1', 'attempt-01', 'b01q02r1-render-01', 29, 8),
    ('B02-Q03', 'B02-Q03-R1', 'flexo-slotter', 'q03r1', 'attempt-02', 'q03r1-render-01', 25, 9),
]:
    folder = home / batch
    write(folder / 'acceptance-input.json', {
        'host': f'.work/native500-quality-20261025/{proof}/{attempt}',
        'native_persistence': f'.work/native500-quality-20261025/{proof}/Native-persistence-r1-checks.json',
        'native_render': f'.work/native500-quality-20261025/{render}',
    })
    visual = read(home / old / 'visual-review.json')
    visual['dimensions'] = {device: visual['dimensions'][device]}
    visual['dimensions'][device]['MQ-06'] = '1.1.1修正印辊/承托辊/网纹辊旋向及凸版初相位；源码和实际GLB切点实测与纸路同向同速，正反seek通过。'
    visual['dimensions'][device]['MQ-08'] = '本修订7个真实工况、16个Native业务视图、两普通Native进程各20时点及保存重开通过；旧1.1.0仍撤回，不借用其视图补签。'
    visual['scope'] = '只接收本修复设备1.1.1；双LOD七向/机构/真实物品及16个Native视图逐机AI复审，不代人工验收。'
    if old == 'B02-Q03':
        visual['dimensions'][device]['MQ-04'] = '闭合输送带、轴承/驱动及安装座、刀轴支撑、墨杯/供墨管/刮刀承托、侧导向、废料车轮架、透明护罩、操作屏/急停/灯和原创铭牌。'
    write(folder / 'visual-review.json', visual)
    scope = read(home / old / 'review-scope.json')
    scope.update(review_batch=batch, version='1.1.1', repair='Printing tangent direction and relief angular phase repaired; old1.1.0 remains withdrawn.')
    if old == 'B02-Q03':
        scope['equipment'] = [e for e in scope['equipment'] if e['id'] == device]
    write(folder / 'review-scope.json', scope)
    issues = [{
        'id': batch + '-direction', 'observed': '1.1.0印刷接触面与纸路前进方向相反。',
        'fix': '修正印辊/承托辊/网纹辊旋向并镜像凸版相位；源码及实际GLB切点同向同速检查与全部业务/Native验收重新通过。',
        'status': 'resolved',
    }]
    if old == 'B02-Q03':
        issues.append({'id': batch + '-nip', 'observed': '旧版印辊低于纸顶，未明确计入墨层间隙及刮刀支撑。', 'fix': '按纸路/纸厚/墨层/版半径计算两辊轴线，刮刀同步抬升并增加轴承承托桥。', 'status': 'resolved'})
        business = '0.30m²纸板和2mL实际油墨变为0.2964m²坯、六条各0.0006m²边料，七个输出保留两个输入来源。'
    else:
        business = '同一纸张保持身份；蓝/红两个独立0.2mL batch墨剂，分别形成20µm固定色块，单独缺墨和体积守恒核对。'
    write(folder / 'open-defects.json', {'issues': issues})
    notes = f'''# {batch} · {device}@1.1.1修复重验

旧1.1.0因打印接触面逆向撤回，本目录为独立1.1.1；不覆盖旧模型、源码、Run或回执。修正三类印刷辊旋向，镜像凸版角相位以保持转印时点；源码和实际产品GLB分别核对切点同向同速。双色为2m/归一化auto，印刷开槽为1.4m/归一化auto；实际凸版在色块中点接触完整墨层。

{business} 原默认工艺、质量和阶段资源契约保持，适用出料及独立边料阻塞均恢复。宏观示例不声明任意图案/RIP、干燥、流体或接触力求解。

双LOD2个设备GLB及实际物品均由包内源码独立重建一致。{views}张实际GLB图、{jobs}个Host任务、7个设备工况重新通过。最终Host为`.work/native500-quality-20261025/{proof}/{attempt}`。普通Native两个独立进程各20个历史时点，菜单保存至`{proof}/native-saved-r1/saved`后新进程重开；完整Run保持，14条逐工况持有检查和正常关闭通过。最终16个Native视图为`{render}`，不借用旧视图；不以整幅PNG相等替代暂停有效进度检查。

逐机MQ八项见visual-review.json，实际输入见acceptance-input.json。只有本修订全部通过才恢复一个设备类型计数，首启精确登记另验。旧1.1.0继续撤回；本批不关闭P3-02、正式NFR或人工验收。
'''
    (folder / 'review-notes.md').write_text(notes, encoding='utf8')

print('Both repaired revision review inputs prepared; publication remains gated.')
