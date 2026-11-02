# Temas

Um tema do Otomori e **dado, nunca codigo**: dois arquivos TOML numa pasta, e
opcionalmente uma imagem de fundo e fontes. A tela decide o layout; o tema
decide cor, forma, fonte, movimento e efeitos.

O jeito mais facil de fazer um tema nao e escrever TOML: e abrir
**Configuracoes > Aparencia**, ajustar no modo liberdade total e clicar em
**Salvar como tema**. Este guia e para quem quer ir alem, ou entender o que o
arquivo salvo diz.

> O guia da 1.0, com `layout.toml`, cenas e o resto do formato da tela em Slint,
> esta em [archive/THEMING-1.0-slint.md](archive/THEMING-1.0-slint.md). Temas
> da 1.0 continuam abrindo, mas `layout.toml` e `[scene]` sao ignorados.

## Onde ficam

```
%APPDATA%\otomori\Otomori\config\themes\
  meu-tema\
    manifest.toml   obrigatorio
    theme.toml      cor, fonte, forma, movimento, efeitos
    fundo.jpg       opcional, citado em [background]
    fonts\          opcional
```

Os quatro temas de fabrica (Lume, Vidro, Grade, Claro) ficam na mesma pasta. O
app so os atualiza enquanto continuam como ele instalou.

## manifest.toml

```toml
schema_version = 1          # obrigatorio
id = "meu-tema"             # obrigatorio, [a-z0-9_-], igual ao nome da pasta
name = "Meu Tema"           # obrigatorio
version = "1.0.0"
author = "Seu Nome"
description = "Uma frase sobre o tema."
appearance = "dark"         # "dark" | "light"
based_on = "lume"           # opcional: herda deste tema
```

`appearance = "light"` muda tambem o esquema de cores das barras de rolagem e
dos campos de texto.

Um campo desconhecido no manifesto e **erro**: um `autor = "..."` digitado
errado se perderia em silencio. Com `based_on`, o pai carrega primeiro e o
filho substitui **secoes inteiras** por cima -- declarar `[color]` troca a
paleta toda, entao copie a secao do pai inteira e mude o que quiser.

## theme.toml

Toda chave e opcional. Cor e `#rrggbb` ou `#rrggbbaa`.

### `[color]`

| Chave | Onde aparece |
|---|---|
| `background` | fundo da janela. Com alfa e `acrylic`, deixa ver o que esta atras |
| `surface` | paineis e cartoes |
| `surface_raised` | menus e o que flutua por cima |
| `sidebar_background` | barra lateral |
| `player_background` | barra de reproducao |
| `text`, `text_muted` | texto principal e secundario |
| `accent`, `accent_hover` | botao de tocar, progresso, item ativo |
| `text_on_accent` | texto em cima do acento |
| `border`, `border_highlight` | contorno dos paineis e a aresta de luz |
| `hover`, `selected`, `focus_ring` | estados de interacao |
| `danger`, `success` | avisos |

A cor de destaque e a paleta escolhidas em Configuracoes vencem as do tema, sem
mudar o arquivo.

### `[typography]`

`family` e `display_family` (titulos) recebem um nome de familia instalada;
se faltar, cai no Segoe UI. `size_xs` a `size_xl` e `size_display` em pixels,
multiplicados por `scale`. `weight_normal`, `weight_medium`, `weight_bold`,
`line_height` e `letter_spacing` (px).

### `[shape]`

`radius_sm`, `radius_md`, `radius_lg` e `radius_artwork` (capas), em px;
`spacing_xs` a `spacing_xl`; `border_width` (contorno dos paineis, `0` tira) e
`progress_thickness` (a barra de progresso). Em `[control]`, `button_radius`:
`999` da botoes em pilula.

### `[motion]`

`duration_fast`, `duration_normal`, `duration_slow` em ms, divididos por
`speed`. `spring_stiffness` e `spring_damping` dao a mola da troca de faixa.
`enabled = false` desliga todo movimento -- o mesmo que a preferencia de
reduzir movimento.

Qualquer que seja o tema, **o movimento para sozinho** quando um jogo ou video
ocupa a tela cheia, e com a janela minimizada.

### `[effects]`

| Chave | Efeito |
|---|---|
| `acrylic` | pede o acrilico do Windows atras da janela. Com a transparencia desligada no Windows, o fundo vira solido sozinho |
| `window_opacity` | opacidade da janela com acrilico, de 0,2 a 1 |
| `material` | desenho dos paineis: `legacy`, `liquid` ou `aero` |
| `backdrop_blur` | desfoque atras dos paineis de vidro, em px |
| `artwork_glow` | brilho da cor da capa em volta dela, de 0 a 1 |
| `breathe` | esse brilho pulsa de leve com a musica |
| `artwork_tint`, `artwork_tint_strength` | a capa tinge o fundo |
| `artwork_accent` | a cor da capa vira o acento vivo (progresso, "tocando") |
| `artwork_paints_ui` | a cor da capa tinge tambem cabecalhos, selecao e foco |

### `[background]`

`image` aponta um arquivo dentro da pasta do tema. Fundo com GIF ou video se
escolhe em Configuracoes, nao no tema: e escolha de cada pessoa, e o peso de
um video 4K e dela.

## O que acontece com um tema errado

**Um tema quebrado nunca impede o app de abrir.** Pasta ou manifesto invalidos
caem no tema padrao; valor fora de faixa e ajustado ao limite; campo ausente
usa o padrao. Tudo vai para o registro (`otomori.log`).

## Guardar, exportar e importar

Em **Configuracoes > Aparencia > Guardar e compartilhar**:

- **Salvar como tema** grava o que esta na tela como um tema seu;
- **Exportar** gera um `.musicpack` do tema ativo;
- **Importar** le um `.musicpack`. O pacote e conferido inteiro antes de
  qualquer escrita em disco -- id restrito, caminhos contidos, limites de
  tamanho -- e um pacote recusado nao deixa resto. Ver
  [SECURITY.md](../SECURITY.md).
