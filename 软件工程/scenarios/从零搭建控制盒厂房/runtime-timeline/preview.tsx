// Isolated component consumer: data + seek come from two actual existing Engines.
// Commit responses below are explicitly test doubles, never Host acceptance.
import React, { useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { RunTimeline } from '../../../apps/planner/src/features/timeline/RunTimeline';
import type { TimelineInputCommit, TimelineSelection } from '../../../apps/planner/src/features/timeline/types';
import type { RunObjectsPage } from '../../../packages/commands/generated/typescript/RunObjectsPage';
const [baseline, delayed, prefix] = await Promise.all(['baseline', 'delayed-pcb', 'baseline-prefix'].map(name => fetch(`./${name}.actual.json`).then(r => r.json())));
const evidence = { seeks: [] as object[], commits: [] as TimelineInputCommit[], selected: null as TimelineSelection | null, descriptionVisible: false };
(window as unknown as { __timelineEvidence: typeof evidence }).__timelineEvidence = evidence;
function Preview() {
  const [record, setRecord] = useState(delayed), [at, setAt] = useState(600000), [observation, setObservation] = useState<RunObjectsPage>(delayed.observations.at(-1));
  const [scenario, setScenario] = useState(delayed.scenario), [project, setProject] = useState(delayed.project), [selected, setSelected] = useState<TimelineSelection | null>(null);
  const [description, setDescription] = useState(false), [visible, setVisible] = useState(true), [reject, setReject] = useState(false), epoch = useRef(0);
  async function seek(request: { run_id: string; at_ms: number }) {
    const token = ++epoch.current; evidence.seeks.push(request);
    const objects = await fetch(`/__p204_engine_observation?run_id=${encodeURIComponent(request.run_id)}&at_ms=${request.at_ms}`).then(r => r.json());
    if (token !== epoch.current || objects.run_id !== request.run_id || objects.at_ms !== request.at_ms || objects.next_index !== null) return;
    const next = request.run_id === baseline.run.id ? baseline : delayed;
    if (next !== record) { setRecord(next); setScenario(next.scenario); setProject(next.project); setSelected(null); }
    setAt(request.at_ms); setObservation(objects);
  }
  return <main style={{ font:'14px Segoe UI, Microsoft YaHei, sans-serif', color:'#233747', background:'#edf3f7', height:'100vh', display:'flex', flexDirection:'column' }}>
    <header style={{padding:12,background:'#dfeaf2',borderBottom:'1px solid #a9bdcb'}}><strong>超级工厂 · P2-04 真实记录组件核验</strong> · 独立 Engine 消费者；输入提交为明确测试替身</header>
    <nav style={{padding:8,display:'flex',gap:8,flexWrap:'wrap'}}><button onClick={() => { const next=record===baseline?delayed:baseline; ++epoch.current; setRecord(next);setAt(600000);setObservation(next.observations.at(-1));setScenario(next.scenario);setProject(next.project);setSelected(null); }}>切换真实 Run</button><button onClick={() => { ++epoch.current;setRecord(prefix);setScenario(prefix.scenario);setProject(prefix.project);setAt(prefix.run.computed_until_ms);setObservation(prefix.observations[0]);setSelected(null); }}>读取真实计算前缀</button><button onClick={() => { setDescription(!description); evidence.descriptionVisible=!description; }}>显示 / 隐藏描述</button><button onClick={() => setVisible(!visible)}>显示 / 隐藏时间轴</button><label><input type="checkbox" checked={reject} onChange={e => setReject(e.target.checked)} />测试输入冲突响应</label><label>面板高度 <select aria-label="测试面板高度" defaultValue="320" onChange={e => document.documentElement.style.setProperty('--preview-height', `${e.target.value}px`)}><option>160</option><option>320</option></select></label></nav>
    <section style={{flex:1,padding:20}}><h2>同一结果 · 同一绝对时钟</h2><p>实际 Run：<output data-testid="run">{record.run.id}</output> · 共享时点 <output data-testid="at">{at}</output> ms</p><p>事件 {record.run.events.length} · 作业 {record.run.jobs.length} · 运输 {record.run.transport.length} · 真实接受终点 {(record.run.acceptance_events.length ? Math.max(...record.run.acceptance_events.map((e:any)=>e.at_ms)) : "尚未发生")} ms</p><p>时点物品 {observation.objects?.length ?? '未知'}；所有拖动、折叠与显示切换只读生产状态。</p><p>本页证明同一组件直接消费真实 Engine 记录与 ObjectsAt；完整 Native Workbench、双画布和用户验收由产品接线另验。</p></section>
    <div style={{height:'var(--preview-height,320px)',display:'flex',borderTop:'2px solid #97afc0',minHeight:120}}>{description && <aside style={{width:300,minWidth:240,overflow:'auto',padding:8,background:'#fff'}} aria-label="测试共享描述"><strong>{selected?.label ?? '尚未选择'}</strong>{selected?.fields.map((f,i)=><p key={i}>{f.label}：{f.value}</p>)}</aside>}{visible && <RunTimeline project={record.project} draftProject={project} run={record.run} runScenario={record.scenario} scenario={scenario} observation={observation} atMs={at} selected={selected} comparison={{project:(record===baseline?delayed:baseline).project,run:(record===baseline?delayed:baseline).run,scenario:(record===baseline?delayed:baseline).scenario}}
      onTime={at_ms=>void seek({run_id:record.run.id,at_ms})} onSeek={r=>void seek(r)} onSelect={()=>{}} onInspect={s=>{setSelected(s);evidence.selected=s;}}
      onCommitInput={async p=>{ evidence.commits.push(p); if(reject || p.base_project_revision!==project.revision || p.base_scenario_revision!==scenario.revision)return {ok:false,error:{code:'RevisionConflict',message:'明确测试替身：修订已变化',object_refs:[],field_errors:[],expected_revision:p.base_project_revision,actual_revision:project.revision+1}} as any;
        const next={...p.command.data.data,revision:scenario.revision+1,project_revision:project.revision+1};setScenario(next);setProject({...project,revision:project.revision+1});return {ok:true,scenario:next}; }} />}</div>
  </main>;
}
const previewHost = window as unknown as { __p204ReactRoot?: ReturnType<typeof createRoot> };
previewHost.__p204ReactRoot ??= createRoot(document.getElementById('root')!);
previewHost.__p204ReactRoot.render(<Preview />);
