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
  return <aside className="sf-node-inspector" aria-label="节点属性">
    <header><h3>节点属性</h3><button onClick={props.onClose}>关闭节点检查</button></header>
    {(selection.context.selection.kind === 'frozen_device' || selection.context.selection.kind === 'frozen_definition') && <p>历史只读：显示该次运行开始时保存的局部位置，不随播放变化。</p>}
    <details><summary>来源与技术信息</summary><dl><dt>所属对象</dt><dd>{selection.owner.object_id.kind} · {selection.owner.object_id.id}</dd><dt>节点来源</dt><dd><code>{JSON.stringify(selection.owner.source)}</code></dd>
      <dt>会话 / 工程</dt><dd>{selection.context.session_id} / {selection.context.project_id}</dd><dt>授权范围版本</dt><dd>{selection.context.scope.kind} · r{selection.context.scope_revision}</dd>
      <dt>精度档位 / 节点索引</dt><dd>{selection.lod} / {selection.node_index}</dd><dt>素材</dt><dd>{selection.asset ? `${selection.asset.id} · ${selection.asset.path}` : '无独立素材'}</dd><dt>素材读取来源</dt><dd><code>{JSON.stringify(selection.byte_source)}</code></dd></dl></details>
    {!value && !failure && <p role="status">正在加载节点…</p>}
    {value && <><dl><dt>节点编号</dt><dd>{value.node.id}</dd><dt>名称</dt><dd>{value.node.name}</dd><dt>父节点索引</dt><dd>{value.node.parent_index ?? '无父节点'}</dd></dl>
      {(['transform','geometry','material'] as const).map(field => <details key={field}><summary>{{transform:'局部位置与朝向',geometry:'几何信息',material:'材质信息'}[field]}</summary><pre>{JSON.stringify(value.node[field], null, 2)}</pre></details>)}
      {target && props.onEditSource ? <button disabled={acting} onClick={() => { void edit(); }}>{nodeEditLabel(target)}</button> : <p>此节点目前仅供查看。</p>}
      {target?.frozen && <p>编辑会创建新版本，已有运行记录不变。</p>}
    </>}
    {failure && <p role="alert">{failure}</p>}
  </aside>;
}
