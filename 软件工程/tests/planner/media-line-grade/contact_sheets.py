"""Arrange actual consumer captures for review; no geometry or screenshot content retouching."""
from pathlib import Path
from PIL import Image,ImageDraw,ImageFont
import sys
root=Path(__file__).resolve().parents[3]/'scenarios/媒体表现/runtime-line-grade'
font=ImageFont.truetype('C:/Windows/Fonts/arial.ttf',18)
families=sys.argv[1:] or ['assembly','rework','packing','robot','conveyor','inspection','sealer','pallet']
for family in families:
    views=['iso','front','back','bottom','tool']
    sheet=Image.new('RGB',(1230,616),'#f1f6fa');d=ImageDraw.Draw(sheet)
    for i,v in enumerate(views):
        file=root/'views'/f'{family}-{v}-full.png'
        if not file.exists():continue
        im=Image.open(file).convert('RGB').crop((19,248,976,864));im.save(root/'views'/f'{family}-{v}.png');im.thumbnail((400,270))
        x=(i%3)*410;y=(i//3)*308
        sheet.paste(im,(x,y+28));d.text((x+10,y+5),f'{family} / {v}',font=font,fill='#172e48')
    sheet.save(root/f'{family}-review.png')
