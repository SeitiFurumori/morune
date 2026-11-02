"""Cor do Otomori por método: mapa de matizes do setor, contraste WCAG nas duas
barras de tarefas do Windows 11 e teste em 24/16 px. Gera cor-metodo.html."""
import colorsys
import math
from pathlib import Path

AQUI = Path(__file__).parent

# Cores de marca (levantamento de 02/11/2026; só SoundCloud de fonte oficial).
SETOR = {
    "Spotify": "#1DB954", "YouTube Music": "#FF0033", "Deezer": "#A238FF", "Amazon Music": "#0DBFF5",
    "SoundCloud": "#FF5500", "Last.fm": "#D51007", "Discord": "#5865F2", "Twitch": "#9146FF",
    "Xbox": "#107C10", "NVIDIA": "#76B900", "Riot": "#D13639", "Morune (hoje)": "#7549F2",
}
BARRA_ESCURA, BARRA_CLARA = "#202020", "#F3F3F3"  # barra de tarefas do Windows 11


def rgb(h):
    h = h.lstrip("#")
    return [int(h[i:i + 2], 16) / 255 for i in (0, 2, 4)]


def lum(h):
    c = [x / 12.92 if x <= 0.04045 else ((x + 0.055) / 1.055) ** 2.4 for x in rgb(h)]
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]


def contraste(a, b):
    la, lb = sorted([lum(a), lum(b)], reverse=True)
    return (la + 0.05) / (lb + 0.05)


def matiz(h):
    return colorsys.rgb_to_hls(*rgb(h))[0] * 360


def de_hsl(hue, sat, luz):
    r, g, b = colorsys.hls_to_rgb(hue / 360, luz, sat)
    return "#%02X%02X%02X" % tuple(round(x * 255) for x in (r, g, b))


def melhor_tom(hue, sat):
    """O tom dessa matiz que passa 3:1 (WCAG 1.4.11, objetos gráficos) nas duas
    barras com a maior folga mínima."""
    melhor = None
    for i in range(20, 81):
        c = de_hsl(hue, sat, i / 100)
        folga = min(contraste(c, BARRA_ESCURA), contraste(c, BARRA_CLARA))
        if melhor is None or folga > melhor[1]:
            melhor = (c, folga)
    return melhor


CAMINHOS = [
    ("Orquídea", 300, 0.62, "Vizinha do roxo, mas numa faixa que nenhuma marca da lista usa (entre Twitch, 264°, e YouTube, 352°)."),
    ("Âmbar", 38, 0.85, "Faixa livre entre SoundCloud (20°) e NVIDIA (82°). Quente: contraria o 'um pouco frio' da identidade."),
    ("Azul-aço", 212, 0.55, "Faixa livre entre Amazon Music (194°) e Discord (235°). Frio e sóbrio, mas azul é a cor mais comum de software."),
]

# --- mapa ---
def ponto(a, r):
    t = math.radians(a - 90)
    return 200 + r * math.cos(t), 200 + r * math.sin(t)


roda = ""
for i in range(360):
    x1, y1 = ponto(i, 150); x2, y2 = ponto(i + 1.2, 150)
    roda += f'<path d="M200 200 L{x1:.1f} {y1:.1f} L{x2:.1f} {y2:.1f} Z" fill="{de_hsl(i, 0.7, 0.55)}" opacity=".35"/>'
for nome, h in SETOR.items():
    a = matiz(h); x, y = ponto(a, 150); tx, ty = ponto(a, 178)
    roda += f'<circle cx="{x:.1f}" cy="{y:.1f}" r="9" fill="{h}" stroke="#fff" stroke-width="2"/>'
    roda += f'<text x="{tx:.1f}" y="{ty:.1f}" font-size="11" text-anchor="middle" dominant-baseline="central">{nome}</text>'
for nome, hue, sat, _ in CAMINHOS:
    c, _f = melhor_tom(hue, sat); x, y = ponto(hue, 110)
    roda += f'<circle cx="{x:.1f}" cy="{y:.1f}" r="11" fill="{c}" stroke="#111" stroke-width="2"/>'
    roda += f'<text x="{x:.1f}" y="{y + 22:.1f}" font-size="11" font-weight="700" text-anchor="middle">{nome}</text>'
mapa = f'<svg viewBox="-40 -10 480 420" width="520">{roda}<circle cx="200" cy="200" r="60" fill="#f4f3ef"/></svg>'

# --- tabela + teste ---
simbolo = (AQUI / "otomori-simbolo.svg").read_text(encoding="utf-8").strip()
pequeno = (AQUI / "otomori-simbolo-pequeno.svg").read_text(encoding="utf-8").strip()


def icone(s, cor, px):
    return s.replace("<svg ", f'<svg width="{px}" height="{px}" style="color:{cor}" ', 1)


linhas = ""
for nome, hue, sat, porque in CAMINHOS:
    c, folga = melhor_tom(hue, sat)
    vizinho = min(SETOR.items(), key=lambda kv: min(abs(matiz(kv[1]) - hue), 360 - abs(matiz(kv[1]) - hue)))
    dist = min(abs(matiz(vizinho[1]) - hue), 360 - abs(matiz(vizinho[1]) - hue))
    barras = ""
    for fundo in (BARRA_ESCURA, BARRA_CLARA):
        barras += f'<div class="barra" style="background:{fundo}">{icone(simbolo, c, 24)}{icone(pequeno, c, 16)}{icone(simbolo, c, 48)}</div>'
    linhas += f"""<tr><td><b>{nome}</b><br><span>{porque}</span></td><td><div class="am" style="background:{c}"></div><code>{c}</code></td>
    <td>{contraste(c, BARRA_ESCURA):.2f} : 1</td><td>{contraste(c, BARRA_CLARA):.2f} : 1</td>
    <td>{vizinho[0]}<br><span>{dist:.0f}° de distância</span></td><td>{barras}</td></tr>"""

html = f"""<!doctype html><html lang="pt-BR"><head><meta charset="utf-8"><title>Otomori — cor por método</title><style>
body{{margin:0;padding:24px 36px;background:#f4f3ef;font:14px/1.4 'Segoe UI',sans-serif;color:#222;width:1128px}}
h1{{font-size:22px;margin:0 0 4px}} p{{margin:0 0 10px;color:#555}}
.topo{{display:grid;grid-template-columns:520px 1fr;gap:24px;align-items:center}}
table{{border-collapse:collapse;width:100%;margin-top:8px}} td,th{{padding:10px 8px;border-top:1px solid #ddd;vertical-align:middle;text-align:left}}
th{{font-size:12px;color:#777;font-weight:600}} span{{color:#666;font-size:12px}}
.am{{width:44px;height:44px;border-radius:10px;margin-bottom:4px}} code{{font-size:12px}}
.barra{{display:flex;gap:12px;align-items:center;padding:8px 12px;border-radius:8px;margin:3px 0;width:max-content}}
</style></head><body><h1>Otomori — cor por método</h1>
<div class="topo">{mapa}<div><p><b>Como ler.</b> Cada bolinha de fora é a cor de uma marca do setor (apps de música e o que quem joga usa ao lado), na posição da sua matiz. As de dentro são os três caminhos, nas faixas que ninguém da lista ocupa.</p>
<p><b>Faixa do roxo (235° a 272°):</b> Discord, Twitch, Deezer e o Morune de hoje estão todos ali.</p>
<p><b>Contraste:</b> o tom de cada caminho foi calculado para passar 3:1 (o mínimo da WCAG para ícones e gráficos) na barra de tarefas escura (#202020) e na clara (#F3F3F3) do Windows 11 ao mesmo tempo.</p>
<p><b>Fontes:</b> só o SoundCloud tem cor de fonte oficial; as outras são de sites de terceiros, boas para a posição no círculo, não para o hex exato.</p></div></div>
<table><tr><th>Caminho</th><th>Cor</th><th>Barra escura</th><th>Barra clara</th><th>Marca mais perto</th><th>Na barra: 24 px, 16 px (só anel), 48 px</th></tr>{linhas}</table>
</body></html>"""
(AQUI / "cor-metodo.html").write_text(html, encoding="utf-8")
for nome, hue, sat, _ in CAMINHOS:
    c, f = melhor_tom(hue, sat)
    print(nome, c, f"{contraste(c, BARRA_ESCURA):.2f}", f"{contraste(c, BARRA_CLARA):.2f}")
for n, h in SETOR.items():
    print(n, round(matiz(h)))
