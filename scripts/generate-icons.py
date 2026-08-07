"""Generate all application icon formats from the checked-in source artwork."""

from __future__ import annotations

import argparse
from pathlib import Path

from PIL import Image, ImageDraw


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SOURCE = ROOT / "assets" / "icons" / "icon-source.png"
DEFAULT_OUTPUT = ROOT / "assets" / "icons"


def rounded_icon(source: Path, size: int = 1024) -> Image.Image:
    """Normalize artwork to a square icon with antialiased transparent corners."""
    with Image.open(source) as opened:
        image = opened.convert("RGBA")

    crop_size = min(image.size)
    left = (image.width - crop_size) // 2
    top = (image.height - crop_size) // 2
    image = image.crop((left, top, left + crop_size, top + crop_size))
    image = image.resize((size, size), Image.Resampling.LANCZOS)

    scale = 4
    mask = Image.new("L", (size * scale, size * scale), 0)
    draw = ImageDraw.Draw(mask)
    radius = int(size * 0.18 * scale)
    draw.rounded_rectangle(
        (0, 0, size * scale - 1, size * scale - 1),
        radius=radius,
        fill=255,
    )
    mask = mask.resize((size, size), Image.Resampling.LANCZOS)
    image.putalpha(mask)
    return image


def save_png(image: Image.Image, path: Path, size: int) -> None:
    resized = image.resize((size, size), Image.Resampling.LANCZOS)
    resized.save(path, "PNG", optimize=True)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    master = rounded_icon(args.source.resolve())

    save_png(master, output / "icon.png", 512)
    save_png(master, output / "32x32.png", 32)
    save_png(master, output / "128x128.png", 128)
    save_png(master, output / "128x128@2x.png", 256)

    master.save(
        output / "icon.ico",
        "ICO",
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )
    master.save(output / "icon.icns", "ICNS")

    window = master.resize((256, 256), Image.Resampling.LANCZOS)
    tray = master.resize((32, 32), Image.Resampling.LANCZOS)
    (output / "window.rgba").write_bytes(window.tobytes())
    (output / "tray.rgba").write_bytes(tray.tobytes())

    print(f"Generated application icons in {output}")


if __name__ == "__main__":
    main()
