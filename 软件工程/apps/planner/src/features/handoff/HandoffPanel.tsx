import { useState } from 'react';
import type { PackagePurpose } from '../../../../../packages/commands/generated/typescript/PackagePurpose';
import type { WorkspaceSnapshot } from '../../services/projectHost';
import type { DeliveryServices, DeliveryWorkspaceProps } from '../comparison/types';
import type { DeliveryTask } from '../reporting/useDeliveryTask';
import { IssueList, JsonDetails } from '../comparison/Shared';

export function HandoffPanel({ workspace, services, task, onOpenPackage, onEditModule }: {
  workspace: WorkspaceSnapshot; services: DeliveryServices; task: DeliveryTask;
  onOpenPackage: DeliveryWorkspaceProps['onOpenPackage']; onEditModule: DeliveryWorkspaceProps['onEditModule'];
}) {
  const project = workspace.project!;
  const [purpose, setPurpose] = useState<PackagePurpose>('handoff_project');
  const [scenarioIds, setScenarioIds] = useState<string[]>(workspace.scenarios.map(s => s.id));
  const [runIds, setRunIds] = useState<string[]>([]);
  const [moduleKeys, setModuleKeys] = useState<string[]>(project.module_refs.map(ref => JSON.stringify([ref.id, ref.version])));
  const [artifactIds, setArtifactIds] = useState<string[]>(project.artifact_refs);
  const [assets, setAssets] = useState(false);
  const modulePurpose = purpose === 'private_module' || purpose === 'community_module';
  const modules = project.module_refs.filter(ref => moduleKeys.includes(JSON.stringify([ref.id, ref.version])));
  const runs = (workspace.context.run_summaries ?? []).filter(r => r.input.project_id === project.id);
  const selectedRuns = runs.filter(r => runIds.includes(r.run_id));
  const problem = modulePurpose && modules.length !== 1 ? '模块包请选择一个精确版本；工程包可携带多个必要定义。' : purpose === 'community_module' && modules.some(ref => {
    const pkg = workspace.modules.get(JSON.stringify([ref.id, ref.version]));
    return !pkg || !pkg.definition.contribution.license || !pkg.definition.contribution.author;
  }) ? '社区贡献缺少已声明作者、代码许可或实际定义；请先在模块创作中补全。' : '';
  const toggle = (items: string[], id: string, checked: boolean) => checked ? [...items, id] : items.filter(value => value !== id);
  const output = task.result?.kind === 'package' ? task.result.data : null;
  const saved = task.result?.kind === 'save' ? task.result.data : null;
  return <section className="delivery-panel" aria-label="可编辑工程与贡献交接">
    <header><div><h2>交出别人能接着修改的工程</h2><p>工程、定义、允许携带的素材和实际结果分别选择。精确版本随包保留，不以本机最新版本替换缺失依赖。</p></div></header>
    <div className="delivery-actions"><button disabled={task.active || !services.startSave} onClick={() => services.startSave && void task.start(() => services.startSave!(project), 'save')}>另选目录保存工程</button><button disabled={task.active} onClick={onOpenPackage}>打开并审查工程包…</button></div>
    <p>{workspace.context.project?.dirty ? '当前工程有未保存改动。切换工程仍须由工作台处理保存／放弃／留在当前。' : '当前工程未标记未保存改动。'} 打开包先读取缺项与冲突，不直接替换活动工程。</p>
    {saved && <section aria-label="实际保存回执"><h3>宿主返回的保存结果</h3><p>已保存 {saved.project_id} r{saved.saved_revision}；当前 r{saved.current_revision}。{saved.still_dirty ? '保存期间仍有新编辑，工程仍未全部保存。' : '宿主确认该保存终态没有剩余未保存修订。'}</p><p>目标选择 {saved.destination.selection_id} · 清单 <code>{saved.manifest.path}</code></p></section>}
    <label className="delivery-select">交付用途<select value={purpose} onChange={e => setPurpose(e.target.value as PackagePurpose)}><option value="handoff_project">可编辑工程交接包</option><option value="private_project">个人工程归档包</option><option value="community_module">社区模块贡献包</option><option value="private_module">个人模块包</option></select></label>
    <fieldset><legend>定义与精确版本</legend>{project.module_refs.map(ref => {
      const key = JSON.stringify([ref.id, ref.version]); const pkg = workspace.modules.get(key);
      return <article className="delivery-module" key={key}><label><input type="checkbox" checked={moduleKeys.includes(key)} onChange={e => setModuleKeys(toggle(moduleKeys, key, e.target.checked))} /><strong>{ref.id} · {ref.version}</strong></label>{pkg ? <><p>作者：{pkg.definition.contribution.author ?? '未知'} · 代码许可：{pkg.definition.contribution.license ?? '未知'}</p><IssueList issues={pkg.issues} /><ul>{pkg.rights.map(r => <li key={r.asset_ref}>{r.asset_ref} · {r.source ?? '来源未知'} · {r.license ?? '许可未知'} · {r.redistribution_confirmed ? '已声明可再分发' : '未确认可再分发'}{r.missing_reason && ` · ${r.missing_reason}`}</li>)}</ul></> : <p className="delivery-error">缺少实际定义，不能用同名或新版替换。</p>}{onEditModule && <button disabled={task.active} onClick={() => onEditModule(ref)}>查看／补全模块元数据</button>}</article>;
    })}{!project.module_refs.length && <p>当前工程没有采用定义。</p>}</fieldset>
    {!modulePurpose && <><fieldset><legend>可编辑方案</legend>{workspace.scenarios.map(s => <label key={s.id}><input type="checkbox" checked={scenarioIds.includes(s.id)} onChange={e => setScenarioIds(toggle(scenarioIds, s.id, e.target.checked))} />{s.title} · {s.id} r{s.revision}</label>)}</fieldset>
      <fieldset><legend>所选历史实际结果（仍归原冻结输入）</legend>{runs.map(r => <label key={r.run_id}><input type="checkbox" checked={runIds.includes(r.run_id)} onChange={e => setRunIds(toggle(runIds, r.run_id, e.target.checked))} />{r.run_id} · r{r.input.project_revision} / {r.input.scenario_id} r{r.input.scenario_revision} · {r.whole_window_computed ? '完整窗口' : '仅前缀'}</label>)}{!runs.length && <p>{workspace.context.run_summaries === undefined ? '宿主未提供运行列表；不能据此认定没有历史结果。' : '没有实际Run；本包是可编辑设计，不包含虚构产量。'}</p>}</fieldset>
      <fieldset><legend>实际专业成果引用</legend>{project.artifact_refs.map(id => <label key={id}><input type="checkbox" checked={artifactIds.includes(id)} onChange={e => setArtifactIds(toggle(artifactIds, id, e.target.checked))} />{id}</label>)}{!project.artifact_refs.length && <p>未引用专业成果。</p>}</fieldset>
      <JsonDetails label="已有用途认可与工位绑定（由宿主检查当前有效性）" value={{ acknowledgments: project.acknowledgments, bindings: project.bindings }} />
    </>}
    <label><input type="checkbox" checked={assets} onChange={e => setAssets(e.target.checked)} />确认并携带已允许再分发的素材</label><p>本次勾选确认所选素材已获再分发许可，不授予新的许可。宿主仍核对实际素材内容、来源与许可；不满足条件的条目保留引用并列为缺项。</p>
    <IssueList issues={project.issues} /><IssueList issues={workspace.moduleIssues.flatMap(i => [{ code: i.error.code, message: `${i.ref.id}@${i.ref.version}: ${i.error.message}`, blocks: [] }])} />
    <section className="delivery-hint"><h3>接手后的运行方式</h3><p>设计包可以打开、修改并从初始输入运行；带实际结果的工程包可以读取原Run与其冻结输入。首版不承诺冷启动续跑，不要求用完整快照才能交付工程。</p><p>报告JSON不具备工程重开能力。必要行为或专业结果缺失时，接收者可以保留草稿，但不能据此声称模拟完整；表现素材的替换应明确记录。</p></section>
    {problem && <p role="alert" className="delivery-error">{problem}</p>}<button className="primary" disabled={task.active || !!problem} onClick={() => void task.start(() => services.startPackage({ revision: project.revision, package_scope: { purpose, scenarios: modulePurpose ? [] : workspace.scenarios.filter(s => scenarioIds.includes(s.id)).map(s => s.id), run_ids: modulePurpose ? [] : selectedRuns.map(r => r.run_id), artifact_ids: modulePurpose ? [] : project.artifact_refs.filter(id => artifactIds.includes(id)), module_refs: modules, include_redistributable_assets: assets } }), 'package')}>选择位置并导出ZIP包</button>
    {output && <section><h3>上次任务实际交付结果</h3><p>目标选择 {output.destination.selection_id}；工程包 <code>{output.package.path}</code></p><ul>{output.included_files.map(f => <li key={f.path}><code>{f.path}</code> · {f.kind}</li>)}</ul><h4>保留的缺项</h4>{output.missing.length ? <IssueList issues={output.missing} /> : <p>本次宿主结果未列出缺项。</p>}<button disabled={task.active} onClick={onOpenPackage}>在另一位置打开并检查</button></section>}
  </section>;
}
