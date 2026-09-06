# Material das pecas -- o brilho, a orla e o reflexo, desenhados por nos.
#
# ---------------------------------------------------------------------------
# Por que este arquivo existe
# ---------------------------------------------------------------------------
#
# O tema tinha 75 tokens de cor, raio e espacamento, e trocar de tema mudava
# pouco. A conclusao apressada foi "limitacao do Slint": nao ha `backdrop-filter`,
# nao ha filtro, nao ha reflexo. Isso e verdade e e irrelevante -- porque
# **Aero, Wii e XMB nunca tiveram nada disso**. Aquelas interfaces eram BITMAP:
# a rampa de brilho, a orla, o reflexo e o realce eram imagens desenhadas por
# alguem e esticadas por cima do controle. O efeito nao vinha do motor grafico,
# vinha do artista.
#
# Este script desenha esse material. O que ele produz e neutro -- so branco e
# preto com alfa --, entao a mesma folha serve a qualquer tema: quem da a cor e
# a superficie por baixo. Um tema fosco simplesmente nao usa.
#
# ---------------------------------------------------------------------------
# Como as folhas sao usadas
# ---------------------------------------------------------------------------
#
# `Image` do Slint 1.17 aceita `@image-url(..., nine-slice(t r b l))`: os quatro
# cantos ficam intactos e as bordas esticam. E o mesmo mecanismo do `border-image`
# do CSS, e e por isso que uma unica folha de 96x96 veste um botao de 34px e um
# cartao de 190px sem deformar a orla.
#
# As folhas sao embutidas no binario (`@image-url` e resolvido na compilacao),
# nao carregadas da pasta do tema. E deliberado: material e forma, e forma nao
# esta no contrato de tema -- o contrato e de cor, medida e movimento. O tema
# escolhe QUAL material usar, nao desenha um.
import numpy as np
from PIL import Image

SAIDA = "crates/morune-app/ui/materiais"
LADO = 96  # nine-slice de 28px: sobra 40px de miolo para esticar
CANTO = 28


def salvar(nome, rgba):
    img = Image.fromarray((np.clip(rgba, 0, 1) * 255).astype(np.uint8), "RGBA")
    img.save(f"{SAIDA}/{nome}.png", optimize=True)
    print(f"  {nome}.png")


def mascara_arredondada(lado, raio, suave=1.2):
    """Alfa de um retangulo de cantos arredondados, com a borda antisserrilhada."""
    y, x = np.mgrid[0:lado, 0:lado].astype(np.float32) + 0.5
    dx = np.maximum(np.maximum(raio - x, x - (lado - raio)), 0.0)
    dy = np.maximum(np.maximum(raio - y, y - (lado - raio)), 0.0)
    dist = np.sqrt(dx * dx + dy * dy)
    return np.clip((raio - dist) / suave + 0.5, 0.0, 1.0)


def suavizar(a, b, t):
    t = np.clip((t - a) / (b - a), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


# ---------------------------------------------------------------------------
# 1. gel -- a peca de vidro colorido do Aquario
# ---------------------------------------------------------------------------
#
# Tres coisas empilhadas, e as tres tem nome no oficio:
#
# - **Brilho de topo.** Uma rampa clara que ocupa a metade de cima e morre num
#   corte suave no meio. E a assinatura do estilo: e ela que diz "isto tem uma
#   superficie curva pegando a luz do ceu". Sem ela, qualquer botao arredondado
#   e so um retangulo com raio.
# - **Orla interna.** Um fio claro por dentro da borda de cima e um fio escuro
#   por dentro da de baixo. E o que da espessura -- a peca deixa de ser um
#   decalque e passa a ter uma parede.
# - **Luz de retorno.** Um clarao fraco na base, como se o chao devolvesse luz
#   por dentro do material. E o detalhe que separa "vidro" de "plastico".
# **Bitmap para o brilho, vetor para a orla.**
#
# A folha nao tem canto arredondado nem orla desenhada: e uma rampa de bordo a
# bordo, esticada por cima da peca, e quem recorta e o `clip` do proprio
# Rectangle com o `border-radius` do tema. Desenhar a orla aqui obrigaria a um
# raio fixo -- 28px na folha contra 18 do Aquario e 22 do Bruma --, e o canto
# sairia errado em quase todo tema. A orla continua sendo `border-width` do
# Slint, que segue o raio exato de graca.
y, x = np.mgrid[0:LADO, 0:LADO].astype(np.float32)
v = y / (LADO - 1)

alfa_forma = np.ones((LADO, LADO), np.float32)

# Brilho de topo: forte no alto, cortado no meio, com um pequeno respiro abaixo.
brilho = (1.0 - suavizar(0.02, 0.46, v)) * 0.42
brilho += np.exp(-(((v - 0.50) / 0.05) ** 2)) * 0.05  # o fio do corte

# Luz de retorno, na base.
retorno = suavizar(0.72, 1.0, v) * 0.13

# Uma sombra fraca logo abaixo do corte: e ela que faz a metade de baixo ler
# como a parte da peca que a luz do topo nao alcanca.
sombra_meio = suavizar(0.50, 0.72, v) * (1.0 - suavizar(0.80, 1.0, v)) * 0.10

branco = np.clip(brilho + retorno, 0, 1)
preto = np.clip(sombra_meio, 0, 1)

def compor(branco, preto, forma):
    """Junta um realce branco e uma sombra preta num PNG de alfa RETO.

    **O erro que isto conserta.** A primeira versao escrevia a intensidade no
    RGB e a mesma intensidade no alfa. Em alfa reto isso e cinza: um realce de
    42% saia como `rgb(107,107,107)` a 42% de opacidade, que ESCURECE o que esta
    embaixo em vez de clarear. Na folha `gel`, o topo -- que devia ser a parte
    mais luminosa da peca -- era a mais escura.

    Em alfa reto o RGB diz QUAL cor e o alfa diz QUANTO dela. Realce e branco
    puro, sombra e preto puro, e onde os dois se sobrepoem o RGB e a razao entre
    eles enquanto o alfa e a soma.
    """
    total = branco + preto
    cor = np.where(total > 1e-6, branco / np.maximum(total, 1e-6), 0.0)
    alfa = np.clip(total, 0, 1) * forma
    return np.dstack([cor, cor, cor, alfa])


salvar("gel", compor(branco, preto, alfa_forma))

# ---------------------------------------------------------------------------
# 2. reflexo -- a varredura especular do botao primario
# ---------------------------------------------------------------------------
#
# O botao de tocar e a unica peca redonda e grande da interface, e e onde o
# olho pousa. Aqui a luz nao e uma rampa: e um ovalo deslocado para o alto e
# para a esquerda, que e como uma fonte pontual bate numa esfera. A diferenca
# entre isto e a rampa e o que separa "botao brilhante" de "botao esferico".
cy, cx = np.mgrid[0:LADO, 0:LADO].astype(np.float32)
nx = (cx / (LADO - 1)) * 2 - 1
ny = (cy / (LADO - 1)) * 2 - 1
disco = np.clip((1.0 - np.sqrt(nx * nx + ny * ny)) * LADO / 2.0, 0, 1)

# Especular alto e a esquerda, achatado na horizontal.
sx, sy, rx, ry = -0.30, -0.46, 0.52, 0.30
spec = np.exp(-(((nx - sx) / rx) ** 2 + ((ny - sy) / ry) ** 2)) * 0.72
# Fresnel: a orla de uma esfera sempre devolve luz.
fres = np.clip(np.sqrt(nx * nx + ny * ny), 0, 1) ** 5 * 0.34
# Sombra interna embaixo, para o volume nao ficar so no alto.
sombra = np.clip(ny, 0, 1) ** 2 * 0.22

salvar("reflexo", compor(np.clip(spec + fres, 0, 1), sombra, disco))

# ---------------------------------------------------------------------------
# 3. vidro -- a versao sobria, para o Bruma
# ---------------------------------------------------------------------------
#
# Mesmo material, metade da conviccao. Liquid Glass nao e Aero: a Apple pede
# borda luminosa e interior quase neutro, e nao uma rampa que anuncia. Aqui o
# brilho de topo cai para um terco e a orla ganha peso -- o inverso do `gel`.
brilho = (1.0 - suavizar(0.0, 0.34, v)) * 0.17
retorno = suavizar(0.78, 1.0, v) * 0.07
branco = np.clip(brilho + retorno, 0, 1)
preto = np.clip(suavizar(0.42, 0.78, v) * (1.0 - suavizar(0.86, 1.0, v)) * 0.05, 0, 1)
salvar("vidro", compor(branco, preto, alfa_forma))

print(f"nine-slice: {CANTO} {CANTO} {CANTO} {CANTO}   folha {LADO}x{LADO}")
