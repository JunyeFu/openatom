// Build the separate work-introduction PDF from its editable candidate source.
// Uses an already installed browser/runtime. Never starts the product or Vite.
const fs = require('node:fs');
const path = require('node:path');
const {pathToFileURL} = require('node:url');
const root = path.resolve(__dirname, '../..');
const {chromium} = require(process.argv[2]);
const edgeExe = process.argv[3];
if (!edgeExe || !fs.existsSync(edgeExe)) throw new Error('Provide an existing Edge executable; no installation is performed');
const docs = path.join(root, 'docs/05_开源与参赛/release-preparation');
const out = path.join(docs, 'output');
const runtime = process.argv[4] ? path.resolve(process.argv[4]) : path.join(root, '.work/whole-packages/window2-release-preparation');
const source = JSON.parse(fs.readFileSync(path.join(docs,'作品介绍候选.json'),'utf8'));
const escape = value => String(value).replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;').replaceAll('"','&quot;');
const local = file => 'data:'+(file.endsWith('.otf')?'font/otf':'image/png')+';base64,'+fs.readFileSync(path.join(root,file)).toString('base64');
fs.mkdirSync(path.join(out,'pdf'), {recursive:true});
fs.mkdirSync(runtime, {recursive:true});
const sections = source.pages.map((page, index) => {
  const table = page.table ? '<table><thead><tr>'+page.table.headers.map(x=>'<th>'+escape(x)+'</th>').join('')+'</tr></thead><tbody>'+page.table.rows.map(row=>'<tr>'+row.map(x=>'<td>'+escape(x)+'</td>').join('')+'</tr>').join('')+'</tbody></table>' : '';
  let figure = '';
  if (page.image) {
    const [left,top,right,bottom] = page.image.crop;
    const width=right-left, height=bottom-top, viewWidth=665;
    const scale=viewWidth/width;
    figure='<figure><div class="capture" style="height:'+Math.round(height*scale)+'px"><img alt="历史实际桌面截图，范围见图注" src="'+local(page.image.source)+'" style="position:absolute;max-width:none;transform-origin:top left;transform:scale('+scale+');left:'+(-left*scale)+'px;top:'+(-top*scale)+'px"></div><figcaption>'+escape(page.image.caption)+'（只裁切放大原截图；未改写画面内容。）</figcaption></figure>';
  }
  const links=page.links?'<div style="font-size:11px;line-height:1.8">'+page.links.map(link=>'<a style="color:#287b87;margin-right:12px" href="'+escape(link.url)+'">'+escape(link.title)+'</a>').join('')+'</div>':'';
  return '<section class="page '+(index===0?'cover':'')+'"><div class="eyebrow">SUPER FACTORY / 作品介绍 · 开发候选</div>'+(index===0?'<div class="brand">'+escape(source.title)+'</div><div class="subtitle">'+escape(source.subtitle)+'</div>':'')+'<h1>'+escape(page.title)+'</h1><p class="lead">'+escape(page.lead)+'</p>'+figure+page.paragraphs.map(p=>'<p>'+escape(p)+'</p>').join('')+table+links+(page.note?'<aside>'+escape(page.note)+'</aside>':'')+'<footer><span>'+escape(source.author)+' · '+escape(source.edition)+'</span><span>'+String(index+1).padStart(2,'0')+' / '+source.pages.length+'</span></footer></section>';
}).join('');
const html='<!doctype html><html lang="zh-CN"><meta charset="utf-8"><title>超级工厂作品介绍开发候选</title><style>'+`
@font-face{font-family:SFDoc;src:url('${local('tools/architecture-proof/assets/SourceHanSansCN-Regular.otf')}')}@page{size:A4;margin:0}*{box-sizing:border-box}body{margin:0;background:#edf1f5;color:#192e43;font-family:SFDoc,sans-serif;font-size:14px;line-height:1.7}.page{width:210mm;height:297mm;background:#fff;padding:15mm 17mm 17mm;position:relative;break-after:page;overflow:visible}.page:last-child{break-after:auto}.eyebrow{font-family:SFDoc,sans-serif;color:#287b87;letter-spacing:1px;font-size:10px;border-top:4px solid #287b87;padding-top:10px;margin-bottom:16px}h1{font-size:24px;line-height:1.45;margin:10px 0 13px;letter-spacing:.2px}.lead{font-size:16px;color:#2b7180;border-left:3px solid #2b7180;padding-left:13px;margin-bottom:20px}p{margin:0 0 15px;text-align:justify}table{border-collapse:collapse;width:100%;font-size:12px;line-height:1.55;margin:15px 0;table-layout:fixed}th,td{border-bottom:1px solid #d8e2ea;padding:9px 10px;vertical-align:top;overflow-wrap:anywhere}th{background:#eaf3f6;color:#225567;text-align:left}tr:nth-child(even) td{background:#f7f9fb}aside{margin-top:18px;padding:12px 14px;background:#f2f6f8;border-left:3px solid #9bafbc;font-size:11px;color:#50677b;line-height:1.7}footer{position:absolute;left:17mm;right:17mm;bottom:10mm;border-top:1px solid #d6e0e8;padding-top:6px;display:flex;justify-content:space-between;color:#75889a;font-size:9px}.brand{font-size:48px;font-weight:normal;color:#18394d;letter-spacing:8px;margin-top:28px}.subtitle{font-size:15px;color:#457b8a;margin-bottom:28px}.cover h1{font-size:25px;margin-top:30px}figure{margin:14px 0 20px}.capture{position:relative;overflow:hidden;width:665px;border:1px solid #c9d7e2;background:#f4f7fa}figcaption{font-size:10px;color:#60758a;line-height:1.6;margin-top:8px}@media screen{.page{margin:20px auto;box-shadow:0 3px 18px #2342}}@media print{body{background:white}.page{margin:0}}
`+'</style><body>'+sections+'<template id="font-license">'+escape(fs.readFileSync(path.join(root,'tools/architecture-proof/assets/LICENSE.SourceHanSans.txt'),'utf8'))+'</template></body></html>';
const htmlPath=path.join(out,'超级工厂_作品介绍_开发候选.html');
fs.writeFileSync(htmlPath,html,'utf8');
(async()=>{
  const browser=await chromium.launch({executablePath:edgeExe,headless:true});
  try {
    const page=await browser.newPage({viewport:{width:1000,height:1300}});
    await page.goto(pathToFileURL(htmlPath).href);
    await page.evaluate(()=>document.fonts.ready);
    if(!await page.evaluate(()=>document.fonts.check('14px SFDoc'))) throw new Error('The selected OFL document font did not load');
    const layout=await page.evaluate(()=>Array.from(document.querySelectorAll('.page')).map((s,i)=>{
      const footer=s.querySelector('footer').getBoundingClientRect();
      const bounds=Array.from(s.children).filter(n=>n.tagName!=='FOOTER').map(n=>n.getBoundingClientRect().bottom);
      return {page:i+1,content_bottom:Math.max(...bounds),footer_top:footer.top,overflow:Math.max(...bounds)>footer.top-8};
    }));
    const errors=await page.evaluate(()=>Array.from(document.images).filter(x=>!x.complete||!x.naturalWidth).map(x=>x.src));
    if(layout.some(x=>x.overflow)||errors.length) throw new Error(JSON.stringify({layout,errors}));
    await page.pdf({path:path.join(out,'pdf/超级工厂_作品介绍_开发候选.pdf'),format:'A4',printBackground:true,preferCSSPageSize:true});
    fs.writeFileSync(path.join(runtime,'pdf-build.json'),JSON.stringify({source:'docs/05_开源与参赛/release-preparation/作品介绍候选.json',pages:source.pages.length,layout,images_loaded:errors.length===0,font:'SourceHanSansCN-Regular.otf / OFL-1.1',product_started:false,Native_used:false,headless_document_render_only:true},null,2)+'\n');
    console.log(JSON.stringify({pdf:path.join(out,'pdf/超级工厂_作品介绍_开发候选.pdf'),pages:source.pages.length,overflow:0,missing_images:0}));
  } finally {await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
