# Campo de fundo do tema Aquario. Gerado, nao baixado: nenhum direito de
# terceiro entra num tema que a gente distribui.
#
# Duas coisas que a versao anterior errava, e que sao a diferenca entre
# "degrade azulado" e Frutiger Aero:
#
# 1. **Registro claro.** A referencia da linguagem e clara -- ceu, agua rasa,
#    grama ao sol. O fundo antigo era escuro porque o texto branco caia direto
#    sobre ele; com o tema claro o texto e escuro, e o campo pode finalmente
#    abrir.
# 2. **Alguma coisa em foco.** Desfoque sem sujeito nao le como profundidade,
#    le como degrade inacabado. Aqui o borrado fica ATRAS: na frente vao
#    esferas de vidro nitidas, com especular curto e luz de retorno na orla --
#    que e como todo objeto daquela era era desenhado.
import math, random
from PIL import Image, ImageDraw, ImageFilter

W, H = 1600, 1000
random.seed(23)

# Ceu -> agua rasa -> grama. Tudo claro: o texto que cai por cima e escuro, e
# nenhuma faixa pode escurecer a ponto de comer a leitura.
STOPS = [
    (0.00, (0xbf, 0xe4, 0xff)),
    (0.34, (0xa9, 0xe3, 0xf6)),
    (0.56, (0xbd, 0xee, 0xdc)),
    (0.78, (0xc8, 0xef, 0xa8)),
    (1.00, (0xd8, 0xf2, 0x9a)),
]

def lerp(a, b, t):
    return tuple(round(x + (y - x) * t) for x, y in zip(a, b))

base = Image.new("RGB", (W, H))
px = base.load()
for y in range(H):
    t = y / (H - 1)
    for i in range(len(STOPS) - 1):
        t0, c0 = STOPS[i]
        t1, c1 = STOPS[i + 1]
        if t0 <= t <= t1:
            cor = lerp(c0, c1, (t - t0) / (t1 - t0))
            break
    for x in range(W):
        px[x, y] = cor
base = base.convert("RGBA")

# --- Camada de tras: o que esta fora de foco. -------------------------------
#
# Desenhada como MASCARA em tons de cinza, e nao como RGBA sobre camada
# transparente. Desenhar branco com alfa sobre pixel transparente puxa a cor
# para o preto no PIL -- o bokeh saia como mancha cinza suja em vez de luz.
# Mascara nao tem esse problema: ela diz so *quanto* de branco entra.
mascara = Image.new("L", (W, H), 0)
d = ImageDraw.Draw(mascara)
for x0, larg, alfa in [(220, 340, 30), (820, 250, 22), (1280, 380, 26)]:
    d.polygon(
        [(x0, -50), (x0 + larg, -50), (x0 + larg + 300, H + 50), (x0 + 170, H + 50)],
        fill=alfa,
    )
for _ in range(22):
    r = random.randint(30, 130)
    x = random.randint(-40, W + 40)
    y = int(random.triangular(0, H, H * 0.2))
    a = random.randint(16, 40)
    d.ellipse([x - r, y - r, x + r, y + r], fill=a)
    d.ellipse([x - r, y - r, x + r, y + r], outline=a + 26, width=max(2, r // 18))
mascara = mascara.filter(ImageFilter.GaussianBlur(20))
base.paste(Image.new("RGB", (W, H), (0xff, 0xff, 0xff)), (0, 0), mascara)

# Luz do horizonte, continua de ponta a ponta.
luz = Image.new("L", (W, H), 0)
dl = ImageDraw.Draw(luz)
hy, sig = 0.56 * H, 0.06 * H
for y in range(H):
    a = math.exp(-((y - hy) ** 2) / (2 * sig * sig))
    if a > 0.004:
        dl.line([(0, y), (W, y)], fill=int(96 * a))
luz = luz.filter(ImageFilter.GaussianBlur(30))
base.paste(Image.new("RGB", (W, H), (0xff, 0xff, 0xff)), (0, 0), luz)


def esfera(lado=512, cor=(0x4f, 0xbe, 0xf0)):
    """Uma bola de vidro, resolvida por pixel.

    Difusa para o volume, especular curto para o ponto de luz, e Fresnel na
    orla -- que e o que faz vidro parecer vidro e nao bola pintada. Desenhada
    UMA vez em tamanho grande e reduzida na hora de colar: reduzir e o que da
    a borda limpa, sem serrilhado e sem desfoque.
    """
    img = Image.new("RGBA", (lado, lado), (0, 0, 0, 0))
    p = img.load()
    lx, ly, lz = -0.45, -0.62, 0.64
    n = math.sqrt(lx * lx + ly * ly + lz * lz)
    lx, ly, lz = lx / n, ly / n, lz / n
    for j in range(lado):
        ny = (j + 0.5) / lado * 2 - 1
        for i in range(lado):
            nx = (i + 0.5) / lado * 2 - 1
            r2 = nx * nx + ny * ny
            if r2 > 1.0:
                continue
            nz = math.sqrt(1.0 - r2)
            ndotl = max(0.0, nx * lx + ny * ly + nz * lz)
            # Reflexao da luz contra o olho (que olha de frente, 0,0,1).
            d2 = 2 * ndotl
            rz = d2 * nz - lz
            spec = max(0.0, rz) ** 46
            fres = (1.0 - nz) ** 2.6
            difusa = 0.34 + 0.66 * ndotl
            r = cor[0] * difusa + 255 * (spec * 0.95 + fres * 0.42)
            g = cor[1] * difusa + 255 * (spec * 0.95 + fres * 0.42)
            b = cor[2] * difusa + 255 * (spec * 0.95 + fres * 0.42)
            alfa = 150 + 70 * fres + 105 * spec
            # Suaviza so o ultimo fio da borda, para nao serrilhar.
            borda = min(1.0, (1.0 - math.sqrt(r2)) * lado / 2.0)
            p[i, j] = (
                min(255, int(r)), min(255, int(g)), min(255, int(b)),
                min(255, int(alfa * borda)),
            )
    return img

azul = esfera(cor=(0x4f, 0xbe, 0xf0))
verde = esfera(cor=(0x9a, 0xdc, 0x54))

# Na frente, nitidas, e fora do miolo da tela -- o conteudo da janela mora la.
for img, cx, cy, r in [
    (azul,  148, 812, 172),
    (verde, 1468, 838, 118),
    (azul,  1352, 118,  64),
    (verde,  392,  96,  46),
    (azul,  1556, 452,  96),
    (azul,   902, 946,  30),
    (verde,  742,  62,  26),
]:
    p = img.resize((r * 2, r * 2), Image.LANCZOS)
    base.alpha_composite(p, (cx - r, cy - r))

base.convert("RGB").save("crates/morune-app/themes/aquario/fundo.png", optimize=True)
print("ok")
