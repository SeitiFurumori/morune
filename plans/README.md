# Fila de polimento — ordem de execução

Aberta em 04/09/2026, sobre o commit `6b127aa`. Reúne o que quatro skills
apontaram para o Morune e **ordena para que uma não desfaça a outra**.

## O problema que esta ordem resolve

Quatro frentes mexem nos mesmos arquivos:

| frente | o que altera | arquivos |
|---|---|---|
| `code-review` / `security-review` | correções de defeito | todo o repositório |
| `better-ui` | geometria: raio, alinhamento óptico, área de clique, profundidade | `app.slint`, `components.slint` |
| `ux-writing` | texto de interface | `app.slint`, `state.rs` |
| `improve-animations` | blocos `animate` | `app.slint`, `components.slint` |

Rodadas juntas, três delas escrevem em `app.slint` ao mesmo tempo. E há
dependência real, não só conflito de arquivo: **animar uma estrutura que vai
mudar é trabalho jogado fora**, e revisar um branch que ainda está crescendo é
revisar um alvo que se move.

## A ordem, e o motivo de cada posição

### Fase 0 — Revisar o que já existe, antes de empilhar mais

1. **`security-review`** — o fluxo OAuth foi reescrito (porta 5588, `state`
   anti-CSRF, token no cofre do Windows) e **nunca foi testado**; o atualizador
   baixa e executa um instalador. É a superfície mais sensível do projeto e
   nunca passou por revisão.
2. **`code-review`** — 15 commits à frente do `origin/main`, nenhum revisado,
   mexendo em OAuth, SMTC, saída de áudio, cache e interface.

Primeiro porque quinze commits locais são baratos de corrigir e quinze commits
publicados não são. E porque tudo que vier depois entra por cima: revisar agora
é revisar um alvo parado.

**Portão:** nada da Fase 1 começa antes de a Fase 0 fechar ou de o Felipe
dispensar explicitamente um achado.

### Fase 1 — Estrutura

3. **`better-ui`** — raio concêntrico, alinhamento óptico, área de clique,
   profundidade de superfície.
4. **[002](002-barra-lateral-anima-largura.md)** — a barra lateral anima
   `width`. **Está aqui e não na fase de animação de propósito:** a correção é
   trocar a estrutura da barra (clip + `x`), e não ajustar uma curva. Feita
   junto com `better-ui`, é uma mexida só no mesmo trecho.

Estrutura antes de texto e antes de movimento: as duas seguintes se apoiam nela.

### Fase 2 — Texto

5. **`ux-writing`** — `"100 faixas carregadas"` e `"Carregar mais"` são
   linguagem de programador, `"Nada tocando"` é estado vazio sem saída, e os
   erros de rede ainda falam como log.

Depois da estrutura porque estado vazio novo é caixa nova, e antes da animação
porque animar um texto que vai mudar de tamanho é medir errado.

### Fase 3 — Movimento

6. **[001](001-easing-linear.md)** — 16 animações rodando em `linear`.
7. **[003](003-preferencia-de-movimento-do-sistema.md)** — respeitar a
   preferência de animação do Windows.
8. **[004](004-fade-de-entrada-so-no-cabecalho.md)** — o fade de entrada compõe
   a página inteira.
9. **[005](005-entrada-com-deslocamento.md)** — entrada só por opacidade.
10. **[006](006-duplicacao-da-barra-lateral.md)** — bloco duplicado.
11. **[007](007-oportunidades.md)** — troca de faixa, troca de página, toast.

Por último porque movimento descreve a estrutura e o texto finais.

**Exceção que pode furar a fila:** o plano **003** é Rust (`main.rs` + uma
chamada Win32), não toca `.slint` e não colide com nada. Pode ser feito a
qualquer momento.

## Status

| # | Plano | Fase | Status |
|---|---|---|---|
| — | `security-review` | 0 | **feito** — nenhum achado acima do corte |
| — | `code-review` | 0 | **feito** — 2 achados, os dois corrigidos |
| — | `better-ui` | 1 | **feito** — 3 achados aplicados |
| 002 | Barra lateral anima largura | 1 | **fechado sem mudanca** — severidade reavaliada para MEDIA |
| — | `ux-writing` | 2 | pendente |
| 001 | Easing linear | 3 | pendente |
| 003 | Preferência de movimento do sistema | 3 (livre) | pendente |
| 004 | Fade só no cabeçalho | 3 | pendente |
| 005 | Entrada com deslocamento | 3 | pendente |
| 006 | Duplicação da barra lateral | 3 | pendente — deixou de depender do 002 |
| 007 | Oportunidades | 3 | pendente |

## Restrição que vale para todos

O Morune toca música enquanto alguém joga em tela cheia. Movimento não pode
custar CPU ou GPU perceptível, e toda duração sai de `Theme.motion-fast`
(120 ms), `motion-normal` (200 ms) ou `motion-slow` (320 ms) — um tema pode
zerar os três, e aí a interface volta a ser instantânea sem nenhum caminho de
código novo. Nenhum plano pode introduzir duração fora desses tokens.
