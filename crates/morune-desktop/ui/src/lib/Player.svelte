<script lang="ts">
  import Capa from './Capa.svelte'
  import Icone from './Icone.svelte'
  import { acao, motor, Pagina } from './motor.svelte'
  import { entrarComMola, estourar } from './tema'

  const e = $derived(motor.e)
  let agoraEl: HTMLElement | undefined = $state()
  // A faixa trocou: a capa e o nome entram com a mola do tema.
  let ultimaFaixa = ''
  $effect(() => {
    const id = e.now_id
    if (id && ultimaFaixa && id !== ultimaFaixa && agoraEl) entrarComMola(agoraEl)
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
  function volume(ev: PointerEvent) {
    if (ev.buttons !== 1 && ev.type !== 'pointerdown') return
    const el = ev.currentTarget as HTMLElement
    if (ev.type === 'pointerdown') el.setPointerCapture(ev.pointerId)
    acao('set-volume', posicao(ev, el))
  }
  function rodaVolume(ev: WheelEvent) {
    ev.preventDefault()
    acao('nudge-volume', ev.deltaY < 0 ? 0.05 : -0.05)
  }
</script>

<footer class="player">
  <div class="agora" bind:this={agoraEl}>
    {#if e.has_track}
      <div class="capa-agora" style:--brilho={e.capa_viva || e.live_accent}
        style:--respira={motor.tema?.spec?.effects?.breathe ? (e.nivel ?? 0) : 1}>
        <Capa src={e.now_cover} inicial={e.now_cover_initial} matiz={e.now_cover_hue} pendente={e.now_cover_pending} tamanho="56px" />
      </div>
      <div class="txt">
        <div class="t corta">{e.now_title}</div>
        {#if e.now_artist_id}
          <button class="a corta suave link" title="Abrir {e.now_artist}" onclick={() => acao('play-track', e.now_artist_id)}>{e.now_artist}</button>
        {:else}
          <div class="a corta suave">{e.now_artist}</div>
        {/if}
      </div>
      <button class="ic-bt" class:on={e.now_favorite} title="Curtir" onclick={(ev) => { if (!e.now_favorite) estourar(ev.currentTarget as HTMLElement); acao('toggle-favorite', e.now_id) }}>
        <Icone nome="coracao" cheio={e.now_favorite} tamanho={18} />
      </button>
    {/if}
  </div>

  <div class="centro">
    <div class="bts">
      <button class="ic-bt" class:on={e.shuffle} title="Aleatório" onclick={() => acao('toggle-shuffle')}><Icone nome="aleatorio" tamanho={18} /></button>
      <button class="ic-bt" title="Anterior" onclick={() => acao('previous-track')}><Icone nome="anterior" cheio tamanho={18} /></button>
      <button class="pp" title={e.playing ? 'Pausar' : 'Tocar'} onclick={() => acao('toggle-play')}>
        <Icone nome={e.playing ? 'pausar' : 'tocar'} cheio tamanho={20} />
      </button>
      <button class="ic-bt" title="Próxima" onclick={() => acao('next-track')}><Icone nome="proxima" cheio tamanho={18} /></button>
      <button class="ic-bt rep" class:on={e.repeat > 0} title="Repetir" onclick={() => acao('cycle-repeat')}>
        <Icone nome="repetir" tamanho={18} />{#if e.repeat === 2}<b>1</b>{/if}
      </button>
    </div>
    <div class="tempo">
      <span>{e.elapsed ?? '0:00'}</span>
      <div class="barra" role="slider" tabindex="0" aria-label="Posição" aria-valuenow={Math.round(progresso * 100)}
        onpointerdown={barraDown} onpointermove={barraMove} onpointerup={barraUp}
        onkeydown={(ev) => (ev.key === 'ArrowRight' ? acao('seek-by', 10) : ev.key === 'ArrowLeft' ? acao('seek-by', -10) : null)}>
        <div class="trilho"><div class="cheio" style:width="{progresso * 100}%"></div></div>
      </div>
      <span>{e.total ?? '0:00'}</span>
    </div>
  </div>

  <div class="direita">
    {#if e.sleep_label}<span class="sono suave">{e.sleep_label}</span>{/if}
    <button class="ic-bt" title="Mini player" onclick={() => acao('toggle-mini-player')}><Icone nome="mini" tamanho={18} /></button>
    <button class="ic-bt" class:on={e.page === Pagina.Fila} title="Fila" onclick={() => acao(e.page === Pagina.Fila ? 'close-queue' : 'navigate', Pagina.Fila)}><Icone nome="fila" tamanho={18} /></button>
    <button class="ic-bt" title={e.volume > 0 ? 'Silenciar' : 'Voltar o som'} onclick={() => acao('toggle-mute')}>
      <Icone nome={e.volume > 0 ? 'volume' : 'mudo'} tamanho={18} />
    </button>
    <div class="vol" role="slider" tabindex="0" aria-label="Volume" aria-valuenow={Math.round((e.volume ?? 0) * 100)}
      onpointerdown={volume} onpointermove={volume} onwheel={rodaVolume}>
      <div class="trilho"><div class="cheio" style:width="{(e.volume ?? 0) * 100}%"></div></div>
    </div>
  </div>
</footer>

<style>
  .player { display: grid; grid-template-columns: 1fr minmax(360px, 600px) 1fr; align-items: center; gap: var(--e-lg); padding: 0 var(--e-md); height: 100%; }
  .agora { display: flex; align-items: center; gap: var(--e-md); min-width: 0; }
  /* `isolation` segura o brilho (z-index -1) atras da capa, e nao atras do painel. */
  .capa-agora { position: relative; isolation: isolate; border-radius: var(--r-capa); }
  /* O brilho da capa: a cor viva dela, desfocada por tras. Parado; so muda de cor com a faixa. */
  /* Um halo curto: sombra colorida, sem camada desfocada. Ao respirar, so a
     opacidade muda (`--respira`, de 0 a 1), o que nao repinta nada. */
  .capa-agora::before {
    content: ''; position: absolute; inset: 0; border-radius: inherit; z-index: -1;
    box-shadow: 0 0 16px 2px var(--brilho, transparent);
    opacity: calc(var(--brilho-capa, 0.45) * (0.55 + 0.45 * var(--respira, 1)));
    transition: box-shadow var(--m-lento), opacity 90ms linear;
  }
  .txt { min-width: 0; }
  .t { font-weight: var(--peso-medio); font-size: var(--t-sm); }
  .a { font-size: var(--t-xs); }
  .link { display: block; max-width: 100%; text-align: left; }
  .link:hover { color: var(--texto); text-decoration: underline; }
  .centro { display: flex; flex-direction: column; align-items: center; gap: 6px; }
  .bts { display: flex; align-items: center; gap: var(--e-lg); }
  .ic-bt { position: relative; display: grid; place-items: center; width: 32px; height: 32px; border-radius: 99px; color: var(--texto-suave); transition: color var(--m-rapido), transform var(--m-rapido); }
  .ic-bt:hover { color: var(--texto); }
  .ic-bt:active, .pp:active { transform: scale(0.92); }
  .ic-bt.on { color: var(--acento); }
  .rep b { position: absolute; right: 2px; bottom: 2px; font-size: 9px; }
  .pp { display: grid; place-items: center; width: 40px; height: 40px; border-radius: var(--r-botao, 99px); background: var(--texto); color: var(--sobre-texto); transition: transform var(--m-rapido); }
  .pp:hover { transform: scale(1.05); }
  .tempo { display: flex; align-items: center; gap: var(--e-sm); width: 100%; font-size: var(--t-xs); color: var(--texto-suave); font-variant-numeric: tabular-nums; }
  .barra, .vol { flex: 1; height: 16px; display: flex; align-items: center; cursor: pointer; }
  .vol { flex: none; width: 110px; }
  .trilho { position: relative; width: 100%; height: var(--trilho, 4px); border-radius: 99px; background: color-mix(in srgb, var(--texto) 20%, transparent); overflow: hidden; }
  .cheio { height: 100%; background: var(--texto); border-radius: 99px; }
  .barra:hover .cheio, .vol:hover .cheio { background: var(--acento); }
  .direita { display: flex; align-items: center; justify-content: flex-end; gap: var(--e-sm); }
  .sono { font-size: var(--t-xs); }
</style>
