"""将已确认Q选择同步到原S/P任务要求；不改复审进度或在审交互文件。"""
from pathlib import Path
import json
import os
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
A = 'docs/04_开发与验收/'
B = 'docs/00_项目基线/'
QREF = A + '全量设计编码确认/项目全量设计编码问答.json'
CREF = B + '全量设计编码确认选择.json'
SREF = A + '全量设计编码确认/SXX-XX_确认选择落实.json'
SMREF = SREF.replace('.json', '.md')
TIMELINE_REF = 'docs/02_交互设计/时间轴主轴与轨道组_监督细化.json'
REPORT = A + '全量设计编码确认/确认选择逐文件更新结果.json'

# 当前复审窗口2的交互源、公共Commands及公共工量进度保持只读。
PENDING = {
    'docs/02_交互设计/业务界面与方案评审.md': list(range(3, 20)) + [20, 25, 30, 37, 41, 51, 53, 54],
    'docs/02_交互设计/完整操作流程与方案交付.md': [4, 6, 7, 8, 9, 10, 11, 12, 20, 25, 37, 51, 53],
    'docs/02_交互设计/自定义设备创作与采用.md': [9, 10, 12, 14, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30],
    'docs/02_交互设计/设备时间轴与标签设计.md': [10, 12, 16, 17, 29, 33, 37],
    'docs/02_交互设计/主流程与视图导航.md': [4, 6, 7, 8, 9, 10, 11, 12, 37, 51, 53],
    'docs/02_交互设计/交互流程与视图导航.json': [4, 6, 7, 8, 9, 10, 11, 12, 37, 51, 53],
    'packages/presentation/README.md': [13, 14, 15, 16, 17, 18, 19, 29, 33],
    'docs/03_架构与契约/首版类型格式与操作契约.md': [8, 10, 25, 29, 30, 43, 47, 49, 51, 52, 53, 55, 56],
    'packages/commands/command-catalog.design.json': [10, 47, 49],
    A + '任务框架与工量里程碑.json': [5, 10, 63],
}

DOCS = {
    B + '首版交付基线.md': list(range(1, 65)),
    B + '用户任务与产品边界.md': [1, 2, 3, 4, 5, 11, 58, 59, 60, 62, 63, 64],
    B + '技术与经济可行性.md': [5, 13, 15, 18, 19, 42, 45, 49, 57, 58, 59, 60, 63],
    'docs/01_业务需求/订单工艺与物料分配.md': [21, 30, 31, 34, 35, 36],
    'docs/01_业务需求/换单班次与质量处置.md': [23, 24, 31, 32, 34, 35, 36],
    'docs/01_业务需求/非功能需求与测量验收.md': [11, 12, 17, 18, 19, 42, 51, 52, 56, 58],
    'docs/03_架构与契约/九类模块清单与消费者.md': list(range(20, 30)) + [9, 10, 14, 61],
    'docs/03_架构与契约/专业作业与模块接口.md': [28, 38, 39, 40, 41, 42, 43, 44],
    'docs/03_架构与契约/全厂时间轴与阶段资源设计.md': [16, 17, 23, 24, 26, 30, 31, 32, 33, 34, 35, 36, 37],
    'docs/03_架构与契约/通用Agent接入与专业软件连接器.md': list(range(45, 51)) + [10, 20, 28, 33, 54, 59],
    'docs/03_架构与契约/目标驱动厂房生成.md': list(range(45, 51)) + [4, 8, 9, 27, 34, 53, 54, 57],
    'docs/03_架构与契约/工程交接与打开.md': [11, 25, 44, 51, 52, 53, 55, 56, 58, 59, 62],
    'docs/03_架构与契约/评估报告与数据导出.md': [3, 37, 44, 49, 53, 54, 55, 56, 57, 62],
    'docs/05_开源与参赛/社区贡献与版本维护.md': [5, 52, 55, 60, 61, 64],
    'packages/packaging/README.md': list(range(38, 45)) + [28],
    'packages/module-kit/demo-equipment/README.md': list(range(13, 20)) + [2, 29, 33],
    'packages/module-kit/README.md': list(range(20, 30)) + [9, 10, 14, 61],
    'modules/README.md': list(range(20, 29)) + [25, 61],
    'modules/templates/README.md': [4, 9, 27, 46, 48, 49, 50],
    'community/GOVERNANCE.md': [5, 55, 60, 61, 64],
    'community/CONTRIBUTING.md': [20, 25, 27, 55, 60, 61, 64],
    'community/templates/README.md': [20, 25, 27, 55, 60, 61],
}
JSONS = {
    B + '首版确认选择.json': list(range(1, 65)),
    B + '项目主线.json': list(range(1, 65)),
    'packages/module-kit/category-contracts.design.json': list(range(20, 30)) + [9, 10, 14, 61],
    'packages/module-kit/demo-equipment/definitions.design.json': list(range(13, 20)) + [2, 29, 33],
    'packages/domain/factory-generation.design.json': list(range(45, 51)) + [4, 8, 9, 27, 34],
    'packages/reporting/evaluation-report.contract.design.json': [3, 37, 44, 49, 53, 54, 55, 56, 57],
    'docs/01_业务需求/非功能需求与测量验收.json': [11, 12, 17, 18, 19, 42, 51, 52, 56, 58],
}
EXTRA_S = {
    'S02-01': [10, 19, 42, 49, 60], 'S02-02': [10, 49], 'S02-03': [10, 47],
    'S02-04': [10], 'S03-01': [10], 'S03-05': [10, 19, 42, 49, 62],
    'S05-01': [10, 11, 20, 28, 33, 47, 56], 'S05-02': [10],
    'S06-02': [10], 'S06-05': [10, 19, 42, 49, 62],
    'S07-01': [10, 20, 58, 59], 'S07-02': [3, 58, 59], 'S07-03': [5, 55, 60],
    'S07-05': [10, 19, 33, 42, 49, 51, 52],
    'S08-01': [10, 11, 21, 25, 30, 51, 52, 55, 56],
    'S08-02': [16, 17, 21, 22, 23, 24, 26, 31, 32, 33, 34, 35, 36],
    'S08-03': [3, 6, 7, 8, 9, 10, 12, 20, 29, 30, 37, 53],
    'S08-04': list(range(13, 19)) + list(range(20, 30)),
    'S08-05': [10, 21, 24, 31, 33, 35, 36, 43, 49, 51, 52],
    'S09-01': [2, 10, 19, 33, 42, 43, 49, 51, 52, 56, 62],
    'S09-02': [10, 20, 25, 28, 33, 43, 47, 49, 54, 55],
    'S10-03': list(range(1, 65)),
    'S11-04': [25, 60, 64],
}

def read(rel):
    return json.loads((ROOT / rel).read_text(encoding='utf-8-sig'))

def write_json(rel, data):
    write_text(rel, json.dumps(data, ensure_ascii=False, indent=2) + '\n')

CHANGES = []
def write_text(rel, text):
    assert rel not in PENDING, rel
    path = ROOT / rel
    old = path.read_text(encoding='utf-8-sig') if path.exists() else None
    if old != text:
        path.write_text(text, encoding='utf-8')
    CHANGES.append(rel)

def qids(nums):
    return [f'Q{n:02}' for n in sorted(set(nums))]

def source_link(rel, target):
    return os.path.relpath(ROOT / target, (ROOT / rel).parent).replace('\\', '/')

def replace_section(rel, body):
    path = ROOT / rel
    old = path.read_text(encoding='utf-8-sig')
    begin = '<!-- confirmed-choice-source:start -->'
    end = '<!-- confirmed-choice-source:end -->'
    block = begin + '\n' + body + '\n' + end
    if begin in old:
        start = old.index(begin)
        stop = old.index(end, start) + len(end)
        new = old[:start] + block + old[stop:]
    else:
        # SKILL的YAML头保持完整；普通Markdown仍插入首个标题后。
        if old.startswith('---\n'):
            header_end = old.index('\n---', 4) + len('\n---')
            head, rest = old[:header_end], old[header_end:]
        else:
            head, rest = old.split('\n', 1)
        new = head + '\n\n' + block + '\n\n' + rest.lstrip('\n')
    write_text(rel, new)

def attach_timeline_requirements(projection):
    """消费现行时间轴选择；只增加原 S 项要求，不改变进度或公共类型。"""
    timeline = read(TIMELINE_REF)
    if 'checkpoint_requirements' not in timeline:
        return
    decisions = {d['id']: d for d in timeline['decisions']}
    detail_ref = timeline.get('complete_definition_ref')
    detail = read(detail_ref) if detail_ref else None
    if detail_ref:
        projection['timeline_complete_definition_ref'] = detail_ref
    projection['timeline_selection_ref'] = TIMELINE_REF
    projection['timeline_application_rule'] = timeline['application_rule']
    projection['timeline_contract_requests_ref'] = TIMELINE_REF + '#/public_contract_change_requests'
    for checkpoint in projection['checkpoints']:
        requirement = timeline['checkpoint_requirements'].get(checkpoint['id'])
        for key in ['timeline_decision_ids', 'timeline_requirements', 'timeline_required_delivery',
                    'timeline_consumer_task_ids', 'timeline_source_refs',
                    'timeline_detail_section_ids', 'timeline_detail_ref']:
            checkpoint.pop(key, None)
        if requirement is None:
            continue
        ids = requirement['decision_ids']
        checkpoint['timeline_decision_ids'] = ids
        checkpoint['timeline_requirements'] = [
            {'decision_id': ident, 'selected_option': decisions[ident]['selected_option'],
             'requirement': decisions[ident]['specification']} for ident in ids]
        checkpoint['timeline_required_delivery'] = requirement['required_delivery']
        checkpoint['timeline_consumer_task_ids'] = requirement['consumer_task_ids']
        checkpoint['timeline_source_refs'] = [TIMELINE_REF] + requirement['source_refs']
        if detail:
            checkpoint['timeline_detail_ref'] = detail_ref
            checkpoint['timeline_detail_section_ids'] = [
                s['id'] for s in detail['sections'] if checkpoint['id'] in s['checkpoints']]
            if checkpoint['id'] in {'S04-04', 'S04-05', 'S06-05', 'S08-03',
                                    'S09-01', 'S09-02', 'S09-03', 'S10-01', 'S10-02', 'S10-03'}:
                checkpoint['timeline_detail_section_ids'] = [s['id'] for s in detail['sections']]
            if detail_ref not in checkpoint['timeline_source_refs']:
                checkpoint['timeline_source_refs'].append(detail_ref)


def render_requirement_projection(projection):
    lines = ['# SXX-XX任务的Q01—Q64确认要求与时间轴细化', '',
             '本文件保留原60个S编号与Q01—Q64选择；时间轴T01—T36按用户本轮授权落实到适用S项。'
             '不新增任务、完成数或运行结果，原退出条件继续适用。', '',
             f'[机器要求]({Path(SREF).name}) · [Q选择来源]({source_link(SMREF, CREF)}) · '
             f'[时间轴选择]({source_link(SMREF, TIMELINE_REF)}) · [现行阶段契约](../阶段实现计划.json)', '',
             'Q选择与时间轴细化是设计和验收输入，不是通过记录。公共字段消费现行W00唯一Rust/API及派生记录，'
             'S04视觉认可、原型及本人或用户授权AI代走查按实际证据核验。既有20条基线及旧Q1—Q12答案保留。', '']
    for checkpoint in projection['checkpoints']:
        lines += [f"## {checkpoint['id']} {checkpoint['title']}", '',
                  '|题号|本项必须落实的选择|', '|---|---|']
        lines += [f"|{v['question_id']}|{v['selected_text']}|" for v in checkpoint['selected_requirements']]
        lines += ['', '具体细化：' + ('、'.join(checkpoint['detail_ids']) or '沿原任务规格') +
                  '；Q要求编码消费者：' + '、'.join(checkpoint['task_ids']) + '。', '']
        if checkpoint.get('review_evidence_policy'):
            policy = checkpoint['review_evidence_policy']
            lines += ['本项最新操作者授权：' + policy['rule'], '',
                      '依据：[' + policy['authorization_message_id'] + '](' + source_link(SMREF, policy['canonical_ref']) + ')。', '']
        if checkpoint.get('timeline_requirements'):
            lines += ['本项时间轴要求：', '', '|编号|本项必须落实的时间轴规定|', '|---|---|']
            lines += [f"|{v['decision_id']} · {v['selected_option']}|{v['requirement']}|"
                      for v in checkpoint['timeline_requirements']]
            lines += ['', '**本项时间轴交付：**' + checkpoint['timeline_required_delivery'], '',
                      '时间轴消费者：' + '、'.join(checkpoint['timeline_consumer_task_ids']) + '。', '',
                      '依据：' + ' · '.join(f'[{Path(ref).name}]({source_link(SMREF, ref)})'
                                           for ref in checkpoint['timeline_source_refs']) + '。', '']
            if checkpoint.get('timeline_detail_section_ids'):
                lines += ['完整定义须落实：' + '、'.join(checkpoint['timeline_detail_section_ids']) + '。', '']
    return '\n'.join(lines) + '\n'


def timeline_only():
    projection = read(SREF)
    attach_timeline_requirements(projection)
    write_json(SREF, projection)
    write_text(SMREF, render_requirement_projection(projection))
    print(json.dumps({'S_checkpoints': len(projection['checkpoints']),
                      'timeline_checkpoints': sum(bool(c.get('timeline_requirements')) for c in projection['checkpoints']),
                      'updated_files': [SREF, SMREF], 'public_progress_written': False}, ensure_ascii=False))


def main():
    questionnaire = read(QREF)
    choices = {q['id']: q for q in questionnaire['owner_questions']}
    confirmation = read(CREF)
    assert list(choices) == qids(range(1, 65))
    assert all(q['answer'] == 'A' and q['selected_text'] == q['options']['A'] for q in choices.values())
    maturity = read(A + '软件开发流程与当前成熟度评估.json')
    selection = read(B + '首版任务选择.json')
    stage = read(A + '阶段实现计划.json')
    # 已由各拥有者收口的同一组答案，重跑不得恢复早期待合入文案。
    if confirmation['application_state'].get('shared_sources_merged'):
        projected = {row['question_id']: row['selected_text']
                     for checkpoint in read(SREF)['checkpoints']
                     for row in checkpoint['selected_requirements']}
        current = {ident: row['selected_text'] for ident, row in choices.items()}
        if projected == current:
            print(json.dumps({'selected_questions': len(current),
                              'owner_completed_merge_preserved': True,
                              'public_progress_written': False}, ensure_ascii=False))
            return
    # 保留旧十二项选择及所有任务/阶段实际状态。
    old_twelve = read(B + '首版确认选择.json')['choices']
    old_task_state = [(t['id'], t['status'], t['actual_result'], t['execution_plan']['actual_result'], t['implementation_dependencies'], t['release_dependencies']) for t in selection['selected']]
    old_stage_state = [(p['id'], p['document_review_state'], p['actual_exit_result']) for p in stage['lifecycle_phases']]
    undo = confirmation['new_selected_requirement']
    choices['Q10']['task_refs'] = list(undo['task_refs'])

    projection = {'document_kind': 'selected_requirement_projection_for_existing_SXX_P_tasks_not_progress_or_new_backlog',
                  'selection_ref': CREF, 'questionnaire_ref': QREF,
                  'checkpoint_identity_source_ref': A + '软件开发流程与当前成熟度评估.json',
                  'task_identity_source_ref': B + '首版任务选择.json',
                  'rule': 'Q选择是现行设计/验收要求；具体设计成果与实际证据按原S任务完成，原review和实际结果不因选择更新而增加。',
                  'checkpoints': []}
    for phase in maturity['phases']:
        for c in phase['checkpoints']:
            ids = {q['id'] for q in choices.values() if c['id'] in q['checkpoint_refs']}
            ids.update(qids(EXTRA_S.get(c['id'], [])))
            details = [d['id'] for d in questionnaire['engineer_owned_details'] if c['id'] in d['checkpoint_refs']]
            projection['checkpoints'].append({'id': c['id'], 'title': c['checkpoint'],
                'question_ids': sorted(ids), 'selected_requirements': [{'question_id': ident, 'selected_text': choices[ident]['selected_text']} for ident in sorted(ids)],
                'detail_ids': details, 'task_ids': sorted({t for ident in ids for t in choices[ident]['task_refs']}),
                'source_refs': c['local_evidence'],
                'required_delivery': '落实本项所列已选要求及适用D细化；按原退出条件提供对应设计、试做、实现或真实验收成果。'})
    assert len(projection['checkpoints']) == 60
    assert all(c['question_ids'] for c in projection['checkpoints'])
    attach_timeline_requirements(projection)
    write_json(SREF, projection)
    write_text(SMREF, render_requirement_projection(projection))

    for rel, nums in DOCS.items():
        if not (ROOT / rel).exists():
            raise FileNotFoundError(rel)
        body = ['## 已确认的本文件设计要求（2026-10-14）', '',
                f'[统一选择]({source_link(rel, CREF)}) · [SXX逐项要求]({source_link(rel, SMREF)})', '',
                '下列为本文件当前设计/验收要求，具体细节仍按所属S任务落实；原文中的历史候选不代替这些选择。', '', '|题号|已选A及本文件要求|', '|---|---|']
        body += [f"|{ident}|{choices[ident]['selected_text']}|" for ident in qids(nums)]
        if rel == B + '技术与经济可行性.md':
            body += ['', 'Q10草稿撤销/重做已纳入原五项任务；现行增量与总预算取原工量框架的Q10_estimate_update及totals，不把旧历史总数当更新后预算。']
        replace_section(rel, '\n'.join(body))
    for rel in ['community/GOVERNANCE.md', 'docs/05_开源与参赛/社区贡献与版本维护.md']:
        old = (ROOT / rel).read_text(encoding='utf-8-sig')
        new = old.replace('代码计划采用Apache-2.0', '原创代码已选MIT候选').replace('代码拟采用Apache-2.0', '原创代码已选MIT候选')
        write_text(rel, new)

    for rel, nums in JSONS.items():
        data = read(rel)
        data['confirmed_design_choices'] = {'selection_ref': CREF, 'checkpoint_requirements_ref': SREF,
            'question_ids': qids(nums), 'requirements': {ident: choices[ident]['selected_text'] for ident in qids(nums)}}
        if 'category-contracts' in rel:
            for c in data['categories']:
                category_q = {'items':[21], 'devices':[9,24,25,29], 'resources':[22], 'tasks':[23], 'operations':[24], 'transport':[26], 'templates':[9,27], 'professional':[28], 'media':[14,29]}[c['id']]
                c['confirmed_question_ids'] = qids([20, 25, 61] + category_q)
            data['draft_edit_history_requirement_ref'] = CREF + '#new_selected_requirement'
        if 'factory-generation' in rel:
            data['delivery_policy']['supported_shortfall_or_failed_check']['max_feedback_repair_rounds'] = 3
            data['delivery_policy']['supported_shortfall_or_failed_check']['initial_run_counts_as_repair'] = False
            data['delivery_policy']['supported_shortfall_or_failed_check']['stop_on_no_effective_change'] = True
            data['allowed_repair_variables'] = {'default':['schedule','selected_template_layout','explicitly_allowed_device_instance_count'],
                'needs_explicit_authorization':['phase_duration','BOM','quality_rule','additional_personnel','cost_parameters']}
            data['first_client_validation_target'] = {'name':'Codex','actual_compatibility_result':None}
        if 'reporting' in rel:
            data['rules']['report_language'] = 'zh-CN; stable identifiers and code remain English'
            data['rules']['main_comparison'] = 'Display two scenarios together; retain additional saved scenarios. No comparison required for an initial successful run.'
            data['rules']['unknown_price'] = 'List missing prices and affected cost conclusions; do not fill zero or claim complete total cost.'
            data['optional_generation_context']['max_feedback_repair_rounds'] = 3
        if '非功能' in rel:
            nfr = next(x for x in data['requirements'] if x['id'] == 'NFR-10')
            nfr['measurement']['trial_experience_target_seconds'] = 30
            nfr['measurement']['trial_target_kind'] = 'candidate_experience_target_not_hard_timeout_or_actual_measurement'
            nfr['measurement']['hard_solver_timeout_seconds'] = None
            procedure = '按Q42实际试做完整检查30秒体验目标，记录进度和可取消性；容差/预算仍须S06-04依据冻结，预算耗尽返回Undetermined，不为达标省略过程。'
            if procedure not in nfr['procedure']:
                nfr['procedure'].append(procedure)
        write_json(rel, data)

    selection['confirmed_choice_ref'] = CREF
    selection['checkpoint_choice_requirements_ref'] = SREF
    for task in selection['selected']:
        ids = sorted(q['id'] for q in choices.values() if task['id'] in q['task_refs'])
        task['confirmed_question_ids'] = ids
        task['confirmed_design_requirements'] = [{'question_id': ident, 'selected_text': choices[ident]['selected_text']} for ident in ids]
        task['confirmed_design_input_refs'] = [CREF, SREF]
        task['execution_plan']['input_refs'] = [ref for ref in task['execution_plan']['input_refs'] if ref not in [CREF, SREF]]
        if task['id'] in undo['task_refs']:
            task['confirmed_undo_requirement'] = {key: undo[key] for key in ['supported_scope','revision_policy','file_policy','history_policy','acceptance_expectations']}
            scope = '本次会话内草稿撤销/重做，经共同Commands形成新修订，不回滚当前运行/认可/外部动作'
            if scope not in task['scope']:
                task['scope'] += '；' + scope
            step = '接入本会话草稿撤销/重做：共同命令/修订冲突、redo分支与工程切换；与当前Run/人工认可/磁盘副作用分开'
            if step not in task['execution_plan']['steps']:
                task['execution_plan']['steps'].append(step)
            acc = '草稿编辑撤销/重做正确且产生新修订；旧Run/库存/认可不变，修订冲突不覆盖，新编辑清除失效redo，跨工程不串历史'
            if acc not in task['release_acceptance']:
                task['release_acceptance'] += '；' + acc
            task.setdefault('incremental_effort_due_to_Q10', {'status':'pending_concrete_design_estimate','existing_budget_includes_Q10':False,'framework_owner':'W00','human_hours':None,'window_active_hours':None})
        if task['id'] == 'P3-02':
            task['scope'] = task['scope'].replace('原生简模与代表性动画', '完整可旋转3D模型、三档呈现与代表性物品动画')
        if task['id'] in ['P3-04','P4-01']:
            task['feedback_repair_limit'] = {'max_rounds':3,'initial_run_counts':False,'stop_on_target_met_or_missing_inputs_or_no_effective_change':True}
        if task['id'] == 'P4-02':
            task['packaging_trial_target'] = {'full_check_seconds':30,'kind':'experience_trial_not_hard_timeout','geometric_tolerance':'explicit_job_input_per_packages/packaging/algorithm.design.json#/numerics','actual_measurement':None}
    write_json(B + '首版任务选择.json', selection)
    task_map = {task['id']: task for task in selection['selected']}
    backlog = read(A + 'tasks.json')
    backlog_states = [(task['id'], task['status'], task['actual_result']) for task in backlog['tasks']]
    backlog['confirmed_choice_ref'] = CREF
    backlog['checkpoint_choice_requirements_ref'] = SREF
    for task in backlog['tasks']:
        if task['id'] in task_map:
            current = task_map[task['id']]
            task['first_release_scope'] = current['scope']
            task['first_release_acceptance'] = current['release_acceptance']
            task['first_release_execution_plan'] = current['execution_plan']
            task['first_release_confirmed_question_ids'] = current['confirmed_question_ids']
            task['first_release_confirmed_design_requirements'] = current['confirmed_design_requirements']
    assert backlog_states == [(task['id'], task['status'], task['actual_result']) for task in backlog['tasks']]
    write_json(A + 'tasks.json', backlog)
    trace_rel = A + '首版需求验收追踪.json'
    trace = read(trace_rel)
    trace_states = [(row['task_id'], row['runtime_status'], row['actual_result']) for row in trace['task_projection']]
    trace['confirmed_choice_ref'] = CREF
    trace['checkpoint_choice_requirements_ref'] = SREF
    for row in trace['task_projection']:
        row['scope'] = task_map[row['task_id']]['scope']
        row['confirmed_question_ids'] = task_map[row['task_id']]['confirmed_question_ids']
    for sf in trace['functional_requirements']:
        ids = sorted({ident for task in selection['selected'] if task['id'] != 'P5-01' and sf['id'] in task['requirement_ids'] for ident in task['confirmed_question_ids']})
        sf['confirmed_question_ids'] = ids
        sf['confirmed_requirement_source_ref'] = SREF
    trace['confirmed_acceptance_supplements'] = [
        {'question_id':'Q10','task_ids':undo['task_refs'],'functional_ids':['SF-01','SF-03','SF-04','SF-05','SF-06'],
         'input':{'draft_revision':0,'object_position_m':[0,0,0],'edited_position_m':[1,0,0]},
         'procedure':['提交位置编辑','撤销此编辑','重做此编辑'],
         'expected':{'positions_m':[[1,0,0],[0,0,0],[1,0,0]],'new_revisions':[1,2,3],
                     'current_RunId_inventory_clock_acceptance_unchanged':True},
         'additional_expectations':undo['acceptance_expectations'],
         'exact_command_format':'S06-03待完成；预期冻结在接口实现前','actual_result':None},
        {'question_id':'Q42','task_ids':['P4-02'],'nonfunctional_ids':['NFR-10'],
         'expected':'完整检查30秒为体验试做目标；预算耗尽Undetermined，不删过程检查，不将30秒当硬超时或已有实测。',
         'actual_result':None},
        {'question_id':'Q49','task_ids':['P3-04','P4-01'],'functional_ids':['SF-17'],
         'expected':'初案达标零次修正；独立不足输入最多3轮反馈修正，每轮真实检查/可运行时实际重跑；达标/关键缺项/无有效改动立即结束。',
         'initial_run_counts_as_repair':False,'actual_result':None}]
    assert trace_states == [(row['task_id'],row['runtime_status'],row['actual_result']) for row in trace['task_projection']]
    write_json(trace_rel, trace)
    baseline = B + '首版交付基线.md'
    text = (ROOT / baseline).read_text(encoding='utf-8-sig')
    for ident, task in task_map.items():
        text = re.sub(r'^\|' + re.escape(ident) + r'\|[^\n]*$',
                      lambda match: f"|{ident}|{task['scope']}|{'、'.join(task['requirement_ids'])}|", text, flags=re.M)
    write_text(baseline, text)
    stage['confirmed_choice_ref'] = CREF
    stage['checkpoint_choice_requirements_ref'] = SREF
    stage['checkpoint_requirement_rule'] = '各SXX-XX除原输入/输出/退出要求外消费确认选择落实JSON中同ID记录；此要求更新不修改公共复审进度。'
    for p in stage['lifecycle_phases']:
        p['confirmed_question_ids'] = sorted({ident for c in projection['checkpoints'] if c['id'].startswith(p['id']+'-') for ident in c['question_ids']})
        p['confirmed_checkpoint_requirements_ref'] = SREF
        if SREF not in p['source_refs']:
            p['source_refs'].append(SREF)
    for family in stage['product_families']:
        for contract in family['task_contracts']:
            contract['scope'] = task_map[contract['task_id']]['scope']
            contract['confirmed_question_ids'] = task_map[contract['task_id']]['confirmed_question_ids']
        family['confirmed_checkpoint_requirements_ref'] = SREF
    write_json(A + '阶段实现计划.json', stage)
    replace_section(A + '阶段实现计划.md', '## 现行SXX确认要求\n\n[60项逐条要求](全量设计编码确认/SXX-XX_确认选择落实.md)及[机器要求](全量设计编码确认/SXX-XX_确认选择落实.json)是本阶段契约的设计/验收输入。Q01—Q64已选择A，21项P任务源同步消费；原复审进度、退出结果与授权保持原值。Q10已进入原五项任务，具体Commands/增量工量及在审交互源列在逐文件更新结果中。')
    replace_section(A + '任务拆解与依赖.md', '## 现行确认选择的任务投影\n\n原[首版任务源](../00_项目基线/首版任务选择.json)逐项包含confirmed_question_ids及confirmed_design_requirements，编码包由同一源生成。草稿撤销/重做进入P0-01/P2-01/P2-05/P3-01/P3-03；不新增P/SF编号，不改变两种前置。SXX逐项设计/验收要求见[确认落实](全量设计编码确认/SXX-XX_确认选择落实.md)。')
    # 实际Agent工作流保持英文，落实可执行限制。
    skill = 'integrations/agent/skills/superfactory-workflow/SKILL.md'
    replace_section(skill, '## Confirmed first-release choices\n\nRead the [selected decisions](../../../../docs/00_项目基线/全量设计编码确认选择.json) and the applicable requirements in the canonical selected task JSON. Codex is the first actual client validation target; support for other clients requires real validation. Ask for critical missing inputs together and label editable example assumptions. Edit only the authorized draft and fields through shared Commands. Default repair variables are scheduling, selected template layout and explicitly allowed device instance count; phase durations, BOM, quality, extra personnel and cost changes require explicit authorization.\n\nIf the initial run meets the goal, deliver it immediately. Otherwise allow at most three feedback repair rounds, excluding the initial run. Each round checks the changed draft and performs a real run when runnable; retain actual changes/results. Stop on meeting the target, critical missing inputs or no effective supported change. Preserve partial drafts and explain missing capabilities. Draft undo/redo creates a new revision and never reverses production, human acceptance or file exports. Packaging’s 30-second experience trial target does not establish a hard solver timeout. Deliver Chinese engineering reports from the same actual result in four formats and a separate editable package.')

    assert old_twelve == read(B + '首版确认选择.json')['choices']
    assert old_task_state == [(t['id'],t['status'],t['actual_result'],t['execution_plan']['actual_result'],t['implementation_dependencies'],t['release_dependencies']) for t in selection['selected']]
    assert old_stage_state == [(p['id'],p['document_review_state'],p['actual_exit_result']) for p in stage['lifecycle_phases']]
    rows = []
    for rel, nums in PENDING.items():
        if not (ROOT / rel).exists():
            continue
        field = '正文/载荷中的相应Q设计要求'
        detail = '精准合入所列选择并核对本窗口最新增量；保持复审状态/实际结果。'
        if 'command-catalog' in rel or '首版类型' in rel:
            field = '草稿编辑/undo-redo命令及修订、错误返回、消费者契约'
            detail = undo['revision_policy'] + ' ' + undo['file_policy'] + ' ' + undo['history_policy'] + ' 具体命令/可逆数据须S06-03设计；不凭此扩充18个实际可调用操作。'
        if '工量' in rel:
            field = '原五项P任务effort_basis/估计区间与totals/window_budget_summary'
            detail = '对P0-01/P2-01/P2-05/P3-01/P3-03按具体撤销接口估增量，再更新原框架合计；旧总预算不含Q10，不填伪造实际工时。'
        if '追踪' in rel:
            field = 'SF-03/SF-04/SF-05相关Q10编辑案例；NFR-10 Q42；GF反馈Q49'
            detail = '沿原SF编号/实际结果槽落实撤销新修订、不改Run/库存/认可、冲突/redo/跨工程预期；30秒是体验试做、3轮不计初案。'
        rows.append({'file':rel,'question_ids':qids(nums),'status':'pending_owner_merge_readonly','target_fields':field,'required_change':detail,
                     'selected_requirements':{ident:choices[ident]['selected_text'] for ident in qids(nums)}})
    generator = subprocess.run([sys.executable, '-X', 'utf8', str(ROOT/'tools/准备编码启动包.py'), '--confirmed-choice-only'],
        cwd=ROOT, check=True, capture_output=True, text=True, encoding='utf-8')
    generated = [task['coding_package_ref'] for task in selection['selected']]
    generated.append(A+'编码任务包/任务包索引.json')
    generated += [row['prompt_ref'] for row in read(A+'窗口启动提示词/启动提示词索引.json')['prompts']]
    owned = [CREF, CREF.replace('.json','.md'), QREF, QREF.replace('.json','.md'),
             A+'全量设计编码确认/设计编码覆盖索引.md', A+'全量设计编码确认/确认选择待合入.md',
             REPORT, REPORT.replace('.json','.md'), 'tools/同步确认选择到任务.py','tools/准备编码启动包.py']
    files = sorted(set(CHANGES+generated+owned))
    report = {'document_kind':'filewise_selected_requirement_update_not_runtime_or_progress',
        'user_instruction':'将Q1-Q64的选择更新逐个文件更新到SXX-XX任务体系中',
        'canonical_selection_ref':CREF,'checkpoint_projection_ref':SREF,
        'updated_source_files':sorted(set(CHANGES)), 'generated_requirement_files':generated,
        'updated_files':files, 'updated_file_count':len(files),'pending_owner_files':rows,
        'checkpoint_count':60,'selected_question_count':64,'selected_task_count':21,
        'old_twelve_answers_preserved':True,'recorded_baselines_preserved':len(questionnaire['recorded_baselines']),
        'task_states_and_dependencies_changed':False,'public_progress_written':False,
        'runtime_results_written':False,'generated_packages_refreshed':True,
        'generation_output':json.loads(generator.stdout),
        'Q10_effort_delta':'pending_concrete_design_and_original_framework_merge'}
    write_json(REPORT, report)
    confirmation['new_selected_requirement']['status'] = 'merged_into_existing_task_requirements_pending_exact_contract_and_effort_design'
    confirmation['new_selected_requirement']['design_details_pending'] = ['具体编辑命令/可逆数据格式与消费者','保存/采用/取消及跨修订的详细设计案例','原工量框架增量估量与在审交互/追踪精准合入']
    confirmation['application_state'].update({'shared_sources_merged':False,'non_in_review_sources_and_SXX_requirements_merged':True,
        'filewise_update_report_ref':REPORT,'pending_owner_file_count':len(rows),'public_review_state_written':False,
        'window_1_2_review_sources_written':False,'source_and_generated_task_packages_regenerated':True})
    for group in confirmation['merge_requests']:
        group['updated_target_refs'] = [f for f in group.get('target_refs',[]) if f in files]
        group['pending_target_refs'] = [f for f in group.get('target_refs',[]) if f in PENDING]
        group['targets_written'] = len(group['updated_target_refs']) == len(group.get('target_refs',[]))
        group['status'] = 'source_updates_applied_with_owner_files_pending' if group['pending_target_refs'] else 'source_updates_applied'
    write_json(CREF, confirmation)
    questionnaire.update({'answers_applied_to_baseline':True,'all_owner_sources_applied':False,
        'task_system_requirements_ref':SREF,'filewise_update_report_ref':REPORT,
        'coverage_boundary':'64项已确认选择已进入原60项S/21项P的设计与验收要求；在审源/共同契约与工量待拥有者精准合入，公共进度与实际证据不变。'})
    write_json(QREF, questionnaire)
    # 更新本窗口的旧说明，避免“已同步”与“全部未合入”同时出现。
    formal_md = CREF.replace('.json','.md')
    text = (ROOT/formal_md).read_text(encoding='utf-8-sig')
    text = text.replace('状态：64项推荐选择已完成，共享源回写待合入。', '状态：64项选择已进入SXX/P任务要求和生成包，在审源/共同契约及工量剩余单列。')
    start = text.index('## 合入与下一步')
    text = text[:start] + '## 合入与下一步\n\nQ01—Q64已进入原60项S检查的设计/验收要求和21项P任务源，现有生成器已同步21编码包与6份启动提示词。逐文件状态见[更新结果](../04_开发与验收/全量设计编码确认/确认选择逐文件更新结果.md)。仍在审的交互文件及共同契约/工量保持只读，具体内容列为跨项合入；公共进度与实际结果未改。\n\nB01—B20和旧Q1—Q12保留，D01—D24未自动认可为完成；Figma监督/批次授权、S06/S07准入及外部动作边界继续适用。\n'
    write_text(formal_md,text)
    question_md = QREF.replace('.json','.md')
    text = (ROOT/question_md).read_text(encoding='utf-8-sig')
    text = text.replace('具体命令/适用任务及增量工量待合入', '适用任务已合入，具体命令及增量工量仍须落实')
    text = text.replace('本次只完成Q选择，不自动认可D01—D24的具体设计，也不启动G01—G03的Figma、编码或外部动作。窗口1仍在完善验收追踪；原在审文件、公共进度与共同源保持只读，逐文件修正见[确认选择待合入](确认选择待合入.md)。',
        'Q选择及本轮SXX/P任务要求已同步，不自动认可D01—D24的具体设计，也不启动G01—G03的Figma、编码或外部动作。在审交互源、公共进度及共同契约/工量剩余保持只读，逐文件修正见[更新结果](确认选择逐文件更新结果.md)。')
    text = text.replace('共享源、具体接口、真实任务包和工量须按待合入清单更新后才能用于派发；本次不增加公共复审通过数或产品完成证据。',
        '现行SXX要求、P任务源及生成编码包已同步，仍需在审文件/共同契约及工量精准合入；具体设计/实际前置未齐时不得派发。本次不增加公共复审通过数或产品完成证据。')
    if '## SXX任务更新入口' not in text:
        text += '\n## SXX任务更新入口\n\n64项选择已逐项消费到[60个S检查要求](SXX-XX_确认选择落实.md)和原21项P任务/生成编码包；[逐文件结果](确认选择逐文件更新结果.md)列明直接更新与只读待合入文件。\n'
    write_text(question_md,text)
    coverage_rel = A+'全量设计编码确认/设计编码覆盖索引.md'
    text = (ROOT/coverage_rel).read_text(encoding='utf-8-sig')
    text = text.replace('共享源回写见[待合入清单](确认选择待合入.md)。', '已回写的源/生成包及只读剩余见[逐文件结果](确认选择逐文件更新结果.md)。')
    start = re.search(r'^## (?:S01—S12共60项|60项SXX检查的现行确认要求)$',text,re.M).start()
    stop = text.index('## 七项后置',start)
    table = ['## 60项SXX检查的现行确认要求', '', '当前复审状态只读取公共成熟度源；下表只列本轮新增要求，逐条正文见[SXX落实](SXX-XX_确认选择落实.md)。', '', '|检查项|标题|确认题|具体细化|','|---|---|---|---|']
    table += [f"|{row['id']}|{row['title']}|{'、'.join(row['question_ids'])}|{'、'.join(row['detail_ids'])}|" for row in projection['checkpoints']]
    text = text[:start]+'\n'.join(table)+'\n\n'+text[stop:]
    for task in selection['selected']:
        pattern = r'^\|' + re.escape(task['id']) + r'\|([^\n]*)$'
        def update_task_table(match):
            columns = match[1].split('|')
            details = re.findall(r'D\d{2}', columns[1])
            return '|'+task['id']+'|'+task['scope']+'|'+'、'.join(task['confirmed_question_ids']+details)+'|'
        text = re.sub(pattern, update_task_table, text, flags=re.M)
    text = text.replace('待合入：答案消费到实际基线/需求/设计/任务包；本次未抢写其他窗口或共同源，具体目标见待合入清单。',
        '已合入：答案进入基线、现行SXX/P要求及生成包；在审交互源、共同契约与增量工量的剩余按逐文件清单合入。')
    write_text(coverage_rel,text)
    report_md = REPORT.replace('.json','.md')
    md = ['# Q01—Q64逐文件更新结果', '',
          f'已将64项选择接入原60项SXX检查要求、21项P任务和21编码包，同步6份窗口提示词；本轮维护文件共{len(files)}个。旧十二项/20条基线及七后置保留。', '',
          '[机器明细](确认选择逐文件更新结果.json) · [SXX逐条要求](SXX-XX_确认选择落实.md) · [原任务源](../../00_项目基线/首版任务选择.json)', '',
          '公共复审进度、任务状态、实际结果和派发授权保持原值。Q10的具体Commands/增量工量仍须原S06与工量主责落实；30秒是体验试做目标，3轮不包括初案，MIT是许可候选。', '',
          '## 已更新文件', '', '|文件|更新内容|', '|---|---|']
    for rel in files:
        if rel in DOCS: content = '、'.join(qids(DOCS[rel]))+'设计要求'
        elif rel in JSONS: content = '、'.join(qids(JSONS[rel]))+'机器可读要求/对应政策'
        elif rel in generated: content = '从现行P任务源同步适用选择；保留原派发状态'
        else: content = 'S/P映射、选择状态或生成源的同步'
        md.append(f'|[{rel}]({source_link(report_md,rel)})|{content}|')
    md += ['', '## 保持只读的逐文件合入内容', '', '|文件|所需修正位置与内容|', '|---|---|']
    for row in rows:
        md.append(f"|[{row['file']}]({source_link(report_md,row['file'])})|{'、'.join(row['question_ids'])}；{row['target_fields']}：{row['required_change']}|")
    md += ['', '逐题完整选择见机器明细pending_owner_files.selected_requirements；本表不是新增backlog，不计设计或产品完成。']
    write_text(report_md,'\n'.join(md)+'\n')
    pending_md = A+'全量设计编码确认/确认选择待合入.md'
    write_text(pending_md,'# 全量选择的剩余合入\n\n64项已选择A并已进入现行SXX/P任务要求，原8组已按可写源落实。已更新文件与剩余'+str(len(rows))+'个只读目标统一见[逐文件更新结果](确认选择逐文件更新结果.md)及[JSON](确认选择逐文件更新结果.json)。\n\n这些目标仍由在审窗口或共同契约/工量拥有者精准合入；具体题号、字段和完整选择已列明，不覆盖他们的增量，不修改公共进度。\n')
    print(json.dumps({'updated_source_files':len(set(CHANGES)),'S_checkpoints':60,'P_tasks':21,'Q_choices':64,'pending_owner_files':len(rows),'public_progress_written':False},ensure_ascii=False))

def design_handoffs_only():
    """精准同步当前公共交接引用，不覆写其他窗口业务/界面源。"""
    refs = {
        'P0-03': ['packages/packaging/consumer-device.design.json',
                  'docs/04_开发与验收/特征细化/DRF03_C02_专业绑定推导.json',
                  'docs/04_开发与验收/特征细化/DRF03_C02_实际工位定义与生成接入.json'],
        'P4-02': ['packages/packaging/consumer-device.design.json',
                  'docs/04_开发与验收/特征细化/DRF03_C02_专业绑定推导.json',
                  'docs/04_开发与验收/特征细化/DRF03_C03_过程样本语义.json'],
        'P3-04': ['docs/04_开发与验收/特征细化/DRF04_Host授权合入.json',
                  'integrations/agent/stdio-connector.design.json'],
        'P4-01': ['docs/04_开发与验收/特征细化/DRF04_Host授权合入.json',
                  'integrations/agent/stdio-connector.design.json'],
    }
    selection = read(B + '首版任务选择.json')
    selected = {row['id']: row for row in selection['selected']}
    index_before = read(A + '编码任务包/任务包索引.json')
    backlog = read(A + 'tasks.json')
    old_states = [(r['id'], r['status'], r['actual_result']) for r in backlog['tasks']]
    for ident, additions in refs.items():
        task = selected[ident]
        for ref in additions:
            assert (ROOT / ref).is_file(), ref
            if ref not in task['execution_plan']['input_refs']:
                task['execution_plan']['input_refs'].append(ref)
        package = task['coding_package_ref']
        old = (ROOT / package).read_text(encoding='utf-8-sig')
        extra = [ref for ref in additions if f'](../../../{ref})' not in old]
        if extra:
            boundary = '\n\n当前输入语义：'
            assert boundary in old, package
            links = '\n'.join(f'- [{ref}](../../../{ref})' for ref in extra)
            write_text(package, old.replace(boundary, '\n' + links + boundary, 1))
    write_json(B + '首版任务选择.json', selection)
    for row in backlog['tasks']:
        if row['id'] in refs:
            row['first_release_execution_plan'] = selected[row['id']]['execution_plan']
    assert old_states == [(r['id'], r['status'], r['actual_result']) for r in backlog['tasks']]
    write_json(A + 'tasks.json', backlog)
    # The framework consumes the same task inputs; preserve every effort/state value.
    frame_ref = selection['effort_framework_ref']
    frame = read(frame_ref)
    for row in frame['tasks']:
        if row['task_id'] in refs:
            row['estimate_input_refs'] = list(selected[row['task_id']]['execution_plan']['input_refs'])
    (ROOT / frame_ref).write_text(json.dumps(frame, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    CHANGES.append(frame_ref)
    projection = read(SREF)
    gate = read(A + '完整设计与执行准入.json')
    delegation = gate['review_policy'].get('S04_04_05_delegation')
    for mirror_ref in [A + '软件开发流程与当前成熟度评估.json', A + '本机并行窗口计划.json']:
        mirror = read(mirror_ref)
        mirror['frontend_supervision'] = gate['frontend_supervision']
        write_json(mirror_ref, mirror)
    for checkpoint in projection['checkpoints']:
        if delegation and checkpoint['id'] in delegation['checkpoints']:
            checkpoint['review_evidence_policy'] = {
                'canonical_ref': A + '完整设计与执行准入.json#/review_policy/S04_04_05_delegation',
                'operator_kind': delegation['operator_kind'],
                'authorization_message_id': delegation['message_id'],
                'rule': '用户明确授权AI实际代走查，动作/观察/缺陷及修后复核如实记录；不得冒称真人操作，独立目标用户S10不变。'
            }
        if checkpoint['id'] in {'S03-03','S05-02','S05-03','S06-02','S06-03','S06-04','S06-05'}:
            for ident in checkpoint['task_ids']:
                for ref in refs.get(ident, []):
                    if ref not in checkpoint['source_refs']:
                        checkpoint['source_refs'].append(ref)
    write_json(SREF, projection)
    write_text(SMREF, render_requirement_projection(projection))
    assert read(A + '编码任务包/任务包索引.json') == index_before
    print(json.dumps({'design_handoff_task_refs': sorted(refs), 'other_window_sources_written': False,
                      'task_states_changed': False, 'coding_packages_dispatch_changed': False}, ensure_ascii=False))


if __name__ == '__main__':
    if sys.argv[1:] == ['--timeline-only']:
        timeline_only()
    elif sys.argv[1:] == ['--design-handoffs-only']:
        design_handoffs_only()
    else:
        main()
        design_handoffs_only()
