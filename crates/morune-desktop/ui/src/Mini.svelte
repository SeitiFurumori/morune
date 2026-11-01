<script lang="ts">
  // Mini player: a mesma janela encolhida num cartao, como o mini player do
  // YouTube. Sem barra de titulo: a capa e a faixa de cima arrastam a janela.
  import Capa from './lib/Capa.svelte'
  import Icone from './lib/Icone.svelte'
  import { acao, motor } from './lib/motor.svelte'
  import { entrarComMola, estourar } from './lib/tema'

  const e = $derived(motor.e)
  let capaEl: HTMLElement | undefined = $state()
  let ultimaFaixa = ''
  $effect(() => {
    const id = e.now_id
    if (id && ultimaFaixa && id !== ultimaFaixa && capaEl) entrarComMola(capaEl)
    ultimaFaixa = id || ''
  })

  let arrastando = $state<number | null>(null)
  const progresso = $derived(arrastando ?? e.progress ?? 0)
  function posicao(ev: PointerEvent, el: HTMLElement) {
    const r = el.getBoundingClientRect()
    return Math.min(1, Math.max(0, (ev.clientX - r.left) / r.width))
  }
  function barraDown(ev: PointerEvent) {
    const el = ev.currentTarget as HTMLElement
    el.setPointerCapture(ev.pointerId)
    arrastando = posicao(ev, el)
  }
  function barraMove(ev: PointerEvent) {
    if (arrastando !== null) arrastando = posicao(ev, ev.currentTarget as HTMLElement)
  }
  function barraUp() {
    if (arrastando !== null) acao('seek', arrastando)
    arrastando = null
  }
</script>

<div class="mini">
  <div class="topo" data-tauri-drag-region>
    <button class="ic" class:on={e.mini_fixo} title={e.mini_fixo ? 'Desafixar' : 'Fixar na frente'} onclick={() => acao('toggle-mini-pin')}>
      <Icone nome="fixar" tamanho={16} />
    </button>
    <span class="vazio" data-tauri-drag-region></span>
    <button class="ic" title="Buscar" onclick={() => acao('mini-search')}><Icone nome="busca" tamanho={16} /></button>
    <button class="ic" title="Voltar ao tamanho normal" onclick={() => acao('toggle-mini-player')}><Icone nome="expandir" tamanho={16} /></button>
    <!-- Como o X da janela grande: a musica segue na bandeja. -->
    <button class="ic" title="Fechar (continua tocando na bandeja)" onclick={() => acao('fechar-janela')}><Icone nome="fechar" tamanho={16} /></button>
  </div>

  <div class="arte" data-tauri-drag-region>
    {#if e.has_track}
      <div class="capa-wrap" bind:this={capaEl} style:--brilho={e.capa_viva || e.live_accent}
        style:--respira={motor.tema?.spec?.effects?.breathe ? (e.nivel ?? 0) : 1}>
        <Capa src={e.now_cover} inicial={e.now_cover_initial} matiz={e.now_cover_hue} pendente={e.now_cover_pending} />
      </div>
    {:else}
      <p class="suave nada">Nada tocando</p>
    {/if}
  </div>

  <div class="txt">
    <div class="t corta">{e.now_title || ' '}</div>
    {#if e.now_artist_id}
      <button class="a corta suave link" title="Abrir {e.now_artist}"
        onclick={() => { acao('toggle-mini-player'); acao('play-track', e.now_artist_id) }}>{e.now_artist}</button>
    {:else}
      <div class="a corta suave">{e.now_artist}</div>
    {/if}
  </div>

  <div class="tempo">
    <span>{e.elapsed ?? '0:00'}</span>
    <div class="barra" role="slider" tabindex="0" aria-label="Posição" aria-valuenow={Math.round(progresso * 100)}
      onpointerdown={barraDown} onpointermove={barraMove} onpointerup={barraUp}>
      <div class="trilho"><div class="cheio" style:width="{progresso * 100}%"></div></div>
    </div>
    <span>{e.total ?? '0:00'}</span>
  </div>

  <div class="bts">
    <button class="ic" class:on={e.now_favorite} title="Curtir" onclick={(ev) => { if (!e.now_favorite) estourar(ev.currentTarget as HTMLElement); acao('toggle-favorite', e.now_id) }}>
      <Icone nome="coracao" cheio={e.now_favorite} tamanho={18} />
    </button>
    <button class="ic" title="Anterior" onclick={() => acao('previous-track')}><Icone nome="anterior" cheio tamanho={20} /></button>
    <button class="pp" title={e.playing ? 'Pausar' : 'Tocar'} onclick={() => acao('toggle-play')}>
      <Icone nome={e.playing ? 'pausar' : 'tocar'} cheio tamanho={22} />
    </button>
    <button class="ic" title="Próxima" onclick={() => acao('next-track')}><Icone nome="proxima" cheio tamanho={20} /></button>
    <button class="ic" class:on={e.shuffle} title="Aleatório" onclick={() => acao('toggle-shuffle')}><Icone nome="aleatorio" tamanho={18} /></button>
  </div>
</div>

<style>
  .mini {
    height: 100vh; display: grid; grid-template-rows: auto minmax(0, 1fr) auto auto auto;
    gap: var(--e-sm); padding: var(--e-sm) var(--e-md) var(--e-md);
    background: var(--superficie); border: var(--borda-painel, 0px) solid var(--borda-destaque, transparent);
  }
  .topo { display: flex; align-items: center; gap: 2px; margin: 0 calc(-1 * var(--e-xs)); }
  .vazio { flex: 1; align-self: stretch; }
  .arte { display: grid; place-items: center; min-height: 0; padding: var(--e-sm); }
  .capa-wrap { position: relative; isolation: isolate; height: 100%; max-height: 220px; aspect-ratio: 1; border-radius: var(--r-capa); }
  .capa-wrap :global(.capa) { width: 100% !important; height: 100%; box-shadow: 0 10px 28px #0007; }
  .capa-wrap::before {
    content: ''; position: absolute; inset: 0; border-radius: inherit; z-index: -1;
    box-shadow: 0 0 22px 3px var(--brilho, transparent);
    opacity: calc(var(--brilho-capa, 0.45) * (0.55 + 0.45 * var(--respira, 1)));
    transition: box-shadow var(--m-lento), opacity 90ms linear;
  }
  .nada { font-size: var(--t-sm); }
  .txt { min-width: 0; text-align: center; }
  .t { font-weight: var(--peso-medio); font-size: var(--t-md); }
  .a { font-size: var(--t-sm); }
  .link { display: block; width: 100%; text-align: center; }
  .link:hover { color: var(--texto); text-decoration: underline; }
  .tempo { display: flex; align-items: center; gap: var(--e-sm); font-size: var(--t-xs); color: var(--texto-suave); font-variant-numeric: tabular-nums; }
  .barra { flex: 1; height: 16px; display: flex; align-items: center; cursor: pointer; }
  .trilho { width: 100%; height: var(--trilho, 4px); border-radius: 99px; background: color-mix(in srgb, var(--texto) 20%, transparent); overflow: hidden; }
  .cheio { height: 100%; background: var(--texto); }
  .barra:hover .cheio { background: var(--acento); }
  .bts { display: flex; align-items: center; justify-content: space-between; }
  .ic { display: grid; place-items: center; width: 34px; height: 34px; border-radius: var(--r-botao, 99px); color: var(--texto-suave); transition: color var(--m-rapido); }
  .ic:hover { color: var(--texto); }
  .ic.on { color: var(--acento); }
  .pp { display: grid; place-items: center; width: 48px; height: 48px; border-radius: var(--r-botao, 99px); background: var(--texto); color: var(--sobre-texto); transition: transform var(--m-rapido); }
  .pp:active { transform: scale(0.94); }
</style>
