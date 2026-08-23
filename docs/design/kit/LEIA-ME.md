# Kit de design do Morune

Seis páginas HTML que reproduzem a interface do Morune para levar a uma
ferramenta de design e voltar com **temas implementáveis**.

## Por que HTML e não print

Print vira desenho bonito que ninguém consegue aplicar. Aqui cada cor, medida e
raio é uma **variável CSS com o mesmo nome da chave do `theme.toml`**:

| CSS | TOML |
|---|---|
| `--color-surface-raised` | `[color] surface_raised` |
| `--control-icon-stroke` | `[control] icon_stroke` |
| `--shape-radius-artwork` | `[shape] radius_artwork` |

Sem exceção. Um tema proposto = um novo conjunto de valores = conversão direta
para TOML, linha a linha.

## Os arquivos

| Arquivo | Grupo | O que é |
|---|---|---|
| `contrato.html` | Contrato | **Comece por aqui.** O que um tema pode e não pode mudar, as 19 cores, a tabela de medidas |
| `modelo.html` | Contrato | O `theme.toml` completo para preencher, comentado. É o formato de saída esperado |
| `janela.html` | Telas | Janela inteira: title bar, barra lateral, início, barra de reprodução |
| `lista.html` | Telas | Playlist: cabeçalho, capa grande, linhas de faixa, estados hover e tocando |
| `configuracoes.html` | Telas | Formulários: interruptores, sliders, botões, lista de temas com prévia |
| `componentes.html` | Componentes | Cada controle isolado em todos os estados, o conjunto de ícones, a escala tipográfica, mini player e menu da bandeja |
| `morune.css` | — | Cópia canônica das variáveis. **Já está embutida** em cada HTML |

Cada página é **autossuficiente**: abre sozinha em qualquer lugar, sem depender
de arquivo vizinho. A primeira linha traz o marcador `@dsCard` que a Claude
Design usa para agrupar os cartões.

## Como usar

1. Suba os seis HTML num projeto de design system.
2. Peça o tema descrevendo a intenção — "escuro quente, papel envelhecido",
   "alto contraste", "vidro sobre a área de trabalho".
3. Exija a saída no formato de `modelo.html`. Sem isso volta imagem, não tema.
4. Traga o TOML de volta. A conversão para um tema instalável é mecânica.

## O que fazer o proponente ler antes

O `contrato.html` responde a maior parte, mas os três pontos que mais derrubam
proposta:

- **Não existe sombra nem gradiente em superfície.** Contraste vem de linha.
- **Nada pode custar repaint por quadro.** O Morune toca enquanto a pessoa joga.
- **Contraste é medido, não opinado.** Mínimo 3:1, verificado no carregamento.

Direitos sobre fonte e ícone de terceiros: [DIREITOS-EM-TEMAS.md](../../DIREITOS-EM-TEMAS.md).

## Manter em dia

O kit reproduz a interface à mão — ele **não** é gerado a partir do `.slint`.
Quando um token novo entrar em `crates/morune-theme/src/tokens.rs`, ele precisa
entrar aqui também, em `morune.css` e no `modelo.html`. Um kit desatualizado
produz propostas que não cabem.
