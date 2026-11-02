"""Ícone do app (.ico): símbolo branco num quadrado escuro arredondado, que
aparece na barra de tarefas clara e na escura. Do 24 px para cima, o símbolo
completo; no 16 e no 20, só o anel (o 森 vira borrão nesse tamanho).

Uso: python gerar_icone.py <msedge.exe> <pasta-temporaria>"""
import subprocess
import sys
from pathlib import Path

from PIL import Image

AQUI = Path(__file__).parent
EDGE, TMP = sys.argv[1], Path(sys.argv[2])
TMP.mkdir(parents=True, exist_ok=True)
FUNDO, TINTA = "#16181d", "#f2f2f2"


def foto(svg_arquivo: str, nome: str, escala: float) -> Image.Image:
    s = (AQUI / svg_arquivo).read_text(encoding="utf-8").strip()
    lado = round(512 * escala)
    s = s.replace("<svg ", f'<svg width="{lado}" height="{lado}" style="color:{TINTA};display:block" ', 1)
    html = TMP / f"{nome}.html"
    html.write_text(
        f"<!doctype html><html><body style='margin:0;background:transparent'>"
        f"<div style='width:512px;height:512px;border-radius:112px;background:{FUNDO};display:grid;place-items:center'>{s}</div>"
        f"</body></html>", encoding="utf-8")
    png = TMP / f"{nome}.png"
    subprocess.run([EDGE, "--headless=new", "--disable-gpu", "--hide-scrollbars", "--default-background-color=00000000",
                    f"--user-data-dir={TMP / 'perfil'}", f"--screenshot={png}", "--window-size=512,512",
                    html.as_uri()], check=True)
    return Image.open(png).convert("RGBA")


completo = foto("otomori-simbolo.svg", "completo", 0.82)
pequeno = foto("otomori-simbolo-pequeno.svg", "pequeno", 0.86)
tamanhos = [16, 20, 24, 32, 40, 48, 64, 128, 256]
imagens = [(pequeno if t < 24 else completo).resize((t, t), Image.LANCZOS) for t in tamanhos]
for destino in (AQUI.parents[2] / "crates/morune-desktop/icons/icon.ico", AQUI.parent / "otomori.ico"):
    imagens[-1].save(destino, format="ICO", sizes=[(t, t) for t in tamanhos], append_images=imagens[:-1])
completo.resize((256, 256), Image.LANCZOS).save(AQUI / "png" / "otomori-icone-256.png")
print("ok")
