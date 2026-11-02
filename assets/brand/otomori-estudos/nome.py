"""O nome "Otomori" ao lado do símbolo, em fontes de licença aberta.
Cada nome é medido pela tinta (canvas) e ganha a mesma altura de maiúscula,
para a comparação ser justa. Gera nome.html."""
from pathlib import Path

AQUI = Path(__file__).parent
S = (AQUI / "otomori-simbolo.svg").read_text(encoding="utf-8").strip()

# (rotulo, arquivo, familia, peso, nota)
FONTES = [
    ("Inter Tight", "InterTight-wght-.ttf", 600, "Neutra e firme; a mais 'sistema'."),
    ("Manrope", "Manrope-wght-.ttf", 700, "Geométrica, cantos suaves, moderna."),
    ("Sora", "Sora-wght-.ttf", 600, "Geométrica larga, cara de tecnologia."),
    ("Outfit", "Outfit-wght-.ttf", 600, "Círculos perfeitos: os 'o' ecoam o anel."),
    ("Space Grotesk", "SpaceGrotesk-wght-.ttf", 600, "Com personalidade nos detalhes, um pouco 'tech'."),
    ("Instrument Sans", "InstrumentSans-wdth-wght-.ttf", 600, "Sóbria e polida, levemente estreita."),
    ("Instrument Serif", "InstrumentSerif-Regular.ttf", 400, "Serifa fina e elegante: contraste com o pincel."),
    ("Shippori Mincho", "ShipporiMincho-Bold.ttf", 700, "Serifa de uma família japonesa (mincho): conversa com o 森."),
    ("Zen Kaku Gothic New", "ZenKakuGothicNew-Bold.ttf", 700, "Sem serifa de uma família japonesa: limpa, com raiz no mesmo desenho."),
]

faces = "\n".join(f"@font-face{{font-family:'F{i}';src:url('fonte-nome/{a}');font-weight:100 900}}" for i, (_, a, _, _) in enumerate(FONTES))
linhas = ""
for i, (nome, _, peso, nota) in enumerate(FONTES):
    linhas += f"""<div class="linha"><div class="rot"><b>{nome}</b><span>{nota}</span></div>
      <div class="ass claro">{S.replace('<svg ', '<svg width="64" height="64" style="color:#111;display:block" ', 1)}<canvas data-f="F{i}" data-p="{peso}" data-c="#111"></canvas></div>
      <div class="ass escuro">{S.replace('<svg ', '<svg width="64" height="64" style="color:#f2f2f2;display:block" ', 1)}<canvas data-f="F{i}" data-p="{peso}" data-c="#f2f2f2"></canvas></div></div>"""

html = f"""<!doctype html><html lang="pt-BR"><head><meta charset="utf-8"><title>Otomori — fonte do nome</title><style>
{faces}
body{{margin:0;padding:24px 32px;background:#f4f3ef;font:14px/1.4 'Segoe UI',sans-serif;color:#222;width:1136px}}
h1{{font-size:20px;margin:0 0 4px}} .sub{{color:#555;margin:0 0 10px}}
.linha{{display:grid;grid-template-columns:300px 1fr 1fr;gap:16px;align-items:center;padding:8px 0;border-top:1px solid #ddd}}
.rot b{{display:block;font-size:15px}} .rot span{{color:#666;font-size:12.5px}}
.ass{{display:flex;gap:14px;align-items:center;padding:12px 18px;border-radius:12px;height:64px}} .claro{{background:#fff}} .escuro{{background:#16181d}}
</style></head><body><h1>Otomori — fonte do nome</h1>
<p class="sub">Todas com a mesma altura de letra, medida pela tinta. Fontes com licença aberta (OFL), liberadas para logo.</p>{linhas}
<script>
document.fonts.load('10px F0').then(async () => {{
  await Promise.all([...document.querySelectorAll('canvas')].map(c => document.fonts.load(`${{c.dataset.p}} 40px ${{c.dataset.f}}`)))
  for (const cv of document.querySelectorAll('canvas')) {{
    const x = cv.getContext('2d'), base = 100, fonte = `${{cv.dataset.p}} ${{base}}px ${{cv.dataset.f}}`
    x.font = fonte
    const alt = x.measureText('O').actualBoundingBoxAscent   // altura da maiuscula
    const tam = base * 40 / alt                              // maiuscula com 40 px
    x.font = `${{cv.dataset.p}} ${{tam}}px ${{cv.dataset.f}}`
    const m = x.measureText('Otomori')
    cv.width = Math.ceil(m.actualBoundingBoxRight + m.actualBoundingBoxLeft) + 4; cv.height = 64
    x.font = `${{cv.dataset.p}} ${{tam}}px ${{cv.dataset.f}}`; x.fillStyle = cv.dataset.c; x.textBaseline = 'alphabetic'
    x.fillText('Otomori', m.actualBoundingBoxLeft + 2, 52)
  }}
}})
</script></body></html>"""
(AQUI / "nome.html").write_text(html, encoding="utf-8")
print("ok")
