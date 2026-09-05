# 005 — Entrada só por opacidade, sem fisicalidade

- **Commit base:** `6b127aa`
- **Fase:** 3 (movimento) — **depende do plano 004**
- **Severidade:** MÉDIA
- **Categoria:** física e origem

## O defeito

A entrada da página de detalhe (`app.slint:1916-1926`) é só opacidade. O
playbook (`AUDIT.md`, seção 3) trata entrada sem transform como falta de
fisicalidade: nada no mundo real aparece por transparência, aparece chegando de
algum lugar.

## A correção

Somar um deslocamento curto ao fade, na mesma duração e na mesma curva: o
cabeçalho entra subindo 8px.

```slint
// 8px: o suficiente para o olho ler "chegou", pouco o suficiente para nao
// virar animacao de apresentacao. No mesmo `ease-out` e na mesma duracao do
// fade -- duas propriedades com curvas diferentes leem como duas coisas
// acontecendo, e nao como uma.
y: root.entrou ? 0px : 8px;
animate y { duration: Theme.motion-slow; easing: ease-out; }
```

Aplicar no mesmo elemento em que o plano 004 deixou a opacidade — por isso este
plano vem depois dele.

**Cuidado do Slint:** `y` de um filho direto de `VerticalLayout` é gerenciado
pelo layout, e a atribuição é ignorada. Se o alvo for posicionado por layout, a
correção é envolver o conteúdo num `Rectangle` que o layout posiciona e animar
o `y` do que está dentro. Conferir no build, não presumir — este projeto já
gastou três tentativas com o layout do Slint por presumir.

## Fora de escopo

- Não animar `x`. Deslocamento lateral sugere navegação entre irmãos, e abrir
  uma playlist não é isso.
- Não acrescentar escala: o Slint não tem transform de escala em `Rectangle`.

## Verificação

1. `cargo build -p morune-app` compila; `clippy` limpo.
2. Abrir três playlists seguidas: o cabeçalho deve subir e assentar, sem
   solavanco no fim. Se ficar parado, o `y` foi engolido pelo layout — ver o
   cuidado acima.
3. Com um tema que zera `motion_slow`, a entrada deve ser instantânea e sem
   deslocamento residual.
