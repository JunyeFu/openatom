"""Compare native browser ImageDecoder pixels with independent GIF LZW/disposal decoding."""
from pathlib import Path
from PIL import Image
import json

ROOT=Path(__file__).resolve().parents[3]
BASE=ROOT/'.work/full-closeout-20261021/delivery'
result=json.loads((BASE/'render/result.json').read_text(encoding='utf-8'))
source=json.loads((BASE/'render/input.json').read_text(encoding='utf-8'))
gif=Image.open(ROOT/source['gif']['file'])
checks=[]
for sample in result['gif']['samples']:
    gif.seek(sample['expected'])
    expected=gif.convert('RGBA')
    actual=(BASE/sample['raw']).read_bytes()
    assert expected.size==(sample['width'],sample['height'])
    pixels=expected.tobytes()
    differences=sum(a!=b for a,b in zip(actual,pixels))
    assert len(actual)==len(pixels)
    assert differences==0, f"Native browser frame mismatch at {sample['t']} ms: {differences} channels"
    checks.append(dict(time_ms=sample['t'],terminal=sample['terminal'],expected_frame=sample['expected'],
                       width=expected.width,height=expected.height,differing_channels=differences))
(BASE/'render/gif-independent-comparison.json').write_text(json.dumps(
    dict(passed=True,native_decoder='browser ImageDecoder and createImageBitmap',
         independent_decoder='Pillow GIF LZW and disposal',checks=checks),indent=2),encoding='utf-8')
print(json.dumps(dict(passed=True,samples=len(checks),differing_channels=0)))
