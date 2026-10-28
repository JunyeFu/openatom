import {createFactoryPresentation,type FactoryPresentationPorts,type FactoryPresentationSource} from '../../services/factoryPresentation';
import {dispatch} from '../../services/projectHost';
import {readMediaAsset} from '../../services/mediaAssets';
import {readRunObservation} from '../../services/runObservation';
import {ProcessSourceChanged,ProcessUnavailable} from '../../services/presentationHost';
import {createFactoryItemLayer} from '../../scene/factoryItems';
import type {FactoryMediaInstance} from '../../scene/factoryMedia';
import type {DomainObject} from '../../../../../packages/commands/generated/typescript/DomainObject';
import type {ActualMediaFrame,PresentationScene} from '../../../../../packages/presentation/src';
import type {MediaPresentationCommonProps} from './types';
import type {ResolvedMedia} from './load';

export interface ProcessItems {
  source:FactoryPresentationSource;
  device_id:string;
  unavailable:string|null;
}
/** Read the same frozen Project and complete ObjectsAt population as the factory.
 * The view's held byte reader may be from an earlier seek, so use the canonical
 * Host ports at this frame's actual time instead of retargeting that reader. */
export async function readProcessItems(frame:ActualMediaFrame,binding:MediaPresentationCommonProps['binding'],
  current:()=>boolean,ports:FactoryPresentationPorts={dispatch,readAsset:readMediaAsset,readObservation:readRunObservation}):Promise<ProcessItems> {
  const assertCurrent=()=>{if(!current())throw new ProcessSourceChanged();};
  assertCurrent();
  if(binding.request.scope.kind!=='project')throw new ProcessUnavailable('DefinitionCycle没有正式采用设备的空间holder；不绘制推测附件。');
  if(binding.request.source.kind!=='run_module_version'||binding.request.source.data.run_id!==frame.run.id)
    throw new ProcessUnavailable('附件字节绑定不属于此实际Run。');
  const selected=frame.snapshot.objects.find(value=>value.kind==='job'&&value.data.id===frame.job_id);
  if(selected?.kind!=='job'||selected.data.execution_ref.kind!=='project_device')throw new ProcessUnavailable('此Job没有实际设备空间holder。');
  const view={run_id:frame.run.id,at_ms:frame.snapshot.at_ms};
  const project=await ports.dispatch<DomainObject>({api_version:'0.1',request_id:`process-items-${crypto.randomUUID()}`,
    project_id:binding.request.project_id,scope:binding.request.scope,operation:'get_object',params:{
      object_id:{kind:'project',id:binding.request.project_id},source:{kind:'run_snapshot',data:view},node_index:null}});
  assertCurrent();
  if(project.kind!=='project'||project.data.id!==frame.run.input_ref.project_id||project.data.revision!==frame.run.input_ref.project_revision)
    throw new ProcessUnavailable('物品附件没有取得本Run的冻结工程修订。');
  const context={session_id:binding.session_id,project_id:binding.request.project_id,scope:binding.request.scope,
    scope_revision:binding.request.scope_revision,project:project.data,view,preparedRun:{run:frame.run,modules:new Map()}};
  const source=await createFactoryPresentation(()=>current()?context:null,ports).read(context);
  assertCurrent();
  const device_id=selected.data.execution_ref.data.device_id,device=source.devices.find(value=>value.device_id===device_id);
  return {source,device_id,unavailable:device?.frame?.job_id===frame.job_id?null:
    '当前设备没有由所选Job唯一持有的实际姿态；不把其他Job的物品挂到此历史动作。'};
}

/** The factory attachment implementation owns every item and nested container.
 * The existing detail rig is already Y-up; its canonical nodes are the parents. */
export function createProcessItemLayer(input:ProcessItems,rig:PresentationScene,presentation:ResolvedMedia['presentation'],
  lod:MediaPresentationCommonProps['lod'],changed:()=>void) {
  const local=input.source.items.filter(value=>value.anchor_ref?.kind==='device'&&value.anchor_ref.id===input.device_id);
  const owner:Pick<FactoryMediaInstance,'root'|'nodes'|'presentation'>={root:rig.root,nodes:rig.nodes,presentation};
  const layer=createFactoryItemLayer(local,lod,id=>!input.unavailable&&id===input.device_id?owner:null,()=>null,changed);
  let live=true;
  return {reconcile:()=>layer.reconcile(),renderState:()=>layer.renderState(true),
    statuses:()=>Object.fromEntries(input.source.items.map(value=>[value.item.id,
      local.includes(value)?input.unavailable??layer.statuses()[value.item.id]:value.unavailable??'实际holder在此设备之外；本视图不绘制其几何。'])),
    dispose(){if(live){live=false;layer.dispose();}}};
}
