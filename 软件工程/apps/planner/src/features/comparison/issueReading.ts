import type { Issue } from '../../../../../packages/commands/generated/typescript/Issue';

const explanations: Record<string, string> = {
  comparison_inputs: '请先选择两条实际运行记录，再比较结果。',
  comparison_frozen_identity: '所选记录的冻结来源不一致，不能据此比较；请重新核对运行记录。',
  comparison_metric_unknown: '所选指标缺少必要的实际测量或完整条件，数值保持未知。',
  metric_not_comparable: '比较条件不同、指标未知或单位不兼容，不能据此认定改善。',
  'evaluation.partial_result': '只计算了观察窗口的一部分，尚不能判断整个窗口是否达标。',
  'evaluation.missing_module': '缺少所引用模块的精确定义，请核对原模块及版本。',
  'evaluation.cost_inputs': '未提供价格与成本范围，完整总成本保持未知。',
  'evaluation.cost_rate': '未填写费率，相关成本保持未知。',
  'evaluation.cost_currency': '未填写币种，完整成本保持未知。',
  'evaluation.cost_unit': '计量单位与费率单位不匹配，不能直接换算成本。',
  'evaluation.cost_overlap': '所选成本存在重叠，需要先明确计费范围，当前未重复相加。',
  'evaluation.cost_prefix': '只计算了部分窗口，整个窗口的总成本保持未知。',
  'evaluation.cost_unassigned_currency': '部分成本尚未确定币种，完整总额保持未知。',
  'evaluation.unit_cost': '完整同口径成本或接受量条件不足，单位成本保持未知。',
  'render.camera_not_applicable': '这是二维视图，不使用三维相机。',
  'render_view.presentation_limitation': '画面存在表现范围限制，具体原因请展开技术来源核对。',
};

/** A reading aid only: the complete original Issue remains the source of truth. */
export function issueMessage(issue: Issue): string {
  if (issue.code === 'render_view.presentation_limitation' && issue.message === '二维时间轴不适用三维相机；可见对象只取当前虚拟窗口内实际DOM轨道/标记。') return '时间轴画面仅显示当前可见范围，不代表全部明细。';
  if (/[\u3400-\u9fff]/u.test(issue.message) && issue.code !== 'evaluation.missing_module') return issue.message;
  const known = explanations[issue.code];
  if (known) return known;
  return issue.blocks.length
    ? '存在影响相关操作的条件限制；具体原因尚需核对，请展开技术来源。'
    : '该记录带有适用范围提示；具体原因尚需核对，请展开技术来源。';
}

export function issueGroups(issues: readonly Issue[]) {
  const groups = new Map<string, { message: string; issues: Issue[] }>();
  for (const issue of issues) {
    const message = issueMessage(issue);
    const group = groups.get(message);
    if (group) group.issues.push(issue);
    else groups.set(message, { message, issues: [issue] });
  }
  return [...groups.values()];
}
