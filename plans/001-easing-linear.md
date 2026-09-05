# 001 — 16 animações rodando em `linear`

- **Commit base:** `6b127aa`
- **Fase:** 3 (movimento) — ver [README](README.md)
- **Severidade:** ALTA
- **Categoria:** easing e duração

## O defeito

O Slint declara `#[default] Linear` para `EasingCurve`
(`i-slint-core-1.17.1/animations.rs:147`). Um bloco `animate` sem `easing`
roda em linear — velocidade constante, sem começo nem fim. É a única curva que
nunca lê como física: o realce de hover acende como um interruptor com atraso,
em vez de responder.

Dos 20 blocos `animate` do projeto, só 4 declaram curva. Os outros 16 são
linear por omissão, e não por escolha.

## Regra que decide a curva

Do playbook (`AUDIT.md`, seção 2), na ordem:

| situação | curva |
|---|---|
| entrando ou saindo da tela | `ease-out` |
| movendo ou transformando na tela | `ease-in-out` |
| hover, mudança de cor | `ease` |
| movimento constante (progresso) | `linear` |

`ease-in` em interface é sempre defeito — começa devagar justamente no
instante em que a pessoa está olhando. Não introduzir nenhum.

## Onde mexer

Todos os blocos abaixo já existem e **só falta acrescentar `easing:`**. Nenhuma
duração muda.

### `crates/morune-app/ui/app.slint`

| linha | trecho atual | curva a acrescentar | por quê |
|---|---|---|---|
| 182 | `animate background { duration: Theme.motion-fast; }` | `ease` | cor de hover/press do botão da barra de título |
| 341 | `animate background { duration: Theme.motion-fast; }` | `ease` | cor de hover do item de navegação |
| 384 | `animate background { duration: Theme.motion-fast; }` | `ease` | cor de hover |
| 438 | `animate background { duration: Theme.motion-fast; }` | `ease` | cor de hover da linha de faixa |
| 587 | `animate opacity { duration: Theme.motion-fast; }` | `ease-out` | coração aparecendo/sumindo |
| 607 | `animate opacity { duration: Theme.motion-fast; }` | `ease-out` | ações de fila aparecendo/sumindo |
| 686 | `animate background { duration: Theme.motion-fast; }` | `ease` | cor do cartão no hover |
| 690 | `animate drop-shadow-blur, drop-shadow-offset-y { duration: Theme.motion-normal; }` | `ease-out` | o cartão subindo |
| 845 | `animate background { duration: Theme.motion-fast; }` | `ease` | cor de hover |
| 1036 | `animate background { duration: Theme.motion-fast; }` | `ease` | cor de hover |
| 1850 | `animate background { duration: Theme.motion-fast; }` | `ease` | cor de hover |
| 2321 | `animate background { duration: Theme.motion-fast; }` | `ease` | cor de hover |
| 2368 | `animate background { duration: Theme.motion-fast; }` | `ease` | cor de hover |

### `crates/morune-app/ui/components.slint`

| linha | trecho atual | curva |
|---|---|---|
| 375 | `animate background { duration: Theme.motion-fast; }` | `ease` |

Os três que **já têm curva e não se mexe**: `app.slint:1925` (`ease-out`),
`app.slint:3511` e `3778` (`ease-in-out`, e o plano 002 reescreve os dois),
`components.slint:525` (`ease-in-out`).

## Forma exata da edição

Cada linha vira:

```slint
animate background { duration: Theme.motion-fast; easing: ease; }
```

```slint
animate opacity { duration: Theme.motion-fast; easing: ease-out; }
```

```slint
animate drop-shadow-blur, drop-shadow-offset-y { duration: Theme.motion-normal; easing: ease-out; }
```

## Fora de escopo

- **Não** mudar nenhuma duração.
- **Não** criar token de easing no tema. O contrato de tema é um conjunto
  fechado de slots; abrir um slot novo exige mexer no crate `morune-theme`, na
  validação e na documentação, e isso é decisão do Felipe — não pode entrar de
  carona num ajuste de curva. Registrar como pergunta no fim, não implementar.
- **Não** acrescentar `animate` onde não existe. Isso é o plano 007.

## Verificação

1. `cargo build -p morune-app` compila. Curva inválida é erro de compilação do
   Slint, então o build já é a primeira checagem.
2. `cargo fmt --all` e `cargo clippy --workspace --all-targets` limpos.
3. **Conferência de sensação, que não dá para julgar pelo código:** abrir a
   lista de faixas e passar o cursor devagar por várias linhas seguidas. Antes,
   o realce entra e sai na mesma velocidade do começo ao fim; depois, ele deve
   partir rápido e assentar. Se ficar igual, a curva não pegou — conferir se o
   bloco editado é o que a linha usa mesmo.
4. Repetir com um tema claro (Paper) e com o Aquário: `ease` sobre superfície
   clara mostra a diferença menos, e é onde um engano passa despercebido.
