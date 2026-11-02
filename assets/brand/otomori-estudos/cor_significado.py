"""Caminhos de cor com significado (cores tradicionais japonesas), medidos com a
mesma régua de cor_metodo.py. Gera cor-significado.html."""
from pathlib import Path

from cor_metodo import BARRA_CLARA, BARRA_ESCURA, SETOR, contraste, de_hsl, matiz, rgb
import colorsys

AQUI = Path(__file__).parent
simbolo = (AQUI / "otomori-simbolo.svg").read_text(encoding="utf-8").strip()
pequeno = (AQUI / "otomori-simbolo-pequeno.svg").read_text(encoding="utf-8").strip()


def ajusta(hexa):
    """Mantém matiz e saturação da cor tradicional; acha a luz que passa 3:1 nas duas barras."""
    h, l, s = colorsys.rgb_to_hls(*rgb(hexa))
    melhor = None
    for i in range(15, 86):
        c = de_hsl(h * 360, s, i / 100)
        folga = min(contraste(c, BARRA_ESCURA), contraste(c, BARRA_CLARA))
        if melhor is None or folga > melhor[1]:
            melhor = (c, folga)
    return melhor[0]


def vizinho(hexa):
    a = matiz(hexa)
    nome, h = min(SETOR.items(), key=lambda kv: min(abs(matiz(kv[1]) - a), 360 - abs(matiz(kv[1]) - a)))
    return nome, min(abs(matiz(h) - a), 360 - abs(matiz(h) - a))


def ic(s, cor, px, fundo=None):
    return s.replace("<svg ", f'<svg width="{px}" height="{px}" style="color:{cor}" ', 1)


CAMINHOS = [
    ("A · Íris (菖蒲色 ayame-iro)", "#763568", None,
     "O roxo tradicional japonês: roxo era a cor da nobreza, tingida da raiz da planta murasaki. "
     "O tom da íris cai na faixa livre do mapa, longe de Discord e Twitch."),
    ("B · Tinta e selo (墨 sumi + 朱 shu)", "#27221F", "#FF3500",
     "O símbolo em preto de tinta, como a caligrafia, e um selo vermelhão pequeno, o carimbo que o "
     "calígrafo põe ao terminar a obra. O vermelhão fica só no selo, como acento."),
    ("C · Pinheiro (松葉色 matsuba-iro)", "#454D32", None,
     "O verde da agulha de pinheiro: liga direto ao 森. Mas cai perto de NVIDIA, Xbox e do verde do Spotify."),
]

linhas = ""
for nome, base, acento, texto in CAMINHOS:
    cor = ajusta(base) if not acento else "#111111"
    viz, dist = vizinho(base if not acento else acento)
    selo = ""
    if acento:
        selo = f'<div style="position:relative;width:120px;height:120px">{ic(simbolo, "#27221F", 120)}<div style="position:absolute;right:-4px;bottom:6px;width:22px;height:30px;border-radius:3px;background:{acento}"></div></div>'
        claro = selo
        escuro = f'<div style="position:relative;width:120px;height:120px">{ic(simbolo, "#EDE8E2", 120)}<div style="position:absolute;right:-4px;bottom:6px;width:22px;height:30px;border-radius:3px;background:{acento}"></div></div>'
        tile = f'<div class="tile" style="background:#27221F;position:relative">{ic(simbolo, "#EDE8E2", 92)}<div style="position:absolute;right:14px;bottom:14px;width:16px;height:22px;border-radius:2px;background:{acento}"></div></div>'
        medida = f"selo: {contraste(acento, BARRA_ESCURA):.2f} / {contraste(acento, BARRA_CLARA):.2f}"
        barras = "".join(f'<div class="barra" style="background:{f}">{ic(simbolo, "#EDE8E2" if f == BARRA_ESCURA else "#27221F", 24)}{ic(pequeno, "#EDE8E2" if f == BARRA_ESCURA else "#27221F", 16)}</div>' for f in (BARRA_ESCURA, BARRA_CLARA))
    else:
        claro, escuro = ic(simbolo, cor, 120), ic(simbolo, cor, 120)
        tile = f'<div class="tile" style="background:#16181d">{ic(simbolo, cor, 92)}</div>'
        medida = f"{contraste(cor, BARRA_ESCURA):.2f} escura / {contraste(cor, BARRA_CLARA):.2f} clara"
        barras = "".join(f'<div class="barra" style="background:{f}">{ic(simbolo, cor, 24)}{ic(pequeno, cor, 16)}</div>' for f in (BARRA_ESCURA, BARRA_CLARA))
    linhas += f"""<div class="linha"><div><b>{nome}</b><p>{texto}</p>
      <code>tradicional {base}{' + ' + acento if acento else ''} → usado {cor if not acento else '#27221F / #EDE8E2 + ' + acento}</code>
      <code>contraste {medida} · marca mais perto: {viz}, a {dist:.0f}°</code></div>
      <div class="q" style="background:#fff">{claro}</div><div class="q" style="background:#16181d">{escuro}</div>{tile}<div>{barras}</div></div>"""

html = f"""<!doctype html><html lang="pt-BR"><head><meta charset="utf-8"><title>Otomori — cor com significado</title><style>
body{{margin:0;padding:24px 36px;background:#f4f3ef;font:14px/1.45 'Segoe UI',sans-serif;color:#222;width:1128px}}
h1{{font-size:22px;margin:0 0 4px}} .sub{{color:#555;margin:0 0 10px}}
.linha{{display:grid;grid-template-columns:330px 150px 150px 140px 1fr;gap:16px;align-items:center;padding:14px 0;border-top:1px solid #ddd}}
.linha b{{font-size:16px}} .linha p{{margin:4px 0 6px;color:#555;font-size:13px}} code{{display:block;font-size:11.5px;color:#777;margin-top:2px}}
.q{{display:grid;place-items:center;width:150px;height:150px;border-radius:12px}}
.tile{{display:grid;place-items:center;width:140px;height:140px;border-radius:30px}}
.barra{{display:flex;gap:12px;align-items:center;padding:8px 12px;border-radius:8px;margin:4px 0;width:max-content}}
</style></head><body><h1>Otomori — cor com significado</h1>
<p class="sub">Cores tradicionais japonesas (hex da lista "Traditional colors of Japan"), com a luz ajustada para passar 3:1 nas duas barras do Windows 11, mantendo a matiz.</p>{linhas}</body></html>"""
(AQUI / "cor-significado.html").write_text(html, encoding="utf-8")
for nome, base, acento, _ in CAMINHOS:
    print(nome.encode("ascii", "replace").decode(), base, ajusta(base), round(matiz(base)), vizinho(base if not acento else acento))
