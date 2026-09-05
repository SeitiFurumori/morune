# 007 — Três lugares que não animam e deveriam

- **Commit base:** `6b127aa`
- **Fase:** 3 (movimento) — **por último**
- **Severidade:** aditivo, não corretivo
- **Categoria:** oportunidades

Não são defeito: são transições ausentes onde a interface hoje teleporta. Os
três são independentes — dá para fazer um e parar.

## 7.1 — Troca de faixa no rodapé

**Onde:** a área de capa e título do `Player` em `app.slint` — o bloco sob
`if Layout.player-artwork: Artwork` e os dois `Text` ao lado.

**Hoje:** ao trocar de faixa, capa, título e artista trocam por corte. É o
elemento que a pessoa olha ao voltar do jogo, e a única indicação de que a
música mudou quando a janela está em segundo plano.

**Alvo:** fade cruzado em `Theme.motion-fast` com `easing: ease`. Sem
deslocamento — a barra é estreita e movimento lateral ali lê como falha.

**Cuidado que decide se funciona:** o rodapé é reescrito a cada 250 ms pelo
relógio de progresso. A animação não pode disparar a cada atualização, só
quando o **id da faixa** muda. Amarrar a um `changed` do id (a propriedade
`now-id` já existe), nunca ao texto.

## 7.2 — Troca de página

**Onde:** o `VerticalLayout` que hospeda `if root.page == N: XPage` em
`app.slint`, por volta da linha 3520.

**Hoje:** Início, Buscar, Biblioteca e Configurações trocam por corte. Só a
página de detalhe tem entrada, e a incoerência é o achado: a mesma ação
(navegar) tem duas leituras diferentes.

**Alvo:** a mesma entrada do detalhe, pelo mesmo mecanismo (`init` +
opacidade), em `Theme.motion-normal` com `ease-out`. Mais curta que a do
detalhe **de propósito**: trocar de aba acontece muito mais vezes por dia que
abrir uma lista, e o playbook manda reduzir movimento em ação frequente.

**Cuidado:** aplicar no conteúdo da página, não na raiz — mesmo motivo do plano
004.

## 7.3 — Toast de status

**Onde:** `app.slint:3697`, `if root.status-message != "": Rectangle { ... }`.

**Hoje:** a caixa aparece e some instantaneamente. Mensagem que pisca na
periferia é o caso clássico do que a pessoa não lê porque não percebeu chegar.

**Alvo:** entrada com opacidade e 8px vindo de baixo, `Theme.motion-normal`,
`ease-out`.

**Cuidado:** o toast some por `if`, e elemento removido da árvore não anima a
saída. Ter saída exige mantê-lo na árvore com opacidade zero — e aí ele não
pode capturar clique nesse estado. Se isso complicar, **entregar só a entrada e
dizer que a saída ficou de fora**, em vez de deixar um retângulo invisível
interceptando cliques no canto da tela.

## Verificação

1. `cargo build -p morune-app`, `clippy` e `cargo test --workspace` limpos.
2. **7.1:** deixar tocando e esperar a faixa virar sozinha. O fade tem de
   acontecer uma vez, não quatro vezes por segundo.
3. **7.2:** alternar Início → Buscar → Biblioteca rápido, dez vezes. Não pode
   acumular nem piscar; se acumular, a entrada está reiniciando do zero em vez
   de retomar.
4. **7.3:** disparar duas mensagens em sequência curta e confirmar que a segunda
   não entra por cima da saída da primeira.
5. **Medida:** `tools\measure.ps1 -Watch` com música tocando, antes e depois. O
   7.1 é o único que roda com a janela em segundo plano — é o que mais precisa
   do número.
