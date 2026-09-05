# Frutiger Aero — o que a fonte primária diz, e onde o Aquário diverge

Pesquisado em 05/09/2026.

O artefato de referência do estilo não é uma paleta: é o **Windows Aero** do
Vista e do 7. Ele tem mecanismo documentado, e é dele que sai o que dá para
medir. O resto do estilo (bokeh, peixe tropical, aurora) é motivo de imagem, não
de interface — vale para o fundo, e é onde o Aquário já acerta.

## O mecanismo do vidro do Aero, que é o achado central

Engenharia reversa do DWM do Windows 7, na discussão do DWMBlurGlass:

```
saída = (cor_primária × balanço_primário)
      + (desfoque_dessaturado × cor_secundária × balanço_secundário)
      + (desfoque × balanço_do_desfoque)
```

Quatro coisas nisso importam, e três o Aquário não faz:

1. O que está atrás passa por **desfoque gaussiano**.
2. Uma cópia desse desfoque é **dessaturada** — vira cinza — e só então é
   tingida pela cor de *afterglow*.
3. As três camadas somam por **blend aditivo**, e é isso que produz o
   estouro de luz característico quando os balanços sobem.
4. Por cima disso vai uma **textura de reflexo** — imagem fixa, não gradiente —
   com `ColorizationGlassReflectionIntensity` controlando a força.

## ACHADO 1 — o Aquário tinge o que é saturado; o Aero tinge o que ele dessaturou

`artwork_tint = true`, `artwork_tint_strength = 0.3`: o Morune puxa a cor
dominante da capa e joga na superfície.

O Aero faz o oposto na etapa equivalente: **dessatura primeiro**, tinge depois,
com uma cor única do sistema. É por isso que o vidro do Vista parece vidro
colorido e não uma janela com a cor do papel de parede vazando. Tingir com a
cor da capa dá uma superfície que muda de identidade a cada faixa — que é
exatamente a queixa de "não está próximo do conceito".

## ACHADO 2 — a superfície verde-limão não tem base no artefato

`surface = "#b7e86ae6"` — verde-limão a 90% de opacidade, como superfície de
cartão e painel.

Na fonte, verde e azul são a paleta do **motivo** — céu, água, grama, o campo
de fundo. A superfície do Aero é vidro **neutro** tingido levemente pela cor de
realce do sistema; o verde vive na imagem atrás, não no painel na frente.

O Aquário inverteu as duas camadas: pôs o verde no painel e deixou o fundo
fazer papel de plano de fundo genérico. É o que faz a tela parecer plástico
verde em vez de vidro sobre um jardim.

## ACHADO 3 — o desfoque não existe, e é a metade do efeito

`backdrop_blur = 0.0` nos dois temas, e o campo é token morto: nenhuma linha de
interface o lê. O `Frost` do projeto já registra o motivo — o Slint não tem
`backdrop-filter`.

Sem desfoque, as duas primeiras parcelas da fórmula do Aero somem. Sobra a
terceira, que é o véu branco que o `Frost` desenha. **Metade do mecanismo não
está no ar**, e nenhuma recomendação deve prometer o contrário.

O que dá para fazer sem `backdrop-filter`: o fundo é uma imagem que **nós**
controlamos (`fundo.png`). Um segundo arquivo, já desfocado e dessaturado em
tempo de geração pelo `tools/fundo-aquario.py`, pode ser desenhado sob os
painéis — dando a leitura de "vidro sobre o jardim" sem custo por quadro.

## ACHADO 4 — a fonte está certa, e por acidente

O estilo leva o nome da **Frutiger**, de Adrian Frutiger. O Aquário usa
`Segoe UI`, que é a humanista que a Microsoft desenhou para o Vista, na mesma
linhagem — é a escolha correta e a mesma do artefato de referência.

Mas `display_family` repete `Segoe UI`. Com o campo agora ligado no título
grande, é um eixo desperdiçado num estilo cuja marca registrada é justamente
a tipografia.

## ACHADO 5 — o gloss está alto e ainda assim não lê como Aero

`gloss = 0.8`, o mais alto dos temas. O `Gloss` do Morune é um gradiente linear
de 160°.

No Aero o brilho é **textura**, não gradiente: uma curva de reflexo com forma
própria, mais intensa na quina superior e com uma "orelha" de luz que corre a
largura. Gradiente linear não faz curva, e por isso a peça lê como plástico
brilhante em vez de vidro.

Isso é implementável: o Morune já sabe embutir imagem em tema (o Aquário traz
`fundo.png`), então uma textura de reflexo é o mesmo caminho.

## ⚠️ RESSALVA HONESTA — o que aqui é medido e o que é leitura

Medido: a fórmula de composição do DWM, os campos do registro, os valores atuais
dos dois temas, e o fato de `backdrop_blur` ser token morto.

**Leitura minha, não dado:** que o verde na superfície é o erro (ACHADO 2) e que
o gradiente no lugar de textura é o que quebra a leitura (ACHADO 5). São
conclusões de comparação com o artefato, e o próximo passo honesto para as duas
é a tela, não mais pesquisa — trocar e olhar.

**Não medido:** os valores padrão de `ColorizationColorBalance` e companhia no
Vista e no 7. A discussão descreve a fórmula mas não fixa os números, e eu não
achei fonte que fixe. Sem eles, qualquer "use 0,58 de balanço" seria inventado.

## Fontes

- [Research into Windows 7 colorization — DWMBlurGlass #195](https://github.com/Maplespe/DWMBlurGlass/discussions/195)
- [DwmGetColorizationColor — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/nf-dwmapi-dwmgetcolorizationcolor)
- [Frutiger Aero — frutiger-aero.org](https://frutiger-aero.org/frutiger-aero)
- [Frutiger Aero — Aesthetics Wiki](https://aesthetics.fandom.com/wiki/Frutiger_Aero)
- [Frutiger Aero Archive](https://frutigeraeroarchive.org/)
