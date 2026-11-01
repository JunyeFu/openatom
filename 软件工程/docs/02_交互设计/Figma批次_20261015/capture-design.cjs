// Extract editable layout and discrete 3D content from the authorized design prototype.
const fs=require('node:fs'),path=require('node:path');
const {chromium}=require('C:/Users/fujunye/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const out=__dirname;
function saved(id){const p=path.join(out,id+'.json');if(!fs.existsSync(p))return null;const d=JSON.parse(fs.readFileSync(p));return {id:d.id,title:d.title,nodes:d.nodes.length,scene:!!d.scene,svgs:d.svgs.length};}
async function extract(page,id,title){
  const data=await page.evaluate(({id,title})=>{
    const nodes=[];const selectors='header,nav,.context,#library,#center,#properties,#tools,footer,dialog[open],.menu';
    const regions=[...document.querySelectorAll(selectors)].filter(el=>{const r=el.getBoundingClientRect();return r.width&&r.height&&getComputedStyle(el).display!=='none';});
    const alpha=c=>c&&c!=='rgba(0, 0, 0, 0)'&&c!=='transparent';
    const rect=e=>{const r=e.getBoundingClientRect();return {x:r.x,y:r.y,w:r.width,h:r.height};};
    const visible=r=>r.w>0&&r.h>0&&r.x<1920&&r.y<1080&&r.x+r.w>0&&r.y+r.h>0;
    const regionName=e=>e.id||e.className||e.tagName;
    const excluded=e=>e.matches('script,style,link,input[type=file],canvas,svg')||e.closest('svg');
    function walk(el,region){
      const cs=getComputedStyle(el),r=rect(el);if(cs.display==='none'||cs.visibility==='hidden'||!visible(r)||excluded(el))return;
      if(el!==region&&regions.includes(el))return;
      const tag=el.tagName,action=el.dataset.action||el.dataset.menu&&('menu:'+el.dataset.menu)||el.dataset.category!==undefined&&('category:'+el.dataset.category)||el.dataset.regionMember&&('member:'+el.dataset.regionMember)||null;
      const base={...r,region:regionName(region),name:el.id||el.getAttribute('aria-label')||action||tag.toLowerCase(),bg:alpha(cs.backgroundColor)?cs.backgroundColor:null,border:parseFloat(cs.borderTopWidth)>0?cs.borderTopColor:null,bw:parseFloat(cs.borderTopWidth)||0,radius:parseFloat(cs.borderTopLeftRadius)||0,color:cs.color,size:parseFloat(cs.fontSize),bold:parseInt(cs.fontWeight)>=600,align:cs.textAlign,action,disabled:!!el.disabled};
      if(tag==='BUTTON'||tag==='INPUT'||tag==='SELECT'||tag==='TEXTAREA'||tag==='SUMMARY'){
        let text=tag==='INPUT'?(el.type==='checkbox'?(el.checked?'☑':'☐'):el.type==='range'?'━━━━●━━━━':el.value||el.placeholder):tag==='SELECT'?el.selectedOptions[0]?.textContent:el.value||el.innerText;
        nodes.push({...base,type:tag==='BUTTON'||tag==='SUMMARY'?'button':'input',text:String(text||'').trim(),name:el.getAttribute('aria-label')||base.name});return;
      }
      if(base.bg||base.border)nodes.push({...base,type:'box'});
      if(tag==='SVG')return;
      for(const n of el.childNodes){
        if(n.nodeType===Node.TEXT_NODE&&n.textContent.trim()){
          const range=document.createRange();range.selectNodeContents(n);const rr=range.getBoundingClientRect();let clipped=false;
          for(let p=el;p&&p!==document.body;p=p.parentElement){const s=getComputedStyle(p);if(/hidden|auto|scroll/.test(s.overflow+s.overflowX+s.overflowY)){const q=p.getBoundingClientRect();if(rr.y>=q.bottom||rr.bottom<=q.top||rr.x>=q.right||rr.right<=q.left)clipped=true;}}
          if(!clipped&&visible({x:rr.x,y:rr.y,w:rr.width,h:rr.height}))nodes.push({...base,type:'text',text:n.textContent.trim(),x:rr.x,y:rr.y,w:rr.width,h:rr.height,bg:null,border:null});
        }else if(n.nodeType===Node.ELEMENT_NODE)walk(n,region);
      }
    }
    for(const region of regions)walk(region,region);
    const canvas=document.querySelector('#scene canvas');let scene=null;
    if(canvas&&getComputedStyle(document.querySelector('#scene')).display!=='none'&&getComputedStyle(document.querySelector('#task-workspace')).display==='none')scene=rect(canvas);
    const svgs=[...document.querySelectorAll('#task-workspace svg')].filter(e=>visible(rect(e))).map(e=>{const clone=e.cloneNode(true),src=[e,...e.querySelectorAll('*')],dst=[clone,...clone.querySelectorAll('*')];src.forEach((n,i)=>{const s=getComputedStyle(n);for(const k of ['fill','stroke','stroke-width','stroke-dasharray','font-size'])dst[i].setAttribute(k,s.getPropertyValue(k));if(n.tagName.toLowerCase()==='text')dst[i].setAttribute('font-family','Noto Sans SC');});return {...rect(e),svg:new XMLSerializer().serializeToString(clone)};});
    return {id,title,width:1920,height:1080,timelineViewport:document.querySelector('#timeline .time-body')?rect(document.querySelector('#timeline .time-body')):null,regions:regions.map(e=>({...rect(e),name:regionName(e)})),nodes,scene,svgs};
  },{id,title});
  if(data.scene){await page.locator('#scene canvas').screenshot({path:path.join(out,id+'-scene.png')});}
  fs.writeFileSync(path.join(out,id+'.json'),JSON.stringify(data));return {id,title,nodes:data.nodes.length,scene:!!data.scene,svgs:data.svgs.length};
}
module.exports={extract};
if(require.main===module)(async()=>{const browser=await chromium.launch({headless:true,channel:'chrome'});const page=await browser.newPage({viewport:{width:1920,height:1080}});const summary=[];
const fresh=async(id='VP05')=>{await page.goto('http://127.0.0.1:8921/?frame='+id);await page.waitForFunction(()=>window.S04_READY||window.S04_GRAPHICS_ERROR,null,{timeout:60000});};
await fresh();const frames=await page.evaluate(()=>window.S04_DATA.frames.map(x=>({id:x.id,title:x.title})));
for(const f of frames){if(saved(f.id)){summary.push(saved(f.id));continue;}await fresh(f.id);await page.waitForTimeout(100);summary.push(await extract(page,f.id,f.title));}
const cats=['物品','固定设备','资源','任务','工艺','物流','厂房与组合','专业计算','表现素材'];
for(let n=0;n<9;n++){if(saved('CAT'+n)){summary.push(saved('CAT'+n));continue;}await fresh();await page.locator(`[data-category="${n}"]`).click();summary.push(await extract(page,'CAT'+n,cats[n]+'定义'));}
for(const name of ['文件','编辑','视图','工程','模块','仿真','窗口','帮助']){if(saved('MENU'+name)){summary.push(saved('MENU'+name));continue;}await fresh();const b=page.locator(`[data-menu="${name}"]`);if(await b.count()){await b.click();summary.push(await extract(page,'MENU'+name,name+'菜单'));}}
for(const [id,title,acts] of [['DOCK0','隐藏时间轴',['toggle-timeline']],['DOCK1','隐藏描述',['toggle-description']],['DOCK2','隐藏双面板',['toggle-timeline','toggle-description']]]){await fresh();for(const a of acts){await page.locator('[data-menu="窗口"]').click();await page.locator(`.menu [data-action="${a}"]`).click();}summary.push(await extract(page,id,title));}
fs.writeFileSync(path.join(out,'capture-index.json'),JSON.stringify(summary,null,2));await browser.close();console.log(JSON.stringify({screens:summary.length,nodes:summary.reduce((a,x)=>a+x.nodes,0),scenes:summary.filter(x=>x.scene).length,ids:summary.map(x=>x.id)}));
})().catch(e=>{console.error(e);process.exit(1)});
