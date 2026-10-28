import { useEffect, useRef, useState } from 'react';
import { nodeEditLabel, nodeEditTarget, nodeSelectionKey } from './model';
import type { NodeEditRequest, NodeInspection, ProcessNodeSelection } from './types';
import './node-inspector.css';

export interface NodeInspectorProps {
  selection: ProcessNodeSelection;
  read(selection: ProcessNodeSelection): Promise<NodeInspection>;
  onEditSource?(request: NodeEditRequest): void | Promise<void>;
  onClose(): void;
}
/** Selection is observational; only the explicit edit button requests navigation. */
export function NodeInspector(props: NodeInspectorProps) {
  const key = nodeSelectionKey(props.selection), current = useRef(props); current.current = props;
  const [loaded, setLoaded] = useState<{ key: string; value: NodeInspection } | null>(null), [failure, setFailure] = useState(''), [acting, setActing] = useState(false);
  useEffect(() => {
    let live = true; setLoaded(null); setFailure('');
    current.current.read(props.selection).then(value => { if (live && nodeSelectionKey(value.selection) === key) setLoaded({ key, value }); }, reason => { if (live) setFailure(reason instanceof Error ? reason.message : String(reason)); });
    return () => { live = false; };
  }, [key]);
  const value = loaded?.key === key ? loaded.value : null, target = value ? nodeEditTarget(value.selection) : null, selection = props.selection;
  async function edit() {
    if (!value || !target || !props.onEditSource || acting) return;
    setActing(true); setFailure('');
    try { await props.onEditSource({ ...value, target }); }
    catch (reason) { if (nodeSelectionKey(current.current.selection) === key) setFailure(reason instanceof Error ? reason.message : String(reason)); }
    finally { setActing(false); }
  }
  return <aside className="sf-node-inspector" aria-label="精确节点检查">
    <header><h3>精确节点检查</h3><button onClick={props.onClose}>关闭节点检查</button></header>
    {(selection.context.selection.kind === 'frozen_device' || selection.context.selection.kind === 'frozen_definition') && <p>冻结静态输入 · 声明局部变换，不是运动后的世界姿态。此检查没有实际Job、阶段、有效进度或端口产量。</p>}
    <dl><dt>直接拥有者</dt><dd>{selection.owner.object_id.kind} · {selection.owner.object_id.id}</dd><dt>节点元数据来源</dt><dd><code>{JSON.stringify(selection.owner.source)}</code></dd>
      <dt>会话 / 工程</dt><dd>{selection.context.session_id} / {selection.context.project_id}</dd><dt>授权 Scope 修订</dt><dd>{selection.context.scope.kind} · r{selection.context.scope_revision}</dd>
      <dt>档位 / canonical 数组位置</dt><dd>{selection.lod} / {selection.node_index}</dd><dt>素材</dt><dd>{selection.asset ? `${selection.asset.id} · ${selection.asset.path}` : '无独立素材（合法原生场景）'}</dd><dt>字节读取基准</dt><dd><code>{JSON.stringify(selection.byte_source)}</code></dd></dl>
    {!value && !failure && <p role="status">正在通过原Host读取此精确拥有者的节点…</p>}
    {value && <><p>已读取Host精确源节点。Run来源返回冻结静态输入中的声明，不是运动后的世界位姿；检查不修改表现、观看时点或未应用输入。</p><dl><dt>节点 id</dt><dd>{value.node.id}</dd><dt>名称</dt><dd>{value.node.name}</dd><dt>父节点 canonical 位置</dt><dd>{value.node.parent_index ?? '无父节点'}</dd></dl>
      {(['transform','geometry','material'] as const).map(field => <details key={field} open><summary>{{transform:'源节点声明局部变换',geometry:'源节点几何',material:'源节点材质'}[field]}</summary><pre>{JSON.stringify(value.node[field], null, 2)}</pre></details>)}
      {target && props.onEditSource ? <button disabled={acting} onClick={() => { void edit(); }}>{nodeEditLabel(target)}</button> : <p>此节点已可检查；当前来源的正常编辑入口尚未接通。</p>}
      {target?.frozen && <p>此Run保持冻结；明确编辑会从该精确来源创作新版本，之后仍须明确采用。</p>}
    </>}
    {failure && <p role="alert">{failure}</p>}
  </aside>;
}
