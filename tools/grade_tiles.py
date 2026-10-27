#!/usr/bin/env python3
"""Draws the select card's grade letters (our own art) into crates/hst/assets/grades.png: one 64-px cell per grade,
F E D C B A S left to right, then a sparkle star for S's twinkle. Chunky rounded capitals (the menu's own M PLUS Rounded
ExtraBold), a dark rim, a glossy two-tone fill in the grade's colour; S white-gold with a warm halo.
Run from the repo root: python3 tools/grade_tiles.py"""
from PIL import Image, ImageDraw, ImageFilter, ImageFont

C = 64
FONT = ImageFont.truetype("crates/hst/assets/fonts/MPLUSRounded1c-ExtraBold.ttf", 58)
# top highlight, upper half, lower half, bottom
COLOURS = {
    "F": ["#f4f4f4", "#c8c8c8", "#909090", "#5e5e5e"],
    "E": ["#f2ccff", "#c676f2", "#8c3cd2", "#581c96"],
    "D": ["#c6e6ff", "#52adff", "#2072e2", "#163ea6"],
    "C": ["#ddffb8", "#76de40", "#2fae24", "#1a7718"],
    "B": ["#ffc6b8", "#ff6150", "#e3262c", "#a3101c"],
    "A": ["#fff7b8", "#ffd02a", "#ff9d00", "#d86200"],
    "S": ["#ffffff", "#fff6da", "#f2d47e", "#b9892a"],
}


def rgb(h):
    return tuple(int(h[i:i + 2], 16) for i in (1, 3, 5))


def mix(a, b, t):
    return tuple(round(x + (y - x) * t) for x, y in zip(a, b))


def gradient(cols, top, bottom):
    """The fill: highlight → upper colour, a hard step at the middle, lower colour → bottom (the original's split)."""
    c = [rgb(h) for h in cols]
    img = Image.new("RGBA", (C, C))
    mid = (top + bottom) / 2
    for y in range(C):
        if y < mid:
            col = mix(c[0], c[1], min(max((y - top) / max(mid - top, 1), 0), 1))
        else:
            col = mix(c[2], c[3], min(max((y - mid) / max(bottom - mid, 1), 0), 1))
        ImageDraw.Draw(img).line([(0, y), (C, y)], fill=col + (255,))
    return img


def tile(letter):
    out = Image.new("RGBA", (C, C), (0, 0, 0, 0))
    mask = Image.new("L", (C, C), 0)
    d = ImageDraw.Draw(mask)
    l, t, r, b = d.textbbox((0, 0), letter, font=FONT)
    at = ((C - (r - l)) / 2 - l, (C - (b - t)) / 2 - t)
    d.text(at, letter, font=FONT, fill=255)
    top, bottom = (C - (b - t)) / 2, (C + (b - t)) / 2
    rim = Image.new("L", (C, C), 0)
    ImageDraw.Draw(rim).text(at, letter, font=FONT, fill=255, stroke_width=5)
    if letter == "S":
        halo = Image.new("L", (C, C), 0)
        ImageDraw.Draw(halo).text(at, letter, font=FONT, fill=255, stroke_width=8)
        halo = halo.filter(ImageFilter.GaussianBlur(2.5)).point(lambda v: v * 0.7)
        out.paste(Image.new("RGBA", (C, C), rgb("#ffe7a0") + (255,)), mask=halo)
    out.paste(Image.new("RGBA", (C, C), rgb("#8a6a22" if letter == "S" else "#1e1d24") + (255,)), mask=rim)
    out.paste(gradient(COLOURS[letter], top, bottom), mask=mask)
    # the gloss: a soft white band over the upper part of the letter
    gloss = Image.new("L", (C, C), 0)
    ImageDraw.Draw(gloss).ellipse([2, top - 14, C - 2, (top + bottom) / 2], fill=100)
    gloss = Image.composite(gloss, Image.new("L", (C, C), 0), mask)
    out.paste(Image.new("RGBA", (C, C), (255, 255, 255, 255)), mask=gloss)
    return out


def star():
    """A four-point sparkle, white with a gold edge."""
    big = 4 * C
    img = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    c, r, w = big / 2, big / 2 - 8, big / 9
    pts = [(c, c - r), (c + w, c - w), (c + r, c), (c + w, c + w), (c, c + r), (c - w, c + w), (c - r, c), (c - w, c - w)]
    d.polygon(pts, fill=rgb("#e8b93c") + (255,))
    inner = [(c + (x - c) * 0.8, c + (y - c) * 0.8) for x, y in pts]
    d.polygon(inner, fill=(255, 255, 255, 255))
    return img.resize((C, C), Image.LANCZOS)


sheet = Image.new("RGBA", (8 * C, C), (0, 0, 0, 0))
for i, letter in enumerate("FEDCBAS"):
    sheet.paste(tile(letter), (i * C, 0))
sheet.paste(star(), (7 * C, 0))
sheet.save("crates/hst/assets/grades.png", optimize=True)
