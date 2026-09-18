"""Build Anya maid spritesheet from green-screen pose stills (ChatGPT/Codex atlas)."""
from __future__ import annotations

import json
from pathlib import Path

from PIL import Image, ImageEnhance, ImageFilter, ImageOps

ROOT = Path(__file__).resolve().parents[1]
ASSETS = Path(r"C:\Users\GuoLv\.cursor\projects\c-My-code-AltAltAi\assets")
OUT = ROOT / "public" / "pets" / "anya"
PLUGIN_OUT = ROOT / "src-tauri" / "plugins" / "pet-gallery" / "ui" / "pets" / "anya"
FW, FH = 192, 208
COLS, ROWS = 8, 9
SHEET_W, SHEET_H = FW * COLS, FH * ROWS
ROW_FRAMES = [6, 8, 8, 4, 5, 8, 6, 6, 6]

POSES = {
    "idle": "anya-maid-idle-v2.png",
    "run": "anya-maid-run-v2.png",
    "wave": "anya-maid-wave-v2.png",
    "jump": "anya-maid-jump-v2.png",
    "fail": "anya-maid-fail-v2.png",
    "wait": "anya-maid-wait-v2.png",
    "work": "anya-maid-work-v2.png",
    "think": "anya-maid-think-v2.png",
}


def strip_bg(im: Image.Image) -> Image.Image:
    """Chroma-key green + border flood for leftover studio bg."""
    im = im.convert("RGBA")
    w, h = im.size
    px = im.load()

    def is_green(r: int, g: int, b: int) -> bool:
        # Broad chroma: any clearly green-dominant pixel (studio key)
        return g >= 70 and g >= r + 25 and g >= b + 25 and (g - max(r, b)) >= 20

    def is_light_studio(r: int, g: int, b: int) -> bool:
        return min(r, g, b) > 225 and max(r, g, b) - min(r, g, b) < 22

    # Pass 1: chroma key green + near-white studio
    for y in range(h):
        for x in range(w):
            r, g, b, a = px[x, y]
            if a == 0:
                continue
            if is_green(r, g, b):
                strength = (g - max(r, b)) / max(1, g)
                if strength > 0.35 or g > 140:
                    px[x, y] = (r, g, b, 0)
                else:
                    px[x, y] = (r, g, b, max(0, int(a * (1 - strength * 2))))
            elif is_light_studio(r, g, b):
                px[x, y] = (r, g, b, 0)

    # Pass 2: flood remaining corner bg
    corners = []
    for x, y in [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1), (w // 2, 0)]:
        r, g, b, a = px[x, y]
        if a > 0:
            corners.append((r, g, b))
    if corners:
        visited = [[False] * w for _ in range(h)]
        stack = [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)]
        while stack:
            x, y = stack.pop()
            if x < 0 or y < 0 or x >= w or y >= h or visited[y][x]:
                continue
            visited[y][x] = True
            r, g, b, a = px[x, y]
            if a == 0:
                stack.extend([(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)])
                continue
            if any(abs(r - cr) + abs(g - cg) + abs(b - cb) <= 36 for cr, cg, cb in corners):
                # Don't eat skin / hair / black dress
                if r > 200 and g > 160 and b > 140 and abs(r - g) < 40:
                    continue  # skin-ish
                if max(r, g, b) < 55:
                    continue  # black outfit
                px[x, y] = (r, g, b, 0)
                stack.extend([(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)])

    return im


def fit(src: Image.Image, scale: float = 0.92, dy: int = 0, flip: bool = False) -> Image.Image:
    img = strip_bg(src)
    if flip:
        img = ImageOps.mirror(img)
    bbox = img.getbbox()
    if bbox:
        img = img.crop(bbox)
    img.thumbnail((int(FW * scale), int(FH * scale)), Image.Resampling.LANCZOS)
    canvas = Image.new("RGBA", (FW, FH), (0, 0, 0, 0))
    x = (FW - img.width) // 2
    y = max(0, FH - img.height - 4 + dy)
    canvas.alpha_composite(img, (x, y))
    return canvas


def nudge(frame: Image.Image, dx: int = 0, dy: int = 0) -> Image.Image:
    out = Image.new("RGBA", (FW, FH), (0, 0, 0, 0))
    out.alpha_composite(frame, (dx, dy))
    return out


def brightness(frame: Image.Image, factor: float) -> Image.Image:
    r, g, b, a = frame.split()
    rgb = ImageEnhance.Brightness(Image.merge("RGB", (r, g, b))).enhance(factor)
    r, g, b = rgb.split()
    return Image.merge("RGBA", (r, g, b, a))


def load_pose(key: str) -> Image.Image:
    path = ASSETS / POSES[key]
    if not path.exists():
        raise FileNotFoundError(path)
    return Image.open(path)


def build_row(base: Image.Image, row: int, frames: int) -> list[Image.Image]:
    cells: list[Image.Image] = []
    for col in range(frames):
        if row == 0:
            bob = [0, -2, -4, -2, 0, 2][col % 6]
            cell = nudge(base, 0, bob)
            if col in (2, 3):
                cell = brightness(cell, 0.98)
        elif row in (1, 2):
            bob = [0, -5, 0, -5, 0, -3, 0, -1][col % 8]
            lean = [0, 2, 4, 2, 0, 2, 4, 1][col % 8]
            if row == 2:
                lean = -lean
            cell = nudge(base, lean, bob)
        elif row == 3:
            lift = [0, -2, -5, -2][col % 4]
            sway = [0, 2, 4, 2][col % 4]
            cell = nudge(base, sway, lift)
        elif row == 4:
            lift = [2, -10, -22, -10, 2][col % 5]
            cell = nudge(base, 0, lift)
            if col == 2:
                cell = brightness(cell, 1.04)
        elif row == 5:
            shake = [0, -3, 3, -3, 3, -2, 2, 0][col % 8]
            cell = nudge(brightness(base, 0.94), shake, 2)
        elif row == 6:
            bob = [0, -1, -2, -1, 0, 1][col % 6]
            cell = nudge(base, bob, bob)
        elif row == 7:
            shim = [0, 1, 2, 1, 0, -1][col % 6]
            cell = nudge(base, shim, 0)
        else:
            bob = [0, -2, -3, -2, 0, 1][col % 6]
            cell = nudge(brightness(base, 1.02 if col % 2 == 0 else 0.98), 0, bob)
        cells.append(cell)
    return cells


def write_outputs(sheet: Image.Image, preview: Image.Image, manifest: dict) -> None:
    for dest in (OUT, PLUGIN_OUT):
        dest.mkdir(parents=True, exist_ok=True)
        sheet.save(dest / "spritesheet.png", optimize=True)
        preview.save(dest / "preview.png", optimize=True)
        (dest / "pet.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        print(f"wrote {dest}")


def main() -> None:
    idle = fit(load_pose("idle"), 0.94)
    run_r = fit(load_pose("run"), 0.9)
    run_l = fit(load_pose("run"), 0.9, flip=True)
    wave = fit(load_pose("wave"), 0.92)
    jump = fit(load_pose("jump"), 0.9)
    fail = fit(load_pose("fail"), 0.92)
    wait = fit(load_pose("wait"), 0.92)
    work = fit(load_pose("work"), 0.92)
    think = fit(load_pose("think"), 0.92)

    bases = [idle, run_r, run_l, wave, jump, fail, wait, work, think]
    sheet = Image.new("RGBA", (SHEET_W, SHEET_H), (0, 0, 0, 0))
    for row, frames in enumerate(ROW_FRAMES):
        for col, cell in enumerate(build_row(bases[row], row, frames)):
            sheet.alpha_composite(cell, (col * FW, row * FH))

    sheet = sheet.filter(ImageFilter.UnsharpMask(radius=0.5, percent=35, threshold=2))
    preview = idle.copy()

    manifest = {
        "id": "anya",
        "displayName": {"en": "Anya", "zh-CN": "Anya"},
        "spritesheet": "spritesheet.png",
        "frameWidth": FW,
        "frameHeight": FH,
        "columns": COLS,
        "rows": ROWS,
        "animations": {
            "idle": {
                "row": 0,
                "frames": 6,
                "durationsMs": [320, 140, 120, 140, 160, 360],
                "loop": True,
            },
            "runRight": {
                "row": 1,
                "frames": 8,
                "durationsMs": [80, 80, 80, 80, 80, 80, 80, 80],
                "loop": True,
            },
            "runLeft": {
                "row": 2,
                "frames": 8,
                "durationsMs": [80, 80, 80, 80, 80, 80, 80, 80],
                "loop": True,
            },
            "wave": {
                "row": 3,
                "frames": 4,
                "durationsMs": [120, 140, 160, 280],
                "loop": False,
            },
            "jump": {
                "row": 4,
                "frames": 5,
                "durationsMs": [100, 110, 160, 120, 220],
                "loop": False,
            },
            "fail": {
                "row": 5,
                "frames": 8,
                "durationsMs": [110, 110, 110, 110, 110, 110, 110, 260],
            },
            "wait": {
                "row": 6,
                "frames": 6,
                "durationsMs": [200, 200, 200, 200, 200, 320],
            },
            "work": {
                "row": 7,
                "frames": 6,
                "durationsMs": [120, 120, 120, 120, 120, 200],
            },
            "review": {
                "row": 8,
                "frames": 6,
                "durationsMs": [180, 180, 180, 180, 180, 280],
            },
        },
    }
    write_outputs(sheet, preview, manifest)
    print(f"sheet={sheet.size}")


if __name__ == "__main__":
    main()
