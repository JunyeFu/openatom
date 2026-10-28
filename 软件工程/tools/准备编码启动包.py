"""从现行任务/窗口/设计关口生成派发草案；不启动窗口或修改产品执行状态。"""
from pathlib import Path
import json
import sys
import re

S = Path(__file__).resolve().parent.parent
A = S / 'docs/04_开发与验收'


def load(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def save(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')


def current_design_notice():
    state = load(S / 'docs/02_交互设计/Figma执行状态.json')
    if state['execution_state'] == 'completed_current_Figma_clickable_design_prototype':
        return 'Figma设计与可点击原型批次已获授权并完成；当前文件/页面及复审范围取Figma执行状态.json。产品前端构建仍须对应授权，不由原型完成自动获得。'
    return 'Figma当前批次及授权范围取Figma执行状态.json；设计授权与产品前端构建授权分别核对。'


def refresh_current_handoff():
    """只刷新现行交接投影，保留设计视图、当前切片和实际派发。"""
    state_ref = 'docs/02_交互设计/Figma执行状态.json'
    state = load(S / state_ref)
    prompt_index = A / '窗口启动提示词/启动提示词索引.json'
    prompts = load(prompt_index)
    for row in prompts['prompts']:
        path = S / row['prompt_ref']
        text = path.read_text(encoding='utf-8-sig')
        if row.get('actual_dispatch'):
            text = text.replace('当前提示词已准备，尚未派发。', '当前W00后端切片已在本聊天获用户明确派发；实际进度取current_execution_ref，其他任务不因此获得授权。')
        text = text.replace('Figma已选择，前端设计和构建授权均未给出。', current_design_notice())
        begin, end = '<!-- current-handoff:start -->', '<!-- current-handoff:end -->'
        actual = state['actual_Figma']
        block = (begin + '\n' + current_design_notice() + '\n'
                 + f"Current design: {actual['design_url']}; page {actual['page_id']}; "
                 + f"{actual['state_frames']} states. Read {state_ref} for the actual review coverage. "
                 + 'Native reactions and design review do not prove product execution. '
                 + 'S07 engineering-shell preparation is separate from business frontend implementation.\n' + end)
        if begin in text:
            text = text[:text.index(begin)] + block + text[text.index(end) + len(end):]
        else:
            text += '\n' + block + '\n'
        path.write_text(text, encoding='utf-8')
    prompts['figma_design_started'] = bool(state['actual_design_write'])
    prompts['current_figma_state'] = state['execution_state']
    prompts['current_figma_state_ref'] = state_ref
    save(prompt_index, prompts)
    path = A / '窗口启动提示词/README.md'
    text = path.read_text(encoding='utf-8-sig')
    text = text.replace('前端已选Figma，设计/构建在用户监督和明确批准后进行；当前未授权开始。', current_design_notice())
    text = text.replace('本轮启动数0、前端开始数0', '产品实施派发数取窗口计划；Figma设计完成状态取现行执行记录')
    path.write_text(text, encoding='utf-8')
    preparation = A / 'S07_工程准备与执行准入.json'
    if preparation.exists():
        result = load(preparation)
        if result['status'] == 'passed_engineering_preparation':
            path = A / '编码任务包/任务包索引.json'
            index = load(path)
            for row in index['packages']:
                if row['actual_dispatch'] is None:
                    row['actual_commands'] = result['verified_commands']
                row['engineering_preparation_ref'] = 'docs/04_开发与验收/S07_工程准备与执行准入.json'
                package = S / row['package_ref']
                text = package.read_text(encoding='utf-8-sig')
                text = text.replace('模块设计已在S06-05认可；S07真实命令、当前切片文件及产品派发尚未齐备',
                                    '模块设计及S07公共工程入口已具备；当前切片文件、业务前置及产品派发尚未齐备')
                text = text.replace('S07仍须落实产品实际入口与验证命令。', 'S07公共工程入口已落实；业务实现仍须当前切片的适用验证。')
                text = text.replace('**待S07完成并由用户/W00按切片派发填写**', '**待用户/W00按切片派发填写；S07公共工程入口已具备**')
                text = text.replace('实际验证命令：**待S07实际入口及本次切片确定后填写。**',
                                    '实际验证命令：**公共命令见下方S07入口；业务切片验证按本次范围落实。**')
                text = text.replace('真实前置产物/当前可用入口：**尚未交付，不编造。**',
                                    '真实前置产物/当前可用入口：**S07公共工程前置已交付；业务生产者仍按implementation_dependencies逐项核对。**')
                begin, end = '<!-- S07-entry:start -->', '<!-- S07-entry:end -->'
                block = (begin + '\n## S07实际工程入口\n\n'
                         + '[工程准备与验证](../S07_工程准备与执行准入.md)已通过；实际命令见JSON的verified_commands。'
                         + '这些命令验证工程壳与准备，不代表本任务业务实现通过。当前步骤、允许文件及真实生产者仍按各自派发填写。\n' + end)
                if begin in text:
                    text = text[:text.index(begin)] + block + text[text.index(end) + len(end):]
                else:
                    text += '\n' + block + '\n'
                package.write_text(text, encoding='utf-8')
            save(path, index)
    print(json.dumps({'handoff_prompts_refreshed': len(prompts['prompts']), 'actual_dispatch_preserved': True}, ensure_ascii=False))


def refresh_confirmed_choices():
    """仅同步确认要求；保留包内增量、派发字段和公共进度。"""
    selection = load(S / 'docs/00_项目基线/首版任务选择.json')
    tasks = {row['id']: row for row in selection['selected']}
    marker = '## Q01—Q64已确认设计要求'
    def replace_block(path, lines, task=None):
        old = path.read_text(encoding='utf-8-sig')
        original = old
        if task is not None:
            old = re.sub(r'(## 已冻结产品范围\n\n).*?(\n\n对应需求：)',
                         lambda m: m[1] + task['scope'] + m[2], old, flags=re.S)
            old = re.sub(r'^整项验收：.*$', lambda m: '整项验收：' + task['release_acceptance'], old, flags=re.M)
            old = re.sub(r'(候选内部步骤（一次选一个可验证增量，不自动做整项）：\n\n).*?(\n\n主责窗口可写区域：)',
                         lambda m: m[1] + '\n'.join(f'{i}. {step}' for i, step in enumerate(task['execution_plan']['steps'], 1)) + m[2], old, flags=re.S)
        begin = '<!-- confirmed-choice-projection:start -->'
        end = '<!-- confirmed-choice-projection:end -->'
        block = begin + '\n' + '\n'.join(lines) + '\n' + end
        if begin in old:
            start = old.index(begin)
            stop = old.index(end, start) + len(end)
            updated = old[:start] + block + old[stop:]
        else:
            updated = old.rstrip() + '\n\n' + block + '\n'
        if updated != original:
            path.write_text(updated, encoding='utf-8')
    for task in tasks.values():
        requirements = task['confirmed_design_requirements']
        lines = [marker, '',
                 '[唯一选择来源](../../00_项目基线/全量设计编码确认选择.json) · [SXX逐项要求](../全量设计编码确认/SXX-XX_确认选择落实.md)', '',
                 '下列要求来自现行首版任务源，属于本包设计/验收范围；具体接口、允许文件和实际命令仍按原准入落实。', '',
                 '|题号|本包必须落实的选择|', '|---|---|']
        lines += [f"|{row['question_id']}|{row['selected_text']}|" for row in requirements]
        if 'confirmed_undo_requirement' in task:
            lines += ['', task['confirmed_undo_requirement']['revision_policy'], '',
                      '撤销验收：' + '；'.join(task['confirmed_undo_requirement']['acceptance_expectations']) + '。', '',
                      ('Q10共同历史/消费者增量已计入原任务现行工量框架；具体区间取任务框架机器源，仍为未实测工程估计。'
                       if task.get('incremental_effort_due_to_Q10', {}).get('existing_budget_includes_Q10')
                       else '原工量区间未包含Q10增量；须依据具体命令/消费者设计回填原框架。')]
        lines += ['', '上述要求已选定，未取得的设计认可/实际结果仍保持原状态；不以选择更新标记ready或done。']
        replace_block(S / task['coding_package_ref'], lines, task)
    index_path = A / '编码任务包/任务包索引.json'
    index = load(index_path)
    index['confirmed_choice_ref'] = selection['confirmed_choice_ref']
    index['checkpoint_choice_requirements_ref'] = selection['checkpoint_choice_requirements_ref']
    for row in index['packages']:
        row['confirmed_question_ids'] = tasks[row['task_id']]['confirmed_question_ids']
    save(index_path, index)
    prompt_index = load(A / '窗口启动提示词/启动提示词索引.json')
    for row in prompt_index['prompts']:
        chosen = {}
        for ident in row['primary_tasks']:
            for requirement in tasks[ident]['confirmed_design_requirements']:
                chosen[requirement['question_id']] = requirement['selected_text']
        lines = [marker, '',
                 'Before design or implementation, consume the selected Q01–Q64 requirements from the canonical first-release task JSON and the SXX checkpoint supplement. Preserve the existing dispatch and Figma gates.', '',
                 '[Selected decisions](../../00_项目基线/全量设计编码确认选择.json) · [Checkpoint requirements](../全量设计编码确认/SXX-XX_确认选择落实.json)', '',
                 'Questions applicable to this window: ' + ', '.join(sorted(chosen)) + '.', '',
                 'Draft undo/redo creates a new revision through shared Commands; it never rewinds production, human acceptance or exported files. Initial goal success requires no repair; otherwise permit at most three feedback repair rounds, excluding the initial run. Packaging’s 30-second target is a trial experience target, not a solver timeout. MIT is the original-code license candidate pending actual dependency and asset review.']
        replace_block(S / row['prompt_ref'], lines)
    print(json.dumps({'coding_packages_refreshed': len(tasks), 'startup_prompts_refreshed': len(prompt_index['prompts']),
                      'public_progress_written': False, 'dispatch_changed': False}, ensure_ascii=False))


def render_effort_framework(frame):
    fmt = lambda values: f'{values[0]}—{values[1]}'
    totals = frame['totals']
    lines = ['# 单人六窗口：任务框架、工量与成果里程碑', '',
             '2026-10-13。用户明确只需任务框架，不排每日日期/作息。本人负责全项目，W00统筹＋W01至W05实施。内部截止仍为北京时间**10月29日结束前备齐材料**。', '',
             '[机器框架](任务框架与工量里程碑.json) · [首版范围](../00_项目基线/首版交付基线.md) · [六窗口](本机并行窗口计划.md) · [全层设计/执行关口](完整设计与执行准入.md)', '',
             '不排开发人员日程，不取消软件内部的生产排期、班次、到料、订单与每日供货目标；这些产品要求继续保留。', '', '## 工量口径与边界', '',
             '**这是按任务拆解作出的计划估计，不是实测校准、统计置信区间或完工承诺。**AI承担生成/调试/局部检查，用户负责关键设计、监督Figma、输入/结果核对、代码审阅、接入修正和材料验收。依赖/未知返修可使实际工作超过区间上端。', '',
             f"21项产品任务的人投入{fmt(totals['selected_task_human_hours'])}小时；另计剩余设计/工程准备/整体接线/选题PDF/手续{fmt(totals['shared_human_hours'])}小时，当前剩余人投入预算合计**{fmt(totals['remaining_human_hours'])}小时**。人投入同一时刻只能计一次，不能乘五成为五个人。", '',
             f"21项产品任务的窗口活动预算合计{fmt(totals['selected_task_window_active_hours'])}小时，可按实际前置并行。它不含未估计的设计/工程准备窗口活动，不是日历时长；**不能与人投入相加，也不能除五得到工期**。当前不要求用户填写每日容量，不据此判断必能按期。", '',
             '原166—282小时一人AI辅助粗估及50—90小时视觉路线判断保留为历史，不作为现行总预算；不把旧人时改名AI时或据扩窗宣称提速。真实闭环后分别记录本人有效投入、窗口活动和等待，更新对应工作包。', '',
             '## 估量依据与实际更新', '',
             frame['effort_accounting']['estimate_basis'], '',
             frame['effort_accounting']['human_unit'], '',
             frame['effort_accounting']['window_unit'], '',
             '区间下端与上端：' + frame['effort_accounting']['range_interpretation'], '',
             '范围驱动与不确定性：' + frame['effort_accounting']['scope_drivers'], '',
             '预算更新责任：' + frame['calibration']['owner_rule'], '',
             '；'.join(frame['calibration']['steps']), '',
             '剩余预算：' + frame['calibration']['remaining_budget_rule'], '',
             '## 五段完整工作链', '',
             '基线/需求 → 全模块视图与技术设计/Figma原型 → S06完整设计冻结及选题PDF → S07实际准备/任务包派发 → 实现/联调/真实验收/比赛材料。只有实际产物和证据关闭里程碑，日期截止不关闭功能。', '',
             '## 21项产品工作包', '',
             '任务scope和两种前置仍取首版任务JSON；下表按主责归集，包含必要协作切片，不授予跨窗口写权。单任务预算已包含本包局部核对/验证，整体接线和正式材料分别计量。', '',
             '|任务|主责|本人小时|窗口活动小时|完整接收里程碑|具体工作与计量边界|', '|---|---|---:|---:|---|---|']
    for r in frame['tasks']:
        lines.append(f"|{r['task_id']}|{r['window']}|{fmt(r['human_effort_hours'])}|{fmt(r['window_active_hours'])}|{r['complete_at_milestone']}|{r['effort_basis']}|")
    lines += ['', '## 不重复计量的共同工作', '', '|编号|共同工作|本人小时|成果节点|不重复的边界|', '|---|---|---:|---|---|']
    for r in frame['shared_work']:
        lines.append(f"|{r['id']}|{r['name']}|{fmt(r['human_effort_hours'])}|{'、'.join(r['milestones'])}|{r['separation']}|")
    lines += ['', '## 按主责归集的窗口工量', '', '|窗口|任务|本人小时|任务窗口活动小时|', '|---|---|---:|---:|']
    for r in frame['window_budget_summary']:
        lines.append(f"|{r['window']}|{'、'.join(r['primary_task_ids'])}|{fmt(r['task_human_hours'])}|{fmt(r['task_window_hours'])}|")
    lines += ['', '该表不是六名员工的可用人力，也不证明负载均衡；共同工作由用户/W00统筹、相应专业窗口支撑。', '',
              '## 只按成果推进的里程碑', '',
              '不指定每个任务的开始/结束日期。MS是完整成果集合，**不替代任务implementation_dependencies**：独立包装核心、桥、模板等切片可按真实前置早开，不必等整个前一组里程碑；整项完成仍满足release_dependencies、SF/IX。', '',
              '|编号|成果|完整接收前置|关口/完整任务|验收出口|', '|---|---|---|---|---|']
    for r in frame['milestones']:
        fields = r['review_ids'] + r['complete_task_ids']
        lines.append(f"|{r['id']}|{r['name']}|{'、'.join(r['depends_on']) or '当前八项基线'}|{'、'.join(fields) or '完整候选版本证据'}|{'；'.join(r['exit_criteria'])}|")
    lines += ['', '## 里程碑交付与接收责任', '',
              '下表为计划接收契约。规范引用给出验收依据，不是已经取得的实际证据。用户最终接收；窗口整理产物及执行结果，不能自行替用户确认原型或交付。', '',
              '|节点|整理交付主责|协作窗口|必须提交的产物或证据|最终接收|', '|---|---|---|---|---|']
    for r in frame['milestones']:
        lines.append(f"|{r['id']}|{r['delivery_owner']}|{'、'.join(r['contributors']) or '无'}|{'；'.join(r['required_deliverables'])}|用户|")
    lines += ['', '未满足出口时：' + frame['milestone_policy']['unmet_exit_action'], '']
    lines += ['', 'MS-03中的UI技术试做同样要先有用户前端/Figma批准；没有批准时可推进授权范围内的纯后端设计，不能偷偷制作界面。MS-11需要实际接手/复现证据，尚无参与者时相应验收仍开放，AI新目录操作不冒充第三人。', '',
              '## 比赛材料与外部手续', '', '|编号|材料或手续|准备责任|完成证据|材料准备出口|', '|---|---|---|---|---|']
    for r in frame['material_checklist']:
        lines.append(f"|{r['id']}|{r['material']}|{r['owner']}；支持{'、'.join(r['support']) or '由用户办理'}|{r['acceptance']}|{'必需' if r['required_for_material_readiness'] else '单列外部手续'}|")
    lines += ['', frame['deadline_control']['material_vs_external_rule'], '']
    lines += ['', '最终选题PDF在S06设计冻结后生成；比赛作品介绍PDF在真实产品候选版后按实际能力生成；工业评估四格式属于软件输出。三者不能互相冒充或重复计成本。', '',
              '官网材料/主体/截止沿用[同日已有核读记录](../../../研究资料/原始资料/赛事/R01_2026-10-13_S01-03复核.json)。此前同日直接页面请求超时；本次独立复审未重新确认官网规则。报名审核/邮件附加格式/回执仍据实际记录，材料内部准备截止不因此放宽。AI窗口不算六名参赛成员。', '',
              '## 执行与截止控制', '',
              '每次只派当前有前置的可验证切片，按[模块设计/任务包/审查](编码任务包与AI实现审查.md)执行；各模块视图/选型/方法/接口先认可，Figma前端必须用户监督与明确开始。余下任务/材料以实际验收状态统计，不凭文件数量或窗口在线数完成。', '',
              '所有成果以MS-12在10月30日前备齐为目标。实际出现关键链延误或证据缺失时说明未完成能力/材料；不自动删九类、专业三机制/三变化、真实Agent或四格式，不用AI图/目标数量伪填完成。用户如需改范围/期限再明确决定。没有实际时间/能力证据前不承诺全部按期。', '',
              'S02-04独立复审见[复审报告](S02-04_工量与成果里程碑复审.md)；阶段闭环/失败处理见[现行阶段规范](阶段实现计划.md)及[S02-05复审](S02-05_阶段闭环与退出条件复审.md)。原八项基线见[联合报告](S01-01至S02-03_联合复核.md)。框架复审不代表工量实测、按期可行、产品完成、Figma授权或外联。']
    (A / '任务框架与工量里程碑.md').write_text('\n'.join(lines) + '\n', encoding='utf-8')


def render_stage_plan(plan):
    current_maturity = load(A / '软件开发流程与当前成熟度评估.json')
    current_reviews = current_maturity['checkpoint_review']
    pending_design_count = sum(c['review']['status'] != 'passed' for phase in current_maturity['phases'][:6] for c in phase['checkpoints'])
    target = A / '阶段实现计划.md'
    old = target.read_text(encoding='utf-8-sig')
    historical = old[old.index('## 全产品与后续储备的历史计划'):]
    lines = ['# 首版阶段闭环、退出与证据契约', '',
             '2026-10-13。S02-05独立文档复审后的现行规范。12个S阶段、6个P工作族、13个MS成果节点分别使用；不增加任务、不改两种前置、不排每日日程。阶段退出须相应实际证据，本次只完成退出规则的设计复审。', '',
             '[机器契约](阶段实现计划.json) · [独立复审](S02-05_阶段闭环与退出条件复审.md) · [21项任务](../00_项目基线/首版任务选择.json) · [里程碑](任务框架与工量里程碑.md) · [设计准入](完整设计与执行准入.md)', '',
             '## 三类阶段含义', '']
    if 'checkpoint_choice_requirements_ref' in plan:
        lines += ['已确认Q01—Q64是各原SXX-XX的设计/验收输入，逐项消费[确认要求](全量设计编码确认/SXX-XX_确认选择落实.md)及[JSON](全量设计编码确认/SXX-XX_确认选择落实.json)。要求更新不修改公共复审进度。', '']
    lines += ['- ' + value for value in plan['stage_axes'].values()]
    lines += ['', plan['entry_policy']['task_start'], '', plan['entry_policy']['family_exit'], '',
              '## S01—S12生命周期的输入、输出和退出', '',
              '下表是退出定义；当前退出只按阶段JSON的actual_exit_result与独立实际证据，不能由百分比推定。S01—S06设计出口可在全层冻结时按用户授权AI验收；产品阶段、S10独立用户及外部动作另验。S04与S05可按明确输入并行设计，当前Figma设计批次状态取Figma执行状态.json，产品前端构建授权另核。S08/S09可交错，不用P5材料完成作测试启动前置。', '',
              '|阶段/整理主责|输入|闭环产物|退出条件|', '|---|---|---|---|']
    for row in plan['lifecycle_phases']:
        lines.append(f"|{row['id']} {row['name']}／{row['delivery_owner']}|{'；'.join(row['inputs'])}|{'；'.join(row['outputs'])}|{'；'.join(row['exit_conditions'])}|")
    lines += ['', '## P0—P5首版闭环', '',
              'P是工作族，不是全族串行屏障。主责窗口按当前任务包的真实输入启动；窗口允许写入区和公共配置所有权不因工作族统筹改变。下面各族任务只覆盖当前21项选择，P0-03只取已选部分。', '']
    for row in plan['product_families']:
        lines += [f"### {row['id']} {row['name']}", '',
                  f"整理交付：{row['delivery_owner']}；成员任务/主责：{'；'.join(ident + '→' + owner for ident, owner in row['task_owners'].items())}；最终接收：用户。", '',
                  '**实际输入：**' + '；'.join(row['inputs']) + '。', '',
                  '**闭环输出：**' + '；'.join(row['outputs']) + '。', '',
                  '**退出：**' + '；'.join(row['exit_conditions']) + '。', '',
                  '**共享证据：**形成' + ('、'.join(row['owned_ix_ids']) or '本族局部证据') + '；支持' + '、'.join(row['requirement_ids']) + '。支持需求不等于该SF最终已通过。', '']
    lines += ['## 17项SF的证据形成与最终收口', '',
              '下表直接按首版任务requirement_ids生成；P5-01的17项映射表示最终核验职责，不把它当实现全部功能。MS-11核验SF-01至14、SF-16/17及全部NFR/IX和许可/接手；SF-15在MS-12核对实际材料后关闭，届时17SF完整收口。材料验收不以自身SF-15通过为前置，P5-01也不需先done才能测试。', '',
              '|需求|功能证据形成任务（不含P5）|完整核验责任|当前结果|', '|---|---|---|---|']
    for row in plan['sf_evidence']:
        lines.append(f"|{row['id']}|{'、'.join(row['producer_tasks'])}|{row['closure_task']}／{row['verification_milestone']}|未执行|")
    lines += ['', '### IX跨包证据', '', '|案例|完整核验任务|生产者|必须核对|', '|---|---|---|---|']
    for row in plan['ix_evidence']:
        lines.append(f"|{row['id']}|{row['owner_task']}|{'、'.join(row['producer_tasks'])}|{row['required_observations']}|")
    lines += ['', '## 如何判定通过，以及失败后怎样处理', '']
    lines += ['- ' + value for value in plan['evidence_policy']['rules']]
    lines += ['', '**业务结果和测试结论分别判断：**' + plan['evidence_policy']['business_outcome_vs_verdict'], '']
    lines += ['- ' + value for value in plan['failure_policy']['actions']]
    lines += ['', '最小实际退出记录需要：' + '、'.join(plan['evidence_policy']['minimum_exit_record']) + '。只引用已有交付与结果，不创建另一套完成数或审计系统。', '',
              '## 本轮复审与现行状态', '',
              f"当前{current_reviews['passed_reviews']}项文档规格通过、{current_reviews['pending_reviews']}项待复审，最早待审{current_reviews['next_checkpoint']}。S02文档100%/支撑70%保持，真实任务仍0/21、18操作未运行；MS-01实际用户接收尚未登记，不由定义复审单独关闭。完整设计余{pending_design_count}项，S06冻结后最终选题PDF，S07实际准备后编码。", '',
              '材料准备仍须10月29日结束前；MAT-01至04与实际版本一致，MAT-05手续另据实。S12的真实运营演进不作本次材料准备的新增前置，但首版已有许可/贡献采用/第三人接手条件不删。', '', historical]
    target.write_text('\n'.join(lines).rstrip() + '\n', encoding='utf-8')


def main():
    selection = load(S / 'docs/00_项目基线/首版任务选择.json')
    plan = load(A / '本机并行窗口计划.json')
    gate = load(A / '完整设计与执行准入.json')
    frame = load(S / selection['effort_framework_ref'])
    stage_plan = load(S / selection['stage_plan_ref'])
    efforts = {r['task_id']: r for r in frame['tasks']}
    render_effort_framework(frame)
    render_stage_plan(stage_plan)
    roles = {r['id']: r for r in plan['window_roles']}
    coding = A / '编码任务包'
    prompts = A / '窗口启动提示词'
    coding.mkdir(exist_ok=True)
    prompts.mkdir(exist_ok=True)
    package_rows = []
    for task in selection['selected']:
        role = roles[task['planned_window_id']]
        ref = f"docs/04_开发与验收/编码任务包/{task['id']}.md"
        lines = [f"# {task['id']} 编码任务包", '',
                 '**状态：draft_not_dispatchable。** 当前为现行任务的派发投影，模块完整设计/技术选型认可、S07真实命令、当前切片文件及用户授权尚未齐备，不可直接开始编码。', '',
                 f"主责：{role['id']} {role['title']}；整体责任与最终接收：用户。", '',
                 '[规则与审查](../编码任务包与AI实现审查.md) · [完整设计关口](../完整设计与执行准入.md) · [窗口归属](../本机并行窗口计划.md) · [唯一任务源](../../00_项目基线/首版任务选择.json)', '',
                 '[非功能规范](../../01_业务需求/非功能需求与测量验收.md)：只执行本包相关NFR和真实系统接收，按规定机器/负载/时钟/样本及失败条件形成证据；目标/合成例/硬件库存不当性能PASS。', '',
                 '## 已冻结产品范围', '', task['scope'], '',
                 '对应需求：' + '、'.join(task['requirement_ids']) + '。原7后置不加入本包。', '',
                 '## 设计与实际输入', '']
        lines += [f'- [{r}](../../../{r})' for r in task['execution_plan']['input_refs']]
        lines += ['', '当前输入语义：' + '；'.join(task['execution_plan']['inputs']), '',
                  '**编码前待落实**：适用的任务/结构数据/状态行为/接口/界面表现/部署验证视图；具体技术/版本、算法/实现方式、完整契约和消费者。按S05/S06认可，不能把上面资料路径存在当设计已经冻结。', '',
                  '## 当前切片与修改边界', '',
                  '当前内部步骤、明确允许文件/新文件、接口与命令版本、设计认可记录：**待S06/S07完成并由用户/W00派发填写**。', '',
                  f"工量框架：本人{efforts[task['id']]['human_effort_hours'][0]}—{efforts[task['id']]['human_effort_hours'][1]}小时，窗口活动{efforts[task['id']]['window_active_hours'][0]}—{efforts[task['id']]['window_active_hours'][1]}小时，完整接收在{efforts[task['id']]['complete_at_milestone']}。均为未实测计划预算，不相加、不按窗口数计算工期；见[任务框架](../任务框架与工量里程碑.md)。", '',
                  '候选内部步骤（一次选一个可验证增量，不自动做整项）：', '']
        lines += [f'{i}. {step}' for i, step in enumerate(task['execution_plan']['steps'], 1)]
        lines += ['', '主责窗口可写区域：' + '；'.join(role['write_roots'] + role['write_files']), '',
                  '**例外**：所有配置/锁文件、共同domain/Commands/宿主归W00。其他窗口部分必须交接；release_paths不是全目录写权限。前端/Figma须用户监督及对应阶段/批次明确授权。', '',
                  '## 前置与协作', '',
                  '核心启动实际前置：' + ('、'.join(task['implementation_dependencies']) or '无产品前置；仍需完整设计/工程准备及派发授权'),
                  '完整验收前置：' + ('、'.join(task['release_dependencies']) or '无其他产品任务前置'),
                  f"工作族退出依据：[阶段闭环契约](../阶段实现计划.md)与[机器定义](../阶段实现计划.json)。{task['id'].split('-')[0]}不是整族启动屏障；局部结果按本包验收，完整SF/IX另行收口。预期等待、目标不足或几何不满足与测试失败分开；需实际结果符合冻结预期，不以所有业务结果成功为通过条件。",
                  '真实前置产物/当前可用入口：**尚未交付，不编造。**', '',
                  '## 输出与验收', '']
        lines += ['- ' + output for output in task['execution_plan']['outputs']]
        lines += ['', '局部验证方式：' + task['execution_plan']['verification_method'], '',
                  '整项验收：' + task['release_acceptance'], '',
                  '实际验证命令：**待S07实际入口及本次切片确定后填写。** 预期来自上述输入/设计及SF/IX，不从实现结果反写。', '',
                  '## 实现审查与交付', '',
                  '先明确视图/技术/方法/接口，再按小切片实现、适用最小验证、只读审查diff及真实结果、消费者接入、用户接收。W00核对范围/契约/单位/状态/预期/真实执行；不写入实施者正在改的文件。', '',
                  '交付：步骤、变更文件、当前接口、实际输入/命令/结果、尚未满足完整验收、跨窗口请求。局部通过不标整项done，真实结果仍为空。']
        (coding / f"{task['id']}.md").write_text('\n'.join(lines) + '\n', encoding='utf-8')
        package_rows.append({'task_id': task['id'], 'primary_window': role['id'], 'package_ref': ref,
                             'status': 'draft_not_dispatchable', 'design_accepted': False,
                             'actual_commands': None, 'actual_dispatch': None})
    index = {'document_kind': 'derived_coding_package_index_not_execution_backlog',
             'source_ref': 'docs/00_项目基线/首版任务选择.json',
             'stage_plan_ref': selection['stage_plan_ref'],
              'nonfunctional_spec_ref': selection['nonfunctional_spec_ref'],
              'navigation_spec_ref': selection['navigation_spec_ref'],
              'form_spec_ref': selection['form_spec_ref'],
              'component_architecture_ref': selection['component_architecture_ref'],
              'ownership_spec_ref': selection['ownership_spec_ref'],
              'module_protocol_ref': selection['module_protocol_ref'],
              'technical_selection_ref': selection['technical_selection_ref'],
              'scheduler_spec_ref': selection['scheduler_spec_ref'],
              'type_format_spec_ref': selection['type_format_spec_ref'],
              'api_contract_spec_ref': selection['api_contract_spec_ref'],
             'requirement_trace_ref': selection['requirement_trace_ref'],
             'protocol_ref': 'docs/04_开发与验收/编码任务包与AI实现审查.md',
             'refresh_rule': 'S06冻结设计/S07入口落实后刷新，用户派发前填当前切片/文件/真实前置；不修改源任务状态',
             'packages': package_rows}
    save(coding / '任务包索引.json', index)
    package_readme = ['# 首版21项编码任务包', '',
                      '验收输入还须消费[S03-05需求追踪](../首版需求验收追踪.md)及[JSON](../首版需求验收追踪.json)：本包首版任务的acceptance_trace_ids定位SF/NFR、案例输入/预期和实际结果槽。没有满足输入冻结及原两种前置时，仍只能准备文档，不能派发或将局部实现计成整项完成。', '',
                      '当前均为**不可派发草案**，不等于21个模块设计已完成。按[规则](../编码任务包与AI实现审查.md)先完成模块视图/技术/实现/接口/契约，S06冻结和S07实际入口后刷新，由用户/W00填写当前切片和权限。', '',
                      '|任务|主责|任务包|', '|---|---|---|']
    package_readme += [f"|{r['task_id']}|{r['primary_window']}|[{r['task_id']}]({r['task_id']}.md)|" for r in package_rows]
    package_readme += ['', '[机器索引](任务包索引.json) · [任务与依赖](../任务拆解与依赖.md)', '',
                       '生成器只重建派发投影；未来实际状态/结果回填唯一任务源，不在任务包草案维护第二套完成数。']
    (coding / 'README.md').write_text('\n'.join(package_readme) + '\n', encoding='utf-8')

    focus = {
        'W00': 'Own the shared Rust domain/schema, generated client types, Commands, Tauri host, build metadata, canonical scenarios and integration. Resolve cross-window contract requests and integrate actual producer outputs. Do not become six independent implementers or overwrite a worker file without handing ownership over.',
        'W01': 'Implement the single production Engine and fixed-duration logistics. Preserve identity/holder, kit consumption, stage calendars/resource exclusion, quality/rework and receiver acceptance versus vehicle return. Publish actual event/state results; request device declarations from W03 and UI wiring from W02.',
        'W02': 'Own the unified frontend and 3D canvas container, camera, shared UI components/styles, timeline and business panels. Preserve two normal canvases, three LODs, full 3D geometry, click-to-detail and return context. Integrate W03/W04/W05 feature panels rather than creating duplicate app state or production clocks.',
        'W03': 'Implement nine-category authoring/import/adoption, devices, media/native presentation, image timelines/tags and the three discrete templates plus carton generation profile. Supply scene objects/presentation components to W02. Do not infer production from animation or create other professional solvers.',
        'W04': 'Implement the full selected paper-packaging loop: all three mechanisms and three change families, forming/opening/fixed-pose linear extraction, Pass/Fail/Undetermined, editable geometry and SVG/material outputs. Keep human acceptance and consumer binding separate; packaging calculations do not add stock. Supply professional results to W00/W01/W05 and your approved panel to W02.',
        'W05': 'Implement editable project handoff, same-result PDF/HTML/CSV/JSON, the real stdio MCP bridge to the existing Windows host and Agent skills/workflow. Use shared Commands, revisions and Engine. Resolve template/parameter sources, inspect/render/run, deliver a successful initial draft directly; exercise repair on a separate supported-shortfall case. Do not run a second simulation state.'}
    prompt_rows = []
    for ident, role in roles.items():
        filename = f"{ident[1:]}_{role['title'].replace('/', '_')}.md"
        text = f'''# {ident} {role['title']}：窗口启动提示词

当前提示词已准备，尚未派发。设计冻结/执行准入/实际前置和用户授权不足时，提示词明确要求停止编码。{current_design_notice()}

以下正文可完整复制到对应窗口；技术交接用英文，产品中文界面与文档保持现行设计。

---

You are window {ident}, the {role['title']} workstream for Super Factory. The human user is the sole accountable project owner. Work only on an explicitly dispatched slice of the selected first release; do not launch other windows, publish, or expand into the seven deferred tasks or the other professional research directions.

Project root: `{S}`.

Before any coding, read AGENTS.md and these current files: docs/00_项目基线/首版交付基线.md, docs/00_项目基线/首版任务选择.json, docs/04_开发与验收/本机并行窗口计划.json, docs/04_开发与验收/完整设计与执行准入.json, docs/04_开发与验收/任务框架与工量里程碑.json, docs/04_开发与验收/阶段实现计划.json, docs/01_业务需求/非功能需求与测量验收.json, docs/02_交互设计/交互流程与视图导航.json, docs/02_交互设计/首版表单与反馈.json, docs/03_架构与契约/全层组件与接点.json, docs/03_架构与契约/共享状态与所有权.json, packages/module-kit/category-contracts.design.json, docs/03_架构与契约/首版技术栈与部署设计.json, docs/03_架构与契约/调度与状态转换算法.json, docs/04_开发与验收/编码任务包与AI实现审查.md, and the package for the current task under docs/04_开发与验收/编码任务包. Read only its relevant specifications, interfaces, scenarios and current code, not the entire research library. Use deliverable milestones, not an invented developer daily schedule. P work families do not introduce whole-family start barriers. Follow the applicable NFR environment/load/sampling and visible-feedback/commit timing rules; synthetic examples and hardware inventory are not performance evidence. Keep expected business outcomes separate from verification verdicts, require actual SF/IX evidence, and never require P5 material completion before starting candidate tests. Retain the software's production calendars, arrival/order/transport schedules and daily supply targets; human effort and parallel-window activity budgets are distinct. Preserve the internal material-readiness cutoff of 2026-10-30 00:00 Asia/Shanghai without asserting that it is already achievable.

START CONDITIONS: verify complete design acceptance through S06-05 and real preparation through S07-05, the accepted module views/technology choices/implementation method/contracts, actual implementation_dependencies outputs, and an explicit task dispatch identifying the current internal step and allowed files. A draft package or old percentage is not approval. If missing, state the concrete missing item and stop coding; do not guess an API, algorithm or successful result.

Your primary work packages: {', '.join(role['primary_tasks'])}.

Your role: {focus[ident]}

Exclusive write areas: {'; '.join(role['write_roots'] + role['write_files'])}. All dependency manifests, lockfiles, build/package configuration, shared domain/Commands/generated types, canonical scenarios and app host/entry wiring belong to W00 even when nested in your package. release_paths identify a task's affected areas, not permission to write another window's files. Request concrete shared changes from W00; W02 composes feature panels. Do not seize tools/3d-proof or replay an old whole-file copy over current changes.

FRONTEND SUPERVISION: the user selected Figma, and all frontend design/prototypes/building must use it under the user's supervision. Do not create/edit Figma files, make UI prototypes, write frontend components/styles or start a frontend build without explicit approval for that phase and batch and the relevant reviewed Figma design. Use the shared Figma design/components coordinated by W02 and the approved frame/node mapping for the current component. Follow the installed Figma design and design-to-code skills when authorized to use that software. This applies to root shell and device, professional, Agent, report and handoff panels in every window. Backend approval is not frontend approval. Continue within an already approved batch without repeatedly requesting the same permission; do not invent approval or user attendance.

WORKFLOW: first verify the accepted task/interaction, structure/data, state/behavior, interface/integration, UI/presentation and deployment/validation views applicable to the module. Verify the selected technology and exact public contracts. Take one bounded, independently verifiable slice from execution_plan; implement against existing types. Validate it with the actual S07 entry and specification-derived finite expectations. Do not rewrite expectations to match the implementation. Send the changed-file diff and actual results for W00's read-only review and consumer integration before asking the user to accept the slice. A local function or compilation does not close release_dependencies/release_acceptance or IX checks.

TYPE SOURCE: consume docs/03_架构与契约/全层类型与工程格式.json and packages/domain/types.design.rs. TypeScript and JSON Schema are derived, not separate field sources. Use the exact module/category bindings, integer/decimal units, coordinate frames and manifest document roles. Old .design inputs require explicit mapping; isolated type/storage proof is not product Save/Open, an implemented API, a professional calculation or frontend authorization. Operation signatures and envelope/TaskScope semantics are now governed by docs/03_架构与契约/公开操作与宿主调用契约.json and packages/commands/api.design.rs. S06-04/05 still govern algorithms and frozen expectations. The typed-contract proof does not implement business handlers or grant coding authorization.

Handoff in English: task/internal step; changed files; consumed current interfaces; actual inputs/commands/results; remaining acceptance/dependency gaps; cross-window actions requested. Preserve Chinese UI text. Report observed failures precisely and fix the scoped cause; do not add speculative fallback, abstractions, logging, retries or unrelated tests. Do not update whole-task done or fabricate actual_result. W00 verifies full evidence and the user makes the final acceptance decision.

Current dispatch fields, to be filled by the user/W00 before execution: task and step; exact input/output; accepted design references; allowed files; actual producer outputs; real verification entry; frontend authorization and reviewed Figma scope if relevant. They are not authorized by this prompt alone.
'''
        (prompts / filename).write_text(text, encoding='utf-8')
        prompt_rows.append({'window_id': ident, 'prompt_ref': f'docs/04_开发与验收/窗口启动提示词/{filename}',
                            'primary_tasks': role['primary_tasks'], 'launch_authorized': False})
    save(prompts / '启动提示词索引.json', {'document_kind': 'prepared_prompts_not_window_dispatch',
          'source_window_plan_ref': 'docs/04_开发与验收/本机并行窗口计划.json',
          'gate_ref': 'docs/04_开发与验收/完整设计与执行准入.json', 'prompts': prompt_rows,
          'refresh_before_coding': True, 'figma_design_started': bool(load(S / 'docs/02_交互设计/Figma执行状态.json')['actual_design_write']), 'frontend_build_started': False})
    prompt_readme = ['# 六窗口启动提示词', '',
                     '用户确认1统筹＋5并行窗口；本目录提供可复制提示词，不启动任何聊天。' + current_design_notice(), '',
                     '进入编码前：S06完整设计冻结后按最新视图/技术/接口核对，S07实际入口落实后刷新任务包，由用户/W00填当前切片/允许文件/实际前置并派发。未经批准的前端部分不能自动开始。', '',
                     '|窗口|职责|完整提示词|', '|---|---|---|']
    prompt_readme += [f"|{r['window_id']}|{roles[r['window_id']]['title']}|[{r['window_id']}]({Path(r['prompt_ref']).name})|" for r in prompt_rows]
    prompt_readme += ['', '[设计与执行准入](../完整设计与执行准入.md) · [编码任务包](../编码任务包/README.md) · [实现审查规则](../编码任务包与AI实现审查.md) · [机器索引](启动提示词索引.json)', '',
                      '最终选题报告当前交付取准入JSON的current_report_delivery_ref。产品实施派发数取窗口计划；Figma设计完成状态取现行执行记录；提示词中的空缺不是默认授权。']
    (prompts / 'README.md').write_text('\n'.join(prompt_readme) + '\n', encoding='utf-8')
    print(json.dumps({'coding_packages': len(package_rows), 'startup_prompts': len(prompt_rows),
                      'status': 'draft_not_dispatchable', 'windows_started': 0, 'frontend_started': False}, ensure_ascii=False))


def refresh_frozen_design():
    """只投影实际S06冻结，不授予产品派发或填写未执行命令。"""
    gate = load(A / '完整设计与执行准入.json')
    if gate['design_freeze']['status'] != 'passed':
        return
    review_ref = 'docs/04_开发与验收/S06-05_全层贯通设计与冻结复审.json'
    review = load(S / review_ref)
    assert review['status'] == 'passed_all_layer_design_freeze'
    matrix = {row['task_id']: row for row in review['task_design_matrix']}
    index_path = A / '编码任务包/任务包索引.json'
    index = load(index_path)
    assert set(matrix) == {row['task_id'] for row in index['packages']}
    begin, end = '<!-- frozen-design:start -->', '<!-- frozen-design:end -->'
    for row in index['packages']:
        assert row['status'] in ['draft_not_dispatchable', 'ready_for_dispatch', 'in_progress']
        if row['status'] == 'in_progress':
            assert row['actual_dispatch'] and row['current_execution_ref']
        row['design_accepted'] = True
        row['design_acceptance_ref'] = review_ref
        detail = matrix[row['task_id']]
        row['design_view_refs'] = detail['views']
        path = S / row['package_ref']
        text = path.read_text(encoding='utf-8-sig')
        text = text.replace('模块完整设计/技术选型认可、S07真实命令、当前切片文件及用户授权尚未齐备',
                            '模块设计已在S06-05认可；S07真实命令、当前切片文件及产品派发尚未齐备')
        text = re.sub(r'\*\*编码前待落实\*\*：[^\n]+',
                      '**设计认可已完成**：适用六类视图、具体技术/算法/公共契约取下列冻结引用；S07仍须落实产品实际入口与验证命令。', text)
        text = text.replace('当前内部步骤、明确允许文件/新文件、接口与命令版本、设计认可记录：**待S06/S07完成并由用户/W00派发填写**。',
                            '设计认可记录已形成；当前内部步骤、明确允许文件/新文件及实际命令：**待S07完成并由用户/W00按切片派发填写**。')
        lines = ['## S06冻结设计输入', '',
                 '[全层联合冻结](../S06-05_全层贯通设计与冻结复审.json)；本包设计已认可；当前产品状态与派发取唯一任务源和本包现行记录，设计认可不单独授予派发。', '']
        for view, refs in detail['views'].items():
            lines += [f'**{view}**：'] + [f'- [{ref}](../../../{ref})' for ref in refs] + ['']
        block = begin + '\n' + '\n'.join(lines) + '\n' + end
        if begin in text:
            start = text.index(begin)
            stop = text.index(end, start) + len(end)
            text = text[:start] + block + text[stop:]
        else:
            text = text.rstrip() + '\n\n' + block + '\n'
        path.write_text(text, encoding='utf-8')
    save(index_path, index)
    prompts = load(A / '窗口启动提示词/启动提示词索引.json')
    for row in prompts['prompts']:
        assert row['launch_authorized'] is bool(row.get('actual_dispatch'))
        row['design_acceptance_ref'] = review_ref
        path = S / row['prompt_ref']
        text = path.read_text(encoding='utf-8-sig')
        marker = '<!-- frozen-design-source -->'
        if marker not in text:
            text += '\n' + marker + '\nS06 design freeze is accepted. Read each assigned task package frozen design views and canonical contracts. Product coding still requires actual S07-05 preparation, an explicit scoped dispatch, and applicable frontend build approval. No actual product commands or results are implied.\n'
        path.write_text(text, encoding='utf-8')
    save(A / '窗口启动提示词/启动提示词索引.json', prompts)
    print(json.dumps({'frozen_design_packages': len(matrix), 'dispatch_changed': False}, ensure_ascii=False))


if __name__ == '__main__':
    if '--handoff-only' in sys.argv:
        refresh_current_handoff()
    elif '--confirmed-choice-only' in sys.argv:
        refresh_confirmed_choices()
    elif '--design-freeze-only' in sys.argv:
        refresh_frozen_design()
    else:
        if not (A / '编码任务包/任务包索引.json').exists():
            main()
        if 'confirmed_choice_ref' in load(S / 'docs/00_项目基线/首版任务选择.json'):
            refresh_confirmed_choices()
        refresh_frozen_design()
        refresh_current_handoff()
