"""Original, repeatable 2K micro-surfaces and equipment nameplates. No external images."""
from pathlib import Path
import math
import json
from PIL import Image, ImageDraw, ImageFont

HOME = Path(__file__).resolve().parent
out = HOME / 'surfaces'
out.mkdir(exist_ok=True)
size = 2048
for name in ['metal-color', 'metal-orm', 'metal-normal', 'paint-normal']:
    image = Image.new('RGB', (size, size))
    rows = []
    for y in range(size):
        grain = round(3 * math.sin(y * 1.7) + 2 * math.sin(y * .29))
        if name == 'metal-color': value = (225 + grain, 226 + grain, 228 + grain)
        elif name == 'metal-orm': value = (255, 185 + grain, 255)
        elif name == 'metal-normal': value = (128, 128 + grain, 254)
        else: value = (128, 128 + (y % 3 - 1), 255)
        rows.extend([value] * size)
    image.putdata(rows)
    image.save(out / (name + '.png'), optimize=True)
print('Original 2048px material surfaces ready')
catalog = HOME.parent / 'catalog.json'
if catalog.exists():
    labels = HOME.parent / 'nameplates'
    labels.mkdir(exist_ok=True)
    font_path = Path('C:/Windows/Fonts/msyh.ttc')
    title_font = ImageFont.truetype(str(font_path), 55)
    small_font = ImageFont.truetype(str(font_path), 30)
    for spec in json.loads(catalog.read_text(encoding='utf-8'))['equipment']:
        image = Image.new('RGB', (1280, 320), '#e5e9e7')
        draw = ImageDraw.Draw(image)
        draw.rectangle((0, 0, 1280, 14), fill='#337e91')
        draw.text((32, 32), spec['title'], font=title_font, fill='#183445')
        draw.text((32, 126), 'SF NATIVE 50 / %02d / %s' % (spec['index'], spec['id']), font=small_font, fill='#24546c')
        draw.text((32, 186), '原创示例设备 · 声明尺寸与工时 · CC0-1.0', font=small_font, fill='#44565e')
        draw.text((32, 244), 'LOAD / AUTO / UNLOAD     |     METRE / Y-UP', font=small_font, fill='#44565e')
        image.save(labels / (spec['id'] + '.png'), optimize=True)
