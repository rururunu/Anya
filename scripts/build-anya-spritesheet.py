"""Build host Anya mascot spritesheet (ChatGPT/Codex atlas) into public/pets/anya/."""
from __future__ import annotations

import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageEnhance, ImageOps

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "public" / "pets" / "anya"
FW, FH = 192, 208
COLS, ROWS = 8, 9
SHEET_W, SHEET_H = FW * COLS, FH * ROWS
ROW_FRAMES = [6, 8, 8, 4, 5, 8, 6, 6, 6]


def draw_anya_blob() -> Image.Image:
    """Pixel-ish Anya blob matching the blue SVG mascot palette."""
    img = Image.new("RGBA", (128, 128), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    # Body
    d.ellipse((18, 28, 110, 118), fill=(107, 140, 255, 255), outline=(45, 70, 160, 255))
    # Soft highlight
    d.ellipse((34, 40, 70, 72), fill=(168, 192, 255, 220))
    # Eyes
    d.ellipse((42, 58, 58, 78), fill=(255, 255, 255, 255), outline=(26, 26, 40, 255))
    d.ellipse((70, 58, 86, 78), fill=(255, 255, 255, 255), outline=(26, 26, 40, 255))
    d.ellipse((48, 64, 56, 74), fill=(26, 32, 64, 255))
    d.ellipse((76, 64, 84, 74), fill=(26, 32, 64, 255))
    d.point((50, 65), fill=(255, 255, 255, 255))
    d.point((78, 65), fill=(255, 255, 255, 255))
    # Smile
    d.arc((52, 78, 76, 96), start=20, end=160, fill=(26, 32, 64, 255), width=3)
    # Side fins
    d.ellipse((6, 70, 24, 96), fill=(107, 140, 255, 255), outline=(45, 70, 160, 255))
    d.ellipse((104, 70, 122, 96), fill=(107, 140, 255, 255), outline=(45, 70, 160, 255))
    # Upscale with nearest for pixel look
    return img.resize((256, 256), Image.Resampling.NEAREST)


def fit_character(src: Image.Image, scale: float = 0.84, dy: int = 0, flip: bool = False) -> Image.Image:
    img = src.convert("RGBA")
    if flip:
        img = ImageOps.mirror(img)
    bbox = img.getbbox()
    if bbox:
        img = img.crop(bbox)
    max_w = int(FW * scale)
    max_h = int(FH * scale)
    img.thumbnail((max_w, max_h), Image.Resampling.NEAREST)
    canvas = Image.new("RGBA", (FW, FH), (0, 0, 0, 0))
    x = (FW - img.width) // 2
    y = FH - img.height - 8 + dy
    canvas.alpha_composite(img, (x, max(0, y)))
    return canvas


def variant(frame: Image.Image, row: int, col: int) -> Image.Image:
    out = frame.copy()
    if row in (0, 6, 8):
        shift = [0, -2, -3, -2, 0, 1, 0, -1][col % 8]
        shifted = Image.new("RGBA", (FW, FH), (0, 0, 0, 0))
        shifted.alpha_composite(out, (0, shift))
        out = shifted
    if row == 3:
        shift_x = [0, 2, 4, 2][col % 4]
        shifted = Image.new("RGBA", (FW, FH), (0, 0, 0, 0))
        shifted.alpha_composite(out, (shift_x, -abs(shift_x) // 2))
        out = shifted
    if row == 4:
        lift = [0, -6, -12, -6, 0][col % 5]
        shifted = Image.new("RGBA", (FW, FH), (0, 0, 0, 0))
        shifted.alpha_composite(out, (0, lift))
        out = shifted
    if row == 5:
        r, g, b, a = out.split()
        rgb = Image.merge("RGB", (r, g, b))
        rgb = ImageEnhance.Brightness(rgb).enhance(0.85)
        r, g, b = rgb.split()
        out = Image.merge("RGBA", (r, g, b, a))
        shake = [0, -2, 2, -2, 2, -1, 1, 0][col % 8]
        shifted = Image.new("RGBA", (FW, FH), (0, 0, 0, 0))
        shifted.alpha_composite(out, (shake, 0))
        out = shifted
    if row == 7:
        sx = [0, 1, 2, 1, 0, -1][col % 6]
        shifted = Image.new("RGBA", (FW, FH), (0, 0, 0, 0))
        shifted.alpha_composite(out, (sx, 0))
        out = shifted
    if row in (1, 2):
        bob = [0, -2, 0, 2, 0, -2, 0, 1][col % 8]
        shifted = Image.new("RGBA", (FW, FH), (0, 0, 0, 0))
        shifted.alpha_composite(out, (0, bob))
        out = shifted
    return out


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    src = draw_anya_blob()
    base = fit_character(src, scale=0.88)
    base_flip = fit_character(src, scale=0.88, flip=True)
    sheet = Image.new("RGBA", (SHEET_W, SHEET_H), (0, 0, 0, 0))
    for row in range(ROWS):
        frames = ROW_FRAMES[row]
        for col in range(COLS):
            if col >= frames:
                continue
            cell = base_flip if row == 2 else base
            sheet.alpha_composite(variant(cell, row, col), (col * FW, row * FH))
    preview = fit_character(src, scale=0.92)
    sheet.save(OUT / "spritesheet.png", optimize=True)
    preview.save(OUT / "preview.png", optimize=True)
    manifest = {
        "id": "anya",
        "displayName": {"en": "Anya", "zh-CN": "Anya"},
        "spritesheet": "spritesheet.png",
        "frameWidth": FW,
        "frameHeight": FH,
        "columns": COLS,
        "rows": ROWS,
        "animations": {
            "idle": {"row": 0, "frames": 6, "durationsMs": [280, 110, 110, 140, 140, 320]},
            "runRight": {"row": 1, "frames": 8},
            "runLeft": {"row": 2, "frames": 8},
            "wave": {"row": 3, "frames": 4, "loop": False},
            "jump": {"row": 4, "frames": 5, "loop": False},
            "fail": {"row": 5, "frames": 8},
            "wait": {"row": 6, "frames": 6},
            "work": {"row": 7, "frames": 6},
            "review": {"row": 8, "frames": 6},
        },
    }
    (OUT / "pet.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {OUT} sheet={sheet.size}")


if __name__ == "__main__":
    main()
