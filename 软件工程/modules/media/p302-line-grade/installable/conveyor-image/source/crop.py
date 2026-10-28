from pathlib import Path
from PIL import Image
p=Path(__file__).resolve().parent
Image.open(p/'consumer-canvas.png').convert('RGB').save(p.parent/'view.png')
