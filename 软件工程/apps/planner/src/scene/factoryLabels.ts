import type { ProjectState } from '../../../../packages/commands/generated/typescript/ProjectState';
import type { RuntimeItem } from '../../../../packages/commands/generated/typescript/RuntimeItem';
import type { RunResult } from '../../../../packages/commands/generated/typescript/RunResult';
import type { Lifecycle } from '../../../../packages/commands/generated/typescript/Lifecycle';
import type { QualityState } from '../../../../packages/commands/generated/typescript/QualityState';

export const factoryLifecycleLabel: Record<Lifecycle, string> = { available: '可用', in_process: '加工中', transformed: '已投入加工', scrapped: '已报废' };
export const factoryQualityLabel: Record<QualityState, string> = { unchecked: '待检', pending: '等待判定', released: '已放行', rejected: '不合格' };
export const factoryUnitLabel = (unit: string) => ({ piece: '件', kg: '千克', vehicle: '辆', person: '人', slot: '工位', meter: '米', pair: '副', roll: '卷', ml: '毫升' } as Record<string, string>)[unit] ?? unit;
export const factoryDeviceTitle = (title: string) => title.split('（')[0];
const productTitles: Record<string, { original: string; label: string }> = {
  'qa.paper.b01q13.items.folder-gluer-output': { original: 'folder-gluer-output', label: '折糊纸套' },
  'sf.paperfactory.families.items.paper-core-grinder-output': { original: 'paper-core-grinder-output', label: '精磨纸管' },
  'sf.paperfactory.families.items.label-peeler-output': { original: 'label-peeler-output', label: '80×60mm标签' },
};
const roleTitles: Record<string, string> = { output: '产出', input: '待加工件', auxiliary: '辅料', auxiliary_red: '红色油墨', component: '配件', container: '包装材料', waste: '边料', scrap: '边角料', side_scrap: '侧边料', end_scrap: '端边料', backing: '底纸', spent_backing: '废衬膜', rejected: '待处理工件', isolation: '待处理工件' };

/** Declared names win; inherited identifier-only titles use their real bound role. */
export function factoryItemTitle(project: ProjectState | null, item: Pick<RuntimeItem, 'item_type_ref'>) {
  return factoryItemTypeTitle(project, item.item_type_ref);
}

export function factoryItemTypeTitle(project: ProjectState | null, ref: RuntimeItem['item_type_ref']) {
  const type = project?.item_types.find(t => t.id === ref.id && t.revision === ref.revision);
  if (!type) return '物品类型未定义';
  if (type.title !== type.id && !type.id.endsWith(':' + type.title)) return type.title;
  const alias = type.module_ref ? productTitles[type.module_ref.id] : undefined;
  if (alias?.original === type.title) return alias.label;
  for (const role of Object.keys(roleTitles)) {
    const device = project!.devices.find(d => d.role_bindings.item_types[role] === type.id);
    if (device) return factoryDeviceTitle(device.title) + roleTitles[role];
  }
  return '未绑定的物料';
}

export function factoryResourceTitle(project: ProjectState | null, id: string): string {
  const resource = project?.resources.find(value => value.id === id);
  if (!resource) return '资源未定义';
  if (resource.title !== resource.id && !resource.id.endsWith(':' + resource.title)) return resource.title;
  for (const device of project!.devices) {
    const binding = Object.entries(device.role_bindings.resources).find(([, value]) => value === id);
    if (binding) {
      const role = ({ execution_capacity: '加工工位', operator: '操作人员', auto_fixture: '自动工装' } as Record<string, string>)[binding[0]] ?? '作业资源';
      return `${factoryDeviceTitle(device.title)} · ${role}`;
    }
  }
  return resource.title;
}

export function factoryItemPlace(project: ProjectState | null, item: RuntimeItem, run: RunResult | null = null, peers: RuntimeItem[] = []): string {
  const holder = item.holder;
  if (holder.kind === 'location') return project?.locations.find(l => l.id === holder.id)?.title ?? '库位未定义';
  if (holder.kind === 'transport') return '厂内转运中';
  if (holder.kind === 'container') { const parent = peers.find(i => i.id === holder.id); return parent ? factoryItemTitle(project, parent) + '内' : '容器内'; }
  const execution = run?.jobs.find(j => j.id === holder.id)?.execution_ref;
  if (execution?.kind === 'project_device') {
    const device = project?.devices.find(d => d.id === execution.data.device_id);
    if (device) return factoryDeviceTitle(device.title) + '工位';
  }
  return '工序中';
}
