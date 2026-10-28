import {readFile,writeFile} from 'node:fs/promises';
import {createRequire} from 'node:module';
import assert from 'node:assert/strict';
const root=new URL('../../../',import.meta.url),out=new URL('scenarios/Agent共建/runtime-public-render-consumer/',root);
const require=createRequire('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/package.json'),{chromium}=require('playwright');
const browser=await chromium.launch({headless:true,executablePath:'C:/Users/fujunye/AppData/Local/ms-playwright/chromium-1243/chrome-win64/chrome.exe'});
try {
  const page=await browser.newPage();
  const urls=await Promise.all(['supplement-accepted-gif-3-canvas.png','supplement-accepted-gif-5-canvas.png','supplement-accepted-png-canvas.png'].map(async name=>'data:image/png;base64,'+(await readFile(new URL(name,out))).toString('base64')));
  const result=await page.evaluate(async urls=>{
    const pixels=await Promise.all(urls.map(async url=>{const image=new Image();image.src=url;await image.decode();const canvas=document.createElement('canvas');canvas.width=image.width;canvas.height=image.height;const c=canvas.getContext('2d');c.drawImage(image,0,0);return {width:canvas.width,height:canvas.height,data:[...c.getImageData(0,0,canvas.width,canvas.height).data]};}));
    const [a,b,p]=pixels;let different=0;for(let i=0;i<a.data.length;i+=4)if(a.data.slice(i,i+4).some((value,j)=>value!==b.data[i+j]))different++;
    const colors=new Map();for(let i=0;i<p.data.length;i+=4){const rgba=p.data.slice(i,i+4).join(',');colors.set(rgba,(colors.get(rgba)??0)+1);}
    return {gif:{width:a.width,height:a.height,other_width:b.width,other_height:b.height,different_pixels:different,total_pixels:a.width*a.height},png:{width:p.width,height:p.height,distinct_rgba_colors:colors.size,most_common_colors:[...colors].sort((a,b)=>b[1]-a[1]).slice(0,8)}};
  },urls);
  assert.equal(result.gif.width,result.gif.other_width);assert.equal(result.gif.height,result.gif.other_height);assert.ok(result.gif.different_pixels>0);assert.ok(result.png.distinct_rgba_colors>1);
  await writeFile(new URL('actual-image-pixels.json',out),JSON.stringify({...result,source:'actual Host supplement captures3/5 GIF and4 PNG, real shared component canvas PNG; decoded screenshot pixels only, captions excluded',Native:false},null,2));
  console.log(JSON.stringify(result));
}finally{await browser.close();}
