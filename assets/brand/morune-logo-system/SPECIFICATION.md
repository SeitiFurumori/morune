# Morune — especificação do símbolo

## Construção

- Prancheta mestre: `512 × 512` unidades, fundo transparente.
- Eixo de simetria controlada: `x = 256`.
- Módulo geométrico: `u = 8 px` no master de 512 px.
- Progressão de pesos: `2u : 3u : 5u` — pontos `16 px`, barras `24 px`, onda `40 px`.
- Vão óptico lateral constante: `2u = 16 px` entre todas as formas, dos dois lados.
- Todos os terminais usam raio igual a metade da espessura do respectivo traço.
- A curva central usa Béziers cúbicas com continuidade visual nos ápices e no vale.
- O vale central é deliberadamente mais profundo para compensar o peso dos dois arcos.
- Cor sólida recomendada: `#7549F2`.
- Gradiente principal: `#8B63F6 → #6937EC`, diagonal superior-esquerda para inferior-direita.

## Versões responsivas

| Tamanho | Traço principal | Tratamento |
|---:|---:|---|
| 512 px | 40 px | geometria mestre modular e overshoot completo |
| 128 px | 10 px | redução proporcional com vão exato de 4 px |
| 32 px | 3 px | hinting óptico, maior abertura interna e pontos reforçados |
| 16 px | 2 px | curva simplificada, barras laterais compactas e contraste reforçado |

As versões de 32 e 16 px são desenhos responsivos próprios. Elas preservam a hierarquia `2:3:5`, mas recebem compensação visual para sobreviver à rasterização. Não devem ser substituídas por uma redução automática do arquivo de 512 px.

## Uso no app

- Use os SVGs quando a plataforma aceitar vetores.
- Use os PNGs no tamanho nativo, sem reamostragem adicional.
- Preserve uma área livre mínima de `32 u` ao redor do símbolo em tamanhos grandes.
- Não altere individualmente alturas, espaçamentos ou espessuras.
- Para versão monocromática, substitua o gradiente por `#7549F2` ou por uma única cor de alto contraste.
