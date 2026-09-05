# 004 — O fade de entrada compõe a página inteira

- **Commit base:** `6b127aa`
- **Fase:** 3 (movimento)
- **Severidade:** MÉDIA
- **Categoria:** desempenho

## O defeito

```slint
// crates/morune-app/ui/app.slint:1916-1926
private property <bool> entrou: false;
init => { root.entrou = true; }
opacity: root.entrou ? 1.0 : 0.0;
animate opacity {
    duration: Theme.motion-slow;
    easing: ease-out;
}
```

A opacidade está na **raiz do `DetailPage`**. Opacidade parcial num pai força o
renderizador a compor a subárvore inteira numa camada: cabeçalho, campo de
filtro, chips de ordenação e as cem linhas da lista, com capa em cada uma,
durante 320 ms. É uma superfície grande sendo composta a cada quadro por um
efeito que só precisa acontecer no que a pessoa está olhando.

## A correção

Mover as três linhas de opacidade e o bloco `animate` da raiz do `DetailPage`
para o `HorizontalLayout` do cabeçalho — o que contém a capa, o rótulo do tipo,
o nome, o subtítulo e o botão Tocar, logo abaixo do trecho citado. `entrou` e o
`init` continuam na raiz; só o consumo da propriedade desce.

A lista aparecer sem fade não é perda: ela chega preenchida, e o que precisava
de transição era a troca de contexto, que o cabeçalho já conta.

## Fora de escopo

- Não mudar duração nem curva: `motion-slow` e `ease-out` estão certos.
- Não acrescentar entrada para as linhas da lista. Stagger em cem linhas é
  decorativo e custa; a ausência é deliberada.

## Verificação

1. `cargo build -p morune-app` compila; `clippy` limpo.
2. Abrir uma playlist grande (121 faixas): o cabeçalho entra com fade, a lista
   aparece pronta, e não há atraso perceptível entre o clique e a lista.
3. **Medida:** `tools\measure.ps1 -Watch -IdleSeconds 20` abrindo e fechando
   playlists em sequência, antes e depois. A diferença esperada é pequena; se
   não houver diferença nenhuma, dizer isso em vez de afirmar ganho.
