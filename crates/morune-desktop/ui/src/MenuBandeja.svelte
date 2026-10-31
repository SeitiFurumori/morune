<script lang="ts">
  // Menu da bandeja: uma janelinha que so existe enquanto esta aberta. Fecha
  // ao perder o foco (o motor cuida disso) ou com Esc.
  import Capa from './lib/Capa.svelte'
  import Icone from './lib/Icone.svelte'
  import { acao, motor } from './lib/motor.svelte'

  const e = $derived(motor.e)

  function volume(ev: PointerEvent) {
    if (ev.buttons !== 1 && ev.type !== 'pointerdown') return
    const el = ev.currentTarget as HTMLElement
    if (ev.type === 'pointerdown') el.setPointerCapture(ev.pointerId)
    const r = el.getBoundingClientRect()
    acao('set-volume', Math.min(1, Math.max(0, (ev.clientX - r.left) / r.width)))
  }
</script>

<svelte:window onkeydown={(ev) => ev.key === 'Escape' && acao('fechar-bandeja')} />

<div class="cartao" class:carregando={!motor.pronto}>
  {#if e.has_track}
    <div class="agora">
      <Capa src={e.now_cover} inicial={e.now_cover_initial} matiz={e.now_cover_hue} pendente={e.now_cover_pending} tamanho="44px" />
      <div class="txt">
        <div class="t corta">{e.now_title}</div>
        <div class="a corta suave">{e.now_artist}</div>
      </div>
    </div>
    <div class="bts">
      <button class="ic" title="Anterior" onclick={() => acao('previous-track')}><Icone nome="anterior" cheio tamanho={18} /></button>
      <button class="pp" title={e.playing ? 'Pausar' : 'Tocar'} onclick={() => acao('toggle-play')}>
        <Icone nome={e.playing ? 'pausar' : 'tocar'} cheio tamanho={20} />
      </button>
      <button class="ic" title="Próxima" onclick={() => acao('next-track')}><Icone nome="proxima" cheio tamanho={18} /></button>
    </div>
  {/if}
  <div class="vol">
    <button class="ic" title={e.volume > 0 ? 'Silenciar' : 'Voltar o som'} onclick={() => acao('toggle-mute')}>
      <Icone nome={e.volume > 0 ? 'volume' : 'mudo'} tamanho={18} />
    </button>
    <div class="barra" role="slider" tabindex="0" aria-label="Volume" aria-valuenow={Math.round((e.volume ?? 0) * 100)}
      onpointerdown={volume} onpointermove={volume}>
      <div class="trilho"><div class="cheio" style:width="{(e.volume ?? 0) * 100}%"></div></div>
    </div>
  </div>
  <hr />
  <button class="item" onclick={() => acao('abrir-janela')}><Icone nome="inicio" tamanho={18} />Abrir Morune</button>
  <button class="item" onclick={() => acao('sair')}><Icone nome="sair" tamanho={18} />Sair do Morune</button>
</div>

<style>
  :global(body) { overflow: hidden; }
  .cartao {
    display: flex; flex-direction: column; gap: var(--e-xs); height: 100vh; padding: var(--e-sm);
    background: var(--superficie-alta); transition: opacity var(--m-rapido);
  }
  .cartao.carregando { opacity: 0; }
  .agora { display: flex; align-items: center; gap: var(--e-sm); padding: var(--e-xs); }
  .txt { min-width: 0; }
  .t { font-weight: var(--peso-medio); }
  .a { font-size: var(--t-sm); }
  .bts { display: flex; align-items: center; justify-content: center; gap: var(--e-lg); padding: var(--e-xs) 0; }
  .ic { display: grid; place-items: center; width: 32px; height: 32px; border-radius: 99px; color: var(--texto-suave); flex: none; transition: color var(--m-rapido); }
  .ic:hover { color: var(--texto); }
  .pp { display: grid; place-items: center; width: 40px; height: 40px; border-radius: 99px; background: var(--texto); color: var(--fundo); }
  .vol { display: flex; align-items: center; gap: var(--e-sm); padding: 0 var(--e-xs); }
  .barra { flex: 1; height: 16px; display: flex; align-items: center; cursor: pointer; }
  .trilho { width: 100%; height: 4px; border-radius: 99px; background: color-mix(in srgb, var(--texto) 20%, transparent); overflow: hidden; }
  .cheio { height: 100%; background: var(--texto); }
  .barra:hover .cheio { background: var(--acento); }
  hr { border: 0; height: 1px; background: var(--borda); margin: var(--e-xs) 6px; }
  .item { display: flex; align-items: center; gap: 10px; height: 36px; padding: 0 10px; border-radius: var(--r-sm); font-size: var(--t-sm); text-align: left; }
  .item:hover { background: var(--hover); }
</style>
