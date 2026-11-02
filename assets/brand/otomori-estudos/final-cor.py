"""Marca neutra (preto/branco) e o símbolo no app seguindo a cor de destaque
escolhida pela pessoa. Gera final-cor.html."""
from pathlib import Path

AQUI = Path(__file__).parent
S = (AQUI / "otomori-simbolo.svg").read_text(encoding="utf-8").strip()


def ic(cor, px):
    return S.replace("<svg ", f'<svg width="{px}" height="{px}" style="color:{cor};display:block" ', 1)


def assinatura(cor, fundo):
    return f'<div class="ass" style="background:{fundo};color:{cor}">{ic(cor, 72)}<div><b>Otomori</b><span>O som fica. O peso não.</span></div></div>'


# Os acentos que o app ja oferece (lib/cores) aproximados; o roxo seria o padrao.
ACENTOS = [("Roxo (padrão)", "#8b63f6"), ("Azul", "#5b8cff"), ("Verde", "#3ccf8e"), ("Laranja", "#f2994a"), ("Rosa", "#f06292")]
apps = "".join(f"""<div class="app"><div class="logo">{ic(c, 26)}<b style="color:{c}">Otomori</b></div><div class="it" style="background:{c}22;color:{c}">Início</div><div class="it">Buscar</div><small>{n}</small></div>""" for n, c in ACENTOS)

html = f"""<!doctype html><html lang="pt-BR"><head><meta charset="utf-8"><title>Otomori — marca neutra</title><style>
body{{margin:0;padding:24px 32px;background:#f4f3ef;font:14px/1.4 'Segoe UI',sans-serif;color:#222;width:1136px}}
h1{{font-size:20px;margin:0 0 4px}} h4{{margin:18px 0 8px;font-size:12px;color:#777;text-transform:uppercase;letter-spacing:.05em}} .sub{{color:#555;margin:0}}
.linha{{display:flex;gap:16px}}
.ass{{display:flex;gap:16px;align-items:center;padding:22px 28px;border-radius:14px;flex:1}} .ass b{{font-size:30px;display:block;line-height:1.1}} .ass span{{opacity:.75}}
.app{{background:#0d1016;color:#b0b9c4;border-radius:12px;padding:14px;width:190px}} .logo{{display:flex;gap:8px;align-items:center;margin-bottom:12px}} .logo b{{font-size:15px}}
.it{{padding:6px 8px;border-radius:6px;font-size:13px;margin-bottom:2px}} small{{display:block;margin-top:10px;color:#6b7584}}
</style></head><body><h1>Otomori — marca neutra que segue a cor de cada um</h1>
<p class="sub">Símbolo e nome sempre na mesma cor. Fora do app, preto ou branco. Dentro do app, a cor de destaque que a pessoa escolheu.</p>
<h4>A marca (site, instalador, GitHub, barra de tarefas)</h4><div class="linha">{assinatura('#111111', '#ffffff')}{assinatura('#f2f2f2', '#16181d')}</div>
<h4>Dentro do app, com a cor de cada pessoa</h4><div class="linha">{apps}</div></body></html>"""
(AQUI / "final-cor.html").write_text(html, encoding="utf-8")
print("ok")
