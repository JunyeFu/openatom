import {createServer} from 'vite';
import fs from 'node:fs/promises';
import path from 'node:path';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
const root=process.cwd(),out=path.join(root,'scenarios/媒体表现/model-quality-20261022');
await fs.mkdir(out,{recursive:true});
const ffmpeg='D:/MathModelingTools/envs/cumcm/Lib/site-packages/imageio_ffmpeg/binaries/ffmpeg-win-x86_64-v7.1.exe';
let video=null,frames=0,log='',activeFamily=null;
// This asset viewer is reloaded after exports; recursively watching the growing
// repository prevented even its initial HTTP request from completing.
const server=await createServer({root,configFile:false,cacheDir:path.join(out,'vite-cache'),optimizeDeps:{noDiscovery:true,include:[]},server:{host:'127.0.0.1',port:19289,strictPort:true,watch:null},plugins:[{
 name:'actual-glb-capture',configureServer(s){s.middlewares.use(async(req,res,next)=>{
  if(!req.url.startsWith('/quality-api/'))return next();
  const url=new URL(req.url,'http://127.0.0.1:19289');
  const send=(value)=>{res.setHeader('Content-Type','application/json');res.end(JSON.stringify(value));};
  try{
   if(url.pathname==='/quality-api/status')return send({activeFamily,frames,video_active:!!video});
   const data=Buffer.concat(await Array.fromAsync(req));
   if(url.pathname==='/quality-api/save'){
    const name=url.searchParams.get('name');if(!/^[a-z0-9_-]+\/[a-z0-9_.-]+$/.test(name))throw Error('Unknown capture destination');
    const target=path.join(out,name);await fs.mkdir(path.dirname(target),{recursive:true});await fs.writeFile(target,data);return send({saved:name});
   }
   if(url.pathname==='/quality-api/video-start'){
    if(video)throw Error('One capture is already active');
    activeFamily=url.searchParams.get('family');if(!/^[a-z0-9-]+$/.test(activeFamily))throw Error('Unknown model');
    await fs.mkdir(path.join(out,activeFamily),{recursive:true});frames=0;log='';
    video=spawn(ffmpeg,['-hide_banner','-loglevel','error','-y','-f','image2pipe','-vcodec','png','-framerate','24','-i','pipe:0','-an','-c:v','libx264','-preset','fast','-crf','18','-pix_fmt','yuv420p',path.join(out,activeFamily,'motion.mp4')],{windowsHide:true,stdio:['pipe','ignore','pipe']});
    video.stderr.on('data',b=>log+=b.toString());return send({started:activeFamily});
   }
   if(url.pathname==='/quality-api/video-frame'){
    if(!video)throw Error('No active capture');if(!video.stdin.write(data))await once(video.stdin,'drain');frames++;return send({frames});
   }
   if(url.pathname==='/quality-api/video-finish'){
    const process=video;const done=once(process,'close');process.stdin.end();const [exit]=await done;video=null;
    if(exit!==0)throw Error('Video encoding failed: '+log);return send({family:activeFamily,frames});
   }
   throw Error('Unknown capture operation');
  }catch(error){res.statusCode=500;send({error:String(error)});}
 });}
}]});
await server.listen();console.log('Actual GLB review http://127.0.0.1:19289/tools/model-quality-review/v042/index.html');
process.on('SIGINT',async()=>{video?.stdin.end();await server.close();process.exit(0);});
