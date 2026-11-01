import {spawn} from 'node:child_process';
import {createInterface} from 'node:readline';
import {fileURLToPath} from 'node:url';
export const root=new URL('../../../',import.meta.url),out=new URL('.work/p201-whole-20261017/',root);
export async function openCore(fixture='scenarios/图片时间轴/runtime/phase.input.json',executable='current-core.exe'){
 const child=spawn(fileURLToPath(new URL(executable,out)),[fileURLToPath(new URL('core-root/',out)),fixture],{windowsHide:true,stdio:['pipe','pipe','pipe']}),queue=[],records=[];let resolveReady,rejectReady,stderr='',serial=Promise.resolve();
 const initial=new Promise((resolve,reject)=>{resolveReady=resolve;rejectReady=reject;});child.stderr.on('data',chunk=>stderr+=chunk);
 const exited=new Promise(resolve=>child.on('exit',code=>{if(code){const error=new Error(stderr);rejectReady(error);for(const row of queue.splice(0))row.reject(error);}resolve(code);}));
 createInterface({input:child.stdout}).on('line',line=>{const value=JSON.parse(line);if(value.ready)resolveReady(value);else queue.shift()?.resolve(value);});
 const ready=await initial;
 const rpc=request=>{const pending=serial.then(()=>new Promise((resolve,reject)=>{queue.push({resolve:reply=>{records.push({request,reply});resolve(reply);},reject});child.stdin.write(JSON.stringify(request)+'\n');}));serial=pending;return pending;};
 return {ready,rpc,records,pid:child.pid,stderr:()=>stderr,async close(){child.stdin.end();return exited;}};
}

