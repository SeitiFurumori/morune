# Liquid Glass — o que a fonte primária diz, e onde o Bruma diverge

Pesquisado em 05/09/2026.

**Nota de método:** `developer.apple.com` é aplicação JavaScript e devolve só o
título ao ser buscado — a página de Materials veio vazia, e o endpoint `.json`
que a HIG usa respondeu 404. O que está aqui vem de fontes secundárias que
citam a HIG e a API, com destaque para uma referência técnica que compila a
documentação da SwiftUI. **Isso é uma ressalva real:** os nomes de API e as
regras batem entre as fontes, mas nada aqui foi lido da HIG em primeira mão.

## O que o material é

Camadas que compõem: desfoque do que está atrás, translucidez, tinta, brilho, e
**refração** — a luz entra e entorta, com realce especular que responde ao
movimento do aparelho. Quando a peça cresce, o material simula ser **mais
espesso**: sombra mais funda e refração mais pronunciada.

Duas variantes, e a escolha não é estética:

| | Regular | Clear |
|---|---|---|
| uso | padrão de quase toda interface | fundo cheio de mídia |
| transparência | média | alta |
| adaptação ao conteúdo | total | limitada |
| exige | nada | camada de escurecimento por baixo |

## ACHADO 1 — a regra que o Morune quebra em quase toda tela

> "Never apply to content itself (lists, tables, media)."

O `Glass` do Morune é aplicado **a linha de faixa no hover**
(`app.slint`, `TrackListRow`), **a cartão** (`Card`), a item de navegação e a
linha de playlist da barra lateral. Isso é conteúdo, não cromo.

No vocabulário da Apple, vidro é a camada de **controle** que flutua sobre o
conteúdo — barra, botão, painel. Lista é conteúdo e recebe o vidro por baixo,
nunca em cima.

É a divergência mais estrutural encontrada, e explica por que o Bruma "parece
vidro" e ao mesmo tempo não parece Liquid Glass: tudo é vidro, então nada
flutua sobre nada.

## ACHADO 2 — empilhar vidro sobre vidro é anti-padrão declarado

A lista de anti-padrões traz, textualmente, *glass-on-glass stacking*, *multiple
separate glass effects without container* e *overuse — "glass everywhere"*.

O Bruma faz os três ao mesmo tempo: janela acrílica → barra lateral com `Glass`
→ cartão com `Glass` → linha com `Glass` no hover. E o Morune ainda soma três
componentes de vidro na mesma peça (`GlassEdge` + `Gloss` + `Frost`).

A Apple resolve isso com `GlassEffectContainer`: peças próximas entram num
recipiente só e **se fundem** em vez de se empilharem.

## ACHADO 3 — sombra zerada é o oposto da especificação

Bruma: `shadow = "#00000000"`, `shadow_strength = 0.0`.

A fonte diz que a sombra do Liquid Glass é **adaptativa** — "opacity increases
over text, decreases over white backgrounds" — e que peça maior ganha sombra
mais funda para simular espessura. Sombra é o que diz que o vidro está *acima*
de alguma coisa.

Sem sombra nenhuma, o vidro do Bruma não flutua: ele é um véu na mesma altura
do que está atrás. Combinado com o ACHADO 1, é a explicação completa da
sensação de chapado.

## ACHADO 4 — a tinta tem função semântica, e o Bruma não tem tinta

> "Convey semantic meaning (primary action, state), NOT decoration. Use
> selectively for call-to-action only."

`accent` do Bruma é `#f2f4f8` — branco levemente frio, a mesma família de
`text` (`#f7f7f8`). Ou seja: o tema **não tem cor de ação**. O botão primário,
a faixa tocando e o realce de seleção saem todos na mesma cor do texto.

Isso é o contrário do "tinja pouco": não é tingir de menos, é não ter com o que
tingir. E `anti-padrão: tinting everything` também não se aplica aqui — o
problema é o outro extremo.

## ACHADO 5 — raio concêntrico é regra, não gosto

A API expõe `RoundedRectangle(cornerRadius: .containerConcentric)`, que "casa
automaticamente com as quinas do contêiner". A forma padrão do vidro é
**cápsula**.

Bruma antes de 05/09: `radius_lg` 26 contra `radius_artwork` 14 +
`spacing_sm` 8 = **22**. Errava por 4px. A correção de raio somado, feita hoje
por outro motivo, já alinhou isto sem que a pesquisa fosse necessária.

`button_radius = 999` ✔ — cápsula, como a fonte pede.

## ACHADO 6 — três ajustes de acessibilidade mudam o material, e o Morune atende zero

| ajuste | o que o sistema faz |
|---|---|
| Reduce Transparency | aumenta o fosco, para dar legibilidade |
| Increase Contrast | cores duras e bordas marcadas |
| Reduce Motion | reduz a animação e o efeito elástico |

O Morune passou a respeitar **movimento** em 05/09. Transparência e contraste
continuam sem tratamento: quem liga "reduzir transparência" no Windows continua
recebendo o acrílico inteiro.

## ⚠️ RESSALVA HONESTA — o teto do Slint

Duas partes centrais do material **não são alcançáveis** neste renderizador, e
nenhuma recomendação abaixo deve fingir o contrário:

- **Refração e lente.** Exige distorcer o que está atrás. O Slint não tem
  `backdrop-filter` nem shader por elemento; o `Frost` já é a imitação possível
  e o próprio comentário dele admite o limite.
- **Realce especular que segue o movimento.** Exige acelerômetro ou pelo menos
  posição do ponteiro alimentando o gradiente por quadro — e o critério de
  desempenho do projeto proíbe trabalho por quadro que não seja necessário.

O que **é** alcançável: hierarquia de camadas (ACHADO 1), fusão em vez de
empilhamento (2), sombra adaptativa (3), tinta semântica (4), raio concêntrico
(5) e os ajustes de acessibilidade (6). Cinco dos seis achados não dependem de
recurso que o Slint não tem.

## Fontes

- [Apple Updated Its HIG for Liquid Glass — Pixel Envy](https://pxlnv.com/linklog/hig-liquid-glass/)
- [LiquidGlassReference — conorluddy (GitHub)](https://github.com/conorluddy/LiquidGlassReference)
- [Meet Liquid Glass — WWDC25](https://developer.apple.com/videos/play/wwdc2025/219/)
- [Liquid Glass in SwiftUI: patterns from shipping on iOS 26](https://blakecrosley.com/blog/liquid-glass-swiftui-patterns)
