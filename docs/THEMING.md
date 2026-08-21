# Temas

Um tema do Morune muda muito mais que cor. Ele decide onde a barra lateral fica,
se o player esta em cima ou embaixo, se colecoes aparecem em grade ou lista, o
quanto a interface e densa, quanto tempo duram as animacoes e o formato de cada
canto.

Tudo em TOML. Nenhuma linha de codigo, nunca.

---

## Anatomia

```
meu-tema/
  manifest.toml    obrigatorio  identificacao e versao do esquema
  theme.toml       opcional     cor, tipografia, forma, movimento, efeitos
  layout.toml      opcional     composicao da tela
  assets/          opcional     imagens e icones
  fonts/           opcional     fontes empacotadas
```

Um `.musicpack` e exatamente esse diretorio compactado em ZIP.

Os dois arquivos opcionais sao independentes: um tema pode mudar so as cores, ou
so a composicao, ou os dois.

Pasta de temas do usuario:

```
%APPDATA%\morune\Morune\config\themes\
```

---

## manifest.toml

```toml
schema_version = 1          # obrigatorio
id = "meu-tema"             # obrigatorio, [a-z0-9_-], vira nome de pasta
name = "Meu Tema"           # obrigatorio
version = "1.0.0"
author = "Seu Nome"
description = "Uma frase sobre o tema."
license = "MIT"
homepage = "https://exemplo"
based_on = "midnight"       # herda deste tema
min_app_version = "0.1.0"
appearance = "dark"         # "dark" | "light", so agrupa na lista
```

`id` precisa bater com o nome da pasta. Um tema em que os dois divergem e
ignorado na descoberta.

Um campo desconhecido no manifesto e **erro**, nao aviso: um `autor = "..."` com
erro de digitacao seria silenciosamente perdido, e e melhor saber na hora.

### Heranca

Com `based_on`, o tema pai e carregado primeiro e o filho aplica por cima. Herdar
e simplesmente **nao declarar** uma secao. O tema `pulse` que acompanha o
aplicativo faz isso: declara `[color]`, `[shape]` e `[motion]`, nao declara
`layout.toml`, e por isso usa a composicao do `midnight`.

A substituicao e **por secao**, nao por campo. Declarar `[color]` substitui a
paleta inteira. Isso mantem o formato previsivel para quem escreve a mao.

Cadeias de heranca sao limitadas a 4 niveis, e um ciclo termina no limite em vez
de travar.

---

## theme.toml

### `[color]`

Nomes semanticos, nao literais. Um tema claro define `surface` como quase branco
e nada acima precisa saber disso.

| Campo | Papel |
|---|---|
| `background` | fundo da janela |
| `surface` | paineis sobre o fundo (cards, sidebar) |
| `surface_raised` | segundo nivel (menus, modais) |
| `player_background` | fundo do player |
| `sidebar_background` | fundo da barra lateral |
| `text` | texto principal |
| `text_muted` | texto secundario |
| `text_on_accent` | texto sobre a cor de destaque |
| `accent` | cor de marca: botao primario, progresso |
| `accent_hover` | destaque sob o cursor |
| `border` | separadores e contornos |
| `hover` | realce sob o cursor |
| `selected` | realce de item selecionado |
| `focus_ring` | anel de foco de teclado |
| `success` / `warning` / `danger` | estados |
| `shadow` | sombra de superficies elevadas |
| `scrollbar` | trilho de rolagem |

Formatos aceitos:

```toml
accent = "#6dd49e"          # rrggbb
accent = "#6dd49eaa"        # rrggbbaa
accent = "#6d9"             # rgb
accent = "#6d9a"            # rgba
accent = "rgb(109, 212, 158)"
accent = "rgba(109, 212, 158, 0.5)"   # alfa como fracao
accent = "rgba(109, 212, 158, 128)"   # ou como byte
```

### `[typography]`

```toml
family = "Inter"            # vazio = fonte do sistema
display_family = "Inter"    # vazio = usa family
bundled_font = "Inter.ttf"  # arquivo dentro de fonts/
scale = 1.0                 # 0.5 .. 3.0, multiplica todos os tamanhos
size_xs = 11.0              # 6 .. 96
size_sm = 12.0
size_md = 14.0
size_lg = 17.0
size_xl = 22.0
size_display = 32.0
weight_normal = 400
weight_medium = 500
weight_bold = 700
line_height = 1.4           # 1.0 .. 2.5
letter_spacing = 0.0
```

`scale` existe para o usuario aumentar a interface inteira sem editar oito
valores. A escala de DPI do Windows e aplicada **por cima** disso, nao no lugar.

### `[shape]`

```toml
radius_sm = 4.0
radius_md = 8.0
radius_lg = 14.0
radius_artwork = 6.0        # 0 deixa capas quadradas
radius_avatar = 999.0       # valor alto vira circulo
spacing_xs = 4.0
spacing_sm = 8.0
spacing_md = 12.0
spacing_lg = 20.0
spacing_xl = 32.0
border_width = 1.0
progress_thickness = 4.0
scrollbar_width = 10.0
```

### `[motion]`

```toml
enabled = true              # false zera todas as duracoes
speed = 1.0                 # 0.0 .. 5.0, multiplica as duracoes
duration_fast = 120         # ms, hover e foco
duration_normal = 200       # ms, troca de pagina
duration_slow = 320         # ms, transicoes grandes
easing = "ease-in-out"      # linear | ease-in | ease-out | ease-in-out
```

A interface recebe as duracoes **ja resolvidas**: com movimento reduzido tudo
chega como zero, e nenhuma tela precisa checar a preferencia.

### `[effects]`

```toml
window_opacity = 1.0        # 0.2 .. 1.0
acrylic = false             # fundo translucido estilo Windows, quando disponivel
shadow_strength = 0.5       # 0.0 .. 1.0
backdrop_blur = 0.0         # px atras de modais, 0 desliga
artwork_tint = true         # tinge o fundo com a capa
artwork_tint_strength = 0.35
```

O piso de `window_opacity` e alto de proposito: uma janela quase transparente
deixaria o aplicativo irrecuperavel pelo proprio usuario.

---

## layout.toml

### `[window]`

```toml
default_width = 1180.0      # 480 .. 8000
default_height = 760.0      # 360 .. 8000
min_width = 720.0
min_height = 480.0
custom_titlebar = false
show_page_title = true
```

O tamanho do tema so vale na primeira abertura; depois o ultimo tamanho
escolhido pelo usuario tem prioridade, e trocar de tema nunca redimensiona a
janela.

### `[sidebar]`

```toml
position = "left"           # left | right | hidden
width = 232.0               # 120 .. 600
collapsed_width = 64.0      # 40 .. 200
collapsible = true
start_collapsed = false
show_icons = true
show_labels = true
show_playlists = true
items = ["home", "search", "library"]
```

### `[player]`

```toml
position = "bottom"         # bottom | top | sidebar
height = 88.0               # 48 .. 260
show_artwork = true
artwork_size = 56.0
show_progress = true
progress_edge_to_edge = false   # barra colada na borda da janela
show_volume = true
show_shuffle_repeat = true
show_queue_button = true
show_times = true
center_controls = true
```

### `[content]`

```toml
density = "normal"          # comfortable | normal | compact
view_mode = "grid"          # grid | list | compact
card_width = 168.0          # 80 .. 480
row_height = 44.0           # 24 .. 120
max_content_width = 0.0     # 0 = ocupa tudo
show_hero = true
hero_height = 240.0
page_size = 100             # 1 .. 1000
```

---

## O que acontece com um tema errado

**Um tema quebrado nunca impede o aplicativo de abrir.** Essa e a garantia
central, e ela e testada.

| Problema | Resultado |
|---|---|
| Pasta inexistente | tema embutido, erro registrado |
| `manifest.toml` invalido | tema embutido, erro registrado |
| `theme.toml` invalido | o tema carrega; tokens caem para o padrao, erro registrado |
| Valor fora de faixa | ajustado para o limite, aviso registrado |
| `NaN` ou infinito | substituido por valor finito, aviso registrado |
| Secao ausente | herda do pai ou usa o padrao |
| Campo ausente | usa o padrao |
| Campo com nome errado | erro de leitura da secao — nao passa em silencio |
| Contraste ilegivel | **aplicado assim mesmo**, com aviso |

Sobre a ultima linha: um tema pode ser de baixo contraste de proposito, e nao
cabe ao aplicativo proibir. Mas o aviso aparece, com a razao WCAG calculada, nos
diagnosticos em Configuracoes.

Valores como `sidebar.width = 9999` ou `player.height = 1` sao corrigidos em
silencio com aviso, em vez de recusarem o tema. O objetivo e sempre chegar numa
tela utilizavel.

---

## Fluxo de trabalho

### Criar um tema

1. Configuracoes → **Duplicar** num tema existente. A copia e completa e nao
   herda do original, entao apagar o original nao a quebra.
2. Configuracoes → **Abrir pasta de temas**.
3. Edite `theme.toml` ou `layout.toml`.
4. Configuracoes → **Recarregar**.

### Exportar e compartilhar

Configuracoes → **Exportar** gera um `.musicpack`. So sao empacotados arquivos
que o importador aceitaria de volta, entao um pacote exportado pelo Morune
sempre pode ser reimportado.

### Importar

Configuracoes → **Importar tema**. O pacote passa por validacao completa antes
de qualquer escrita em disco — ver [SECURITY.md](../SECURITY.md). Um pacote
recusado nunca deixa residuo.

### Voltar atras

Configuracoes → **Restaurar padrao** volta ao tema embutido. Ele nunca pode ser
apagado nem corrompido, porque nao esta em disco: e o `Default` do proprio
codigo.

---

## Temas que acompanham o aplicativo

| Id | O que demonstra |
|---|---|
| `midnight` | tema embutido. Escuro, verde, cantos medios, grade, sidebar a esquerda, player embaixo |
| `paper` | o contrario em todos os eixos: claro, serifado, cantos retos, lista, sidebar a **direita**, player em **cima**, progresso de ponta a ponta, densidade compacta |
| `pulse` | tema **derivado**: herda a composicao do `midnight` e muda so cor, forma e movimento. E o exemplo minimo para quem for escrever o proprio |

`paper` existe justamente para tornar visivel que a customizacao vai alem de
trocar cor. Compare as capturas geradas por `tools/snapshot.ps1`.

---

## Limites atuais

- **Layout arbitrario nao e possivel.** Um tema escolhe entre os eixos
  oferecidos; nao desenha telas novas. Layout livre via `slint-interpreter` esta
  previsto atras do Developer Mode, e o motivo de nao ser o padrao esta em
[ADR-0004](adr/0004-temas-declarativos.md).
- **Icones ainda nao sao substituiveis por tema.** Os caminhos vetoriais estao
  na interface. Icones vindos de `assets/` estao no roteiro.
- **A marca nao e customizavel, e isso e proposital.** O simbolo do Morune tem
  geometria e cor fixas (`#8B63F6 → #6937EC`), fora do esquema de tema: aparece
  na barra lateral, na janela, na barra de tarefas, na bandeja e no instalador.
  Um tema muda tudo o que o usuario ve, menos o que diz de qual aplicativo se
  trata. A especificacao esta em
  `assets/brand/morune-logo-system/SPECIFICATION.md`.
- **Fontes empacotadas ainda nao sao registradas.** O campo `bundled_font` ja
  existe no esquema, mas o carregamento nao foi implementado; fontes instaladas
  no sistema funcionam normalmente por `family`.
- **Recarga a quente** existe na crate (`morune-theme`, feature `hot-reload`)
  mas ainda nao esta ligada ao Developer Mode na interface. Use **Recarregar**.
