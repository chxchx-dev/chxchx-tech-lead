"""Makes the web client's icons from the product mark (assets/brand/mark-256.png).

    python3 scripts/make_remote_icons.py

The mark is round with a transparent background, so it goes on a dark square. Maskable icons
keep it inside the central 80 % (the part Android never crops).
"""

from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
MARK = Image.open(ROOT / "assets/brand/mark-256.png").convert("RGBA")
OUT = ROOT / "apps/remote/web/icons"
BACKGROUND = (15, 21, 32, 255)  # #0f1520, the dark theme's background


def make(size: int, mark_fraction: float, rounded: float = 0.0) -> Image.Image:
    # Drawn at 4x and brought down: smooth edges without a vector source.
    big = size * 4
    canvas = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    draw = ImageDraw.Draw(canvas)
    if rounded:
        draw.rounded_rectangle((0, 0, big - 1, big - 1), radius=int(big * rounded), fill=BACKGROUND)
    else:
        draw.rectangle((0, 0, big, big), fill=BACKGROUND)
    mark = MARK.resize((int(big * mark_fraction),) * 2, Image.LANCZOS)
    canvas.alpha_composite(mark, ((big - mark.width) // 2, (big - mark.height) // 2))
    return canvas.resize((size, size), Image.LANCZOS)


OUT.mkdir(parents=True, exist_ok=True)
# Any: rounded, the mark fills most of it.
make(192, 0.70, 0.22).save(OUT / "icon-192.png", optimize=True)
make(512, 0.70, 0.22).save(OUT / "icon-512.png", optimize=True)
# Maskable: full bleed, the mark inside the safe zone.
make(512, 0.60).save(OUT / "maskable-512.png", optimize=True)
# iOS rounds the corners itself and turns transparency black: full bleed.
make(180, 0.66).convert("RGB").save(OUT / "apple-touch-icon.png", optimize=True)
make(32, 0.84, 0.22).save(OUT / "favicon-32.png", optimize=True)
for path in sorted(OUT.iterdir()):
    print(path.name, path.stat().st_size, "bytes")
