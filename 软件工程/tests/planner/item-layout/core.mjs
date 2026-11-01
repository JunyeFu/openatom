import {spawn} from 'node:child_process';
import {createInterface} from 'node:readline';
import {fileURLToPath} from 'node:url';
export const root=new URL('../../../',import.meta.url),out=new URL('scenarios/媒体表现/runtime-item-layout-consumer/',root);
export async function openCore(fixture='scenarios/媒体表现/runtime-factory-items/root-input.json'){
 const child=spawn(fileURLToPath(new URL('core_consumer.exe',out)),[fileURLToPath(root),fixture],{windowsHide:true,stdio:['pipe','pipe','pipe']}),queue=[],records=[];let resolveReady,rejectReady,stderr='',serial=Promise.resolve();
 const initial=new Promise((resolve,reject)=>{resolveReady=resolve;rejectReady=reject;});child.stderr.on('data',chunk=>stderr+=chunk);
 const exited=new Promise(resolve=>child.on('exit',code=>{if(code){const error=new Error(stderr);rejectReady(error);for(const row of queue.splice(0))row.reject(error);}resolve(code);}));
 createInterface({input:child.stdout}).on('line',line=>{const value=JSON.parse(line);if(value.ready)resolveReady(value);else queue.shift()?.resolve(value);});
 const ready=await initial;
 const rpc=request=>{const pending=serial.then(()=>new Promise((resolve,reject)=>{queue.push({resolve:reply=>{records.push({request,reply});resolve(reply);},reject});child.stdin.write(JSON.stringify(request)+'\n');}));serial=pending;return pending;};
 return {ready,rpc,records,stderr:()=>stderr,async close(){child.stdin.end();return exited;}};
}

