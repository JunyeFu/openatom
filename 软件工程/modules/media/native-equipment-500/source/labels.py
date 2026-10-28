from pathlib import Path
import json,sys
from PIL import Image,ImageDraw,ImageFont
catalog=Path(sys.argv[1]);out=catalog.parent.parent/'nameplates';out.mkdir(exist_ok=True)
font=ImageFont.truetype('C:/Windows/Fonts/msyh.ttc',50)
small=ImageFont.truetype('C:/Windows/Fonts/msyh.ttc',29)
for s in json.loads(catalog.read_text(encoding='utf-8'))['equipment']:
 im=Image.new('RGB',(1280,320),'#e5e9e7');d=ImageDraw.Draw(im);d.rectangle((0,0,1280,14),fill='#337e91')
 d.text((32,32),s['title'],font=font,fill='#183445');d.text((32,115),f"SF NATIVE / {s['index']:03d} / {s['batch']}",font=small,fill='#24546c')
 d.text((32,167),s['id'],font=small,fill='#24546c');d.text((32,223),'原创示例设备 · 声明尺寸与周期 · CC0-1.0',font=small,fill='#44565e')
 d.text((32,272),'LOAD / AUTO / UNLOAD   |   METRE / Y-UP',font=small,fill='#44565e');im.save(out/(s['id']+'.png'),optimize=True)
