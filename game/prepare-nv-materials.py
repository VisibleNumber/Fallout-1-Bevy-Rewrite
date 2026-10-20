"""Prepare locally owned New Vegas diffuse materials; outputs remain ignored."""
from pathlib import Path
import subprocess
from PIL import Image
ROOT = Path(__file__).resolve().parent
MATERIALS = {
    'wall': 'textures/architecture/noso/nv_noso_rowhouse_plaster.dds',
    'roof': 'textures/architecture/urban/gnrroof01.dds',
}
for slot, source in MATERIALS.items():
    subprocess.run(['node', str(ROOT/'nv-assets.mjs'), 'extract', source], check=True)
    output = ROOT/'assets/nv/materials'/f'{slot}.png'
    output.parent.mkdir(parents=True, exist_ok=True)
    with Image.open(ROOT/'assets/nv/raw'/source) as image:
        image.convert('RGB').save(output)
    print(f'{slot}: {output}')
