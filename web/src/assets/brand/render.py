#!/usr/bin/env python3
"""Everything derived from the mascot, rendered from it.

`mascot.png` is the only file anyone edits. The app icons, and the mark the
README and the window share, come out of here — for the same reason
`web/src/gen/` is generated: a copy kept by hand is a copy that drifts.

    python3 web/src/assets/brand/render.py
"""
from pathlib import Path
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[4]
SOURCE = ROOT / "web/src/assets/brand/mascot.png"
CROP = (200, 200, 1254, 1254)   # the framing the window's own chip uses
RADIUS = 0.14                   # gentle: an app icon, not a squircle


def rounded(size: int) -> Image.Image:
    """Rendered at 4x and reduced, so the corner is clean at 32px too."""
    big = size * 4
    tile = Image.open(SOURCE).convert("RGB").crop(CROP)
    tile = tile.resize((big, big), Image.LANCZOS).convert("RGBA")
    mask = Image.new("L", (big, big), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        [0, 0, big - 1, big - 1], radius=int(big * RADIUS), fill=255
    )
    tile.putalpha(mask)
    return tile.resize((size, size), Image.LANCZOS)


def main() -> None:
    icons = ROOT / "apps/desktop/icons"
    icons.mkdir(parents=True, exist_ok=True)
    for name, size in (("32x32.png", 32), ("128x128.png", 128),
                       ("128x128@2x.png", 256), ("icon.png", 512)):
        rounded(size).save(icons / name, "PNG", optimize=True)
    rounded(256).save(
        icons / "icon.ico", "ICO",
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )
    # One mark, two readers: the README and the window.
    rounded(400).save(ROOT / "web/src/assets/brand/mark.png", "PNG", optimize=True)
    print("rendered icons and mark from", SOURCE.name)


if __name__ == "__main__":
    main()
