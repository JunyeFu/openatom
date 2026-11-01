import { useRef, useState } from 'react';
import { PoseEditor } from '../device-authoring/PresentationEditor';
import { SchemaField } from '../modules/SchemaField';
import { formSchemas } from '../modules/formSchema';
import { tagOf, variants } from '../modules/model';
import type { ProjectState, EditCommand } from '../../services/projectHost';
import type { InlineNodeFields, NodeInspection } from './types';
import type { ProcessReadContext } from '../../services/presentationHost';
import { canEditInlineNode, inlineNodeFields, nodeEditContextIdentity, withInlineNode } from './model';

export interface InlineNodeEditorProps {
  inspection: NodeInspection;
  base: ProjectState;
  current: boolean;
  currentContext: ProcessReadContext | null;
  onApply(commands: EditCommand[], base: ProjectState): Promise<ProjectState>;
  onDirtyChange?(dirty: boolean): void;
  onClose(): void;
}
/** Current Project inline nodes use the existing complete-owner PutObject. */
export function InlineNodeEditor(props: InlineNodeEditorProps) {
  const latest = useRef(props); latest.current = props;
  const captured = useRef({ inspection: props.inspection, base: props.base }).current;
  const expired = useRef(false); if (!props.current) expired.current = true;
  const current = props.current && !expired.current;
  const [fields, setFields] = useState<InlineNodeFields>(() => inlineNodeFields(captured.inspection.node)), [saving, setSaving] = useState(false), [error, setError] = useState(''), [confirmClose, setConfirmClose] = useState(false);
  const dirty = JSON.stringify(fields) !== JSON.stringify(inlineNodeFields(captured.inspection.node));
  const editable = canEditInlineNode(captured.inspection.selection, captured.inspection.parent_device)
    && captured.base.id === captured.inspection.selection.context.project_id && captured.base.revision === captured.inspection.selection.context.scope_revision;
  const unsupported = captured.inspection.node.geometry.kind === 'imported_node' && captured.inspection.node.geometry.data.unsupported_extensions.length > 0;
  function change(value: InlineNodeFields) { setFields(value); props.onDirtyChange?.(JSON.stringify(value) !== JSON.stringify(inlineNodeFields(captured.inspection.node))); }
  async function apply() {
    const parent = captured.inspection.parent_device;
    if (!editable || !current || !parent || saving) return;
    setSaving(true); setError('');
    try {
      const device = withInlineNode(parent, captured.inspection.selection.node_index, fields);
      await props.onApply([{ kind: 'put_object', data: { kind: 'device', data: device } }], captured.base);
      if (nodeEditContextIdentity(latest.current.currentContext) !== nodeEditContextIdentity(captured.inspection.selection.context)) { setError('原实例提交已返回；当前来源已切换，原输入保留。'); return; }
      props.onDirtyChange?.(false); props.onClose();
    } catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); }
    finally { setSaving(false); }
  }
  const disabled = saving || unsupported || !editable;
  return <form aria-label="实例自有节点编辑" onSubmit={event => { event.preventDefault(); void apply(); }}>
    <h3>编辑实例自有节点 · canonical {captured.inspection.selection.node_index}</h3><p>仅修改此节点字段；其它节点、Presentation声明、设备及许可保留。提交使用Project r{captured.base.revision}的原PutObject。</p>
    {!current && <p role="alert">原会话/Scope/实例来源已变化；编辑输入保留，不能提交。请关闭后重新读取实际来源。</p>}
    {!editable && <p role="alert">只有当前Project中明确选中实例的自有override节点可在此修改；冻结Run、Draft和发布模块沿原定义创作入口。</p>}
    <label>部件名称<input required value={fields.name} disabled={disabled} onChange={event => change({ ...fields, name: event.target.value })}/></label>
    <PoseEditor value={fields.transform} disabled={disabled} onChange={transform => change({ ...fields, transform })}/>
    {fields.geometry && <SchemaField schema={{ oneOf: variants(formSchemas.SceneGeometry).filter(schema => tagOf(schema)?.value !== 'imported_node') }} value={fields.geometry} name="geometry" disabled={disabled} onChange={geometry => change({ ...fields, geometry: geometry as InlineNodeFields['geometry'] })}/>}
    {!fields.geometry && <p>原导入AssetRef、GLB节点索引及texture保持；此入口不导入或替换文件。</p>}
    <SchemaField schema={formSchemas.SceneMaterial} value={fields.material} name="material" omit={['texture']} disabled={disabled} onChange={material => change({ ...fields, material: material as InlineNodeFields['material'] })}/>
    {unsupported && <p role="alert">此导入节点声明未支持扩展，沿原编辑能力保留只读。</p>}
    <button disabled={!dirty || disabled || !current || !captured.inspection.parent_device}>应用此实例节点</button><button type="button" disabled={saving} onClick={() => dirty ? setConfirmClose(true) : props.onClose()}>取消节点编辑</button>
    {confirmClose && <div role="alertdialog" aria-label="保留节点编辑输入"><p>节点有未应用输入。</p><button type="button" onClick={() => setConfirmClose(false)}>保留节点编辑</button><button type="button" onClick={() => { props.onDirtyChange?.(false); props.onClose(); }}>放弃节点输入并关闭</button></div>}
    {error && <p role="alert">{error} · 本地节点输入保留。</p>}
  </form>;
}
