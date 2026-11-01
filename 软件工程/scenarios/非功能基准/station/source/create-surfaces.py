"""Reuse original project CC0 micro-surfaces; create editable B60 identification."""
from pathlib import Path
from shutil import copyfile
from PIL import Image, ImageDraw, ImageFont

here = Path(__file__).resolve().parent
root = here.parents[3]
out = here / 'surfaces'
out.mkdir(exist_ok=True)
for name in ['metal-color', 'metal-orm', 'metal-normal', 'paint-normal']:
    copyfile(root / 'modules/media/native-equipment-50/source/surfaces' / (name + '.png'), out / (name + '.png'))
font = lambda size: ImageFont.truetype('C:/Windows/Fonts/consola.ttf', size)
image = Image.new('RGB', (1024, 320), '#d9e0e3')
d = ImageDraw.Draw(image)
d.rectangle((0, 0, 1024, 15), fill='#38758c')
d.text((30, 30), 'SUPER FACTORY / B60', font=font(59), fill='#173748')
d.text((32, 115), 'FIXED PROCESS STATION  /  1.0.5', font=font(36), fill='#344a54')
d.text((32, 177), 'METRE / Z-UP  |  DECLARED CYCLE 2.0s', font=font(34), fill='#344a54')
d.text((32, 235), 'ORIGINAL SCHEMATIC BENCHMARK  /  CC0', font=font(33), fill='#344a54')
image.save(out / 'nameplate.png', optimize=True)
image = Image.new('RGB', (1024, 640), '#12303d')
d = ImageDraw.Draw(image)
d.text((45, 35), 'B60 / PROCESS STATION', font=font(65), fill='#b7d6dd')
d.line((45, 140, 970, 140), fill='#6ab29e', width=6)
d.text((45, 185), 'DECLARED CYCLE', font=font(52), fill='#d4dfdc')
d.text((45, 265), '2.0 s / ONE PIECE', font=font(70), fill='#92ceb1')
d.text((45, 405), 'SCHEMATIC CONTROL PANEL', font=font(43), fill='#b7d6dd')
d.text((45, 495), 'LIVE JOB DATA: WORKBENCH', font=font(43), fill='#b7d6dd')
image.save(out / 'screen.png', optimize=True)
print('B60 original labels and four existing CC0 micro-surfaces prepared.')
