"""生成审查用联系表，只读取现有截图，不生成假软件画面。"""
from pathlib import Path
import json
from PIL import Image,ImageDraw,ImageFont
D=Path(__file__).resolve().parent;M=D.parent/'S04-02_drawio母版_20261014';O=D/'output/visual-review';O.mkdir(exist_ok=True)
font=ImageFont.truetype('C:/Windows/Fonts/msyh.ttc',22)
frames=json.loads((D/'output/playwright/frames.json').read_text(encoding='utf-8'))
master=json.loads((M/'母版核对结果.json').read_text(encoding='utf-8-sig'))['exports']
for kind,rows,root,key in [('actual',frames,D,'file'),('master',master,M,'file')]:
 for start in range(0,len(rows),4):
  sheet=Image.new('RGB',(1920,1140),'#eef3f7');draw=ImageDraw.Draw(sheet)
  for n,r in enumerate(rows[start:start+4]):
   x=(n%2)*960;y=(n//2)*570;img=Image.open(root/r[key]).convert('RGB');img.thumbnail((950,532));sheet.paste(img,(x,y+32));draw.text((x+8,y+3),r.get('frame',r[key]),font=font,fill='#172e48')
  sheet.save(O/f'{kind}-{start//4+1:02}.jpg',quality=94)
print(json.dumps({'actual_sheets':6,'master_sheets':7,'actual_frames':len(frames),'master_pages':len(master)},ensure_ascii=False))
