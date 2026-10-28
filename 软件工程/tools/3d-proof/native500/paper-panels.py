from pathlib import Path
from PIL import Image,ImageDraw,ImageFont
import json,sys
ROOT=Path(__file__).resolve().parents[3];home=ROOT/'scenarios/媒体表现/native500-20261024';batch=sys.argv[2] if len(sys.argv)>2 else 'B02-Q01';out=ROOT/'scenarios/媒体表现/native500-quality-20261025';out=out/batch if batch!='B02-Q01' else out;out.mkdir(exist_ok=True,parents=True)
catalog=json.loads((ROOT/'modules/media/native-equipment-500/reviews'/batch/'catalog.json').read_text(encoding='utf8'))['equipment']
font=ImageFont.truetype('C:/Windows/Fonts/msyh.ttc',18)
for lod in ['detailed','simple']:
 panel=Image.new('RGB',(2240,5*226),'#e4e9ee');draw=ImageDraw.Draw(panel)
 for row,spec in enumerate(catalog):
  for col,view in enumerate(['iso','front','back','left','right','top','bottom']):
   path=home/(batch.lower()+'-'+spec['id'])/(lod+'-'+view+'.png');pic=Image.open(path);pic.thumbnail((320,200));panel.paste(pic,(col*320,row*226));draw.text((col*320+5,row*226+201),spec['title']+' / '+view,font=font,fill='#142c3b')
 panel.save(out/(lod+'-seven-views.jpg'),quality=95)
if len(sys.argv)>1:
 native=ROOT/sys.argv[1];plan=json.loads((native/'plan.json').read_text(encoding='utf8'))
 for row,spec in enumerate(catalog):
  panel=Image.new('RGB',(1920,4*322),'#e4e9ee');draw=ImageDraw.Draw(panel)
  for i in range(12):
   index=row*12+i;pic=Image.open(native/f'case-{index+1:02}.png');pic.thumbnail((640,290));x=i%3*640;y=i//3*322;panel.paste(pic,(x,y));draw.text((x+6,y+294),plan['cases'][index]['name'].split(' / ')[1],font=font,fill='#142c3b')
  panel.save(out/(spec['id']+'-native.jpg'),quality=95)
print('Actual GLB seven-view and Native condition panels prepared.')
