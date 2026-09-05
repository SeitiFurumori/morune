# 002 — A barra lateral anima `width`

- **Commit base:** `6b127aa`
- **Fase:** 1 (estrutura) — ver [README](README.md)
- **Severidade:** ALTA
- **Categoria:** desempenho

## O defeito

```slint
// crates/morune-app/ui/app.slint:3506-3511  (barra à esquerda)
if Layout.sidebar-position == 0: Rectangle {
    width: Layout.sidebar-collapsed
        ? Layout.sidebar-collapsed-width
        : Layout.sidebar-width;
    background: Theme.sidebar-background;
    animate width { duration: Theme.motion-normal; easing: ease-in-out; }
```

O mesmo bloco existe em `app.slint:3773-3778` para `sidebar-position == 1`.

`width` é propriedade de layout. Animá-la refaz o layout **da barra e da área
de conteúdo ao lado** a cada quadro, por 200 ms: a lista de playlists
reposiciona, a página inteira recalcula largura, e no meio disso a grade de
cartões pode recalcular quantas colunas cabem. É a animação mais cara do app, e
o critério do projeto é não disputar recurso com quem está jogando.

O playbook (`AUDIT.md`, seção 5) é direto: animar só `transform` e `opacity`;
`width`/`height`/`margin`/`padding` disparam layout + pintura + composição.

## Por que este plano está na fase de estrutura

Porque a correção **não é ajustar a curva** — é trocar como a barra encolhe. Se
for feita depois, desfaz o que a fase de animação tiver encostado ali. Fazer
junto com o `better-ui`, que já vai mexer nessa região, é uma mexida só.

## A correção

Manter a caixa externa com largura fixa e mover o conteúdo por dentro, com
recorte. O que anima passa a ser `x`, que não refaz layout.

Esboço da forma-alvo:

```slint
if Layout.sidebar-position == 0: Rectangle {
    // Largura fixa: quem encolhe e o conteudo por dentro, e nao a caixa. Antes
    // isto animava `width`, e cada quadro refazia o layout da barra e da area
    // de conteudo ao lado -- 200 ms de relayout continuo por um gesto que so
    // muda o que aparece.
    width: Layout.sidebar-collapsed
        ? Layout.sidebar-collapsed-width
        : Layout.sidebar-width;
    clip: true;
    background: Theme.sidebar-background;
    // ... conteudo com x animado
}
```

**Cuidado que decide se vale a pena:** encolher a caixa externa sem animar
continua sendo uma mudança de layout — só que instantânea, num quadro, em vez
de espalhada por doze. Se a leitura na tela ficar pior que o custo que se
economiza, a decisão correta é **remover a animação** e deixar a barra
recolher de uma vez. Isso é resultado legítimo deste plano; o playbook diz que
a correção mais forte muitas vezes é apagar a animação.

Medir antes de decidir: `tools\measure.ps1 -Watch -IdleSeconds 20` com a barra
sendo recolhida e expandida em sequência, contra a mesma sequência sem
animação.

## Fora de escopo

- Não mexer na largura dos tokens (`sidebar-width`, `sidebar-collapsed-width`).
- Não mexer no conteúdo da barra (lista de playlists, navegação, rodapé).
- Não tocar em `components.slint:525`, que anima `x` e já está certo.

## Verificação

1. `cargo build -p morune-app` compila; `clippy` limpo.
2. Recolher e expandir a barra dez vezes seguidas: nenhum salto de layout na
   área de conteúdo, e a grade de cartões não pisca trocando de número de
   colunas no meio do gesto.
3. **Medida, não impressão:** `tools\measure.ps1 -Watch` durante a sequência de
   recolher/expandir, antes e depois. Registrar os dois números em
   `docs/PERFORMANCE.md` — a página já tem seção para medições com cenário.
4. Conferir nas duas posições da barra (`sidebar-position` 0 e 1), porque o
   bloco é duplicado e é fácil corrigir só um.
