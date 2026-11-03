import { factoryUnitLabel, factoryItemTypeTitle, factoryItemTitle, factoryItemPlace, factoryResourceTitle, factoryLifecycleLabel, factoryQualityLabel } from '../../scene/factoryLabels';
import { useMemo, useState } from 'react';
import type { BusinessEditorProps } from './types';
import { frozenProjectMatches, observedItems, quantityText as rawQuantityText, reasonNames } from '../timeline/projection';
import { timeText } from '../timeline/time';
import { runStatusLabel } from '../../services/workbenchLabels';
export function BusinessRuntime(props:BusinessEditorProps){
  const [filter,setFilter]=useState(''),run=props.run;
  const items=useMemo(()=>observedItems(props.observation,run??null,props.atMs??null),[props.observation,run,props.atMs]);
  if(!run)return <><h2>实际接受与等待</h2><p>还没有运行记录。先应用输入，再点击“运行当前方案”；运行后在这里查看接收数量与等待。</p></>;
  const quantityText=(quantity:Parameters<typeof rawQuantityText>[0])=>rawQuantityText({...quantity,unit:factoryUnitLabel(quantity.unit)});
  const frozen=frozenProjectMatches(props.frozenProject??null,run)?props.frozenProject:null;
  const objectName=(id:string)=>{
    const order=props.frozenScenario?.orders.find(value=>value.id===id);
    if(order){const product=frozen?.products.find(value=>value.id===order.product_ref.id);return product?factoryItemTypeTitle(frozen??null,product.item_type_ref):'订单';}
    if(frozen?.resources.some(value=>value.id===id))return factoryResourceTitle(frozen,id);
    const itemType=frozen?.item_types.find(value=>value.id===id);if(itemType)return factoryItemTypeTitle(frozen??null,itemType);
    for(const values of [frozen?.devices,frozen?.locations]){const value=values?.find(value=>value.id===id);if(value)return value.title;}
    const job=run.jobs.find(value=>value.id===id);
    if(job?.execution_ref.kind==='project_device'){const deviceId=job.execution_ref.data.device_id;return frozen?.devices.find(value=>value.id===deviceId)?.title??'设备作业';}
    const trip=run.transport.find(value=>value.id===id);if(trip)return `${factoryResourceTitle(frozen??null,trip.carrier_id)}的搬运作业`;
    return '生产作业';
  };
  const match=(id:string)=>[id,objectName(id)].some(value=>value.toLowerCase().includes(filter.toLowerCase()));
  const locate=(kind:Parameters<BusinessEditorProps['onLocate']>[0]['object']['kind'],id:string,at=props.atMs??run.computed_until_ms??run.window.start_ms)=>props.onLocate({object:{kind,id},source:'run',run_id:run.id,at_ms:at});
  return <><h2>实际接受与等待 · 只读</h2><p>{props.frozenScenario?.title || '所选历史方案'} · 运行结果</p><details><summary>运行来源详情</summary><p>运行编号：{run.id} · 来源：{run.evidence_origin} · 工程版本：{run.input_ref.project_revision} · 方案版本：{run.input_ref.scenario_revision}</p></details><p>观察窗 {timeText(run.window.start_ms)}—{timeText(run.window.end_ms)}；已计算至 {run.computed_until_ms==null?'未知':timeText(run.computed_until_ms)}。未完成不记作0秒。</p>
    {!frozenProjectMatches(props.frozenProject??null,run)&&<p role="alert">未提供匹配的冻结工程；下列结果保留原ID，不用当前草稿重解释历史。</p>}
    <p>{runStatusLabel[run.computation_status]} · 接受目标 {run.goal_met===true?'已达到':run.goal_met===false?'尚未达到':'未评价'}；计算结束不等于订单履行完成。</p>
    <input aria-label="查找实际身份" placeholder="按订单、设备名称或编号查找" value={filter} onChange={e=>setFilter(e.target.value)}/>
    <h3>整窗实际订单结果（区别于当前时点库存）</h3><table><thead><tr><th>订单</th><th>需求/接受/未满足</th><th>完工时刻 / 迟交时长（秒）</th></tr></thead><tbody>{run.orders.filter(o=>match(o.order_id)).map(o=><tr key={o.order_id}><td><button onClick={()=>locate('order',o.order_id)}>{objectName(o.order_id)}</button><details><summary>订单编号</summary><code>{o.order_id}</code></details></td><td>{quantityText(o.demand)} / {quantityText(o.accepted)} / {quantityText(o.outstanding)}</td><td>{o.completion_ms==null?'未完成':timeText(o.completion_ms)} / {o.tardiness_ms==null?'未知':timeText(o.tardiness_ms)}</td></tr>)}</tbody></table>
    <h3>等待条件与相关对象</h3>{run.waits.filter(w=>match(w.object_ref.id)||w.reasons.some(r=>match(r.blocking_object?.id??''))).slice(0,100).map((w,i)=><article className="sf-business-readonly" key={i}><button onClick={()=>locate(w.object_ref.kind,w.object_ref.id,w.interval.start_ms)}>{objectName(w.object_ref.id)} · {timeText(w.interval.start_ms)}—{timeText(w.interval.end_ms)}</button><details><summary>等待技术详情</summary><pre>{JSON.stringify(w,null,2)}</pre></details>{w.reasons.map((reason,j)=><p key={j}>{reasonNames[reason.kind]} · 下一已知事件 {reason.next_known_boundary_ms==null?'未安排/未知':timeText(reason.next_known_boundary_ms)} {reason.blocking_object&&<button onClick={()=>locate(reason.blocking_object!.kind,reason.blocking_object!.id,w.interval.start_ms)}>定位 {objectName(reason.blocking_object.id)}</button>}</p>)}</article>)}
    <h3>真实接受事件</h3>{run.acceptance_events.map((event,i)=><p key={i}>{timeText(event.at_ms)} · {quantityText(event.quantity)} · {event.acceptance_stage} · <button onClick={()=>locate('location',event.destination,event.at_ms)}>{event.destination}</button> · {event.order_id??'无订单接受目标'}</p>)}
    <h3>观察时点物品</h3><p>{props.atMs==null?'未选择实际时点':timeText(props.atMs)} · {items===null?'完整同Run时点数据未齐，不使用末态库存填补':`${items.length}条真实身份记录；输入变化不会修改这些物品`}</p>{items?.filter(i=>match(i.id)||match(i.holder.id)).slice(0,100).map(i=><article className="sf-business-readonly" key={i.id}><button onClick={()=>locate('item',i.id)}>{factoryItemTitle(frozen??null,i)}</button><p>{quantityText(i.quantity)} · {factoryLifecycleLabel[i.lifecycle]} / {factoryQualityLabel[i.quality]} · {factoryItemPlace(frozen??null,i,run,items??[])}</p><details><summary>物品身份与位置详情</summary><p>{i.id} · {i.holder.kind} / {i.holder.id}</p></details></article>)}
    <h3>真实资源持有</h3>{run.resource_claims.filter(c=>match(c.resource_id)||match(c.job_id)).slice(0,100).map(c=><article className="sf-business-readonly" key={c.id}><button onClick={()=>locate('resource',c.resource_id,c.intervals[0]?.start_ms)}>{objectName(c.resource_id)}</button><p>占用 {c.units} 容量 · {objectName(c.job_id)} · {c.phase_ref?'阶段占用':'全作业占用'} · {c.intervals.map(i=>`${timeText(i.start_ms)}—${timeText(i.end_ms)}`).join('；')}</p><details><summary>资源占用技术详情</summary><pre>{JSON.stringify(c,null,2)}</pre></details></article>)}
    <h3>来源与待补</h3>{run.limitations.map((issue,i)=><p key={i}>{issue.message}{issue.object_ref&&<button onClick={()=>locate(issue.object_ref!.kind,issue.object_ref!.id)}>定位缺项</button>}</p>)}<p>成本输入缺项保持未知，不能据时间结果补出价格。完整评估从同源报告面板读取。</p>
    <p>身份、等待和资源列表每次显示最多100项；按完整ID筛选可定位其余记录，未截断原Run。</p>
  </>;
}
