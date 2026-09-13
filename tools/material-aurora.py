# A aurora do vidro do Windows 7 -- as listras diagonais que andam.
#
# O vidro do Windows 7 tem, por cima do borrao e da cor, um reflexo em listras
# finas a 54 graus, brancas com alfa baixo. E o detalhe que mais identifica
# aquele vidro, e o unico que se move: no original ele e fixo em relacao a
# tela e "anda" quando a janela e arrastada. Aqui ele anda sozinho, devagar,
# porque foi o que se pediu.
#
# ---------------------------------------------------------------------------
# Por que a folha ja vem inclinada
# ---------------------------------------------------------------------------
#
# A primeira versao era uma tira de listras verticais que o Slint girava em
# -36 graus. Girar uma imagem dentro de um painel com recorte faz o
# renderizador desenhar numa camada intermediaria do tamanho da imagem girada
# -- e a imagem precisava ser maior que a diagonal do painel. Medido: 6 GB de
# memoria privada e 11% de GPU com quatro paineis. Inaceitavel.
#
# Esta versao desenha as listras JA a 54 graus numa peca que emenda sem
# costura nas duas direcoes. Listras com periodo `p` (medido na perpendicular)
# repetem na horizontal a cada `p / sen(54)` e na vertical a cada
# `p / cos(54)`; uma peca com exatamente essas medidas e um ladrilho perfeito.
# A interface entao cobre o painel com uma grade de ladrilhos -- meia duzia de
# imagens pequenas, sem giro, sem camada -- e desloca a grade em `x` por ate
# uma largura de ladrilho antes de voltar ao inicio.
#
# As paradas sao as do 7.css (khang-nd/7.css, `--w7-w-glass`), com os alfas
# pela metade: la o vidro e escuro e as listras precisam gritar; aqui elas
# ficam sobre azul claro e o mesmo valor virava faixa. So branco com alfa:
# quem da a cor e o vidro por baixo.
import math

from PIL import Image

ANGULO = math.radians(54.0)
# Periodo da sequencia inteira de listras, na perpendicular. Menor que o do
# 7.css (la e o comprimento do elemento): em painel de 1900 px as listras do
# tamanho original viravam faixas de 50 px.
PERIODO = 640
LARGURA = round(PERIODO / math.sin(ANGULO))   # 791
ALTURA = round(PERIODO / math.cos(ANGULO))    # 1089
SAIDA = "crates/morune-app/ui/materiais/aurora.png"

# (inicio %, fim %, alfa 0-255) ao longo de um periodo. Fora, alfa zero.
LISTRAS = [
    (6.0, 6.6, 14),
    (17.0, 18.0, 16),
    (25.0, 26.0, 22),
    (36.0, 40.0, 22),
    (44.0, 45.0, 22),
    (46.0, 47.0, 22),
    (58.0, 60.0, 20),
    (73.0, 74.0, 20),
    (84.0, 86.0, 16),
    (93.0, 93.6, 14),
]


def alfa_em(pct: float) -> int:
    for inicio, fim, alfa in LISTRAS:
        if inicio <= pct < fim:
            return alfa
    return 0


def main() -> None:
    folha = Image.new("RGBA", (LARGURA, ALTURA), (255, 255, 255, 0))
    px = folha.load()
    # Distancia perpendicular de cada pixel as listras: projecao no vetor
    # normal (sen 54, -cos 54). O modulo pelo periodo faz a sequencia repetir.
    nx, ny = math.sin(ANGULO), -math.cos(ANGULO)
    for y in range(ALTURA):
        for x in range(LARGURA):
            d = (x * nx + y * ny) % PERIODO
            px[x, y] = (255, 255, 255, alfa_em(100.0 * d / PERIODO))
    folha.save(SAIDA, optimize=True)
    print(f"{SAIDA}: {LARGURA}x{ALTURA} (periodo {PERIODO} px a 54 graus)")


if __name__ == "__main__":
    main()
