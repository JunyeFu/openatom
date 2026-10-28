"""Render and verify the authored audit record; does not decide design quality."""
from pathlib import Path
from collections import Counter
import json
import os

ROOT = Path(__file__).resolve().parents[2]
DOC = ROOT / 'docs/04_开发与验收/设计阶段全量质量检查.json'
MD = DOC.with_suffix('.md')


def resolve_pointer(data, pointer):
    for token in pointer.lstrip('/').split('/') if pointer else []:
        token = token.replace('~1', '/').replace('~0', '~')
        data = data[int(token)] if isinstance(data, list) else data[token]
    return data


def link(ref, label=None):
    path = Path(ref) if Path(ref).is_absolute() else ROOT / ref
    target = os.path.relpath(path, MD.parent).replace('\\', '/') if path.drive == MD.drive else path.as_posix()
    return f'[{label or path.name}]({target})'


def main():
    d = json.loads(DOC.read_text(encoding='utf-8'))
    maturity = json.loads((ROOT / 'docs/04_开发与验收/软件开发流程与当前成熟度评估.json').read_text(encoding='utf-8-sig'))
    expected = {c['id'] for p in maturity['phases'] if p['id'] in d['scope']['stages'] for c in p['checkpoints']}
    assert len(d['coverage']) == len(expected) == d['summary']['reviewed_checkpoints'] == 30
    assert {row['id'] for row in d['coverage']} == expected
    ids = {f['id'] for f in d['findings']}
    assert len(ids) == len(d['findings'])
    closed = [f for f in d['findings'] if f['status'] == 'closed_after_repair_and_independent_review']
    active = [f for f in d['findings'] if f not in closed]
    assert len(active) == d['summary']['open_defects']
    assert dict(Counter(f['priority'] for f in d['findings'])) == {k: v for k, v in d['summary']['priority_counts'].items() if v}
    for row in d['coverage']:
        assert set(row['finding_ids']) <= ids
        for key in ['source', 'consumer']:
            assert (ROOT / row[key]).is_file(), row[key]
    for finding in d['findings']:
        assert set(finding['affected_checkpoints']) <= expected
        for evidence in finding['evidence']:
            path = ROOT / evidence['path']
            text = path.read_text(encoding='utf-8-sig')
            # Original evidence line numbers describe the pre-repair source snapshot.
            if 'line' in evidence and not d.get('repair_round'):
                assert text.splitlines()[evidence['line'] - 1].strip(), evidence
            if 'json_pointer' in evidence:
                value = resolve_pointer(json.loads(text), evidence['json_pointer'])
                assert value is not None, evidence
    repaired = d.get('repair_round', {}).get('status') == 'completed'
    lines = [
        '# S01—S06设计阶段全量质量检查与修复清单', '',
        f"初检日期：{d['audited_on']}。当前状态：{'修复及独立复核完成' if repaired else '质量修复进行中' if d.get('repair_round') else '初检完成'}。", '',
        f"**30/30项已检查；初检确认5个P2，现已关闭{len(closed)}项，仍待关闭{len(active)}项。产品任务仍按原准入执行。**", '',
        d['summary']['judgment'], '',
        d['scope']['coverage_meaning'], '',
        '## 检查技能与方法', '',
        f"已安装 {link(d['skill']['installed_path'], '$superfactory-design-quality-audit')}，并接入已有开发续接技能。一个入口包含契约完整、依据一致、模块独立、算法成本、变更局部性五类检查，适用阶段细则在其 references/review-method.md。", '',
        '检查沿“用户任务/规格 → 拥有者类型与规则 → 实际消费者 → 反例/有限证据 → 验收”进行。既有passed记录用于定位，不能代替语义核对；注明日期的历史结果不按现行错误处理。', '',
        f"技能结构验证通过；独立前向测试实际覆盖S04五项并发现三个状态序列缺陷，见{link(d['verification']['independent_forward_review_ref'])}。已据此补充“成功/认可/保存后再改输入、切工程或撤销”的检查要求。", '',
        '## 缺陷总表', '',
        '|ID|优先级|当前状态|问题|主要影响|推荐处理|',
        '|---|---|---|---|---|---|'
    ]
    for f in d['findings']:
        state = '已修复并独立复核' if f in closed else '修复待复核' if f['status'] == 'fixed_pending_independent_recheck' else '待关闭'
        lines.append(f"|{f['id']}|{f['priority']}|{state}|{f['title']}|{'、'.join(f['affected_checkpoints'])}|{f['recommendation']}|")
    if d.get('repair_round'):
        lines += ['', '## 六窗口本轮分工与实际交付', '', '|窗口|责任|当前状态|交付|', '|---|---|---|---|']
        for row in d['repair_round']['assignments']:
            delivery = link(row['delivery_ref']) if row.get('delivery_ref') and (ROOT / row['delivery_ref']).is_file() else '统筹源修正与最终收口' if row['window'] == '复审窗口1' else '进行中'
            lines.append(f"|{row['window']}|{row['scope']}|{row['status']}|{delivery}|")
        lines += ['', d['repair_round']['completion_rule']]
    lines += ['', '初检建议修复顺序：' + ' → '.join(d['repair_order']) + '。根因合并记录，不将同一旧摘要拆成多条以增加缺陷数。', '', '## 逐项缺陷与关闭条件', '']
    for f in d['findings']:
        state = '已修复并独立复核' if f in closed else '修复待复核' if f['status'] == 'fixed_pending_independent_recheck' else '待关闭'
        lines += [f"### {f['id']} · {f['title']}", '', f"{f['priority']}，{state}；归属：{f['owner']}。", '', '**原发现证据（行号为修前位置，JSON Pointer保留原语义入口）：**', '']
        for e in f['evidence']:
            position = f"，第{e['line']}行" if 'line' in e else ''
            pointer = f"，`{e['json_pointer']}`" if 'json_pointer' in e else ''
            observed = f"：{e['observed']}" if 'observed' in e else ''
            lines.append(f"- {link(e['path'])}{position}{pointer}{observed}")
        lines += ['', '**最小反例：**' + f['counterexample'], '', '**影响：**' + f['impact'], '', '**推荐修复：**' + f['recommendation'], '', '**关闭条件：**', '']
        lines += ['- ' + s for s in f['closure_criteria']]
        lines += ['', '复现证据：' + link(f['probe_ref'].split('#')[0]) + '。']
        if f.get('limits'):
            lines += ['', f['limits']]
        if f.get('repair'):
            lines += ['', '**实际修复：**', '']
            lines += ['- ' + s for s in f['repair'].get('changes', [])]
            for key in ['verification_refs', 'changed_refs']:
                if f['repair'].get(key):lines += ['', ('修后证据：' if key == 'verification_refs' else '修改文件：') + '、'.join(link(ref) for ref in f['repair'][key]) + '。']
        lines.append('')
    lines += ['## 30项检查覆盖', '', '|检查项|核对的设计决定|源 → 消费者|结论/缺陷|', '|---|---|---|---|']
    for row in d['coverage']:
        finding = '；' + '、'.join(row['finding_ids']) if row['finding_ids'] else ''
        lines.append(f"|{row['id']} {row['name']}|{row['checked']}|{link(row['source'])} → {link(row['consumer'])}|{row['conclusion']}{finding}|")
    lines += ['', '## 原AQ问题复核', '', '|原项|本轮结论|依据/证据范围|', '|---|---|---|']
    lines += [f"|{r['id']}|{r['conclusion']}|{r['evidence']}|" for r in d['AQ_regression_review']]
    lines += ['', '## 本轮验证及其含义', '',
              '- 30检查项、171登记入口均可读取；全部30项有源和消费者覆盖记录。',
              '- 初检结构探针核对两类21节点DAG、P5的20项前置闭包、16组件依赖、21任务六视图包、17SF/11NFR/38组/3IX及18组操作Schema引用；剩余本人115—189小时的JSON算术一致。',
              '- 初检Node探针复现三条原型状态反例，保存探针定位故障验收缺口；这些修前证据保留。技能结构与30项覆盖核对通过。']
    if repaired:
        closure = json.loads((ROOT / d['verification']['repair_closure_ref']).read_text(encoding='utf-8'))
        lines += [
            f"- DQ修复接收时实际Chrome原型回归{closure['actual_browser_design_prototype']['cases']}组通过；独立原型函数序列：交互/会话{closure['DQ03_DQ04']['actual_source_sequences']}条、专业生命周期{closure['DQ05']['actual_source_sequences']}条全部通过。当轮源码与交互复核所消费快照一致。",
            '- 保存四故障出口以有限文件夹具读回完整旧内容；三个“仅有备份但不完整”的负例均被拒绝。故障处理、旧内容保留、原目标健康与正常保存结果分别判定。',
            '- 非功能规格及主线同步检查通过，跨层独立复核无剩余DQ阻塞。真实Windows1176/1177、产品Host/Engine与真人/工业验收仍按原阶段门槛执行。']
    lines += ['',
              '可复跑命令（工程根目录；Python使用现有环境，quick_validate要求已有PyYAML）：', '', '```powershell']
    lines += d['verification']['commands']
    evidence_keys = ['repair_closure_ref', 'actual_browser_ref'] if repaired else ['structural_probe_ref', 'state_sequence_probe_ref', 'independent_forward_review_ref']
    lines += ['```', '', '验证输出：' + '、'.join(link(d['verification'][key]) for key in evidence_keys) + '。', '',
              d['verification']['interpretation'], '', '## 已知后续门槛，未重复列为新缺陷', '', '|范围|仍缺的实际证据|', '|---|---|']
    lines += [f"|{g['scope']}|{g['remaining']}|" for g in d['preserved_later_gates']]
    if d['verification'].get('subsequent_prototype_review_ref'):
        lines += ['', '后续原型逻辑修订与当前源码证据另见' + link(d['verification']['subsequent_prototype_review_ref']) + '；原DQ修复快照和独立序列保留为当轮证据。']
    lines += ['', d['summary']['acceptance_implication'], '', d['repair_scope_rule'], '',
              '机器记录：' + link(str(DOC.relative_to(ROOT))) + '；入口清单：' + link(d['scope']['source_inventory_ref']) + '。', '']
    MD.write_text('\n'.join(lines), encoding='utf-8')
    result = {'status': 'audit_record_consistency_and_evidence_locations_verified', 'coverage': len(expected),
              'defects': len(ids), 'priorities': d['summary']['priority_counts'], 'MD_rendered_from_JSON': True,
              'closed_defects': len(closed), 'open_defects': len(active),
              'repair_round_complete': repaired, 'source_designs_modified': bool(d.get('repair_round')),
              'actual_product_runtime': None}
    (ROOT / 'tools/design-quality-audit/output/report-validation.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(result, ensure_ascii=False))


if __name__ == '__main__':
    main()
