"""Genera los másteres del kit de marca finanzasVZ (sin dependencias de red).

Requiere fontTools (solo para outlinear el wordmark): python3 -m pip install fonttools
El símbolo es un contorno relleno: tres barras ascendentes, la última con punta
de flecha. Se propaga a Android y a la app desde este mismo path.
"""

from __future__ import annotations

from pathlib import Path

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont

HERE = Path(__file__).parent
FONT_SEMI = Path("/usr/share/fonts/open-sans/OpenSans-Semibold.ttf")
FONT_BOLD = Path("/usr/share/fonts/open-sans/OpenSans-Bold.ttf")

TEAL = "#2DD4A7"
INK = "#0F1420"
CLOUD = "#EEF2FB"


SYMBOL = (
    "M 58 136 H 90 A 14 14 0 0 1 104 150 V 202 A 14 14 0 0 1 90 216 "
    "H 58 A 14 14 0 0 1 44 202 V 150 A 14 14 0 0 1 58 136 Z "
    "M 118 96 H 150 A 14 14 0 0 1 164 110 V 202 A 14 14 0 0 1 150 216 "
    "H 118 A 14 14 0 0 1 104 202 V 110 A 14 14 0 0 1 118 96 Z "
    "M 178 76 H 210 A 14 14 0 0 1 224 90 V 202 A 14 14 0 0 1 210 216 "
    "H 178 A 14 14 0 0 1 164 202 V 90 A 14 14 0 0 1 178 76 Z "
    "M 192 18 L 226 74 L 158 74 Z"
)
SYMBOL_SMALL = SYMBOL


def text_width(font: TTFont, text: str, size: float, tracking: float = 0.0) -> float:
    upm = font["head"].unitsPerEm
    cmap = font.getBestCmap()
    hmtx = font["hmtx"]
    total = 0.0
    for char in text:
        glyph = cmap[ord(char)]
        total += hmtx[glyph][0] / upm * size + tracking
    return total - tracking if text else 0.0


def outline_text(font_path: Path, text: str, size: float, x: float, baseline: float,
                 color: str, tracking: float = 0.0) -> tuple[str, float]:
    font = TTFont(font_path)
    upm = font["head"].unitsPerEm
    cmap = font.getBestCmap()
    hmtx = font["hmtx"]
    glyph_set = font.getGlyphSet()
    scale = size / upm

    pen = SVGPathPen(glyph_set, ntos=lambda n: f"{n:.2f}")
    cursor = x
    for char in text:
        glyph_name = cmap[ord(char)]
        transform = TransformPen(pen, (scale, 0, 0, -scale, cursor, baseline))
        glyph_set[glyph_name].draw(transform)
        cursor += hmtx[glyph_name][0] * scale + tracking

    width = cursor - x - (tracking if text else 0)
    return f'<path d="{pen.getCommands()}" fill="{color}"/>', width


def svg(body: str, view_box: str, title: str) -> str:
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{view_box}" role="img" aria-label="{title}">\n'
        f"  <title>{title}</title>\n{body}\n</svg>\n"
    )


def symbol_mark(color: str, transform: str = "") -> str:
    t = f' transform="{transform}"' if transform else ""
    return f'  <path d="{SYMBOL}" fill="{color}"{t}/>'


def write(name: str, content: str) -> None:
    (HERE / name).write_text(content, encoding="utf-8")
    print("escrito", name)


def main() -> None:
    write("financevz-symbol.svg", svg(
        f'  <path d="{SYMBOL}" fill="{TEAL}"/>', "0 0 256 256", "finanzasVZ — símbolo"))
    write("financevz-symbol-small.svg", svg(
        f'  <path d="{SYMBOL_SMALL}" fill="{TEAL}"/>', "0 0 256 256", "finanzasVZ — símbolo pequeño"))

    semi = TTFont(FONT_SEMI)
    bold = TTFont(FONT_BOLD)
    size = 92.0
    tracking = -0.6
    fin_w = text_width(semi, "finanzas", size, tracking)
    vz_w = text_width(bold, "VZ", size, tracking)

    def wordmark(x: float, baseline: float, fin_color: str, vz_color: str) -> tuple[str, float]:
        a, _ = outline_text(FONT_SEMI, "finanzas", size, x, baseline, fin_color, tracking)
        b, _ = outline_text(FONT_BOLD, "VZ", size, x + fin_w + 2, baseline, vz_color, tracking)
        return f"  {a}\n  {b}", fin_w + 2 + vz_w

    # Horizontal: símbolo + wordmark
    text_x = 172.0
    baseline = 112.0
    word, _ = wordmark(text_x, baseline, INK, TEAL)
    write("financevz-horizontal.svg", svg(
        symbol_mark(TEAL, "translate(6 6) scale(0.53)") + "\n" + word,
        f"0 15 {int(text_x + fin_w + 2 + vz_w + 16)} 118", "finanzasVZ — lockup horizontal"))

    # Vertical / apilado
    word2, w2 = wordmark(0, 0, INK, TEAL)
    write("financevz-stacked.svg", svg(
        symbol_mark(TEAL, "translate(160 0) scale(0.55)") + "\n"
        + f'  <g transform="translate({(460 - w2) / 2:.2f} 232)">{word2}</g>',
        "0 0 460 300", "finanzasVZ — lockup apilado"))

    # Wordmark solo
    word3, w3 = wordmark(0, 0, INK, TEAL)
    write("financevz-wordmark.svg", svg(
        f'  <g transform="translate(10 78)">{word3}</g>',
        f"0 7 {int(w3 + 20)} 80", "finanzasVZ — wordmark"))

    # Lockups sobre fondo oscuro (texto claro)
    word_dark, wd = wordmark(text_x, baseline, CLOUD, TEAL)
    write("financevz-horizontal-dark.svg", svg(
        symbol_mark(TEAL, "translate(6 6) scale(0.53)") + "\n" + word_dark,
        f"0 15 {int(text_x + wd + 16)} 118", "finanzasVZ — lockup horizontal sobre fondo oscuro"))

    word_dark2, wd2 = wordmark(0, 0, CLOUD, TEAL)
    write("financevz-stacked-dark.svg", svg(
        symbol_mark(TEAL, "translate(160 0) scale(0.55)") + "\n"
        + f'  <g transform="translate({(460 - wd2) / 2:.2f} 232)">{word_dark2}</g>',
        "0 0 460 300", "finanzasVZ — lockup apilado sobre fondo oscuro"))


if __name__ == "__main__":
    main()
