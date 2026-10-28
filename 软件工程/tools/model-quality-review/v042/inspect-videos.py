"""Decode the actual model movies and extract reviewable motion frames."""
from pathlib import Path
import json
import subprocess

root = Path(__file__).resolve().parents[3]
out = root / 'scenarios/媒体表现/model-quality-20261022'
ffmpeg = r'D:/MathModelingTools/envs/cumcm/Lib/site-packages/imageio_ffmpeg/binaries/ffmpeg-win-x86_64-v7.1.exe'
entries = json.loads((root / 'modules/media/equipment-quality-v042/inventory.json').read_text())['entries']
results = []
for entry in entries:
    folder = out / (entry['family'] + ('-simple' if entry['suffix'] == 'simple' else ''))
    capture = json.loads((folder / 'capture.json').read_text())
    if capture.get('renderer_profile') != 'depth-fitted-v1-vsm' or capture.get('glb_bytes') != entry['bytes']:
        raise RuntimeError('Capture is not the final export: ' + entry['family'] + '/' + entry['suffix'])
    video = folder / 'motion.mp4'
    decoded = subprocess.run([ffmpeg, '-v', 'error', '-i', str(video), '-f', 'null', '-'], capture_output=True, creationflags=subprocess.CREATE_NO_WINDOW)
    decoded.check_returncode()
    frames = subprocess.run([ffmpeg, '-v', 'error', '-y', '-i', str(video), '-vf', 'fps=1/3,scale=640:360,tile=2x2', '-frames:v', '1', str(folder / 'video-frames.jpg')], capture_output=True, creationflags=subprocess.CREATE_NO_WINDOW)
    frames.check_returncode()
    results.append(dict(family=entry['family'], suffix=entry['suffix'], version=entry['version'], bytes=video.stat().st_size, full_decode=True, frames=capture['video']['frames'], duration_seconds=capture['video']['frames']/24, renderer_profile=capture['renderer_profile']))
(out / 'video-decode.json').write_text(json.dumps({'scope':'video readability and frame extraction only; visual judgment is separate', 'results':results}, ensure_ascii=False, indent=2), encoding='utf-8')
print(json.dumps({'decoded':len(results), 'frames_each':288, 'duration_each':12, 'bytes':sum(r['bytes'] for r in results)}))
