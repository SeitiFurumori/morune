# Handoff: Morune — refino visual (tema Pulse / Paper)

## Visão geral
Redesenho visual da interface do Morune, player de música desktop nativo (Rust + Slint, Windows, backend Spotify).
A **estrutura de navegação e as telas não mudam** — o que muda é hierarquia tipográfica, respiro, acabamento de
cartões/listas e o tratamento de estados sem capa. Restrição de projeto: a UI não pode atrapalhar quem está jogando,
então **nada de sombras, blur, gradientes animados ou transições caras**. O único gradiente usado é placeholder de capa
e a marca d'água da playlist "Músicas curtidas".

## Sobre os arquivos deste pacote
`mockup.html` é uma **referência de design feita em HTML** — um protótipo que mostra aparência e medidas
pretendidas, **não código para copiar**. A tarefa é **recriar esses layouts em Slint**, usando os componentes,
`global` de tema e padrões já existentes no repositório do Morune. Onde o HTML usa truques de navegador
(`aspect-ratio`, `repeating-linear-gradient`, `ellipsis`), use o equivalente idiomático de Slint
(`Rectangle` com `width == height`, imagem/`Path` de placeholder, `Text { overflow: elide; }`).

Abra o arquivo em qualquer navegador: os três mockups ficam lado a lado num canvas, com ids `1a`, `1b`, `1c`.

## Fidelidade
**Alta (hi-fi).** Cores, tamanhos de fonte, pesos, alturas de linha e espaçamentos são valores finais e devem ser
reproduzidos. O que é deliberadamente ilustrativo: capas de álbum (listras diagonais = "entra imagem aqui") e
os ícones desenhados com retângulos/triângulos CSS — substitua pelo icon set real do app, mantendo o tamanho ótico
(16px na navegação, 12–14px no player).

## Regra dura: tudo é token
Nenhuma composição pode depender da paleta escura. Cada mockup declara os tokens na sua raiz e todo filho consome
o token, nunca o hex. Em Slint, isso deve virar um `global Theme` (ou struct carregada de arquivo), porque o usuário
pode trocar o tema inteiro por um arquivo.

Verificações que o design já passou e que a implementação precisa manter:
- o mesmo layout renderizado em Pulse (escuro) e Paper (claro) — ver bloco Paper em `1c`;
- toda playlist/cartão/faixa funciona **com e sem capa**;
- nenhuma cor hardcoded fora do tema; contraste vem de `--line` / `--muted`, não de sombra.

## Design tokens

### Escala (independente de tema)
| token | valor | uso |
|---|---|---|
| `r1` | 6px | capas 40/48px, campos, itens de navegação |
| `r2` | 10px | cartões de prateleira, capa 160px, blocos de lista |
| `r3` | 14px | contêineres grandes (reservado) |
| espaçamento | 2 / 4 / 6 / 8 / 10 / 12 / 14 / 16 / 20 / 22 / 26 / 30 / 34 / 40 | passos usados no arquivo |
| linha de 1px | `line` / `line-strong` | substitui sombra em 100% dos casos |

### Pulse (escuro — padrão)
| token | hex |
|---|---|
| `bg` | `#0a0612` |
| `surf1` | `#150d24` |
| `surf2` | `#1f1434` |
| `chrome` (sidebar, player, titlebar) | `#0d0817` |
| `text` | `#f2ecff` |
| `muted` | `rgba(242,236,255,.60)` |
| `faint` | `rgba(242,236,255,.38)` |
| `line` | `rgba(242,236,255,.08)` |
| `line-strong` | `rgba(242,236,255,.14)` |
| `accent` | `#c66bff` |
| `accent-soft` | `rgba(198,107,255,.13)` |
| `on-accent` (texto sobre o acento) | `#1a0a2a` |

### Paper (claro)
| token | hex |
|---|---|
| `bg` | `#faf8fc` · `surf1` `#f2eef8` · `surf2` `#e6e0f0` · `chrome` `#ffffff` |
| `text` | `#1a1226` · `muted` `rgba(26,18,38,.62)` · `faint` `rgba(26,18,38,.42)` |
| `line` | `rgba(26,18,38,.09)` · `line-strong` `rgba(26,18,38,.16)` |
| `accent` | `#7b2fd6` · `accent-soft` `rgba(123,47,214,.10)` |

### Tipografia
Duas famílias apenas:
- **UI/display**: `Segoe UI Variable Display` → `Segoe UI` → sans do sistema. (No HTML há fallback web `Source Sans 3`; no app use a fonte do sistema.)
- **Mono**: qualquer mono do sistema (o HTML usa `JetBrains Mono`) — só para números, durações, kickers e contadores.

Escala (4 níveis, é a mudança principal em relação ao estado atual):

| papel | tamanho / peso / tracking |
|---|---|
| Kicker / rótulo de seção | 10.5–11px, 500–600, mono, uppercase, tracking .09–.12em, cor `faint` |
| Título de tela ("Início") | 30px / 600 / -.02em |
| Nome da playlist (cabeçalho) | 42px / 700 / -.03em / line-height 1.05 |
| Título de prateleira | 17px / 600 / -.01em |
| Título de item (faixa, cartão, playlist) | 13.5–14px / 600 |
| Secundário (artista, subtítulo, álbum) | 12–12.5px / 400, cor `muted` |
| Números e durações | 11.5–12.5px, mono, cor `faint` (cor `accent` na faixa tocando) |

## Telas

### 1a — Início (mockup 1280 × 1350; janela real ~1280×800 com rolagem)
**Titlebar** 36px, fundo `chrome`, borda inferior `line`. À esquerda "MORUNE" 11.5px mono uppercase tracking .09em cor `faint`; à direita três botões de 44×36 (minimizar / maximizar / fechar).

**Sidebar** 248px, fundo `chrome`, borda direita `line`, coluna flex:
- Cabeçalho `padding: 20px 20px 16px`, gap 11px: símbolo da marca (três traços verticais alinhados na base, alturas 11/20/7px, larguras 3/5/2px, raio 2px, cor `accent`, o terceiro com opacidade .6) + "Morune" 16px/600/-.01em + chevron `«` de recolher à direita.
- Navegação: `padding: 0 12px`, gap 2px, itens de 38px, `padding: 0 12px`, raio `r1`, ícone 16px + rótulo 14px. Ativo: fundo `accent-soft`, texto `text` peso 600, e uma **barra de 2px × (altura − 18px) em `accent`** encostada na borda esquerda do item, raio 2px. Inativo: cor `muted`.
- Bloco de playlists: rótulo mono "PLAYLISTS" + contagem à direita; campo "Filtrar playlists" com 32px, `surf1`, borda `line-strong`, raio `r1`, ícone de lupa 11px + placeholder 12.5px `faint`.
- Lista de playlists: linhas de **52px** (antes eram uma linha só de nome — agora são duas: nome 13.5px + contagem de faixas 11.5px `faint`), gap 1px, `padding: 0 12px`, raio `r1`. Capa 40×40 raio `r1`. Ativa: fundo `surf1`.
  - "Músicas curtidas" é sempre a primeira e sua capa é o único gradiente de marca permitido: `linear-gradient(135deg, accent, #6b2fb3)` com um losango branco de 9px, opacidade .85.
  - **Sem capa**: quadrado 40px com borda **tracejada** `line-strong` e o símbolo da marca em miniatura (traços 8/14/5px, o do meio em `accent`).
- Rodapé: borda superior `line`, "Configurações" (item de 36px) e conta com avatar circular de 22px em `surf2` + inicial.

**Conteúdo**: cabeçalho `padding: 34px 40px 0` com kicker "BOA NOITE" + "Início" 30px, e à direita um chip de status: 30px, raio 999px, borda `line-strong`, ponto de 6px em `accent` + "Spotify conectado" 12px.
Corpo `padding: 30px 40px 8px`, **gap de 30px entre prateleiras**. Ordem obrigatória:
1. **Músicas curtidas** — lista, não cartões. Bloco `surf1`, borda `line`, raio `r2`; linhas de 46px separadas por borda `line`; colunas: número (18px, direita, mono) · capa 30px (opcional) · título 13.5px/600 + artista 12.5px `muted` na mesma linha de base, gap 10px · duração mono à direita. Faixa tocando: fundo `accent-soft`, título e duração em `accent`, e no lugar do número um **equalizador estático** de 3 barras de 2px (alturas 6/11/9px) — estático de propósito, sem animação.
2. **Feito para você** — cartões de 158px: capa quadrada raio `r2` borda `line`; bloco de rótulo com **min-height 40px** (garante alinhamento entre cartões de 1 e 2 linhas), título 13.5px/600 + subtítulo 12px `muted`. Último cartão do trilho com opacidade .45 = affordance de rolagem horizontal.
3. **Estações recomendadas** — mesmos cartões.
4. **Seus mais ouvidos** — mesmos cartões; subtítulo carrega o tipo ("Artista · 412 plays", "Álbum · Biosphere").

Cartão sem capa: quadrado com borda tracejada `line-strong` e o símbolo da marca em escala grande (traços 22/40/14px, larguras 5/9/4px, o do meio em `accent`), centralizado.

**Player** (rodapé, largura total): ver 1c.

### 1b — Playlist aberta (mockup 900 × 760)
- Linha de volta: `padding: 22px 34px 0`, botão 30×30 raio `r1` borda `line-strong` com `‹`, seguido de breadcrumb mono "BIBLIOTECA / PLAYLIST".
- Cabeçalho `padding: 26px 34px 24px`, gap 26px, itens alinhados à base:
  - capa **160×160** raio `r2` borda `line`;
  - coluna: kicker "PLAYLIST" 11px mono em `accent` · nome 42px/700/-.03em · linha de meta 12.5px com dono em 600 `text` e demais em `muted`, separados por pontos de 3px em `faint`: dono · nº de faixas · duração total (mono);
  - ações: **Tocar** — 38px de altura, `padding: 0 22px`, raio 999px, fundo `accent`, texto `on-accent` 13.5px/700, triângulo de 8px; ao lado dois botões circulares de 38px com borda `line-strong` (embaralhar, mais opções).
- Controles de lista `padding: 0 34px 14px`: campo "Filtrar faixas" (210px, 32px, `surf1`) · divisor vertical de 1px × 20px · chips de 28px raio 999px: **Ordem da lista** (ativo: fundo `accent-soft`, borda e texto `accent`, peso 600, seta `▲`/`▼` de 10px indicando direção), Título, Artista, Álbum, Duração (inativos: borda `line-strong`, texto `muted`).
- Tabela: cabeçalho de 30px em mono uppercase 10.5px `faint` com borda inferior `line-strong`; colunas fixas **# 22px · Título (flex) · Álbum 170px · Dur. 44px à direita**, gap 16px, `padding: 0 12px`.
  Linhas de **52px**, borda inferior `line`, zebra por superfície (`surf1` nas pares — sem sombra). Título 14px/600 + artista 12.5px `muted` embaixo (margin-top 3px). Álbum com elipse. Sem álbum: `—` em `faint`.
  Faixa tocando: mesmo equalizador estático no lugar do número, título e duração em `accent`.

### 1c — Peças e estados
- **Sidebar recolhida**: 64px, coluna centralizada, `padding: 18px 0 12px`, gap 16px. Marca no topo; ícones de navegação em alvos de 40×36 (ativo com fundo `accent-soft` e ícone em `accent`); divisor de 28×1px; capas de playlist 40×40 empilhadas com gap 8px; rodapé com ícone de configurações e avatar de 24px. Sem rótulos, sem tooltip desenhado no mockup (implementar tooltip nativo).
- **Cartão com capa / sem capa / lista sem imagem nenhuma** lado a lado: a lista sem arte usa as mesmas colunas, só remove a coluna de capa — o número mantém a coluna de 18px, então o texto continua alinhado verticalmente entre listas com e sem arte.
- **Bloco Paper**: prateleira "Seus mais ouvidos" com cartões de 132px e uma lista embutida, provando que o sistema inteiro sobrevive à troca de tema.
- **Barra de reprodução** (rodapé, largura total, altura **78px** = 2px de progresso + 76px de conteúdo):
  - trilha de progresso **no topo da barra**, 2px, fundo `line-strong`, preenchida em `accent` — leitura periférica sem custo de render;
  - três zonas de largura fixa: **esquerda 250–280px** (capa 46–48px raio `r1` + título 13.5px/600 + "Artista · Álbum" 12px `muted`, ambos com elipse) · **centro flex** (embaralhar · anterior · play/pause 36px circular · próxima · repetir, gap 20px; abaixo, tempo decorrido mono 11.5px alinhado à direita em 34px + barra de 3px raio 2px preenchida em `text` + duração, largura máxima 380–420px) · **direita 250–280px** (chip "Fila 12" de 26px raio 999px borda `line-strong`, e volume: ícone + trilha de 76px × 3px).
  - Play em `accent` com glifo em `chrome` (variante 1a) ou circular com borda `text` (variante 1c) — escolher uma; recomendo a de `accent` para o botão primário.
  - Repetir/embaralhar ativos são coloridos em `accent`; inativos em `muted`.

## Interações e comportamento
Sem animação de layout. Só mudanças de cor/fundo instantâneas (ou ≤ 80ms se o Slint já tiver curva padrão no projeto):
- hover em linha de lista/faixa/playlist: fundo `surf1` (Pulse) — nada de escala, sombra ou translação;
- hover em cartão: apenas a borda vai de `line` para `line-strong`;
- pressionado: fundo `surf2`;
- foco de teclado: contorno de 1px em `accent` no raio do elemento (obrigatório: o app é navegável por teclado enquanto o usuário joga);
- chip de ordenação: clicar no ativo inverte a direção (`▲`/`▼`); clicar em outro o torna ativo em ordem ascendente;
- recolher a sidebar: troca 248px ↔ 64px sem animação de largura;
- estado tocando: só troca de cor + equalizador estático; **não** animar as barras (custo de repaint constante no rodapé).

## Estado necessário
`sidebar_collapsed: bool` · `active_nav: enum {Home, Search, Library}` · `playlist_filter: string` ·
`track_filter: string` · `sort_key: enum {ListOrder, Title, Artist, Album, Duration}` · `sort_desc: bool` ·
`current_track_id` · `is_playing: bool` · `progress_ms` / `duration_ms` · `shuffle: bool` ·
`repeat_mode: enum {Off, All, One}` · `volume: float` · `queue_len: int` · `theme: Theme` (carregável de arquivo).
Cada item de lista precisa de `cover: image` **opcional** — `has_cover == false` cai no símbolo da marca.

## Assets
- **Símbolo da marca**: três traços de espessuras diferentes alinhados na base. Existem três escalas no mockup: 20px (sidebar/marca), 40px (capa ausente em lista/player), 160px/cartão (capa ausente grande). Exportar como `.svg` do repositório do projeto — o desenho no HTML é aproximação com retângulos.
- **Ícones** (16px nav, 12–14px player): buscar, biblioteca, configurações, embaralhar, anterior, play, pause, próxima, repetir, fila, volume, lupa, chevron de recolher, voltar. Usar o icon set já adotado pelo app; os do mockup são placeholders geométricos.
- **Capas**: as listras diagonais são placeholder de imagem real vinda da API.

## Arquivos
- `Morune UI.dc.html` — os três mockups (`1a` Início, `1b` playlist aberta, `1c` peças/estados/Paper). Abre direto no navegador.
