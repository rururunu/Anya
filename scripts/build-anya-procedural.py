"""Procedural humanoid Anya spritesheet (ChatGPT/Codex atlas: 1536×1872, 8×9, 192×208)."""
from __future__ import annotations

import json
import math
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "public" / "pets" / "anya"
PLUGIN_OUT = ROOT / "src-tauri" / "plugins" / "pet-gallery" / "ui" / "pets" / "anya"
FW, FH = 192, 208
COLS, ROWS = 8, 9
SHEET_W, SHEET_H = FW * COLS, FH * ROWS
ROW_FRAMES = [6, 8, 8, 4, 5, 8, 6, 6, 6]

# Soft cyan humanoid + warm hair accent + gold crescent pin
C_SKIN = (255, 228, 210, 255)
C_SKIN_SHADE = (235, 190, 170, 255)
C_HAIR = (92, 150, 230, 255)
C_HAIR_DARK = (62, 110, 190, 255)
C_HAIR_LIGHT = (150, 200, 255, 255)
C_EYE = (36, 52, 96, 255)
C_EYE_SHINE = (255, 255, 255, 255)
C_BLUSH = (255, 150, 170, 100)
C_MOUTH = (180, 110, 120, 255)
C_DRESS = (126, 188, 255, 255)
C_DRESS_DARK = (80, 140, 220, 255)
C_DRESS_LIGHT = (190, 225, 255, 255)
C_SOCK = (230, 240, 255, 255)
C_SHOE = (70, 110, 180, 255)
C_CRESCENT = (255, 210, 90, 255)
C_CRESCENT_IN = (255, 236, 160, 255)
C_SPARK = (255, 240, 160, 220)
C_ERR = (255, 130, 140, 255)


def ellipse(draw: ImageDraw.ImageDraw, cx: float, cy: float, rx: float, ry: float, fill) -> None:
    draw.ellipse([cx - rx, cy - ry, cx + rx, cy + ry], fill=fill)


def draw_star(draw: ImageDraw.ImageDraw, cx: float, cy: float, r: float, fill) -> None:
    pts = []
    for i in range(8):
        ang = -math.pi / 2 + i * math.pi / 4
        rad = r if i % 2 == 0 else r * 0.38
        pts.append((cx + math.cos(ang) * rad, cy + math.sin(ang) * rad))
    draw.polygon(pts, fill=fill)


def paste_crescent(target: Image.Image, cx: float, cy: float, r: float, rot: float = -0.4) -> None:
    layer = Image.new("RGBA", target.size, (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    ellipse(d, cx, cy, r, r, C_CRESCENT)
    hi_cx = cx + math.cos(rot - 0.8) * (r * 0.25)
    hi_cy = cy + math.sin(rot - 0.8) * (r * 0.25)
    ellipse(d, hi_cx, hi_cy, r * 0.32, r * 0.32, C_CRESCENT_IN)
    hole = Image.new("L", target.size, 0)
    hd = ImageDraw.Draw(hole)
    hx = cx + math.cos(rot + 0.9) * (r * 0.4)
    hy = cy + math.sin(rot + 0.9) * (r * 0.4)
    hd.ellipse([hx - r * 0.7, hy - r * 0.7, hx + r * 0.7, hy + r * 0.7], fill=255)
    r_ch, g_ch, b_ch, a_ch = layer.split()
    layer = Image.merge("RGBA", (r_ch, g_ch, b_ch, ImageChops.subtract(a_ch, hole)))
    target.alpha_composite(layer)


def limb(
    draw: ImageDraw.ImageDraw,
    x0: float,
    y0: float,
    x1: float,
    y1: float,
    width: float,
    fill,
) -> None:
    draw.line([(x0, y0), (x1, y1)], fill=fill, width=max(2, int(width)))
    ellipse(draw, x1, y1, width * 0.45, width * 0.45, fill)


def draw_anya(
    *,
    bob: float = 0.0,
    lean: float = 0.0,
    eye_sy: float = 1.0,
    look_x: float = 0.0,
    look_y: float = 0.0,
    mouth: str = "smile",
    arm_l: float = 0.15,
    arm_r: float = 0.15,
    leg_l: float = 0.0,
    leg_r: float = 0.0,
    hair_sway: float = 0.0,
    sparkles: list[tuple[float, float, float]] | None = None,
    blush: float = 1.0,
    tint_err: float = 0.0,
    facing: int = 1,
    jump_y: float = 0.0,
) -> Image.Image:
    """Chibi humanoid Anya. arm_* 0=down … 1=raised; leg_* -1..1 stride."""
    img = Image.new("RGBA", (FW, FH), (0, 0, 0, 0))
    shadow = Image.new("RGBA", (FW, FH), (0, 0, 0, 0))
    sd = ImageDraw.Draw(shadow)
    shadow_y = 196 + jump_y * 0.15
    ellipse(sd, 96, shadow_y, 28 - jump_y * 0.2, 6, (40, 60, 100, max(18, int(50 - jump_y))))
    shadow = shadow.filter(ImageFilter.GaussianBlur(2.5))
    img.alpha_composite(shadow)

    layer = Image.new("RGBA", (FW, FH), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)

    cx = 96.0 + lean * 2
    base_y = 118.0 - bob * 3 - jump_y
    hip_y = base_y + 28
    shoulder_y = base_y + 2
    head_cx = cx
    head_cy = base_y - 28

    dress = C_DRESS
    if tint_err > 0:
        dress = tuple(int(dress[i] * (1 - tint_err) + C_ERR[i] * tint_err) for i in range(3)) + (255,)

    # Legs (behind dress)
    foot_y = hip_y + 44
    for side, stride, sign in ((-1, leg_l, -1), (1, leg_r, 1)):
        hx = cx + side * 9
        knee_x = hx + sign * stride * 12
        knee_y = hip_y + 20 - abs(stride) * 5
        fx = hx + sign * stride * 18
        fy = foot_y - abs(stride) * 3
        # Thigh
        d.line([(hx, hip_y + 2), (knee_x, knee_y)], fill=C_SKIN_SHADE, width=9)
        ellipse(d, knee_x, knee_y, 5, 5, C_SKIN_SHADE)
        # Calf + sock
        d.line([(knee_x, knee_y), (fx, fy - 5)], fill=C_SOCK, width=8)
        ellipse(d, fx, fy - 2, 9, 5, C_SHOE)

    # Dress (A-line)
    dress_pts = [
        (cx - 16, shoulder_y + 2),
        (cx + 16, shoulder_y + 2),
        (cx + 30, hip_y + 20),
        (cx - 30, hip_y + 20),
    ]
    d.polygon(dress_pts, fill=dress)
    ellipse(d, cx, shoulder_y + 6, 17, 10, dress)
    # Hem shade
    d.polygon(
        [(cx - 28, hip_y + 12), (cx + 28, hip_y + 12), (cx + 30, hip_y + 20), (cx - 30, hip_y + 20)],
        fill=C_DRESS_DARK if tint_err == 0 else dress,
    )
    ellipse(d, cx, shoulder_y + 4, 8, 4, C_DRESS_LIGHT)

    # Arms (rounded sticks with hands)
    for side, raise_amt, sign in ((-1, arm_l, -1), (1, arm_r, 1)):
        sx = cx + side * 15
        sy = shoulder_y + 10
        ang = math.radians(75 - raise_amt * 145) * sign
        length = 30
        ex = sx + math.sin(ang) * length
        ey = sy + math.cos(ang) * length
        d.line([(sx, sy), (ex, ey)], fill=C_SKIN, width=9)
        ellipse(d, sx, sy, 5, 5, C_SKIN)
        ellipse(d, ex, ey, 6, 6, C_SKIN)
        if raise_amt > 0.55:
            draw_star(d, ex + side * 5, ey - 12, 4.5, C_SPARK)

    # Neck
    d.rectangle([cx - 5, head_cy + 18, cx + 5, shoulder_y + 4], fill=C_SKIN)

    # Head
    ellipse(d, head_cx, head_cy, 26, 24, C_SKIN)
    ellipse(d, head_cx - 24, head_cy + 2, 4.5, 5.5, C_SKIN)
    ellipse(d, head_cx + 24, head_cy + 2, 4.5, 5.5, C_SKIN)

    # Hair: top bob + side tails, face left open
    hair = Image.new("RGBA", (FW, FH), (0, 0, 0, 0))
    hd = ImageDraw.Draw(hair)
    # Crown
    ellipse(hd, head_cx, head_cy - 10, 28, 18, C_HAIR)
    # Side covers (cheeks only partially)
    ellipse(hd, head_cx - 22 + hair_sway * 2, head_cy + 2, 10, 14, C_HAIR)
    ellipse(hd, head_cx + 22 + hair_sway * 2, head_cy + 2, 10, 14, C_HAIR)
    # Bangs
    ellipse(hd, head_cx - 12, head_cy - 12, 9, 7, C_HAIR_DARK)
    ellipse(hd, head_cx + 10, head_cy - 13, 8, 6, C_HAIR_DARK)
    ellipse(hd, head_cx - 1, head_cy - 16, 12, 7, C_HAIR_LIGHT)
    # Twin tails
    hd.polygon(
        [
            (head_cx - 20, head_cy + 4),
            (head_cx - 36 + hair_sway * 8, head_cy + 26),
            (head_cx - 14, head_cy + 12),
        ],
        fill=C_HAIR,
    )
    hd.polygon(
        [
            (head_cx + 20, head_cy + 4),
            (head_cx + 36 + hair_sway * 8, head_cy + 26),
            (head_cx + 14, head_cy + 12),
        ],
        fill=C_HAIR,
    )
    layer.alpha_composite(hair)

    paste_crescent(layer, head_cx + 18 + hair_sway * 2, head_cy - 24, 9, rot=-0.5 + hair_sway * 0.2)

    # Face on top of hair bangs area
    eye_ry = 5.8 * eye_sy
    lx = head_cx - 9 + look_x
    rx = head_cx + 9 + look_x
    ey = head_cy + 1 + look_y
    if eye_sy > 0.18:
        ellipse(d, lx, ey, 5.4, eye_ry, C_EYE)
        ellipse(d, rx, ey, 5.4, eye_ry, C_EYE)
        if eye_sy > 0.5:
            ellipse(d, lx - 1.6, ey - 2, 1.7, 1.9, C_EYE_SHINE)
            ellipse(d, rx - 1.6, ey - 2, 1.7, 1.9, C_EYE_SHINE)
    else:
        d.line([(lx - 5, ey), (lx + 5, ey)], fill=C_EYE, width=2)
        d.line([(rx - 5, ey), (rx + 5, ey)], fill=C_EYE, width=2)

    if blush > 0:
        bl = Image.new("RGBA", (FW, FH), (0, 0, 0, 0))
        bld = ImageDraw.Draw(bl)
        bc = (*C_BLUSH[:3], int(C_BLUSH[3] * blush))
        ellipse(bld, head_cx - 17, head_cy + 10, 5, 3.2, bc)
        ellipse(bld, head_cx + 17, head_cy + 10, 5, 3.2, bc)
        layer.alpha_composite(bl)

    my = head_cy + 13
    if mouth == "smile":
        d.arc([head_cx - 5, my - 2, head_cx + 5, my + 5], 20, 160, fill=C_MOUTH, width=2)
    elif mouth == "o":
        ellipse(d, head_cx, my + 1, 3.2, 3.2, C_MOUTH)
    elif mouth == "flat":
        d.line([(head_cx - 4, my + 1), (head_cx + 4, my + 1)], fill=C_MOUTH, width=2)
    elif mouth == "sad":
        d.arc([head_cx - 5, my, head_cx + 5, my + 7], 200, 340, fill=C_MOUTH, width=2)
    elif mouth == "grin":
        d.arc([head_cx - 7, my - 3, head_cx + 7, my + 6], 15, 165, fill=C_MOUTH, width=2)

    if facing < 0:
        layer = layer.transpose(Image.Transpose.FLIP_LEFT_RIGHT)

    img.alpha_composite(layer)

    if sparkles:
        sp = Image.new("RGBA", (FW, FH), (0, 0, 0, 0))
        spd = ImageDraw.Draw(sp)
        for sx, sy, sr in sparkles:
            draw_star(spd, sx if facing > 0 else FW - sx, sy, sr, C_SPARK)
        img.alpha_composite(sp)

    return img


def row_idle(col: int) -> Image.Image:
    phase = [0.0, 0.5, 1.0, 0.7, 0.25, -0.1][col]
    blink = 0.12 if col in (2, 3) else 1.0
    return draw_anya(
        bob=phase,
        eye_sy=blink,
        hair_sway=math.sin(col * 0.8) * 0.4,
        arm_l=0.12,
        arm_r=0.12,
        mouth="smile",
        sparkles=[(148, 42, 4)] if col % 3 == 0 else None,
    )


def row_run(col: int, left: bool) -> Image.Image:
    # Classic run cycle: opposite arm/leg
    t = col / 8 * math.pi * 2
    stride = math.sin(t)
    bob = abs(math.sin(t)) * 4
    return draw_anya(
        bob=bob * 0.15,
        lean=1.2 if not left else -1.2,
        jump_y=bob,
        leg_l=stride,
        leg_r=-stride,
        arm_l=0.25 + (-stride) * 0.35,
        arm_r=0.25 + stride * 0.35,
        hair_sway=stride * 0.6,
        look_x=-2 if left else 2,
        mouth="o" if col % 2 == 0 else "smile",
        facing=-1 if left else 1,
        sparkles=[(40, 50, 3.5)] if col % 2 == 0 else None,
    )


def row_wave(col: int) -> Image.Image:
    lift = [0.35, 0.75, 1.0, 0.55][col]
    return draw_anya(
        bob=0.3,
        arm_r=lift,
        arm_l=0.1,
        hair_sway=math.sin(col) * 0.3,
        mouth="grin",
        blush=1.2,
        sparkles=[(150, 55, 5), (42, 70, 3.5)] if col >= 1 else [(150, 55, 4)],
    )


def row_jump(col: int) -> Image.Image:
    specs = [
        dict(jump_y=0, bob=-0.4, arm_l=0.2, arm_r=0.2, mouth="o", leg_l=0.2, leg_r=0.2),
        dict(jump_y=18, bob=0.2, arm_l=0.7, arm_r=0.7, mouth="o", leg_l=-0.3, leg_r=-0.3),
        dict(jump_y=34, bob=0.3, arm_l=0.9, arm_r=0.9, mouth="grin", leg_l=-0.4, leg_r=-0.4),
        dict(jump_y=16, bob=0.1, arm_l=0.5, arm_r=0.5, mouth="o", leg_l=-0.2, leg_r=-0.2),
        dict(jump_y=0, bob=-0.3, arm_l=0.15, arm_r=0.15, mouth="smile", leg_l=0.15, leg_r=0.15),
    ][col]
    sparks = [(50, 40, 5), (140, 35, 6), (96, 28, 4)] if col == 2 else None
    return draw_anya(hair_sway=0.3 * col, sparkles=sparks, blush=1.1, **specs)


def row_fail(col: int) -> Image.Image:
    shake = [0, -4, 4, -3, 3, -2, 2, 0][col]
    return draw_anya(
        lean=shake * 0.4,
        bob=-0.2,
        mouth="sad",
        eye_sy=0.85,
        look_y=2,
        arm_l=0.05,
        arm_r=0.05,
        hair_sway=-0.5,
        tint_err=0.3,
        blush=0.4,
    )


def row_wait(col: int) -> Image.Image:
    bob = math.sin(col / 5 * math.pi * 2) * 0.5
    return draw_anya(
        bob=bob,
        hair_sway=math.sin(col * 0.7) * 0.5,
        mouth="flat",
        look_x=math.sin(col) * 3,
        arm_l=0.2,
        arm_r=0.35 + 0.1 * math.sin(col),
        sparkles=[(130, 38, 3)] if col % 2 == 0 else None,
    )


def row_work(col: int) -> Image.Image:
    tap = math.sin(col * 1.5)
    return draw_anya(
        bob=0.15,
        arm_r=0.35 + 0.25 * abs(tap),
        arm_l=0.2,
        hair_sway=tap * 0.2,
        mouth="flat" if col % 2 else "o",
        look_x=-2,
        sparkles=[(45 + col * 6, 48, 3.5), (150 - col * 4, 44, 4)],
    )


def row_review(col: int) -> Image.Image:
    bob = [0, -0.3, -0.6, -0.3, 0, 0.2][col]
    return draw_anya(
        bob=bob,
        eye_sy=1.05,
        look_y=-1,
        hair_sway=0.2 + col * 0.05,
        mouth="smile",
        arm_l=0.25,
        arm_r=0.4,
        sparkles=[(155, 40, 4), (45, 55, 3)] if col in (1, 3, 5) else None,
    )


BUILDERS = [
    row_idle,
    lambda c: row_run(c, left=False),
    lambda c: row_run(c, left=True),
    row_wave,
    row_jump,
    row_fail,
    row_wait,
    row_work,
    row_review,
]


def write_outputs(sheet: Image.Image, preview: Image.Image, manifest: dict) -> None:
    for dest in (OUT, PLUGIN_OUT):
        dest.mkdir(parents=True, exist_ok=True)
        sheet.save(dest / "spritesheet.png", optimize=True)
        preview.save(dest / "preview.png", optimize=True)
        (dest / "pet.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        print(f"wrote {dest}")


def main() -> None:
    sheet = Image.new("RGBA", (SHEET_W, SHEET_H), (0, 0, 0, 0))
    for row, frames in enumerate(ROW_FRAMES):
        builder = BUILDERS[row]
        for col in range(frames):
            sheet.alpha_composite(builder(col), (col * FW, row * FH))

    preview = BUILDERS[0](0)
    sheet = sheet.filter(ImageFilter.UnsharpMask(radius=0.7, percent=55, threshold=3))

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
