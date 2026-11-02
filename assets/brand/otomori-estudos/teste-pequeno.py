"""Mostra, em pixels reais ampliados, o símbolo em 16, 24 e 32 px, ao lado de
três arcos como os do Spotify, e uma versão com o 森 maior para tamanho pequeno."""
import math
from pathlib import Path

from gerar_simbolo import anel, mori, svg

AQUI = Path(__file__).parent
fonte = AQUI / "fonte-pincel" / "YujiMai-Regular.ttf"
import gerar_simbolo as g

normal = svg(anel(), mori(fonte))
# Para tamanho pequeno: anel mais fino e 森 ocupando mais do meio.
g.LADO = 300
grande = svg(anel(212, 222, 26), mori(fonte))
# Tres arcos grossos, como o icone do Spotify (so para comparar a quantidade de tracos).
arcos = ('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512"><circle cx="256" cy="256" r="240" fill="#1db954"/>'
         '<g fill="none" stroke="#000" stroke-linecap="round"><path stroke-width="44" d="M130 190 Q256 140 390 205"/>'
         '<path stroke-width="36" d="M145 265 Q256 225 370 280"/><path stroke-width="30" d="M160 335 Q256 305 350 350"/></g></svg>')

linhas = ""
for nome, s in [("Três arcos (como o Spotify)", arcos), ("Otomori completo", normal), ("Otomori, 森 maior para tela pequena", grande)]:
    cel = ""
    for p in (16, 24, 32):
        real = s.replace("<svg ", f'<svg width="{p}" height="{p}" ', 1)
        cel += f'<div class="c"><div class="real">{real}</div><canvas data-p="{p}" width="{p*8}" height="{p*8}"></canvas><span>{p} px (ampliado 8×)</span></div>'
    linhas += f"<h2>{nome}</h2><div class='l'>{cel}</div>"

html = f"""<!doctype html><html><head><meta charset="utf-8"><style>
body{{margin:0;padding:24px 36px;background:#f4f3ef;font:15px 'Segoe UI',sans-serif;color:#222;width:1120px}}
h2{{font-size:16px;margin:16px 0 8px}} .l{{display:flex;gap:30px;align-items:end}}
.c{{display:grid;justify-items:center;gap:6px}} .c span{{font-size:12px;color:#777}}
.real{{background:#fff;padding:6px;color:#111}} canvas{{image-rendering:pixelated;background:#fff;border:1px solid #ddd}}
</style></head><body>{linhas}
<script>
// Amplia os pixels reais: desenha o SVG no tamanho de verdade e estica sem suavizar.
for (const cv of document.querySelectorAll('canvas')) {{
  const p = +cv.dataset.p, svg = cv.previousElementSibling.querySelector('svg')
  const img = new Image(), src = new XMLSerializer().serializeToString(svg).replace('fill="currentColor"', 'fill="#111"')
  img.onload = () => {{ const t = document.createElement('canvas'); t.width = t.height = p
    const c = t.getContext('2d'); c.fillStyle = '#fff'; c.fillRect(0, 0, p, p); c.drawImage(img, 0, 0, p, p)
    const x = cv.getContext('2d'); x.imageSmoothingEnabled = false; x.drawImage(t, 0, 0, p * 8, p * 8) }}
  img.src = 'data:image/svg+xml,' + encodeURIComponent(src)
}}
</script></body></html>"""
(AQUI / "teste-pequeno.html").write_text(html, encoding="utf-8")
print("ok")
