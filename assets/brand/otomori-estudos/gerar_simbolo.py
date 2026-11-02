"""Gera o símbolo do Otomori: o 森 da Yuji Mai (OFL) convertido em desenho,
dentro do anel que pulsa. Saída: otomori-simbolo.svg (completo) e
otomori-simbolo-pequeno.svg (só o anel, para 16 e 32 px)."""
import math
from pathlib import Path

from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont

AQUI = Path(__file__).parent
C = 256
LADO = 228  # o 森 cabe num quadrado deste tamanho, no centro


def mori(fonte: Path) -> str:
    ft = TTFont(fonte)
    nome = ft.getBestCmap()[ord("森")]
    glifo = ft.getGlyphSet()[nome]
    caixa = BoundsPen(ft.getGlyphSet())
    glifo.draw(caixa)
    x0, y0, x1, y1 = caixa.bounds
    s = LADO / max(x1 - x0, y1 - y0)
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    # Fonte tem o y para cima; SVG, para baixo.
    caneta = SVGPathPen(ft.getGlyphSet(), ntos=lambda n: f"{n:.1f}")
    glifo.draw(TransformPen(caneta, (s, 0, 0, -s, C - cx * s, C + cy * s)))
    return caneta.getCommands()


def anel(dentro: float = 186, base: float = 200, amp: float = 40) -> str:
    pts = []
    for i in range(481):
        a = i / 480 * math.tau - math.pi / 2
        v = 0.5 + 0.3 * math.sin(3 * a + 0.4) + 0.2 * math.sin(5 * a + 1.9)
        r = base + amp * max(0.0, v)
        pts.append(f"{C + math.cos(a) * r:.1f} {C + math.sin(a) * r:.1f}")
    fora = "M" + " L".join(pts) + " Z"
    d = dentro
    furo = f"M{C + d} {C} A{d} {d} 0 1 0 {C - d} {C} A{d} {d} 0 1 0 {C + d} {C} Z"
    return f"{fora} {furo}"


def svg(*caminhos: str) -> str:
    corpo = "".join(f'<path fill-rule="evenodd" d="{c}"/>' for c in caminhos)
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" fill="currentColor">{corpo}</svg>\n'


if __name__ == "__main__":
    k = mori(AQUI / "fonte-pincel" / "YujiMai-Regular.ttf")
    (AQUI / "otomori-simbolo.svg").write_text(svg(anel(), k), encoding="utf-8")
    (AQUI / "otomori-simbolo-pequeno.svg").write_text(svg(anel(146, 192, 52)), encoding="utf-8")
    print("ok", len(k))
