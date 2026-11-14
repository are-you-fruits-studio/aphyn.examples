"""Generates examples/normals/assets/normal_map.png, a tangent-space normal map of beveled tiles
with a round dimple in the middle of each tile (960x960 RGBA8, 4x4 tiles).

The math is a port of `make_tile_normal_map` from the engine's `showcase` example: a height
field is differentiated with central differences and the slope is packed into RGB as
`n * 0.5 + 0.5`. One difference from that function: green points up the image (towards v = 0).
That is the convention this quad's tangent frame needs (tangent +X with w = 1 gives a bitangent
of -Y, while v grows along +Y) and the one the original normal_map.png used, so a tile reads as
raised along both axes. The showcase function points green down the image, which suits meshes
whose bitangent runs along +v. The output is deterministic.

Needs numpy and Pillow. Run from anywhere: python3 examples/normals/tools/generate_normal_map.py
"""

from pathlib import Path

import numpy as np
from PIL import Image

SIZE = 960
TILES = 4
STRENGTH = 0.035
OUTPUT = Path(__file__).resolve().parent.parent / "assets" / "normal_map.png"


def smoothstep(edge0, edge1, x):
    t = np.clip((x - edge0) / np.float32(edge1 - edge0), 0.0, 1.0)
    return t * t * (np.float32(3.0) - np.float32(2.0) * t)


def tile_height(u, v):
    fu = np.modf(u * np.float32(TILES))[0]
    fv = np.modf(v * np.float32(TILES))[0]
    edge = np.minimum(np.minimum(fu, 1.0 - fu), np.minimum(fv, 1.0 - fv))
    bevel = smoothstep(0.0, 0.14, edge)
    center = np.sqrt((fu - np.float32(0.5)) ** 2 + (fv - np.float32(0.5)) ** 2)
    dimple = np.float32(1.0) - smoothstep(0.08, 0.2, center)
    return bevel - np.float32(0.6) * dimple


def main():
    texel = np.float32(1.0 / SIZE)
    coords = (np.arange(SIZE, dtype=np.float32) + np.float32(0.5)) * texel
    v, u = np.meshgrid(coords, coords, indexing="ij")

    dx = (tile_height(u + texel, v) - tile_height(u - texel, v)) / (np.float32(2.0) * texel)
    dy = (tile_height(u, v + texel) - tile_height(u, v - texel)) / (np.float32(2.0) * texel)

    # +dy, not -dy: green points towards v = 0 (up the image)
    n = np.stack([-dx * np.float32(STRENGTH), dy * np.float32(STRENGTH), np.ones_like(dx)], axis=-1)
    n /= np.sqrt(np.sum(n * n, axis=-1, keepdims=True))

    rgb = np.floor((n * np.float32(0.5) + np.float32(0.5)) * np.float32(255.0) + np.float32(0.5))
    rgba = np.empty((SIZE, SIZE, 4), dtype=np.uint8)
    rgba[..., :3] = rgb.astype(np.uint8)
    rgba[..., 3] = 255

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(rgba, "RGBA").save(OUTPUT, optimize=True)
    print(f"wrote {OUTPUT}")


if __name__ == "__main__":
    main()
