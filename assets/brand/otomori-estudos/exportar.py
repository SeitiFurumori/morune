"""Monta a folha do símbolo e os HTMLs de exportação em PNG (o Edge fotografa)."""
from pathlib import Path

AQUI = Path(__file__).parent
cheio = (AQUI / "otomori-simbolo.svg").read_text(encoding="utf-8").strip()
pequeno = (AQUI / "otomori-simbolo-pequeno.svg").read_text(encoding="utf-8").strip()


def tam(svg: str, px: int) -> str:
    return svg.replace("<svg ", f'<svg width="{px}" height="{px}" ', 1)


celulas = ""
for nome, s, tamanhos in [("Completo", cheio, [256, 128, 64, 32]), ("Pequeno (barra de tarefas)", pequeno, [32, 16])]:
    claro = "".join(f'<div class="c">{tam(s, p)}<span>{p}</span></div>' for p in tamanhos)
    escuro = "".join(f'<div class="c e">{tam(s, p)}<span>{p}</span></div>' for p in tamanhos)
    celulas += f"<h2>{nome}</h2><div class='l'>{claro}</div><div class='l'>{escuro}</div>"

folha = f"""<!doctype html><html lang="pt-BR"><head><meta charset="utf-8"><title>Otomori — símbolo</title><style>
body{{margin:0;padding:28px 40px;background:#f4f3ef;font:15px/1.4 'Segoe UI',system-ui,sans-serif;color:#222;width:1120px}}
h1{{font-size:24px;margin:0}} p{{margin:4px 0 8px;color:#555}} h2{{font-size:16px;margin:22px 0 8px}}
.l{{display:flex;gap:28px;align-items:end;margin-bottom:10px}}
.c{{display:grid;justify-items:center;gap:6px;color:#111;padding:12px;border-radius:10px;background:#fff}}
.c.e{{background:#16181d;color:#f2f2f2}} .c span{{font-size:11px;color:#888}}
</style></head><body><h1>Otomori</h1><p>O som fica. O peso não. — 森 da Yuji Mai (licença OFL) dentro do anel que pulsa.</p>{celulas}</body></html>"""
(AQUI / "otomori-simbolo.html").write_text(folha, encoding="utf-8")

png = AQUI / "png"
png.mkdir(exist_ok=True)
for nome, s, tamanhos in [("otomori", cheio, [512, 256, 128, 64]), ("otomori-pequeno", pequeno, [32, 16])]:
    for p in tamanhos:
        (png / f"{nome}-{p}.html").write_text(
            f"<!doctype html><html><body style='margin:0;background:transparent;color:#111'>{tam(s, p)}</body></html>",
            encoding="utf-8")
print("ok")
