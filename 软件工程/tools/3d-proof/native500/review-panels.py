"""Arrange existing component/Native captures for review; never creates acceptance."""
from pathlib import Path
import json
import sys
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[3]
batch, native_pass = sys.argv[1:3]
prefix = batch.lower()
pics = ROOT / 'scenarios/媒体表现/native500-20261024'
native = ROOT / '.work/native500-quality-20261025' / native_pass
review = ROOT / 'modules/media/native-equipment-500/reviews' / batch
devices = [s['id'] for s in json.loads((review / 'catalog.json').read_text(encoding='utf-8-sig'))['equipment']]
cases = json.loads((native / 'report.json').read_text(encoding='utf-8-sig'))['driver']['cases']
dest = ROOT / 'scenarios/媒体表现/native500-quality-20261025' / batch
dest.mkdir(parents=True, exist_ok=True)
views = Image.new('RGB', (1400, 440 * len(devices)), 'white')
states = Image.new('RGB', (1920, 310 * len(devices)), 'white')
stock = Image.new('RGB', (1200, 420 * len(devices)), 'white')

def tile(canvas, path, xy, size, label):
    picture = Image.open(path).convert('RGB')
    picture.thumbnail(size)
    canvas.paste(picture, xy)
    ImageDraw.Draw(canvas).text((xy[0] + 4, xy[1] + size[1]), label, fill='black')

for row, device in enumerate(devices):
    for i, direction in enumerate(['iso', 'front', 'back', 'left', 'right', 'top', 'bottom', 'foot-contact']):
        tile(views, pics / (prefix + '-' + device) / ('detailed-' + direction + '.png'),
             (i % 4 * 350, row * 440 + i // 4 * 220), (350, 200), device + ' ' + direction)
    for col, stage in enumerate(['late-processing', 'delivered']):
        tile(stock, pics / (prefix + '-' + device + '-' + stage + '.png'),
             (col * 600, row * 420), (600, 400), device + ' ' + stage)
    rows = [(i, c) for i, c in enumerate(cases, 1) if c['name'].startswith(device + '--')]
    selected = [next(v for v in rows if part in v[1]['name'])
                for part in ['--normal / actual normal', '--output-wait /']]
    selected.extend(v for v in rows if '--interrupted /' in v[1]['name'])
    finished = next((v for v in rows if 'finished package closeup' in v[1]['name']), None)
    extra = finished if finished is not None else next((v for v in rows if '--missing /' in v[1]['name']), None)
    if extra is not None: selected.insert(1, extra)
    panel = Image.new('RGB', (1440, 320), 'white')
    for col, (index, case) in enumerate(selected):
        path = native / ('case-' + str(index).zfill(2) + '.png')
        tile(states, path, (col * 480, row * 310), (480, 290), case['name'])
        if col < 3:
            tile(panel, path, (col * 480, 0), (480, 290), case['name'])
    panel.save(dest / (device + '-native.jpg'), quality=90)
for image, name in [(views, 'seven-views-panel'), (states, 'native-panel'), (stock, 'coarse-panel')]:
    image.save(pics / (prefix + '-' + name + '.png'))
print(f'{batch}: current component and {native_pass} panels prepared for {len(devices)} devices.')
