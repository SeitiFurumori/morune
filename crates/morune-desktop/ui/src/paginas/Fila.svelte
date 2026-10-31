<script lang="ts">
  import Capa from '../lib/Capa.svelte'
  import Faixas from '../lib/Faixas.svelte'
  import { acao, motor } from '../lib/motor.svelte'
  const e = $derived(motor.e)
</script>

<div class="topo">
  <h1 class="titulo-pagina">Fila</h1>
  {#if e.queue_manual_tracks?.length}<button class="botao" onclick={() => acao('queue-clear')}>Limpar fila</button>{/if}
</div>

{#if e.has_track}
  <section class="bloco">
    <h2 class="secao-titulo">Tocando agora</h2>
    <div class="agora">
      <Capa src={e.now_cover} inicial={e.now_cover_initial} matiz={e.now_cover_hue} pendente={e.now_cover_pending} tamanho="72px" />
      <div><div class="t">{e.now_title}</div><div class="suave">{e.now_artist}</div></div>
    </div>
  </section>
{/if}

{#if e.queue_manual_tracks?.length}
  <section class="bloco">
    <h2 class="secao-titulo">A seguir na fila</h2>
    <Faixas faixas={e.queue_manual_tracks} fila numerar={false} ativar={(_f, i) => acao('queue-play-manual', i)} />
  </section>
{/if}

{#if e.queue_context_tracks?.length}
  <section class="bloco">
    <h2 class="secao-titulo">A seguir{e.queue_context_name ? ` de ${e.queue_context_name}` : ''}</h2>
    <Faixas faixas={e.queue_context_tracks} numerar={false} />
  </section>
{/if}

{#if !e.has_track && !e.queue_manual_tracks?.length && !e.queue_context_tracks?.length}
  <p class="suave bloco">A fila está vazia. Toque algo ou use “Adicionar à fila” no menu de uma música.</p>
{/if}

<style>
  .topo { display: flex; align-items: center; justify-content: space-between; }
  .bloco { margin-top: var(--e-xl); }
  .bloco h2 { margin-bottom: var(--e-md); }
  .agora { display: flex; align-items: center; gap: var(--e-md); }
  .t { font-weight: var(--peso-forte); font-size: var(--t-lg); color: var(--acento); }
</style>
