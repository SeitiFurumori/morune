# Vidro: Liquid Glass, Aero e o que o Windows manda

Pesquisa de 05/09/2026, para o Bruma (Liquid Glass) e o Aquário (Frutiger Aero).

O material atual dos dois temas está medido em
[00-corpus-dos-dois-temas.md](00-corpus-dos-dois-temas.md).

---

## 1. Liquid Glass, pela sessão da Apple

Fontes primárias: a sessão [Meet Liquid Glass, WWDC25](https://developer.apple.com/videos/play/wwdc2025/219/)
e as páginas [Materials](https://developer.apple.com/design/human-interface-guidelines/materials)
e [Color](https://developer.apple.com/design/human-interface-guidelines/color)
da HIG.

**Nota de método:** a HIG é aplicação JavaScript e devolve página vazia a quem a
busca por HTTP; o endpoint JSON dela responde 404. As duas páginas foram lidas
pelo navegador do Felipe, que executa o script — é a diferença entre citar a
Apple e citar quem leu a Apple.

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

### A regra da camada, nas palavras da Apple

> "**Don't use Liquid Glass in the content layer.** Liquid Glass works best when
> it provides a clear distinction between interactive elements and content, and
> including it in the content layer can result in unnecessary complexity and a
> confusing visual hierarchy. **Instead, use standard materials for elements in
> the content layer**, such as app backgrounds."

Com **uma exceção nomeada**, que interessa ao Morune:

> "An exception to this is for controls in the content layer with a transient
> interactive element like **sliders and toggles**; in these cases, the element
> takes on a Liquid Glass appearance to emphasize its interactivity **when a
> person activates it**."

Ou seja: o `Slider` de progresso e de volume pode legitimamente virar vidro **no
momento em que é usado** — e só nele.

E o teto de uso:

> "Use Liquid Glass effects sparingly... **Limit these effects to the most
> important functional elements in your app.**"

### Materiais padrão: quatro espessuras, escolhidas por significado

Para a camada de conteúdo a Apple manda usar **material padrão**, que vem em
`ultraThin`, `thin`, `regular` e `thick`.

> "Thicker materials, which are more opaque, can provide better contrast for
> text and other elements with fine features. Thinner materials, which are more
> translucent, can help people retain their context."

E a regra de escolha, que é a que o Morune quebra ao escolher superfície por cor:

> "**Avoid selecting a material or effect based on the apparent color it imparts
> to your interface**, because system settings can change its appearance and
> behavior. Instead, match the material or vibrancy style to your specific use
> case."

### Cor no vidro — e aqui o Bruma acerta metade

> "**By default, Liquid Glass has no inherent color**, and instead takes on
> colors from the content directly behind it."

> "For smaller elements like toolbars and tab bars, the system can adapt Liquid
> Glass between a light and dark appearance in response to the underlying
> content. By default, symbols and text on these elements follow a
> **monochromatic** color scheme... **Liquid Glass appears more opaque in larger
> elements like sidebars** to preserve legibility."

> "Apply color sparingly... **To emphasize primary actions, apply color to the
> background rather than to symbols or text.** ... **Refrain from adding color to
> the background of multiple controls.**"

> "If your app features colorful backgrounds or visually rich content, **prefer a
> monochromatic appearance** for toolbars and tab bars."

Isso **corrige** o que a versão anterior desta pesquisa disse. O cromo
monocromático do Bruma está certo — um tocador de música é conteúdo colorido, e
monocromático é a recomendação. A barra lateral ser mais opaca (70%) que a
superfície (12%) também está certo, e pela razão que a Apple dá.

O que falta é o **um** lugar onde a cor é obrigatória: a ação primária. Com
`accent = #f2f4f8`, o botão Tocar não tem cor de fundo nenhuma — o tema aplicou
"monocromático" também onde a regra pede exceção.

### Valor concreto para a variante Clear

> "If the underlying content is bright, consider adding a **dark dimming layer
> of 35% opacity**."

### O que a Apple proíbe

Da referência técnica que compila a documentação da SwiftUI
([LiquidGlassReference](https://github.com/conorluddy/LiquidGlassReference)):

> "Never apply to content itself (lists, tables, media)."

E, na lista de anti-padrões: *glass-on-glass stacking*, *multiple separate glass
effects without container*, *mixing Regular and Clear variants*, *tinting
everything*, *content layer glass*, *overuse — "glass everywhere"*.

> **Correção de 06/09/2026.** Eu vinha citando só o primeiro item desta lista, e
> concluí dele que vidro dentro de vidro é proibido. Está errado, e a própria
> lista tem a correção no item seguinte: o que se proíbe é vidro solto **sem
> recipiente**. Aninhar é o padrão — a Apple tem uma peça de API só para agrupar
> vidros, e a Central de Controle do iOS é um painel de vidro cheio de botões de
> vidro, cada um com orla e brilho próprios. O Felipe mandou a captura.
>
> O que a regra realmente protege é contra **borrar o que já está borrado**:
> camadas refazendo o desfoque umas das outras até o resultado virar papa. Se
> toda peça lê a mesma cópia do fundo, na posição dela, aninhar não custa nada e
> não degrada nada.
>
> Isto chegou a virar código — itens da barra lateral ficaram sem vidro por causa
> da citação pela metade. Desfeito.

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

### A origem não é a Microsoft: é uma fábrica de imagem coreana

O achado que mais muda a leitura do estilo, e que só apareceu nas fontes S.

A **Aesthetics Wiki** credita a criação a "**Asadal Design**, Microsoft, Apple" —
nessa ordem — e a imagem que ela usa como capa do verbete é

> "A stock image template created by the South Korean design firm **Asadal
> Design** circa 2007–2008... a computer monitor acting as a portal to nature,
> with tropical fish and water splashing out of the screen."

A CARI publicou uma entrevista com o presidente da Asadal, Chang N. Suh, feita
por Yunseon Yang com introdução de Sofi Xian. O que ela conta:

- A empresa montou uma **"design factory"** de imagem de banco: ícones
  vetoriais, gráficos com textura de aquarela, fotografias — vendidos como
  **"editable images"**, com tudo separado em camadas editáveis. "When users
  opened the files, everything—from water droplets to monitors, fish..."
- O critério de qualidade era o mercado, não o gosto: *"the best design was not
  the most artistic one. The best design was the one **most frequently chosen**
  by our customers and users."* Designers recebiam bônus por download, e
  **quando um estilo vendia, todos copiavam**.
- Isso se espalhou pelo web design coreano e depois pelo Leste Asiático inteiro.

Ou seja: o vocabulário visual do Frutiger Aero foi **selecionado por vendas**
numa fábrica de templates, e só depois encontrou o Windows Aero. Peixe, bolha,
globo e grama não são citação da Microsoft — são o estoque que o mercado
escolheu, replicado até virar linguagem.

Consequência para o Aquário: copiar o Vista é copiar o distribuidor, não a
fonte. O que caracteriza o estilo é a **imagem de banco em camadas** — e o
`fundo.png` é justamente a peça onde isso se aplica.

**O cânone é muito maior que a Microsoft.** Os exemplos que a literatura cita:

| categoria | exemplos |
|---|---|
| sistema | Windows Vista (2006) e 7 (2009), KDE Plasma 4 |
| aparelho | Nintendo Wii, iPhone de 1ª geração (2007), Galaxy S (2010) |
| jogo | Wii Sports (2006), Purble Place (2007), The Sims 3 (2009), Fruit Ninja (2010) |
| outros | MSN Messenger, embalagem, arquitetura de loja |

E as marcas, pela Aesthetics Wiki: Microsoft (Vista/7), Apple (Mac OS X
10.5–10.9, iOS 1–6), **Nintendo** (Wii, DS Lite, Wii U), **Samsung** (TouchWiz
Nature UX), **Sony** (XMB/PS3), LG (Flatron/era Chocolate).

Duas coisas dessa fonte que corrigem o tema:

**As cores-chave são "Blue, green, white, tertiary colours".** Branco está na
lista, e é o que falta no Aquário — cuja superfície é lima saturada.

**O controle usava gradiente e realce, e é aí que mora o brilho:** "Buttons and
other user interface elements often used **linear gradients and highlights** to
create a tactile, realistic feel."

E o revival tem nome próprio: **Neo-Aero** (2022–hoje). Um tema feito em 2026
está fazendo Neo-Aero, não reprodução de 2007 — e vale decidir isso de propósito
em vez de por acidente.

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

Fontes: [CARI](https://cari.institute/aesthetics/frutiger-aero) — lida pelo
navegador, porque recusa acesso automatizado —,
[frutiger-aero.org](https://frutiger-aero.org/frutiger-aero) e a Wikipédia.

Período 2004–2013, sucedendo o Y2K. Motivos: esqueuomorfismo, textura
brilhante, gradiente linear, *lens flare* e bokeh, céu nublado, água, bolhas,
peixe tropical, aurora, objeto 3D renderizado. Paleta de azuis e verdes "to
align with its natural influences", mais amarelos.

A lista da própria CARI — a instituição que cunhou o nome, lida direto do site —
é mais curta e mais precisa:

> Skeumorphism in UI/UX design · Glossy design · **Frutiger/humanist sans-serif
> typefaces** · **Tertiary color palettes** · Glassy/transparent materials ·
> Photographs of aurora borealis · Bokeh photography · **Macro photographs of
> grass**

Dois itens dessa lista não estavam em nenhuma outra fonte e mudam decisão:

**"Tertiary color palettes."** Não é "azul e verde": é a família terciária —
turquesa, verde-limão, âmbar, azul-celeste. É por isso que o estilo lê como
ciano e lima, e não como azul e verde primários. O `accent = #2fc0ee` do Aquário
já está nessa família; a `surface = #b7e86a` também. O problema nunca foi a
matiz — foi a camada em que ela foi aplicada.

**"Macro photographs of grass."** Fotografia macro, não ilustração. É uma
instrução direta para o `fundo.png`, hoje gerado por script como campo de
gradiente com esferas.

A CARI credita o nome a **Froyo Tam e Sofi Xian**, e lista como nomes
alternativos "Aero, Aqua, Web 2.0".

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

### A receita do acrílico, e o que ela custa

A página do [acrílico](https://learn.microsoft.com/en-us/windows/apps/design/style/acrylic)
publica a ordem das camadas:

> "The acrylic recipe: **background, blur, exclusion blend, color/tint overlay,
> noise**"

> "We added an **exclusion blend mode** layer to ensure contrast and legibility
> of UI placed on an acrylic background."

Duas camadas dessa receita não existem no Morune: a mistura por exclusão — que é
o que garante legibilidade sem escurecer tudo — e o **ruído**. O `Frost` faz o
véu e o `Gloss` faz o brilho; contraste e textura ficaram de fora.

E o custo, dito pela Microsoft:

> "Rendering acrylic surfaces is **GPU-intensive, which can increase device
> power consumption and shorten battery life**. Acrylic effects are
> automatically disabled when a device enters Battery Saver mode."

Para um aplicativo cujo critério é não atrapalhar quem está jogando, isso
encerra a discussão sozinho.

Os "don'ts" da página batem um a um com os dois temas:

> "**Don't** put desktop acrylic on large background surfaces of your app."
>
> "**Don't** place multiple acrylic panes next to each other because this
> results in an undesirable visible seam."
>
> "**Don't** place accent-colored text over acrylic surfaces."

E as duas regras finais da página de materiais:

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

**5. Cor só na ação primária, e ela está faltando.** O cromo monocromático do
Bruma está **certo** — a HIG recomenda monocromático para app de conteúdo
colorido. O erro é ter aplicado isso também ao único lugar em que a cor é
obrigatória: "to emphasize primary actions, apply color to the background". Com
`accent = #f2f4f8`, o botão Tocar não tem cor. E a tinta do Liquid Glass é uma
faixa de tons mapeada ao brilho do que está atrás, não uma cor chapada por cima.

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

---

## 6. Placar das fontes

Contra a lista de tiers do Felipe, para ficar claro o que sustenta o quê.

| tier | fonte | lida? |
|---|---|---|
| 🟩 S | Aesthetics Wiki | **sim** — pelo navegador; recusa acesso automatizado (402) |
| 🟩 S | CARI | **sim** — verbete e a entrevista da Asadal; recusa automatizado (403) |
| 🟩 S | Apple HIG / Developer | **sim** — Materials e Color, pelo navegador (é SPA) |
| 🟩 S | WWDC / Apple Design | **parcial** — só "Meet Liquid Glass" (219) |
| 🟨 A | Wikipédia | sim — mas **nenhuma** das referências acadêmicas que ela cita |
| 🟨 A | 2000 Aesthetics Wiki | **não** |
| 🟨 A | Apple Developer — doc do Liquid Glass | **não** (Adopting Liquid Glass, API `glassEffect`) |
| 🟨 A | galerias de referência | **não** |
| 🟧 B | Are.na / Pinterest / Dribbble | **não** |

Fora da lista, e que entraram porque são primárias de mecanismo: as duas
patentes do Aero, a documentação de materiais do Windows (Mica e acrílico), e a
engenharia reversa do DWM.

**Identificada e não lida:** o guia de design do Windows 7
(`tilsgee.github.io/DesignGuidelinesArchive/aero7.pdf`) — o PDF passa de 10 MB e
estourou o limite da ferramenta. É a lacuna de maior valor que sobra.

**Limite de método que vale repetir:** quase tudo aqui é **normativo** — o que as
empresas dizem que se deve fazer. Medição do artefato renderizado só existe para
o Aero, pela engenharia reversa do DWM. Para o Liquid Glass não há nada
equivalente nesta pesquisa.

---

## 7. Os artefatos, olhados

O Felipe forneceu capturas de três exemplos canônicos: Wii Menu, XMB do PS3 e
iOS 6.

**Limite, dito de frente:** foram **olhados, não medidos**. Imagem colada na
conversa não dá para amostrar pixel, então não há aqui altura de realce em
porcentagem nem parada de gradiente. Para medir, os arquivos precisam estar em
disco. Nada nesta seção deve ser lido como número.

**Wii Menu.** A lição não é o brilho, é a **moldura**. Cada canal é um cartão
branco com contorno ciano fino e sombra suave, e o conteúdo vive **dentro** de
uma moldura, sem encostar na borda. O cromo é branco; o acento aparece só em
contorno e em fio. A barra inferior é uma **curva**, com um fio de acento
correndo por cima dela. E os canais vazios são brancos com um cinza fraco —
placeholder **neutro, sem símbolo**, que é a mesma decisão que o Morune tomou na
ficha de capa, por outro caminho.

**XMB do PS3.** O mesmo princípio no extremo: **não há painel nenhum**. Fundo
com uma fita de luz atravessando, e a interface inteira é glifo branco e texto
pequeno por cima. Zero cor no cromo, cor toda no campo.

**iOS 6.** A lição do brilho. O realce do ícone é uma **forma curva** ocupando a
metade de cima, com a borda inferior arqueada — não é gradiente linear. E está
no **controle**, não no fundo.

**A convergência dos três, numa frase:** a cor vive na cena, o cromo é claro e
neutro, e o brilho é uma curva no controle. O Aquário hoje faz o inverso nos
três eixos.

---

## 8. Decisão conjunta: o guia do Windows 7 fica de fora

Registrado como **decisão**, e não como lacuna — a diferença importa para quem
ler isto depois.

O `aero7.pdf` foi identificado, e não foi lido: passa de 10 MB e estoura o
limite da ferramenta de busca. Havia saída (baixar o arquivo localmente), e o
Felipe avaliou que não era necessário — as duas patentes e a engenharia reversa
do DWM já cobrem o mecanismo, e o guia é sobre diretriz de aplicativo Windows,
que não é o que falta aqui.

Fica registrado que a fonte existe, onde ela está, e por que ninguém foi atrás:
para o caso de a pergunta voltar.
