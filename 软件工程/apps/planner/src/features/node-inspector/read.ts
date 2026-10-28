import { dispatch } from '../../services/projectHost';
import { ProcessSourceChanged, ProcessUnavailable, type PresentationReadPorts } from '../../services/presentationHost';
import type { ApiRequest } from '../../../../../packages/commands/generated/typescript/ApiRequest';
import type { DomainObject } from '../../../../../packages/commands/generated/typescript/DomainObject';
import type { ProjectContext } from '../../../../../packages/commands/generated/typescript/ProjectContext';
import { nodeRequestParams, nodeSelectionKey } from './model';
import type { NodeInspection, ProcessNodeSelection } from './types';
type ReadOperation = 'get_context' | 'get_object';
type ReadRequest = { [O in ReadOperation]: { operation: O; params: Extract<ApiRequest, { operation: O }>['params'] } }[ReadOperation];

/** Same authorized get_object; no local-node fallback, Command or new API. */
export function createNodeInspectorReader(current: () => ProcessNodeSelection | null,
  ports: Pick<PresentationReadPorts, 'dispatch'> = { dispatch }) {
  async function read(selection: ProcessNodeSelection): Promise<NodeInspection> {
    const key = nodeSelectionKey(selection), { context } = selection;
    const assertCurrent = () => { const next = current(); if (!next || nodeSelectionKey(next) !== key) throw new ProcessSourceChanged(); };
    async function request<T>(input: ReadRequest): Promise<T> {
      assertCurrent();
      const response = await ports.dispatch<T>({ api_version: '0.1', request_id: `node-${crypto.randomUUID()}`, project_id: context.project_id,
        scope: context.scope, ...input });
      assertCurrent(); return response;
    }
    async function checkContext() {
      const value = await request<ProjectContext>({ operation: 'get_context', params: { selection: [] } });
      if (value.host.session_id !== context.session_id || value.project?.id !== context.project_id
        || (context.scope.kind === 'project' ? value.project.revision !== context.scope_revision
          : value.definition?.draft_id !== context.scope.data || value.definition.revision !== context.scope_revision)) {
        throw new ProcessUnavailable('节点的真实会话/工程/Scope修订已变化；旧结果不替换当前检查。');
      }
    }
    assertCurrent();
    const selected = context.selection, source = selection.owner.source, bytes = selection.byte_source;
    if (selected.kind !== 'definition') {
      if ((source.kind !== 'run_snapshot' && source.kind !== 'run_module_version') || source.data.run_id !== selected.run_id || source.data.at_ms !== selected.at_ms
        || bytes.kind !== 'run_module_version' || bytes.data.run_id !== selected.run_id || bytes.data.at_ms !== selected.at_ms) throw new ProcessUnavailable('节点父来源或字节基准改变了所选冻结Run/时点。');
      if (selected.kind === 'frozen_device' && (context.scope.kind !== 'project' || selection.owner.object_id.kind === 'device' && selection.owner.object_id.id !== selected.device_id)) throw new ProcessUnavailable('静态节点不属于明确选择的冻结设备/Project。');
      if (selected.kind === 'frozen_definition' && (context.scope.kind !== 'definition_draft' || selection.owner.object_id.kind === 'definition_draft' && selection.owner.object_id.id !== context.scope.data)) throw new ProcessUnavailable('静态节点不属于此opaque Draft冻结定义。');
    } else if (source.kind === 'run_snapshot' || source.kind === 'run_module_version' || bytes.kind !== 'module_version') throw new ProcessUnavailable('声明节点不能借用另一个Run的父来源或字节基准。');
    await checkContext();
    let parent_device: NodeInspection['parent_device'];
    if (selection.owner.object_id.kind === 'device' && source.kind === 'draft') {
      if (context.scope.kind !== 'project' || selected.kind !== 'definition' || selected.device_id !== selection.owner.object_id.id) throw new ProcessUnavailable('此节点不是当前明确选中的Project实例自有表现。');
      const parent = await request<DomainObject>({ operation: 'get_object', params: { ...nodeRequestParams(selection), node_index: null } });
      if (parent.kind !== 'device' || parent.data.id !== selected.device_id || parent.data.module_ref.id !== selected.module_ref.id || parent.data.module_ref.version !== selected.module_ref.version) throw new ProcessUnavailable('节点父设备与当前精确实例/模块不匹配。');
      parent_device = parent.data;
    }
    const object = await request<DomainObject>({ operation: 'get_object', params: nodeRequestParams(selection) });
    await checkContext();
    if (object.kind !== 'scene_node') throw new ProcessUnavailable('Host没有返回此精确父来源的SceneNode；不使用渲染缓存代替。');
    if (selection.rendered_node_id !== undefined && object.data.id !== selection.rendered_node_id) throw new ProcessUnavailable('画面节点与已保存源在此canonical位置的身份不同；请先保存或回退原编辑输入。');
    return { selection, node: object.data, ...(parent_device ? { parent_device } : {}) };
  }
  return { read };
}
