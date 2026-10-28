from pathlib import Path
from PIL import Image,ImageDraw,ImageFont
import json
root=Path.cwd();home=root/'scenarios/媒体表现/native500-20261024';out=root/'scenarios/媒体表现/native500-quality-20261025/B02-Q03-R1';out.mkdir(parents=True,exist_ok=True);catalog=json.loads((root/'modules/media/native-equipment-500/reviews/B02-Q03-R1/catalog.json').read_text(encoding='utf-8'))['equipment'];font=ImageFont.truetype('C:/Windows/Fonts/msyh.ttc',16)
for lod in ['detailed','simple']:
 panel=Image.new('RGB',(2240,len(catalog)*226),'#e4e9ee');d=ImageDraw.Draw(panel)
 for row,s in enumerate(catalog):
  for col,v in enumerate(['iso','front','back','left','right','top','bottom']):
   p=Image.open(home/('b02-q03-r1-'+s['id'])/(lod+'-'+v+'.png'));p.thumbnail((320,200));panel.paste(p,(col*320,row*226));d.text((col*320+3,row*226+201),s['title']+' / '+v,font=font,fill='#142c3b')
 panel.save(out/(lod+'-seven-views.jpg'),quality=95)
native=root/'.work/native500-quality-20261025/q03r1-render-01';plan=json.loads((native/'plan.json').read_text(encoding='utf-8'))
if (native/'report.json').exists():
 for s in catalog:
  rows=[(i,c) for i,c in enumerate(plan['cases']) if c['name'].startswith(s['id']+'--')];panel=Image.new('RGB',(1440,((len(rows)+2)//3)*324),'#e4e9ee');d=ImageDraw.Draw(panel)
  for j,(i,c) in enumerate(rows):
   p=Image.open(native/f'case-{i+1:02}.png');p.thumbnail((480,300));x=j%3*480;y=j//3*324;panel.paste(p,(x,y));d.text((x+3,y+301),c['name'].split(' / ')[1],font=font,fill='#142c3b')
  panel.save(out/(s['id']+'-native.jpg'),quality=95)
print(out)
