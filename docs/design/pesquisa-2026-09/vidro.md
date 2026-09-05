# Vidro: Liquid Glass, Aero e o que o Windows manda

Pesquisa de 05/09/2026, para o Bruma (Liquid Glass) e o Aquário (Frutiger Aero).

O material atual dos dois temas está medido em
[00-corpus-dos-dois-temas.md](00-corpus-dos-dois-temas.md).

---

## 1. Liquid Glass, pela sessão da Apple

Fonte: [Meet Liquid Glass, WWDC25](https://developer.apple.com/videos/play/wwdc2025/219/).

A HIG em si não foi lida: `developer.apple.com/design` é aplicação JavaScript e
devolve página vazia a quem busca, e o endpoint JSON que ela usa responde 404. A
sessão do WWDC, essa sim, veio inteira — e é fonte primária.

### O que é

> "a new digital meta-material that dynamically bends and shapes light"

### Lensing é o que separa isto de tudo que veio antes

> "Whereas previous materials **scattered** light, this new set of materials
> dynamically **bends, shapes, and concentrates** light in real time."

Essa frase é a definição inteira. Vidro fosco espalha luz; Liquid Glass a
concentra. É a diferença entre um véu e uma lente — e é por isso que
glassmorphism com `blur` e branco a 12% nunca chega perto: está do lado errado
do verbo.

### As camadas

**Realce.** Fontes de luz dentro do ambiente incidem sobre o material e o realce
responde à geometria. Ao interagir, as luzes se movem no espaço e a luz corre
pela peça, **definindo a silhueta**. Em aparelho com sensor, o realce acompanha
a posição física.

**Sombra, e ela é adaptativa:**

> "The element is aware of what's behind it and **increases the opacity of its
> shadow when it is over text**. Conversely, it **lowers the opacity of its
> shadow when it is over a solid light background**."

**Iluminação por interação.** Ao tocar, o material acende por dentro a partir do
ponto do toque, e o brilho **se espalha para as peças de vidro vizinhas**.

**Tamanho muda o material.** Peça maior simula material mais espesso: sombra
mais funda, lensing e refração mais pronunciados, espalhamento de luz mais
macio.

**Derrame do ambiente.** Em painel grande, a cor do conteúdo próximo escorre
sobre a superfície — e reflete, espalha e **sangra também para dentro da
sombra**.

### Materialização não é fade

> "objects materialize in and out by gradually modulating the light bending and
> lensing, ensuring a graceful transition that preserves the optical integrity
> of the material"

Aparecer e sumir se faz mexendo na lente, não na opacidade.

### Tinta

> "Selecting a color generates a range of tones that are mapped to content
> brightness underneath the tinted element... drawing inspiration from how
> colored glass works in reality: changing its hue, brightness and saturation
> depending on what's behind."

Tinta é **uma faixa de tons mapeada ao que está atrás**, não uma cor chapada por
cima. E vale só para "primary elements and actions".

### Duas variantes que nunca se misturam

**Regular** — adaptativa, legível em qualquer contexto, funciona em qualquer
tamanho, sobre qualquer conteúdo, e qualquer coisa pode ser posta em cima dela.
Glifos viram claros ou escuros sozinhos para manter contraste.

**Clear** — sem comportamento adaptativo, permanentemente mais transparente,
**exige camada de escurecimento**. Só quando as três condições valem: está sobre
mídia, o conteúdo aguenta ser escurecido, e o que fica em cima é grande e claro.

### Scroll edge effect

> "As content begins to scroll underneath a glass element, the effect gently
> dissolves the content into the background, **lifting the glass visually above
> the moving content**."

O Morune tem o `ScrollFade`, que faz metade disso: avisa que há mais conteúdo.
A outra metade — dissolver o conteúdo **sob** a peça de cromo para elevá-la — não
existe, e é justamente a que separa camada de cromo de camada de conteúdo.

### Acessibilidade

| ajuste | efeito no material |
|---|---|
| Reduce Transparency | mais fosco, esconde mais do que está atrás |
| Increase Contrast | peças quase pretas ou brancas, com borda contrastante |
| Reduce Motion | reduz efeitos e desliga a elasticidade |

### O que a Apple proíbe

Da referência técnica que compila a documentação da SwiftUI
([LiquidGlassReference](https://github.com/conorluddy/LiquidGlassReference)):

> "Never apply to content itself (lists, tables, media)."

E, na lista de anti-padrões: *glass-on-glass stacking*, *multiple separate glass
effects without container*, *mixing Regular and Clear variants*, *tinting
everything*, *content layer glass*, *overuse — "glass everywhere"*.

Forma padrão: **cápsula**. Raio: `.containerConcentric`, que casa com a quina do
contêiner.

---

## 2. Frutiger Aero: primeiro o estilo, depois o Windows

**Correção de premissa, feita depois que o Felipe apontou.** A primeira versão
desta pesquisa tratou o Windows Aero como a fonte do estilo. Está errado, e a
diferença muda o que o tema deve copiar.

Fonte: [Frutiger Aero — Wikipédia](https://en.wikipedia.org/wiki/Frutiger_Aero).

**O nome é retrônimo, e de comunidade.** Foi cunhado em 2017–2018 por Sofi Xian
(antes Sofia Lee), do **Consumer Aesthetics Research Institute** — um coletivo
que cataloga estética de consumo. No auge de sua influência, o estilo **não
tinha nome nenhum**. Só virou fenômeno de internet em 2022–2023.

O nome soma "Aero", a diretriz do Vista, à fonte Frutiger, de Adrian Frutiger.
E há uma ironia registrada na própria fonte:

> "the Frutiger family was **never used in a major user interface** associated
> with the style"

O que o Vista usou foi a Segoe UI, inspirada nela. Ou seja: o `Segoe UI` do
Aquário é a escolha historicamente certa — mas por parentesco, não por citação.

**O cânone é muito maior que a Microsoft.** Os exemplos que a literatura cita:

| categoria | exemplos |
|---|---|
| sistema | Windows Vista (2006) e 7 (2009), KDE Plasma 4 |
| aparelho | Nintendo Wii, iPhone de 1ª geração (2007), Galaxy S (2010) |
| jogo | Wii Sports (2006), Purble Place (2007), The Sims 3 (2009), Fruit Ninja (2010) |
| outros | MSN Messenger, embalagem, arquitetura de loja |

Windows Aero é **um** artefato dessa lista. Acontece de ser o único com
mecanismo documentado em patente — por isso a seção seguinte existe —, mas
copiar a moldura do Vista dá "moldura do Vista", não Frutiger Aero.

**E o estilo é sobre uma ideia, não sobre um efeito.** A leitura que a
literatura registra: o natural "intertwined" com um futuro digital, "a utopia
where efficiency and the environment coexist", concebida numa época de
ingenuidade sobre o custo da tecnologia. Contra o Y2K, que é ansioso, este é
**esperançoso** — e o revival de 2023 o lê como "the future we were promised but
never delivered".

Isso tem consequência direta de projeto: a identidade do Aquário mora na
**imagem** — céu, água, grama, bolha, peixe, lens flare — e nos **controles em
gel**, não numa cor de painel. A superfície verde-limão é uma tentativa de pôr o
motivo na camada errada.

### Aero, pelas patentes da Microsoft

A parte mais concreta desta pesquisa, porque patente descreve mecanismo — e
vale como mecanismo, não como definição do estilo.

### O realce é um bitmap, não um gradiente

[US7412663B2 — Dynamic reflective highlighting of a glass appearance window frame](https://patents.google.com/patent/US7412663B2/en):

> "combination of a **pre-defined bitmap image** and a programmatically-described
> position and masking on top of the glass appearance window frame"

O bitmap descrito no exemplo tem **800×700**, desenha o realce em faixas
diagonais, e é esticado às dimensões da tela.

### E o realce se move independente da janela

A posição vem de um **ponto de referência de fonte de luz** no espaço da área de
trabalho — "not a physical point, but merely a reference location". A janela
revela a região do bitmap-mestre que corresponde a onde ela está. Ao mover ou
redimensionar, a posição é recalculada **a cada quadro**, e o realce

> "move[s] independently from the movement of the application window"

É isso que vende reflexo em vez de tinta: a luz não pertence à peça, pertence ao
ambiente, e a peça só mostra a parte dela que lhe cabe.

Janela ativa recebe realce mais opaco que janela inativa. E o padrão do realce
mudava conforme a hora do dia — há uma configuração para as horas entre 2h e 8h.

### A colorização vai na região desfocada, não na superfície

[US7418668B2 — Glass appearance window frame colorization](https://patents.google.com/patent/US7418668B2/en):

> "the color value and the level of opacity value are introduced into the
> programmatically-described **blur region behind** the glass appearance window
> frame"

Ou seja: desfoca o que está atrás, **tinge o desfoque**, e o quadro em si fica
parcialmente transparente com o realce por cima.

### A fórmula do Windows 7, por engenharia reversa

[DWMBlurGlass, discussão #195](https://github.com/Maplespe/DWMBlurGlass/discussions/195)
— secundária, mas técnica:

```
saída = (cor_primária × balanço_primário)
      + (desfoque_DESSATURADO × cor_secundária × balanço_secundário)
      + (desfoque × balanço_do_desfoque)
```

O desfoque é gaussiano; uma cópia dele é **convertida para cinza** antes de
receber a cor de *afterglow*; as três camadas somam por blend aditivo, o que
produz o estouro de luz quando os balanços sobem.

Os valores padrão de `ColorizationColorBalance` e companhia **não foram
encontrados** em fonte nenhuma. Sem eles, qualquer número de balanço aqui seria
inventado.

### Os motivos, pela comunidade

Fonte: [frutiger-aero.org](https://frutiger-aero.org/frutiger-aero) e a Wikipédia.

Período 2004–2013, sucedendo o Y2K. Motivos: esqueuomorfismo, textura
brilhante, gradiente linear, *lens flare* e bokeh, céu nublado, água, bolhas,
peixe tropical, aurora, objeto 3D renderizado. Paleta de azuis e verdes "to
align with its natural influences", mais amarelos.

Fim do estilo: substituído pelo design plano no começo dos anos 2010, com o
**Frutiger Metro** do Windows 8 (2012) como transição.

---

## 3. O que o Windows manda, que vale mais que os dois

O Morune roda no Windows. Esta é a fonte que governa, e é primária:
[Materials used in Windows apps](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/materials)
e [Mica material](https://learn.microsoft.com/en-us/windows/apps/design/style/mica).

> "**Acrylic is used only for transient, light-dismiss surfaces such as flyouts
> and context menus.**"

> "*Mica* is an opaque, dynamic material that incorporates theme and desktop
> wallpaper to paint the background of **long-lived windows such as apps and
> settings**."

> "Mica is specifically designed for app performance as it **only samples the
> desktop wallpaper once** to create its visualization."

**O Bruma e o Aquário declaram `acrylic = true` na janela do aplicativo** — que é
uma janela de vida longa, exatamente o caso que a Microsoft manda usar Mica. O
`RETOMADA-UI.md` já tinha registrado o sintoma sem o nome: acrílico amostra o
desktop, não o jogo em tela cheia, e sobre um jogo colapsa em cinza chapado. A
documentação explica por quê — e Mica, amostrando o papel de parede uma vez só,
não teria esse comportamento.

Mica cai para cor sólida sozinha quando: transparência desligada nas
Configurações, **economia de bateria**, hardware fraco, **janela desativada**, ou
Windows anterior ao 22000. A terceira e a quarta são o critério de desempenho do
Morune escrito por outra pessoa.

E as duas regras finais da página:

> "**Don't** apply backdrop material more than once in an application."
>
> "**Don't** apply backdrop material to a UI element."

São as mesmas duas da Apple, ditas por outra empresa. Quando os dois donos de
plataforma proíbem a mesma coisa, não é questão de gosto.

Mica também tem **estado ativo e inativo embutido**, e a página recomenda um
sistema de duas camadas: Mica na base, e uma camada de conteúdo por cima usando
uma cor sólida de baixa opacidade (`LayerFillColorDefaultBrush`) — não outra
camada de material.

---

## 4. O que isso significa para os dois temas

Em ordem de quanto muda a sensação.

**1. Vidro só no cromo.** Apple e Microsoft dizem a mesma coisa: material de
fundo não vai em elemento de interface, e vidro nunca vai sobre conteúdo. Hoje o
`Glass` do Morune está na linha de faixa, no cartão, no item de navegação e na
linha de playlist. Tudo é vidro, então nada flutua sobre nada. Camada de
conteúdo pede cor sólida de baixa opacidade sobre a base, e é só isso.

**2. Uma luz, não uma por peça.** O `Gloss` desenha um gradiente de 160° dentro
de cada elemento, então cada painel tem sol próprio. No Aero, o realce vem de um
ponto de referência no espaço da tela e cada peça mostra a fatia que lhe cabe. Dá
para fazer no Slint sem custo por quadro: a posição do elemento na janela decide
o deslocamento do gradiente.

**3. Sombra existe e é adaptativa.** Bruma tem `shadow_strength = 0.0`. A regra
da Apple é explícita: mais opaca sobre texto, menos sobre fundo claro sólido — é
o que faz o vidro estar *acima*. Sem ela o material é um véu na mesma altura.

**4. Trocar acrílico por Mica na janela.** É a recomendação da plataforma, e
resolve de graça o pior caso do projeto (o app sobre um jogo em tela cheia).

**5. Tinta que significa.** O `accent` do Bruma é `#f2f4f8`, branco frio, da
mesma família do texto: o tema não tem cor de ação. E a tinta do Liquid Glass é
uma faixa de tons mapeada ao brilho do que está atrás, não uma cor chapada.

**6. O Aquário inverteu as camadas, e esse é o erro maior do tema.**
`surface = #b7e86a` põe verde-limão a 90% no painel. Verde e azul são a paleta do
**motivo** — céu, água, grama, aurora —, e motivo é imagem, não superfície de
controle. O cânone do estilo (Wii, iPhone 1, Sims 3) põe a cor na cena e deixa o
controle como **gel neutro e brilhante** por cima. O Aquário fez o contrário:
pintou o controle de verde e deixou a cena de fundo genérica.

Corolário: a identidade do tema deveria estar quase toda em `fundo.png` e no
brilho dos controles, e quase nada na cor do painel.

**7. O Aquário tinge o que é saturado.** `artwork_tint_strength = 0.3` puxa a cor
dominante da capa. O Aero dessatura primeiro e tinge depois, com **uma** cor do
sistema. Tingir com a cor da capa dá uma superfície que troca de identidade a
cada faixa.

**8. Realce por textura, não por gradiente.** O Aero usa bitmap. O Morune já sabe
embutir imagem em tema — o Aquário traz `fundo.png` —, então é o mesmo caminho.

**9. Acessibilidade.** Os dois sistemas mudam o material com "reduzir
transparência" e "aumentar contraste". O Morune passou a respeitar movimento em
05/09; esses dois continuam sem tratamento.

### O que não dá para fazer, e não adianta prometer

**Refração e lensing.** É a definição do Liquid Glass e exige distorcer o que
está atrás. O Slint não tem `backdrop-filter` nem shader por elemento — o
comentário do `Frost` já registrava isso. Sem lensing, o Bruma pode ser um bom
vidro fosco; não vai ser Liquid Glass.

**Desfoque real do fundo.** Mesma limitação, e é metade da fórmula do Aero. O que
dá: o Aquário controla a própria imagem de fundo, então uma segunda cópia já
desfocada e dessaturada, gerada pelo `tools/fundo-aquario.py`, pode ser desenhada
sob os painéis — o efeito sem o custo por quadro.

**Realce que segue o aparelho.** Exigiria alimentar o gradiente por quadro. O
substituto honesto é o item 2: uma luz fixa em espaço de janela.
