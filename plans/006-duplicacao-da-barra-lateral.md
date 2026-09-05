# 006 — Bloco de animação duplicado na barra lateral

- **Commit base:** `6b127aa`
- **Fase:** 3 (movimento) — **fazer depois do plano 002**
- **Severidade:** BAIXA
- **Categoria:** coesão

## O defeito

`app.slint:3511` e `app.slint:3778` são a mesma linha, em dois ramos que só
diferem pela posição da barra (`sidebar-position == 0` e `== 1`):

```slint
animate width { duration: Theme.motion-normal; easing: ease-in-out; }
```

Duplicação de valor de movimento é achado de coesão: os dois divergem na
primeira vez que alguém ajustar um e esquecer o outro — e o plano 002 é
exatamente uma ocasião dessas.

## A correção

Depois que o plano 002 decidir o que a barra faz, extrair o comportamento para
um componente único usado pelos dois ramos, ou para uma propriedade
compartilhada. **Não fazer antes:** seria extrair código que o 002 vai
reescrever.

## Fora de escopo

- Não unificar os dois ramos `if sidebar-position == 0 / == 1` inteiros. Eles
  diferem em mais do que a animação, e isso é outra discussão.

## Verificação

1. `cargo build -p morune-app` compila; `clippy` limpo.
2. Recolher e expandir a barra nas duas posições, confirmando comportamento
   idêntico.
3. `grep -n "animate width" crates/morune-app/ui/app.slint` não deve devolver
   duas linhas iguais.
