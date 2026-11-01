<script lang="ts">
  // Lista de faixas. O navegador so desenha as linhas visiveis
  // (`content-visibility`), entao listas de milhares rolam sem custo extra.
  import { untrack } from 'svelte'
  import Capa from './Capa.svelte'
  import Icone from './Icone.svelte'
  import { acao } from './motor.svelte'
  import { abrirMenu } from './contexto.svelte'
  import { estourar } from './tema'

  let {
    faixas = [],
    ativar = (f: any, _i: number) => acao('play-track', f.id),
    album = true,
    fila = false,
    numerar = true,
    reordenar,
  }: {
    faixas?: any[]
    ativar?: (f: any, i: number) => void
    album?: boolean
    fila?: boolean
    numerar?: boolean
    /** Com isto, as linhas podem ser arrastadas; recebe a faixa e o destino. */
    reordenar?: (f: any, para: number) => void
  } = $props()

  // A regua desliza ate a linha tocando. Toda linha tem a mesma altura, entao
  // a posicao e so o indice vezes a altura: nada de medir o DOM.
  const tocando = $derived(faixas.findIndex((f) => f.playing))

  // Arrastar. Toda linha tem 60 px, entao o destino e so o deslocamento
  // dividido pela altura: nada de medir o DOM. A linha segue o ponteiro e as do
  // caminho abrem espaco; soltar manda a ordem ao motor, que devolve a lista
  // ja na ordem nova.
  const ALTURA = 60
  let de = $state(-1)
  let desloc = $state(0)
  let soltando = $state(false)
  let pegada = -1
  let inicio = { y: 0, rolagem: 0, id: -1 }
  let rolador: HTMLElement | null = null
  let quadro = 0
  let ultimoY = 0
  const alvo = $derived(de < 0 ? -1 : Math.max(0, Math.min(faixas.length - 1, de + Math.round(desloc / ALTURA))))

  function abaixou(ev: PointerEvent, i: number) {
    if (!reordenar || ev.button !== 0 || (ev.target as HTMLElement).closest('button')) return
    rolador = (ev.currentTarget as HTMLElement).closest('.conteudo')
    inicio = { y: ev.clientY, rolagem: rolador?.scrollTop ?? 0, id: ev.pointerId }
    ultimoY = ev.clientY
    pegada = i
    window.addEventListener('pointermove', moveu)
    window.addEventListener('pointerup', soltou, { once: true })
  }

  function medir() {
    desloc = ultimoY - inicio.y + ((rolador?.scrollTop ?? 0) - inicio.rolagem)
  }

  function moveu(ev: PointerEvent) {
    if (ev.pointerId !== inicio.id) return
    ultimoY = ev.clientY
    if (de < 0) {
      if (Math.abs(ev.clientY - inicio.y) < 6) return
      de = pegada
      window.getSelection()?.removeAllRanges()
      quadro = requestAnimationFrame(rolarNaBorda)
    }
    medir()
  }

  // Perto da borda do painel, a lista rola sozinha, mais rapido quanto mais perto.
  function rolarNaBorda() {
    if (de < 0) return
    if (rolador) {
      const r = rolador.getBoundingClientRect()
      const faixa = 72
      const v = ultimoY < r.top + faixa ? -(r.top + faixa - ultimoY) : ultimoY > r.bottom - faixa ? ultimoY - (r.bottom - faixa) : 0
      if (v) { rolador.scrollTop += Math.max(-24, Math.min(24, v / 3)); medir() }
    }
    quadro = requestAnimationFrame(rolarNaBorda)
  }

  function soltou() {
    window.removeEventListener('pointermove', moveu)
    cancelAnimationFrame(quadro)
    pegada = -1
    if (de < 0) return
    const f = faixas[de]
    const para = alvo
    if (para === de || !f) { de = -1; desloc = 0; return }
    // A linha assenta no lugar novo e espera a lista do motor chegar.
    soltando = true
    desloc = (para - de) * ALTURA
    reordenar?.(f, para)
    setTimeout(() => { if (soltando) limpar() }, 800)
  }

  function limpar() { de = -1; desloc = 0; soltando = false }

  // A lista nova chegou: as linhas ja estao no lugar certo sem deslocamento.
  // `soltando` fica fora das dependencias: so a chegada da lista limpa.
  $effect(() => { faixas; untrack(() => { if (soltando) limpar() }) })

  function deslocamento(i: number) {
    if (de < 0) return 0
    if (i === de) return desloc
    if (de < i && i <= alvo) return -ALTURA
    if (alvo <= i && i < de) return ALTURA
    return 0
  }
</script>

<div class="lista" class:sem-album={!album} class:arrastavel={!!reordenar} class:arrastando={de >= 0}>
  {#if tocando >= 0}<span class="regua" aria-hidden="true" style:--i={tocando}></span>{/if}
  {#each faixas as f, i (f.id + ':' + i)}
    <div class="linha" class:tocando={f.playing} class:indisponivel={!f.playable}
      class:pega={i === de} class:soltando={soltando && i === de}
      style:transform={de >= 0 ? `translateY(${deslocamento(i)}px)` : null}
      onpointerdown={(ev) => abaixou(ev, i)}
      role="button" tabindex="0"
      ondblclick={() => f.playable && ativar(f, i)}
      onkeydown={(e) => e.key === 'Enter' && f.playable && ativar(f, i)}
      oncontextmenu={(e) => abrirMenu(e, f, fila ? i : -1)}>
      <div class="num">
        {#if f.playing}
          <span class="onda" aria-label="tocando"><i></i><i></i><i></i></span>
        {:else}
          <span class="n suave">{numerar ? i + 1 : ''}</span>
        {/if}
        <button class="play" title="Tocar" onclick={() => f.playable && ativar(f, i)}><Icone nome="tocar" cheio tamanho={16} /></button>
      </div>
      <Capa src={f.cover} inicial={f.initial} matiz={f.hue} pendente={f.cover_pending} tamanho="44px" />
      <div class="txt">
        <div class="t corta">{f.title}</div>
        {#if f.artist_id}
          <button class="a corta suave link" title="Abrir {f.artist}"
            onclick={(ev) => { ev.stopPropagation(); acao('play-track', f.artist_id) }}
            ondblclick={(ev) => ev.stopPropagation()}>{f.artist}</button>
        {:else}
          <div class="a corta suave">{f.artist}</div>
        {/if}
      </div>
      {#if album}
        {#if f.album_id}
          <button class="alb corta suave link" title="Abrir {f.album}"
            onclick={(ev) => { ev.stopPropagation(); acao('play-track', f.album_id) }}
            ondblclick={(ev) => ev.stopPropagation()}>{f.album}</button>
        {:else}
          <div class="alb corta suave">{f.album}</div>
        {/if}
      {/if}
      <button class="cor" class:on={f.favorite} title={f.favorite ? 'Remover das curtidas' : 'Curtir'}
        onclick={(ev) => { if (!f.favorite) estourar(ev.currentTarget as HTMLElement); acao('toggle-favorite', f.id) }}>
        <Icone nome="coracao" cheio={f.favorite} tamanho={18} />
      </button>
      <div class="d suave">{f.duration}</div>
      <button class="mais" title="Mais opções" onclick={(e) => abrirMenu(e, f, fila ? i : -1)}><Icone nome="pontos" tamanho={18} /></button>
    </div>
  {/each}
</div>

<style>
  .lista { position: relative; display: flex; flex-direction: column; }
  /* Regua na cor da capa, na borda esquerda da linha tocando. */
  .regua {
    position: absolute; left: 0; top: 0; width: 3px; height: 28px; margin-top: 16px; border-radius: 3px;
    background: var(--acento); box-shadow: 0 0 10px var(--acento); pointer-events: none; z-index: 1;
    transform: translateY(calc(var(--i) * 60px));
    transition: transform 520ms cubic-bezier(0.34, 1.35, 0.64, 1), background var(--m-lento);
  }
  .linha {
    display: grid; grid-template-columns: 36px 44px minmax(0, 3fr) minmax(0, 2fr) 34px 52px 34px;
    align-items: center; gap: var(--e-md); height: 60px; padding: 0 var(--e-sm);
    border-radius: var(--r-sm); content-visibility: auto; contain-intrinsic-size: auto 60px;
    transition: background var(--m-rapido);
  }
  /* Arrastar: a linha pega sobe e segue o ponteiro sem atraso; as outras
     abrem espaco com mola. Soltar assenta com a mesma mola. */
  .arrastavel .linha { user-select: none; }
  .arrastando, .arrastando * { cursor: grabbing !important; }
  .arrastando .linha { transition: transform 240ms cubic-bezier(0.3, 1.25, 0.5, 1), background var(--m-rapido); }
  .arrastando .linha.pega {
    position: relative; z-index: 3; transition: box-shadow var(--m-rapido), background var(--m-rapido);
    background: var(--superficie-alta, var(--hover)); box-shadow: 0 10px 28px #0007; content-visibility: visible;
  }
  .arrastando .linha.pega.soltando { transition: transform 240ms cubic-bezier(0.3, 1.25, 0.5, 1); box-shadow: none; }
  .arrastando .regua { opacity: 0; }
  .sem-album .linha { grid-template-columns: 36px 44px minmax(0, 1fr) 34px 52px 34px; }
  .linha:hover, .linha:focus-visible { background: var(--hover); }
  .linha.indisponivel { opacity: 0.45; }
  .num { position: relative; display: grid; place-items: center; height: 100%; font-variant-numeric: tabular-nums; font-size: var(--t-sm); }
  .play { position: absolute; inset: 0; display: grid; place-items: center; opacity: 0; }
  .linha:hover .play { opacity: 1; }
  .linha:hover .n, .linha:hover .onda { opacity: 0; }
  .t { font-weight: var(--peso-medio); }
  .tocando .t { color: var(--acento); }
  .a, .alb { font-size: var(--t-sm); }
  .link { display: block; max-width: 100%; text-align: left; }
  .link:hover { color: var(--texto); text-decoration: underline; }
  .d { font-size: var(--t-sm); font-variant-numeric: tabular-nums; text-align: right; }
  .cor, .mais { display: grid; place-items: center; width: 32px; height: 32px; border-radius: 99px; color: var(--texto-suave); opacity: 0; transition: opacity var(--m-rapido), color var(--m-rapido); }
  .linha:hover .cor, .linha:hover .mais, .cor.on { opacity: 1; }
  .cor.on { color: var(--acento); }
  .cor:hover, .mais:hover { color: var(--texto); }
  /* Barras de som: cada uma oscila no seu ritmo, e o conjunto cresce com o
     pulso da musica. So `transform`. Pausado, ficam baixas e paradas. */
  .onda {
    display: flex; gap: 2px; align-items: flex-end; height: 14px;
    transform-origin: bottom; transform: scaleY(calc(0.35 + 0.65 * var(--nivel, 0.5)));
    transition: transform 110ms linear;
  }
  .onda i { width: 3px; height: 100%; background: var(--acento); border-radius: 2px; transform-origin: bottom; animation: onda 0.8s ease-in-out infinite alternate; }
  .onda i:nth-child(2) { animation-duration: 0.62s; animation-delay: -0.3s; }
  .onda i:nth-child(3) { animation-duration: 0.95s; animation-delay: -0.5s; }
  @keyframes onda { from { transform: scaleY(0.25); } to { transform: scaleY(1); } }
  :global([data-tocando='0']) .onda i { animation-play-state: paused; }
  .tocando { background: color-mix(in srgb, var(--acento) 9%, transparent); }
</style>
