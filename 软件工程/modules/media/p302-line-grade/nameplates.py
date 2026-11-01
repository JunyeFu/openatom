"""Original painted labels, not a copy of the read-only reference textures. CC0-1.0."""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
import json
root=Path(__file__).parent
data=json.loads((root/'parameters.json').read_text(encoding='utf-8'))
(root/'nameplates').mkdir(exist_ok=True)
font=ImageFont.truetype('C:/Windows/Fonts/arial.ttf',32)
small=ImageFont.truetype('C:/Windows/Fonts/arial.ttf',20)
for name in data['families']+['control-box','carton']:
    im=Image.new('RGB',(512,192),'#e5eced');d=ImageDraw.Draw(im)
    d.rectangle((0,0,512,46),fill='#1f465e');d.text((18,6),'SF / ORIGINAL EQUIPMENT',font=small,fill='#f5f9fa')
    d.text((18,61),name.upper(),font=font,fill='#183044')
    d.text((18,112),'P302  |  0.1.0  |  METER / Y-UP',font=small,fill='#42586a')
    d.rectangle((18,150,494,178),fill='#d5a448')
    for x in range(-20,530,30):d.polygon([(x,150),(x+12,150),(x-3,178),(x-15,178)],fill='#283646')
    im.save(root/'nameplates'/f'{name}.png')
