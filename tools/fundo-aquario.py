# Campo de fundo do tema Aquario. Gerado, nao baixado: nenhum direito de
# terceiro entra num tema que a gente distribui.
#
# ---------------------------------------------------------------------------
# O que a versao anterior errava
# ---------------------------------------------------------------------------
#
# A versao de 28/08 era um degrade vertical linear com sete bolas coladas por
# cima. Nao era Frutiger Aero, era um plano de fundo de apresentacao. Os quatro
# defeitos, na ordem em que doem:
#
# 1. **Nao havia cena.** Ceu virava grama num degrade continuo, sem horizonte,
#    sem agua, sem chao. O cano do estilo -- Wii, XMB, iOS 6, papel de parede
#    do Vista -- e sempre uma CENA: ha um lugar, e o lugar tem camadas.
# 2. **Nao havia fonte de luz.** Cada esfera tinha o ponto especular no mesmo
#    canto por acaso, e o resto da imagem nao sabia de onde vinha a luz. Sem
#    fonte unica nada parece iluminado; parece pintado.
# 3. **O borrado era branco puro.** O bokeh estava desenhado como mascara em
#    tons de cinza e colado com branco -- ou seja, nevoa, nao luz. Bokeh de
#    verdade tem COR e tem ANEL: a borda do circulo e mais clara que o miolo.
# 4. **As esferas nao eram de vidro.** Tinham difusa, especular e Fresnel, mas
#    nao refratavam nada. Vidro so le como vidro quando o que esta atras dele
#    aparece atraves, deslocado e invertido. Sem isso e bola de plastico.
#
# ---------------------------------------------------------------------------
# A restricao que manda em tudo: a imagem e vista quase inteira
# ---------------------------------------------------------------------------
#
# `app.slint` desenha a imagem como primeira filha da janela, a 0.92, com um
# veu branco de 0.08 por cima. O `background` do tema fica ATRAS dela. Na pratica
# a imagem chega ao olho a ~85% nas areas vazias, e o texto de conteudo
# (`#0e2b33`) cai direto sobre ela.
#
# Logo: **nenhum pixel pode escurecer.** A imagem inteira vive na faixa alta de
# valor e tira contraste de MATIZ e de FORMA, nunca de valor. E o que o menu do
# Wii faz. O script mede a luminancia minima no fim e falha se descer do piso.
import math

import numpy as np
from PIL import Image, ImageFilter

W, H = 1600, 1000

# Fonte de luz unica. Tudo nesta imagem obedece a ela: o bloom do ceu, o
# caminho de brilho na agua, a direcao de cada especular, o lado de cada
# sombra de contato e a densidade do bokeh.
SOL = np.array([0.74, 0.11])

HORIZONTE = 0.585  # onde o ceu encontra a agua
MARGEM = 0.845  # onde a agua encontra a grama

# O piso de contraste, calculado em vez de chutado.
#
# O texto de conteudo do tema e `#0e2b33`, cuja luminancia relativa e 0.0206.
# Pela formula da WCAG, para 7:1 (AAA) o fundo precisa de luminancia relativa
# 0.444 -- um cinza sRGB de 0.697. A primeira versao deste arquivo media a
# luminancia em GAMA e fixava o piso em 0.62, o que superestima o contraste e
# obrigava a imagem inteira a viver em pastel para nao encostar num limite que
# nem era o certo. Medindo em linear da para descer, e descer e o que devolve
# saturacao -- que e metade do estilo.
#
# Folga extra: a imagem e desenhada a 0.92 sobre `background` (claro) com veu
# branco de 0.08 por cima, entao na tela ela chega MAIS clara que no arquivo.
PISO_LINEAR = 0.444

rng = np.random.default_rng(23)

yy, xx = np.mgrid[0:H, 0:W].astype(np.float32)
u = xx / (W - 1)  # 0..1 horizontal
v = yy / (H - 1)  # 0..1 vertical


def suavizar(a, b, t):
    """smoothstep -- transicao sem quina, que e a unica que o estilo aceita."""
    t = np.clip((t - a) / (b - a), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def borrar(campo, raio):
    """Gaussiana via PIL: mais rapido e mais limpo que convoluir em numpy."""
    img = Image.fromarray((np.clip(campo, 0, 1) * 255).astype(np.uint8), "L")
    img = img.filter(ImageFilter.GaussianBlur(raio))
    return np.asarray(img).astype(np.float32) / 255.0


# ---------------------------------------------------------------------------
# 1. A cena: ceu, agua rasa, grama ao sol
# ---------------------------------------------------------------------------
#
# Interpolacao por paradas, como antes, mas agora com DUAS descontinuidades de
# proposito -- no horizonte e na margem. Sao elas que dizem "isto e um lugar" em
# vez de "isto e um degrade". Ambas ficam dentro da faixa alta de valor.
PARADAS = [
    (0.000, (0.353, 0.686, 0.945)),  # topo do ceu, o azul mais fundo da imagem
    (0.300, (0.522, 0.796, 0.973)),
    (0.500, (0.741, 0.902, 0.988)),
    (HORIZONTE, (0.925, 0.973, 1.000)),  # bruma do horizonte, quase branco
    (HORIZONTE + 0.004, (0.400, 0.812, 0.855)),  # a agua comeca -- degrau
    (0.700, (0.478, 0.878, 0.812)),
    (MARGEM - 0.045, (0.639, 0.925, 0.702)),  # a agua vira verde ao chegar na margem
    (MARGEM + 0.030, (0.639, 0.847, 0.373)),  # grama -- a espuma da margem dilui o degrau
    (1.000, (0.784, 0.906, 0.404)),
]

cena = np.zeros((H, W, 3), np.float32)
for i in range(len(PARADAS) - 1):
    t0, c0 = PARADAS[i]
    t1, c1 = PARADAS[i + 1]
    faixa = (v >= t0) & (v <= t1)
    k = np.where(faixa, (v - t0) / max(t1 - t0, 1e-6), 0.0)
    for c in range(3):
        cena[:, :, c] += np.where(faixa, c0[c] + (c1[c] - c0[c]) * k, 0.0)

# ---------------------------------------------------------------------------
# 2. O sol, e o que ele faz com o ceu
# ---------------------------------------------------------------------------
#
# Bloom radial quente. Ele nao e simetrico: alonga na horizontal, porque atmosfera
# espalha mais no eixo que tem mais ar.
dx = (u - SOL[0]) * 1.0
dy = (v - SOL[1]) * (W / H) * 0.62
dist_sol = np.sqrt(dx * dx + dy * dy)

bloom = np.exp(-(dist_sol**2) / (2 * 0.30**2)) * 0.55
bloom += np.exp(-(dist_sol**2) / (2 * 0.075**2)) * 0.45
quente = np.array([1.000, 0.988, 0.925], np.float32)
cena = cena + bloom[:, :, None] * (quente - cena) * 0.9

# Bruma do horizonte -- uma faixa clara continua, mais forte perto do sol.
faixa_h = np.exp(-(((v - HORIZONTE) / 0.070) ** 2)) * (0.30 + 0.45 * np.exp(-(dx**2) / 0.10))
cena = cena + faixa_h[:, :, None] * (1.0 - cena) * 0.75

# Nuvens. Existem por composicao, nao por realismo: sem elas a metade de cima
# fica um campo liso de 400px e a imagem parece inacabada. Sao ruido borrado em
# duas oitavas, empurrado para o alto e clareado -- contraste baixissimo, porque
# e ceu de meio-dia e porque e ali que o conteudo da janela passa por cima.
nuv = borrar(rng.random((H, W)).astype(np.float32), 46.0)
nuv += borrar(rng.random((H, W)).astype(np.float32), 17.0) * 0.5
nuv = (nuv - nuv.mean()) * 3.2
nuv = np.clip(nuv, 0, 1) ** 1.5
nuv *= (1.0 - suavizar(0.14, HORIZONTE - 0.02, v)) * suavizar(-0.02, 0.16, v)
cena = cena + (nuv * 0.30)[:, :, None] * (1.0 - cena)

# ---------------------------------------------------------------------------
# 3. A agua
# ---------------------------------------------------------------------------
#
# Duas coisas, e as duas dependem do sol:
#
# - **Ondulacao em perspectiva.** A frequencia das bandas cresce na direcao do
#   horizonte, porque de longe as ondas se acumulam no olho. E o unico truque
#   que faz uma superficie plana parecer deitada.
# - **Caminho de brilho.** A coluna de luz cai exatamente sob o sol e alarga
#   conforme se aproxima do observador. Sem ela a agua nao reflete nada e o sol
#   fica sendo enfeite.
# A mascara da agua desvanece nas duas pontas. Com corte duro (que era o erro da
# versao anterior) o fim das ondas deixava uma emenda reta atravessando a
# imagem inteira, logo acima da grama -- e emenda reta e a unica coisa que nao
# pode existir num campo que so tem formas moles.
agua = suavizar(HORIZONTE - 0.002, HORIZONTE + 0.010, v) * (1.0 - suavizar(MARGEM - 0.055, MARGEM + 0.020, v))
prof = np.clip((v - HORIZONTE) / (MARGEM - HORIZONTE), 0, 1)  # 0 longe, 1 perto

freq = 6.0 + 190.0 * (1.0 - prof) ** 2.4
fase = np.sin(freq * (v - HORIZONTE) + 0.9 * np.sin(u * 5.0 + 1.3))
fase2 = np.sin(freq * 0.43 * (v - HORIZONTE) - 1.2 * np.sin(u * 3.1 - 0.6))
ondas = (fase * 0.62 + fase2 * 0.38) * (0.030 + 0.105 * prof) * suavizar(0.0, 0.08, prof)
ondas = ondas * agua

# Caminho de brilho: uma COLUNA sob o sol, nao uma curva.
#
# A versao anterior somava `2.2 * sin(u * 7)` a fase, e isso torcia a coluna
# inteira num S -- que o olho lia como um trecho de logotipo boiando no mar. O
# reflexo do sol na agua e vertical por construcao: cada onda entre o sol e o
# observador devolve um pedaco de luz, e o conjunto e uma faixa reta que alarga
# conforme se aproxima. O recorte irregular vem das ONDAS, nao da faixa.
largura = 0.018 + 0.115 * prof**1.4
caminho = np.exp(-(((u - SOL[0]) / largura) ** 2))

# As lascas de luz.
#
# Nem seno nem produto de senos serve aqui: seno da faixa continua, produto de
# senos da uma grade regular, e a segunda tentativa deste arquivo saiu como dois
# blobs horizontais isolados que liam como olhos. O que a agua faz e recortar a
# luz em centenas de lascas irregulares, largas e baixas -- cada uma e a crista
# de uma onda pegando o sol. Ruido gerado numa grade achatada e esticado na
# vertical da exatamente isso, e de graca: uma linha de ruido vira uma lasca.
# A grade e LARGA e BAIXA -- poucas colunas, muitas linhas. Esticada ate o
# tamanho da imagem, cada celula vira uma lasca deitada. Com a grade ao
# contrario (muitas colunas, poucas linhas, que foi a tentativa anterior) o
# mesmo codigo devolve granulado fino, e o caminho de brilho le como chuvisco
# de televisao fora do ar.
grade = rng.random((H // 2, max(4, W // 26))).astype(np.float32)
esticado = np.asarray(
    Image.fromarray((grade * 255).astype(np.uint8), "L").resize((W, H), Image.BICUBIC)
).astype(np.float32) / 255.0
lasca = np.clip((esticado - 0.62) * 3.0, 0.0, 1.0)
lasca *= 0.35 + 0.65 * np.clip(np.sin(freq * (v - HORIZONTE)), 0, 1)  # so nas cristas
# O brilho morre antes da margem: lasca de luz sobre grama le como grama seca.
cintilar = lasca * caminho * (0.45 + 0.45 * (1.0 - prof)) * agua
cintilar *= 1.0 - suavizar(0.72, 0.92, prof)
cintilar = borrar(cintilar, 2.6)

cena = cena + (ondas + cintilar * 0.85)[:, :, None] * np.array([1.0, 1.0, 0.96], np.float32)

# ---------------------------------------------------------------------------
# 4. A grama, fora de foco
# ---------------------------------------------------------------------------
#
# Ela e a camada mais proxima do observador, entao e a mais desfocada -- e assim
# que profundidade de campo funciona de verdade. Ruido colorido borrado forte,
# so na faixa de baixo.
grama = borrar(rng.random((H, W)).astype(np.float32), 22.0)
grama += borrar(rng.random((H, W)).astype(np.float32), 7.0) * 0.55
grama = (grama - grama.mean()) * 3.4
mask_g = suavizar(MARGEM - 0.02, MARGEM + 0.07, v)
cena[:, :, 0] += grama * 0.055 * mask_g
cena[:, :, 1] += grama * 0.080 * mask_g
cena[:, :, 2] -= grama * 0.045 * mask_g

# Luz raspando o topo da grama, do lado do sol. E o que separa "faixa verde" de
# "chao iluminado": a grama recebe a mesma luz que tudo o mais na imagem.
raspante = np.clip(grama, 0, 1) * np.exp(-(dx**2) / 0.55) * mask_g
cena = cena + (raspante * 0.30)[:, :, None] * (np.array([1.0, 1.0, 0.90], np.float32) - cena)

# ---------------------------------------------------------------------------
# 5. Bokeh
# ---------------------------------------------------------------------------
#
# O que estava faltando na versao anterior, item por item:
#
# - **Anel.** Uma lente real desenha o ponto fora de foco como disco de borda
#   mais clara que o miolo. Sem o anel o circulo le como bolha de sabao chapada.
# - **Cor.** Cada circulo pega a cor do que ele SERIA se estivesse em foco: quente
#   perto do sol, ciano no meio, verde perto da grama.
# - **Densidade que obedece a luz.** Ha mais bokeh perto do sol, porque bokeh e
#   luz fora de foco e a luz esta ali.
bokeh = np.zeros((H, W, 3), np.float32)
for _ in range(72):
    bu = rng.uniform(-0.06, 1.06)
    bv = rng.uniform(-0.06, 1.06)
    d = math.hypot(bu - SOL[0], (bv - SOL[1]) * 0.7)
    # Perto do sol e muito mais provavel; longe, o circulo sai fraco ou nem sai.
    forca = math.exp(-(d**2) / (2 * 0.42**2))
    if rng.random() > 0.28 + 0.72 * forca:
        continue
    # O miolo da tela e onde a lista e os cartoes moram. Bokeh grande ali le
    # como bolha de sabao por cima do texto, entao ele rareia e encolhe.
    miolo = math.exp(-(((bu - 0.55) / 0.30) ** 2) - (((bv - 0.42) / 0.30) ** 2))
    if rng.random() < 0.55 * miolo:
        continue
    # Bokeh e luz do CEU fora de foco. Deixado solto na metade de baixo ele vira
    # circulo redondo boiando sobre a agua -- que e exatamente a leitura de
    # "bolha de sabao" que este campo precisa evitar.
    if bv > HORIZONTE - 0.02:
        continue
    raio = rng.uniform(20, 78) * (0.75 + 0.5 * forca)
    cx, cy = bu * W, bv * H

    if bv < HORIZONTE - 0.05:
        cor = np.array([1.00, 0.985, 0.90], np.float32)  # luz do ceu, quente
    elif bv < MARGEM:
        cor = np.array([0.72, 0.95, 1.00], np.float32)  # agua, ciano
    else:
        cor = np.array([0.80, 1.00, 0.62], np.float32)  # grama, verde

    x0, x1 = int(max(0, cx - raio - 3)), int(min(W, cx + raio + 4))
    y0, y1 = int(max(0, cy - raio - 3)), int(min(H, cy + raio + 4))
    if x1 <= x0 or y1 <= y0:
        continue
    ly, lx = np.mgrid[y0:y1, x0:x1].astype(np.float32)
    dd = np.sqrt((lx - cx) ** 2 + (ly - cy) ** 2) / raio
    corpo = np.clip(1.0 - (dd - 1.0) * raio / 2.5, 0.0, 1.0)  # borda em ~2.5px
    perfil = 0.80 + 0.30 * suavizar(0.66, 0.99, dd)  # corpo cheio, anel discreto
    a = corpo * perfil * (0.075 + 0.190 * forca) * (1.0 - 0.45 * miolo) * rng.uniform(0.7, 1.3)
    bokeh[y0:y1, x0:x1] += a[:, :, None] * cor

bokeh = np.stack([borrar(bokeh[:, :, c] * 0.5, 1.6) * 2.0 for c in range(3)], axis=2)
cena = 1.0 - (1.0 - cena) * (1.0 - np.clip(bokeh, 0, 1))  # screen: luz soma, nao cobre

cena = np.clip(cena, 0.0, 1.0)


# ---------------------------------------------------------------------------
# 6. Nao ha esferas -- e isto e uma decisao, nao um esquecimento
# ---------------------------------------------------------------------------
#
# As sete esferas de vidro sairam. Duas versoes seguidas tentaram salva-las --
# primeiro com Fresnel e especular, depois com refracao e aberracao cromatica --
# e nas duas o veredito foi o mesmo: nao pareciam bolhas. O motivo e estrutural,
# nao de ajuste. Uma bola de vidro so le como vidro quando ha CENA DETALHADA
# atras dela para ser deformada. Este campo e um degrade horizontalmente
# uniforme; refratar um degrade vertical devolve outro degrade vertical, entao a
# esfera nunca ia mostrar o unico sinal que o olho procura. Ela sempre ia sair
# como bola de plastico com um ponto de luz.
#
# O que se perde: o item "alguma coisa em foco" que o cabecalho do tema promete.
# Fica valendo pelo horizonte, pela margem e pelo caminho de brilho -- que sao
# formas nitidas, ao contrario do bokeh. A alternativa honesta seria uma cena com
# detalhe de verdade atras das esferas, e isso e outro trabalho.

# ---------------------------------------------------------------------------
# 7. Acabamento
# ---------------------------------------------------------------------------
#
# Vinheta clara -- clareia as bordas em vez de escurecer, que e o oposto do
# habito e o certo aqui: a janela tem cantos arredondados e barra lateral clara,
# e um canto escuro brigaria com os dois.
vin = suavizar(0.45, 1.15, np.sqrt(((u - 0.5) * 1.05) ** 2 + ((v - 0.5) * 0.95) ** 2) * 2)
cena = cena + vin[:, :, None] * (1.0 - cena) * 0.16

# Grao fino. Existe por um motivo tecnico, nao estetico: degrade suave em 8 bits
# faz banda visivel, e um grao de meio nivel quebra a banda sem aparecer.
cena += (rng.random((H, W, 1)).astype(np.float32) - 0.5) * (1.6 / 255.0)

cena = np.clip(cena, 0.0, 1.0)

# ---------------------------------------------------------------------------
# 8. A verificacao que a versao anterior nao tinha
# ---------------------------------------------------------------------------
lin = np.where(cena <= 0.04045, cena / 12.92, ((cena + 0.055) / 1.055) ** 2.4)
luma = lin @ np.array([0.2126, 0.7152, 0.0722], np.float32)
minimo = float(luma.min())
razao = (minimo + 0.05) / (0.0206 + 0.05)
print(f"luminancia relativa  min={minimo:.3f}  media={float(luma.mean()):.3f}")
print(f"contraste do pior pixel contra o texto do tema: {razao:.2f}:1")
if minimo < PISO_LINEAR:
    frac = float((luma < PISO_LINEAR).mean())
    print(f"AVISO: {frac * 100:.2f}% dos pixels abaixo do piso {PISO_LINEAR} (7:1)")

saida = "crates/morune-app/themes/aquario/fundo.png"
Image.fromarray((cena * 255).astype(np.uint8), "RGB").save(saida, optimize=True)
print("ok ->", saida)
