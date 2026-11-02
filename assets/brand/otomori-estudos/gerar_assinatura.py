"""Assinatura do Otomori: símbolo + nome em Outfit 600 (OFL) convertido em
desenho. Gera otomori-assinatura.svg (horizontal) e otomori-assinatura-frase.svg
(com "O som fica. O peso não." em Outfit 400)."""
from pathlib import Path

from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

AQUI = Path(__file__).parent
VAR = AQUI / "fonte-nome" / "Outfit-wght-.ttf"


def instancia(peso: int) -> TTFont:
    return instantiateVariableFont(TTFont(VAR), {"wght": peso})


def texto(ft: TTFont, s: str, altura_maiuscula: float, x: float, linha_base: float) -> tuple[str, float]:
    """Desenha `s` com a maiúscula na altura pedida; devolve o caminho e a largura."""
    gs, cmap, hmtx = ft.getGlyphSet(), ft.getBestCmap(), ft["hmtx"]
    caixa = BoundsPen(gs)
    gs[cmap[ord("O")]].draw(caixa)
    esc = altura_maiuscula / caixa.bounds[3]
    caneta = SVGPathPen(gs, ntos=lambda n: f"{n:.2f}")
    cursor = 0.0
    kern = {}
    if "kern" in ft:
        for t in ft["kern"].kernTables:
            kern.update(t.kernTable)
    nomes = [cmap[ord(c)] for c in s]
    for i, g in enumerate(nomes):
        gs[g].draw(TransformPen(caneta, (esc, 0, 0, -esc, x + cursor * esc, linha_base)))
        cursor += hmtx[g][0]
        if i + 1 < len(nomes):
            cursor += kern.get((g, nomes[i + 1]), 0)
    return caneta.getCommands(), cursor * esc


def simbolo_paths() -> list[str]:
    s = (AQUI / "otomori-simbolo.svg").read_text(encoding="utf-8")
    import re
    return re.findall(r'd="([^"]+)"', s)


def assinatura(com_frase: bool) -> str:
    # Símbolo 512 escalado para 120 de altura; nome com maiúscula de 56.
    lado, h = 120, 56
    grupo = "".join(f'<path fill-rule="evenodd" d="{d}"/>' for d in simbolo_paths())
    sim = f'<g transform="scale({lado / 512})">{grupo}</g>'
    x0 = lado + 28
    base = lado / 2 + h / 2 - (8 if com_frase else 0)
    nome, larg = texto(instancia(600), "Otomori", h, x0, base)
    corpo = f'{sim}<path d="{nome}"/>'
    largura = x0 + larg
    if com_frase:
        frase, lf = texto(instancia(400), "O som fica. O peso não.", 15, x0 + 2, base + 30)
        corpo += f'<path d="{frase}" opacity=".72"/>'
        largura = max(largura, x0 + lf)
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {largura + 4:.0f} {lado}" '
            f'fill="currentColor">{corpo}</svg>\n')


if __name__ == "__main__":
    (AQUI / "otomori-assinatura.svg").write_text(assinatura(False), encoding="utf-8")
    (AQUI / "otomori-assinatura-frase.svg").write_text(assinatura(True), encoding="utf-8")
    print("ok")
