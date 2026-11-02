"""A (íris) e B (tinta + selo) nos lugares reais: barra de tarefas, topo do app
e página de download. Gera contexto.html."""
from pathlib import Path

AQUI = Path(__file__).parent
S = (AQUI / "otomori-simbolo.svg").read_text(encoding="utf-8").strip()
IRIS, SUMI, PAPEL, SHU = "#B55AA1", "#27221F", "#EDE8E2", "#FF3500"


def ic(cor, px):
    return S.replace("<svg ", f'<svg width="{px}" height="{px}" style="color:{cor};display:block" ', 1)


def marca(caminho, cor_fundo_escuro, px):
    """O símbolo de cada caminho; no B, com o selo ao lado."""
    if caminho == "A":
        return ic(IRIS, px)
    cor = PAPEL if cor_fundo_escuro else SUMI
    s = max(4, round(px * 0.18))
    return (f'<span style="position:relative;display:inline-block;width:{px}px;height:{px}px">{ic(cor, px)}'
            f'<span style="position:absolute;right:{-s // 3}px;bottom:{s // 3}px;width:{s}px;height:{round(s * 1.35)}px;border-radius:2px;background:{SHU}"></span></span>')


def barra(caminho):
    outros = "".join(f'<span class="out" style="background:{c}"></span>' for c in ("#3a7bd5", "#5865F2", "#1b2838", "#e8e8e8", "#e94e1b"))
    return f'<div class="tb">{outros}<span class="ativo">{marca(caminho, True, 24)}</span><span class="out" style="background:#2e7d32"></span></div>'


def app(caminho):
    return f"""<div class="app"><div class="lado"><div class="logo">{marca(caminho, True, 26)}<b>Otomori</b></div>
      <div class="it sel">Início</div><div class="it">Buscar</div><div class="it">Biblioteca</div></div>
      <div class="conteudo"><h3>Boa noite</h3><div class="cards"><i></i><i></i><i></i><i></i></div></div></div>"""


def site(caminho):
    destaque = IRIS if caminho == "A" else SUMI
    return f"""<div class="site"><div>{marca(caminho, False, 96)}</div><div><h2>Otomori</h2><p>O som fica. O peso não.</p>
      <span class="bt" style="background:{destaque}">Baixar para Windows</span></div></div>"""


col = ""
for c, nome, txt in [("A", "A · Íris", "O símbolo é a cor. Aparece de longe na barra; o app e o site ganham uma cor de assinatura."),
                     ("B", "B · Tinta e selo", "O símbolo é preto ou papel; a cor é só o selo. Sóbrio como BOSS e Nike; o vermelho é o único detalhe.")]:
    col += f"""<div class="col"><h1>{nome}</h1><p class="sub">{txt}</p>
      <h4>Barra de tarefas</h4>{barra(c)}<h4>Topo do app</h4>{app(c)}<h4>Página de download</h4>{site(c)}</div>"""

html = f"""<!doctype html><html lang="pt-BR"><head><meta charset="utf-8"><title>Otomori — A ou B</title><style>
body{{margin:0;padding:24px 32px;background:#f4f3ef;font:14px/1.4 'Segoe UI',sans-serif;color:#222;width:1136px;display:grid;grid-template-columns:1fr 1fr;gap:32px}}
h1{{font-size:20px;margin:0}} .sub{{color:#555;margin:4px 0 6px;min-height:40px}} h4{{margin:14px 0 6px;font-size:12px;color:#777;text-transform:uppercase;letter-spacing:.05em}}
.tb{{display:flex;gap:14px;align-items:center;justify-content:center;background:#202020;border-radius:8px;padding:10px}}
.out{{width:24px;height:24px;border-radius:6px;display:block;opacity:.9}} .ativo{{padding:4px;border-radius:6px;background:#ffffff14;border-bottom:2px solid #9aa0a6}}
.app{{display:grid;grid-template-columns:150px 1fr;height:170px;border-radius:12px;overflow:hidden;background:#12151d;color:#eef1f5}}
.lado{{background:#0d1016;padding:12px}} .logo{{display:flex;gap:8px;align-items:center;margin-bottom:12px}} .logo b{{font-size:15px}}
.it{{padding:6px 8px;border-radius:6px;font-size:13px;color:#b0b9c4}} .it.sel{{background:#ffffff14;color:#fff}}
.conteudo{{padding:12px 16px}} .conteudo h3{{margin:0 0 10px}} .cards{{display:flex;gap:10px}} .cards i{{width:70px;height:70px;border-radius:8px;background:#232834}}
.site{{display:flex;gap:22px;align-items:center;background:#fff;border-radius:12px;padding:22px}} .site h2{{margin:0;font-size:28px}} .site p{{margin:2px 0 12px;color:#555}}
.bt{{display:inline-block;color:#fff;padding:9px 16px;border-radius:999px;font-weight:600}}
</style></head><body>{col}</body></html>"""
(AQUI / "contexto.html").write_text(html, encoding="utf-8")
print("ok")
