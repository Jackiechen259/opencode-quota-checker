"""Draw the application source artwork consumed by ``generate-icons.py``.

The mark is the app's own quota gauge: a 270-degree open meter whose filled
sweep is the used portion and whose faint remainder is the headroom left in the
window. Colours come from ``crates/opencode-desktop/src/theme/colors.rs`` so the
icon and the UI stay in step; edit the constants below and re-run to iterate.
"""

from __future__ import annotations

import argparse
import math
from pathlib import Path

from PIL import Image, ImageDraw


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = ROOT / "assets" / "icons" / "icon-source.png"

SIZE = 1024
SUPERSAMPLE = 3

# theme::palette::PRIMARY and PRIMARY_HOVER.
BACKGROUND_TOP = (59, 130, 246)
BACKGROUND_BOTTOM = (37, 99, 235)

# Gauge geometry, as a fraction of the canvas edge.
GAUGE_RADIUS = 0.302
GAUGE_STROKE = 0.106
HUB_RADIUS = 0.086

# Open meter: a 270-degree sweep with the gap centred at the bottom.
GAUGE_START_DEG = 135.0
GAUGE_SPAN_DEG = 270.0
GAUGE_FILL = 0.72

TRACK_COLOR = (255, 255, 255, 62)
FILL_COLOR = (255, 255, 255, 255)


def _gradient(edge: int) -> Image.Image:
    """Diagonal background gradient, drawn small and scaled for speed."""
    steps = 256
    ramp = Image.new("RGB", (steps, steps))
    pixels = ramp.load()
    for y in range(steps):
        for x in range(steps):
            ratio = (x + y) / (2 * (steps - 1))
            pixels[x, y] = tuple(
                round(BACKGROUND_TOP[i] + (BACKGROUND_BOTTOM[i] - BACKGROUND_TOP[i]) * ratio)
                for i in range(3)
            )
    return ramp.resize((edge, edge), Image.Resampling.BICUBIC).convert("RGBA")


def _stroke_arc(
    base: Image.Image,
    edge: int,
    radius: float,
    width: float,
    start_deg: float,
    span_deg: float,
    color: tuple[int, int, int, int],
) -> None:
    """Composite a round-capped arc whose stroke is centred on ``radius``."""
    layer = Image.new("RGBA", (edge, edge), (0, 0, 0, 0))
    draw = ImageDraw.Draw(layer)

    # Pillow grows arc width inwards from the bounding ellipse, so inflate the
    # box by half the stroke to centre it on the requested radius.
    box = radius + width / 2
    center = edge / 2
    draw.arc(
        [center - box, center - box, center + box, center + box],
        start_deg,
        start_deg + span_deg,
        fill=color,
        width=round(width),
    )
    for angle in (start_deg, start_deg + span_deg):
        radians = math.radians(angle)
        cap_x = center + radius * math.cos(radians)
        cap_y = center + radius * math.sin(radians)
        draw.ellipse(
            [cap_x - width / 2, cap_y - width / 2, cap_x + width / 2, cap_y + width / 2],
            fill=color,
        )

    base.alpha_composite(layer)


def render(size: int = SIZE) -> Image.Image:
    edge = size * SUPERSAMPLE
    canvas = _gradient(edge)

    radius = edge * GAUGE_RADIUS
    stroke = edge * GAUGE_STROKE
    _stroke_arc(canvas, edge, radius, stroke, GAUGE_START_DEG, GAUGE_SPAN_DEG, TRACK_COLOR)
    _stroke_arc(
        canvas,
        edge,
        radius,
        stroke,
        GAUGE_START_DEG,
        GAUGE_SPAN_DEG * GAUGE_FILL,
        FILL_COLOR,
    )

    hub = Image.new("RGBA", (edge, edge), (0, 0, 0, 0))
    hub_radius = edge * HUB_RADIUS
    center = edge / 2
    ImageDraw.Draw(hub).ellipse(
        [
            center - hub_radius,
            center - hub_radius,
            center + hub_radius,
            center + hub_radius,
        ],
        fill=FILL_COLOR,
    )
    canvas.alpha_composite(hub)

    return canvas.resize((size, size), Image.Resampling.LANCZOS)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    render().save(output, "PNG", optimize=True)

    print(f"Generated icon source artwork at {output}")


if __name__ == "__main__":
    main()
