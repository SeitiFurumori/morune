<script lang="ts">
  // Lista de faixas. O navegador so desenha as linhas visiveis
  // (`content-visibility`), entao listas de milhares rolam sem custo extra.
  import Capa from './Capa.svelte'
  import Icone from './Icone.svelte'
  import { acao } from './motor.svelte'
  import { abrirMenu } from './contexto.svelte'

  let {
    faixas = [],
    ativar = (f: any, _i: number) => acao('play-track', f.id),
    album = true,
    fila = false,
    numerar = true,
  }: {
    faixas?: any[]
    ativar?: (f: any, i: number) => void
    album?: boolean
    fila?: boolean
    numerar?: boolean
  } = $props()
</script>

<div class="lista" class:sem-album={!album}>
  {#each faixas as f, i (f.id + ':' + i)}
    <div class="linha" class:tocando={f.playing} class:indisponivel={!f.playable}
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
        onclick={() => acao('toggle-favorite', f.id)}>
        <Icone nome="coracao" cheio={f.favorite} tamanho={18} />
      </button>
      <div class="d suave">{f.duration}</div>
      <button class="mais" title="Mais opções" onclick={(e) => abrirMenu(e, f, fila ? i : -1)}><Icone nome="pontos" tamanho={18} /></button>
    </div>
  {/each}
</div>

<style>
  .lista { display: flex; flex-direction: column; }
  .linha {
    display: grid; grid-template-columns: 36px 44px minmax(0, 3fr) minmax(0, 2fr) 34px 52px 34px;
    align-items: center; gap: var(--e-md); height: 60px; padding: 0 var(--e-sm);
    border-radius: var(--r-sm); content-visibility: auto; contain-intrinsic-size: auto 60px;
    transition: background var(--m-rapido);
  }
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
  .onda { display: flex; gap: 2px; align-items: flex-end; height: 14px; }
  .onda i { width: 3px; background: var(--acento); border-radius: 2px; animation: onda 0.9s ease-in-out infinite alternate; }
  .onda i:nth-child(2) { animation-delay: -0.3s; }
  .onda i:nth-child(3) { animation-delay: -0.6s; }
  @keyframes onda { from { height: 3px; } to { height: 14px; } }
</style>
