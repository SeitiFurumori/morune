"""Folha de cores do símbolo: cada cor no fundo claro, no escuro e como ícone de app."""
from pathlib import Path

AQUI = Path(__file__).parent
s = (AQUI / "otomori-simbolo.svg").read_text(encoding="utf-8").strip()

CORES = [
    ("Preto e branco", "#111111", "#f2f2f2", "#111111", "#ffffff", "Sem cor própria: combina com qualquer tema, como BOSS e Nike."),
    ("Roxo do Morune", "#7549f2", "#a68bff", "#7549f2", "#ffffff", "O roxo de hoje, para comparar. É o que lembrava a Deezer."),
    ("Índigo", "#4a3fd1", "#9d95ff", "#3b31b8", "#ffffff", "Roxo puxado para o azul: mais frio e mais sóbrio."),
    ("Ameixa", "#5a2a82", "#c4a3e8", "#4a1f6e", "#f1e7ff", "Roxo escuro e fechado: mais pesado, mais premium."),
    ("Violeta elétrico", "#8a3ffc", "#b88cff", "#16181d", "#a875ff", "Roxo vivo só no símbolo; o ícone do app é preto com o símbolo roxo."),
    ("Lavanda", "#6d5bb8", "#c9bdf7", "#e9e4fb", "#4b3c94", "Roxo claro e suave; ícone claro com o símbolo escuro."),
]


def cor(svg: str, c: str, px: int) -> str:
    return svg.replace("<svg ", f'<svg width="{px}" height="{px}" style="color:{c}" ', 1)


linhas = ""
for nome, claro, escuro, tile, sobre, desc in CORES:
    linhas += f"""<div class="linha"><div><b>{nome}</b><span>{desc}</span><code>{claro}</code></div>
    <div class="q claro">{cor(s, claro, 120)}</div>
    <div class="q escuro">{cor(s, escuro, 120)}</div>
    <div class="tile" style="background:{tile}">{cor(s, sobre, 96)}</div>
    <div class="barra"><div class="ic" style="background:{tile}">{cor(s, sobre, 22)}</div><div class="ic">{cor(s, escuro, 24)}</div></div></div>"""

html = f"""<!doctype html><html lang="pt-BR"><head><meta charset="utf-8"><title>Otomori — cores</title><style>
body{{margin:0;padding:24px 36px;background:#f4f3ef;font:15px/1.4 'Segoe UI',sans-serif;color:#222;width:1128px}}
h1{{font-size:22px;margin:0 0 4px}} p{{margin:0 0 14px;color:#555}}
.linha{{display:grid;grid-template-columns:300px 150px 150px 150px 1fr;gap:18px;align-items:center;padding:12px 0;border-top:1px solid #ddd}}
.linha b{{display:block;font-size:17px}} .linha span{{display:block;color:#555;font-size:13px;margin:2px 0 6px}}
code{{font-size:12px;color:#888}}
.q{{display:grid;place-items:center;width:150px;height:140px;border-radius:12px}} .claro{{background:#fff}} .escuro{{background:#16181d}}
.tile{{display:grid;place-items:center;width:140px;height:140px;border-radius:30px}}
.barra{{display:flex;gap:10px;align-items:center;background:#1f2128;border-radius:10px;padding:10px 14px;width:max-content}}
.ic{{display:grid;place-items:center;width:34px;height:34px;border-radius:8px}}
</style></head><body><h1>Otomori — cor</h1>
<p>Cada linha: no fundo claro, no fundo escuro, como ícone de app e na barra de tarefas escura (ícone de app e símbolo solto).</p>{linhas}</body></html>"""
(AQUI / "cores.html").write_text(html, encoding="utf-8")
print("ok")
