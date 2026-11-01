import fs from 'node:fs/promises';
import {supportedMechanisms} from './kit.mjs';
export const version='1.0.0';
const preserve=new Set(['pile-turner','paper-jogger','air-table-loader','suction-separator','paper-unwinder','paper-rewinder','label-peeler','board-pile-palletizer']);
const inspect=new Set(['label-web-inspector']);
export async function loadBatch(id){
 const {batch}=await import('./batches/'+id.toLowerCase()+'.mjs');
 const rows=batch.rows.trim().split(/\r?\n/),equipment=rows.map((row,i)=>{
  const [id,title,mechanisms,input,output,rule,auxiliary]=row.trim().split('|');
  const tags=mechanisms.split(',');for(const t of tags)if(!supportedMechanisms.includes(t))throw Error(id+' unauthored '+t);
  if(!input||!output)throw Error('Missing process in '+id);
  return {index:(Number(batch.id.slice(1))-1)*50+i+1,batch:batch.id,id,title,mechanisms:tags,flow:{input,output},rule:rule??(inspect.has(id)?'inspect':preserve.has(id)?'preserve':'transform'),auxiliary:auxiliary??null,cycle_seconds:12+tags.length*4,definition_id:'sf.native500.device.'+id,media_id:'sf.native500.media.'+id,version,basis:'原创示例设备；输入输出、尺寸和周期为模块声明，非厂商额定参数；时间与物品数量由业务内核运算。'};
 });
 if(equipment.length!==50||new Set(equipment.map(e=>e.id)).size!==50)throw Error(batch.id+' must contain 50 distinct device definitions');
 return {...batch,rows:undefined,equipment};
}
