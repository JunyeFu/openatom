import { dispatch } from '../../../apps/planner/src/services/projectHost';
import { readMediaAsset, type MediaAssetReadRequest } from '../../../apps/planner/src/services/mediaAssets';
import { ProcessSourceChanged, ProcessUnavailable, readActualPortOutputs, type PresentationReadPorts, type ProcessScope } from './presentationHost.before';
import type { ApiRequest } from '../../../packages/commands/generated/typescript/ApiRequest';
import type { DomainObject } from '../../../packages/commands/generated/typescript/DomainObject';
import type { ProjectState } from '../../../packages/commands/generated/typescript/ProjectState';
import type { ModuleRef } from '../../../packages/commands/generated/typescript/ModuleRef';
import type { ModulePackage } from '../../../packages/commands/generated/typescript/ModulePackage';
import type { RunResult } from '../../../packages/commands/generated/typescript/RunResult';
import type { RunView } from '../../../packages/commands/generated/typescript/RunView';
import type { RunObjectsPage } from '../../../packages/commands/generated/typescript/RunObjectsPage';
import type { OperationDefinition } from '../../../packages/commands/generated/typescript/OperationDefinition';
import type { ActualMediaFrame, ActualPortOutputFrame } from '../../../packages/presentation/src';
import type { MediaPresentationCommonProps } from '../../../apps/planner/src/features/media/types';
import type { RuntimeItem } from '../../../packages/commands/generated/typescript/RuntimeItem';
import type { JobState } from '../../../packages/commands/generated/typescript/JobState';
import type { ObjectRef } from '../../../packages/commands/generated/typescript/ObjectRef';
import type { Pose } from '../../../packages/commands/generated/typescript/Pose';
import type { AssetRef } from '../../../packages/commands/generated/typescript/AssetRef';

export interface FactoryPresentationContext extends ProcessScope {
  project: ProjectState;
  view: { run_id: string; at_ms: number } | null;
}
export interface FactoryDeviceMedia {
  device_id: string;
  media: Omit<MediaPresentationCommonProps, 'lod'> | null;
  frame: ActualMediaFrame | null;
  unavailable: string | null;
  projection: 'active' | 'terminal' | 'static' | 'unavailable';
  port_outputs?: ActualPortOutputFrame;
}
/** Internal consumer port only. There is currently no persisted public producer. */
export interface FactoryItemAttachment {
  item_id: string;
  item_type_ref: RuntimeItem['item_type_ref'];
  holder: RuntimeItem['holder'];
  media_ref: ModuleRef;
  target: (
    { kind: 'device_node'; device_id: string; device_module_ref: ModuleRef; media_ref: ModuleRef; asset_ref: AssetRef; node_index: number }
    | { kind: 'location'; location_id: string }
    | { kind: 'container_node'; container_id: string; media_ref: ModuleRef; asset_ref: AssetRef; node_index: number }
  ) & { offset: Pose };
}
export interface FactoryItemAttachmentRequest extends ProcessScope {
  run_id: string; at_ms: number; frozen_project_revision: number; item_ids: string[];
}
export interface FactoryItemAttachmentResponse extends ProcessScope {
  run_id: string; at_ms: number; frozen_project_revision: number;
  bindings: FactoryItemAttachment[];
}
export interface FactoryPresentationPorts extends PresentationReadPorts {
  readItemAttachments?(request: FactoryItemAttachmentRequest): Promise<FactoryItemAttachmentResponse>;
}
export interface FactoryActualItem {
  item: RuntimeItem;
  anchor_ref: ObjectRef | null;
  attachment: FactoryItemAttachment | null;
  media: Omit<MediaPresentationCommonProps, 'lod'> | null;
  unavailable: string | null;
}
export interface FactoryPresentationSource {
  key: string;
  project: ProjectState;
  run: RunResult | null;
  observation: RunObjectsPage | null;
  devices: FactoryDeviceMedia[];
  items: FactoryActualItem[];
  raw: { job_pages: RunObjectsPage[]; item_pages: RunObjectsPage[] };
}
export const factoryContextKey = (c: FactoryPresentationContext) => JSON.stringify([
  c.session_id, c.project_id, c.scope, c.scope_revision, c.project.id, c.project.revision, c.view,
]);
const sameRef = (a: ModuleRef, b: ModuleRef) => a.id === b.id && a.version === b.version;
export function factoryItemHolder(item: RuntimeItem, items: RuntimeItem[], jobs: JobState[]): ObjectRef | null {
  let current: RuntimeItem | undefined = item;
  const seen = new Set<string>();
  while (current && !seen.has(current.id)) {
    seen.add(current.id);
    if (current.holder.kind === 'location') return { kind: 'location', id: current.holder.id };
    if (current.holder.kind === 'transport') return null;
    if (current.holder.kind === 'job') {
      const found = jobs.filter(job => job.id === current!.holder.id);
      return found.length === 1 && found[0].execution_ref.kind === 'project_device'
        ? { kind: 'device', id: found[0].execution_ref.data.device_id } : null;
    }
    const parent = items.find(value => value.id === current!.holder.id);
    if (!parent?.contents.includes(current.id)) return null;
    current = parent;
  }
  return null;
}
type ReadOperation = 'get_object' | 'get_run';
type ReadRequest = { [O in ReadOperation]: { operation: O; params: Extract<ApiRequest, { operation: O }>['params'] } }[ReadOperation];

/** A read-only source for the one physical scene. Final Jobs supply IDs only. */
export function createFactoryPresentation(current: () => FactoryPresentationContext | null,
  ports: FactoryPresentationPorts = { dispatch, readAsset: readMediaAsset,
    readPortOutput: async request => (await import('../../../apps/planner/src/services/runPortOutput')).readRunPortOutput(request) }) {
  async function read(context: FactoryPresentationContext): Promise<FactoryPresentationSource> {
    const key = factoryContextKey(context);
    const assertCurrent = () => { const next = current(); if (!next || factoryContextKey(next) !== key) throw new ProcessSourceChanged(); };
    const request = async <T>(input: ReadRequest): Promise<T> => {
      assertCurrent();
      const value = await ports.dispatch<T>({ api_version: '0.1', request_id: `factory-media-${crypto.randomUUID()}`,
        project_id: context.project_id, scope: context.scope, ...input });
      assertCurrent(); return value;
    };
    assertCurrent();
    if (context.project.id !== context.project_id) throw new ProcessUnavailable('主画布工程与授权来源不一致。');
    let project = context.project, run: RunResult | null = null, observation: RunObjectsPage | null = null;
    const job_pages: RunObjectsPage[] = [], item_pages: RunObjectsPage[] = [];
    if (context.view) {
      const view = context.view;
      const full = await request<RunView>({ operation: 'get_run', params: { run_id: view.run_id, query: { kind: 'full_result' } } });
      if (full.kind !== 'full' || full.data.id !== view.run_id || full.data.input_ref.project_id !== context.project_id) throw new ProcessUnavailable('主画布没有取得本工程的精确Run。');
      run = full.data;
      if (run.computed_until_ms == null || view.at_ms < run.window.start_ms || view.at_ms > run.computed_until_ms || view.at_ms > run.window.end_ms) throw new ProcessUnavailable('观看时点超出实际Run前沿，不补画实际阶段。');
      const pages = async (objects: { kind: 'job'; id: string }[], target: RunObjectsPage[]) => {
        let after_index: number | null = null;
        do {
          const page: RunView = await request<RunView>({ operation: 'get_run', params: { run_id: view.run_id,
            query: { kind: 'objects_at', data: { at_ms: view.at_ms, objects, page: { after_index, limit: 200 } } } } });
          if (page.kind !== 'objects' || page.data.run_id !== view.run_id || page.data.at_ms !== view.at_ms) throw new ProcessUnavailable('主画布对象页的Run或时点不匹配。');
          if (page.data.next_index !== null && page.data.next_index <= (after_index ?? -1)) throw new ProcessUnavailable('主画布实际分页没有前进。');
          target.push(page.data); after_index = page.data.next_index;
        } while (after_index !== null);
      };
      // Identity catalog only: states/progress/execution mappings below come exclusively from JobsAt.
      const selectors = run.jobs.map(job => ({ kind: 'job' as const, id: job.id }));
      if (selectors.length) await pages(selectors, job_pages);
      await pages([], item_pages);
      const frozen = await request<DomainObject>({ operation: 'get_object', params: { object_id: { kind: 'project', id: context.project_id },
        source: { kind: 'run_snapshot', data: { run_id: view.run_id, at_ms: view.at_ms } }, node_index: null } });
      if (frozen.kind !== 'project' || frozen.data.id !== context.project_id || frozen.data.revision !== run.input_ref.project_revision) throw new ProcessUnavailable('主画布未取得Run冻结工程修订。');
      project = frozen.data;
      observation = { run_id: run.id, at_ms: view.at_ms, next_index: null, objects: [
        ...job_pages.flatMap(page => page.objects), ...item_pages.flatMap(page => page.objects),
        ...project.resources.map(data => ({ kind: 'resource' as const, data })),
      ] };
    }
    const jobs = job_pages.flatMap(page => page.objects).flatMap(object => object.kind === 'job' ? [object.data] : []);
    const modules = new Map<string, Promise<ModulePackage>>();
    const module = (ref: ModuleRef) => {
      const id = JSON.stringify(ref);
      let pending = modules.get(id);
      if (!pending) {
        pending = (async () => {
          if (context.scope.kind === 'definition_draft' && !context.view) throw new ProcessUnavailable('DefinitionDraft缺少精确ModuleVersion元数据生产者；主画布不改Scope读取。');
          const source = context.view ? { kind: 'run_module_version' as const, data: { ...context.view, module_ref: ref } }
            : { kind: 'module_version' as const, data: { module_ref: ref } };
          const value = await request<DomainObject>({ operation: 'get_object', params: { object_id: { kind: 'module', id: ref.id }, source, node_index: null } });
          if (value.kind !== 'module' || !sameRef(value.data.definition, ref)) throw new ProcessUnavailable('主画布精确模块版本不可用。');
          return value.data;
        })();
        modules.set(id, pending);
      }
      return pending;
    };
    const boundMedia = (ref: ModuleRef, presentation: MediaPresentationCommonProps['presentation'], rights: MediaPresentationCommonProps['rights']): Omit<MediaPresentationCommonProps, 'lod'> => {
      const binding = { session_id: context.session_id, request: { project_id: context.project_id, scope: context.scope, scope_revision: context.scope_revision,
        source: context.view ? { kind: 'run_module_version' as const, data: { ...context.view, module_ref: ref } } : { kind: 'module_version' as const, data: { module_ref: ref } } } };
      const assertSource = (source: MediaAssetReadRequest['source']) => {
        assertCurrent();
        if (context.view ? source.kind !== 'run_module_version' || source.data.run_id !== context.view.run_id || source.data.at_ms !== context.view.at_ms : source.kind !== 'module_version') throw new ProcessUnavailable('媒体字节或依赖改变了已绑定Run/时点/来源。');
      };
      return { presentation, rights, binding, async readAsset(request) {
        assertSource(request.source);
        if (request.project_id !== context.project_id || JSON.stringify(request.scope) !== JSON.stringify(context.scope) || request.scope_revision !== context.scope_revision) throw new ProcessUnavailable('媒体字节改变了已绑定工程或Scope修订。');
        const bytes = await ports.readAsset(request); assertCurrent(); return bytes;
      }, async resolveModule(source) { assertSource(source); return module(source.data.module_ref); } };
    };
    const devices = await Promise.all(project.devices.map(async (device): Promise<FactoryDeviceMedia> => {
      let port_outputs: ActualPortOutputFrame | undefined;
      try {
        const pkg = await module(device.module_ref), payload = pkg.definition.payload;
        if (payload.category !== 'devices') throw new ProcessUnavailable('设备精确模块不是设备定义。');
        if (run && observation) port_outputs = await readActualPortOutputs(context, { run_id: run.id, at_ms: observation.at_ms, device, ports: payload.data.ports }, ports, assertCurrent);
        const presentation = device.presentation_override ?? payload.data.presentation;
        if (!presentation) throw new ProcessUnavailable('此设备未声明详细表现素材。');
        const mapped = jobs.filter(job => job.execution_ref.kind === 'project_device' && job.execution_ref.data.device_id === device.id);
        const at = context.view?.at_ms;
        // JobsAt retains completed and queued executions. Only an actual open
        // execution or an explicit capacity holder currently owns this device.
        const active = mapped.filter(job => job.state !== 'completed' && job.state !== 'cancelled' && at != null
          && (job.finished_at_ms == null || at < job.finished_at_ms)
          && (job.started_at_ms != null && job.started_at_ms <= at || job.execution_ref.kind === 'project_device'
            && [...job.job_resources, ...job.phase_resources].includes(job.execution_ref.data.execution_capacity_ref.id)));
        if (active.length > 1) throw new ProcessUnavailable('同一时点有多个实际活动Job占用/执行此设备；无法唯一投影，未选择首个。');
        // A single historical execution has an unambiguous terminal pose.
        // Multiple completed histories with no owner stay static; no latest/first guess.
        const terminal = mapped.length === 1 && mapped[0].state === 'completed' && mapped[0].finished_at_ms != null
          && at != null && mapped[0].finished_at_ms <= at ? mapped[0] : undefined;
        const job = active.length === 1 ? active[0] : terminal;
        const projection = active.length === 1 ? 'active' : terminal ? 'terminal' : 'static';
        let frame: ActualMediaFrame | null = null;
        if (job && run && observation) {
          let operation: OperationDefinition;
          if (sameRef(job.operation_ref, device.module_ref) && payload.data.behavior?.selected === 'inline' && payload.data.behavior.inline) operation = payload.data.behavior.inline;
          else {
            const declared = payload.data.behavior.selected === 'operation_ref' ? payload.data.behavior.operation_ref : null;
            if (![declared, device.operation_binding, ...(device.route_operation_refs ?? [])].some(ref => ref != null && sameRef(ref, job.operation_ref))) throw new ProcessUnavailable('实际Job的operation不匹配冻结设备声明。');
            const operationPackage = await module(job.operation_ref);
            if (operationPackage.definition.payload.category !== 'operations') throw new ProcessUnavailable('实际operation精确模块类型不匹配。');
            operation = operationPackage.definition.payload.data;
          }
          frame = { run, snapshot: observation, job_id: job.id, operation };
        }
        if (frame) frame = { ...frame, port_outputs };
        return { device_id: device.id, media: boundMedia(device.module_ref, presentation, pkg.rights), frame, unavailable: null, projection, port_outputs };
      } catch (error) {
        assertCurrent();
        if (!(error instanceof ProcessUnavailable)) throw error;
        return { device_id: device.id, media: null, frame: null, unavailable: error.message, projection: 'unavailable', port_outputs };
      }
    }));
    const actualItems = item_pages.flatMap(page => page.objects).flatMap(object => object.kind === 'item' ? [object.data] : []);
    let attachments: FactoryItemAttachment[] = [];
    if (run && observation && ports.readItemAttachments) {
      const value = await ports.readItemAttachments({ session_id: context.session_id, project_id: context.project_id, scope: context.scope, scope_revision: context.scope_revision,
        run_id: run.id, at_ms: observation.at_ms, frozen_project_revision: project.revision, item_ids: actualItems.map(item => item.id) });
      assertCurrent();
      if (value.session_id !== context.session_id || value.project_id !== context.project_id || JSON.stringify(value.scope) !== JSON.stringify(context.scope) || value.scope_revision !== context.scope_revision || value.run_id !== run.id || value.at_ms !== observation.at_ms || value.frozen_project_revision !== project.revision) throw new ProcessUnavailable('物品附着来源改变了会话/工程/Scope/修订/Run/时点。');
      attachments = value.bindings;
    }
    const items = await Promise.all(actualItems.map(async (item): Promise<FactoryActualItem> => {
      const anchor_ref = factoryItemHolder(item, actualItems, jobs);
      let media: FactoryActualItem['media'] = null;
      const unavailable = (reason: string): FactoryActualItem => ({ item, anchor_ref, attachment: null, media, unavailable: reason });
      const itemType = project.item_types.find(type => type.id === item.item_type_ref.id && type.revision === item.item_type_ref.revision);
      if (!itemType?.module_ref) return unavailable('unavailable · 冻结ItemType没有精确Items模块来源，不从名称/spec_ref/module_refs猜测。');
      try {
        const pkg = await module(itemType.module_ref), payload = pkg.definition.payload;
        if (payload.category !== 'items' || !payload.data.presentation_or_media_ref) return unavailable('精确Items模块没有物品媒体声明。');
        media = boundMedia(itemType.module_ref, payload.data.presentation_or_media_ref, pkg.rights);
      } catch (error) { assertCurrent(); if (!(error instanceof ProcessUnavailable)) throw error; return unavailable(error.message); }
      if (item.lifecycle === 'transformed' || item.lifecycle === 'scrapped') return unavailable('实际物品已转化或报废；保留身份，不复制成活动库存几何。');
      const bindings = attachments.filter(binding => binding.item_id === item.id);
      if (bindings.length !== 1) return unavailable('unavailable · 缺少唯一冻结物品媒体版本/holder节点与偏移绑定，未绘制物品几何。');
      const attachment = bindings[0];
      if (!sameRef(attachment.media_ref, itemType.module_ref)) return unavailable('空间附着试图替换冻结ItemType的精确Items模块版本。');
      if (attachment.item_type_ref.id !== item.item_type_ref.id || attachment.item_type_ref.revision !== item.item_type_ref.revision || attachment.holder.kind !== item.holder.kind || attachment.holder.id !== item.holder.id) return unavailable('物品附着绑定与此实际版本或holder不匹配。');
      const target = attachment.target;
      if (target.kind === 'device_node') {
        const owner = project.devices.find(value => value.id === target.device_id);
        if (item.holder.kind !== 'job' || anchor_ref?.kind !== 'device' || anchor_ref.id !== target.device_id || !owner || !sameRef(owner.module_ref, target.device_module_ref)) return unavailable('物品设备节点绑定不属于实际holder及冻结设备版本。');
      } else if (target.kind === 'location') {
        if (item.holder.kind !== 'location' || item.holder.id !== target.location_id || !project.locations.some(location => location.id === target.location_id && location.hall_id)) return unavailable('物品场地绑定没有实际Location坐标。');
      } else {
        const owner = actualItems.find(value => value.id === target.container_id);
        if (item.holder.kind !== 'container' || item.holder.id !== target.container_id || !owner?.contents.includes(item.id)) return unavailable('容器附着没有对应实际contents/holder关系。');
      }
      return { item, anchor_ref, attachment, media, unavailable: null };
    }));
    assertCurrent();
    return { key, project, run, observation, devices, items, raw: { job_pages, item_pages } };
  }
  return { read };
}
