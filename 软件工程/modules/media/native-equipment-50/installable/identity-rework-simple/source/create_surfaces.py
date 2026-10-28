"""Original CC0 deterministic PBR images; requires existing Pillow and NumPy."""
from pathlib import Path
from PIL import Image
import numpy as np

p = Path(__file__).parent / 'surfaces'
p.mkdir(exist_ok=True)
n = 2048
rng = np.random.default_rng(20261019)
y, x = np.mgrid[:n, :n]
noise = rng.normal(0, 1, (n, n))
grain = np.sin(y * .64) * 2 + noise * 1.4


def save(name, values):
    Image.fromarray(np.clip(values, 0, 255).astype('uint8')).save(p / name)


def rgb(values):
    return np.repeat(values[:, :, None], 3, axis=2)


metal = rgb(232 + grain)
metal[:, :, 2] += 1
save('metal-color.png', metal)
save('metal-orm.png', np.stack([np.full_like(grain, 255), 180 + grain * 1.5,
                               np.full_like(grain, 255)], axis=2))
save('metal-normal.png', np.stack([128 + noise * .15, 128 + grain * .65,
                                  np.full_like(grain, 254)], axis=2))
wood = np.sin(x * .034 + np.sin(y * .004) * 1.4 + np.sin(x * .007) * 2) * 8 + np.sin(x * .46) * 2 + noise * 1.5
save('wood-color.png', np.stack([235 + wood, 218 + wood * .9, 181 + wood * .65], axis=2))
save('wood-normal.png', np.stack([128 + wood * .3, 128 + noise * .2,
                                 np.full_like(wood, 254)], axis=2))
kraft = noise * 3 + np.sin(x * .9 + y * .3) * .8
save('kraft-color.png', rgb(238 + kraft))
save('kraft-normal.png', np.stack([128 + noise * 1.1, 128 + rng.normal(0, 1, (n, n)),
                                  np.full_like(kraft, 254)], axis=2))
paint = rng.normal(0, .8, (n, n))
save('paint-normal.png', np.stack([128 + paint, 128 + np.roll(paint, 1, 0),
                                  np.full_like(paint, 254)], axis=2))
print('8 original 2K surfaces created.')
