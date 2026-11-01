<script lang="ts">
  import Capa from '../lib/Capa.svelte'
  import Faixas from '../lib/Faixas.svelte'
  import Icone from '../lib/Icone.svelte'
  import Prateleira from '../lib/Prateleira.svelte'
  import { acao, motor } from '../lib/motor.svelte'
  import { voltarVoando } from '../lib/voo.svelte'
  const e = $derived(motor.e)
  let filtro = $state('')
  const ordens = ['Ordem da lista', 'Título', 'Artista', 'Álbum', 'Duração']
  const artista = $derived(e.detail_kind === 'artist' || e.detail_kind === 'Artista')

  function rolou(ev: Event) {
    const el = ev.currentTarget as HTMLElement
    if (e.detail_has_more && !e.detail_loading && el.scrollTop + el.clientHeight > el.scrollHeight - 800) acao('detail-load-more')
  }
  $effect(() => {
    const el = document.querySelector('.conteudo')
    el?.addEventListener('scroll', rolou, { passive: true })
    return () => el?.removeEventListener('scroll', rolou)
  })
</script>

<!-- Com `artwork_paints_ui`, o topo da pagina ganha a cor da capa aberta. -->
<div class="tinta" aria-hidden="true" style:--cor-detalhe={e.detail_cor}></div>

<button class="voltar" onclick={() => voltarVoando(() => acao('detail-back'))}><Icone nome="voltar" />Voltar</button>

<header class="cab">
  <div class="arte" style:view-transition-name="capa-voo">
    <Capa src={e.detail_cover} inicial={e.detail_cover_initial} matiz={e.detail_cover_hue}
      pendente={e.detail_cover_pending} redonda={artista} curtidas={e.detail_kind === 'liked'} />
  </div>
  <div class="info">
    <span class="tipo suave">{e.detail_kind}</span>
    <h1 class="titulo-pagina">{e.detail_title}</h1>
    {#if e.detail_subtitle}<p class="suave">{e.detail_subtitle}</p>{/if}
    <div class="acoes">
      <button class="tocar" title="Tocar" onclick={() => acao('detail-play')}><Icone nome="tocar" cheio tamanho={24} /></button>
      {#if e.detail_save_label}<button class="botao" onclick={() => acao('toggle-detail-save')}>{e.detail_save_label}</button>{/if}
    </div>
  </div>
</header>

{#if e.detail_tracks?.length || e.detail_filtered}
  <div class="ferr">
    <label class="campo">
      <Icone nome="busca" tamanho={16} />
      <input placeholder="Filtrar" bind:value={filtro} oninput={() => acao('detail-filter', filtro)} />
    </label>
    <select value={e.detail_sort} onchange={(ev) => acao('detail-sort-by', Number((ev.currentTarget as HTMLSelectElement).value))}>
      {#each ordens as o, i}<option value={i}>{o}</option>{/each}
    </select>
  </div>
  <Faixas faixas={e.detail_tracks ?? []} ativar={(f) => acao('detail-activate', f.id)}
    reordenar={e.detail_reordenavel ? (f, para) => acao('liked-move', f.id, para) : undefined} />
  {#if e.detail_loading}<p class="suave mais">Carregando mais…</p>{/if}
{/if}

<Prateleira titulo="Discografia" itens={e.detail_items} />
<Prateleira titulo="Relacionados" itens={e.detail_related} />

<style>
  .tinta { display: none; }
  :global([data-capa-pinta='1']) .tinta {
    /* Por cima do respiro da pagina (`.pagina` tem padding --e-xl) e alem da
       largura maxima dela, ate a borda do painel, que corta o que sobra. */
    display: block; position: absolute; inset: calc(-1 * var(--e-xl)) -100vw auto calc(-1 * var(--e-xl)); height: 440px; pointer-events: none; z-index: -1;
    background: linear-gradient(to bottom, color-mix(in srgb, var(--cor-detalhe) 38%, transparent), transparent);
    animation: aparece-tinta var(--m-lento) ease-out;
  }
  @keyframes aparece-tinta { from { opacity: 0; } }
  .voltar { display: inline-flex; align-items: center; gap: 6px; color: var(--texto-suave); font-weight: var(--peso-medio); margin-bottom: var(--e-md); }
  .voltar:hover { color: var(--texto); }
  .cab { display: flex; align-items: flex-end; gap: var(--e-xl); margin-bottom: var(--e-xl); }
  .arte { width: clamp(160px, 18vw, 232px); flex: none; }
  .arte :global(.capa) { box-shadow: 0 16px 48px #0008; }
  .info { display: grid; gap: var(--e-sm); min-width: 0; }
  .tipo { font-size: var(--t-sm); font-weight: var(--peso-medio); text-transform: capitalize; }
  .acoes { display: flex; align-items: center; gap: var(--e-md); margin-top: var(--e-sm); }
  .tocar { display: grid; place-items: center; width: 52px; height: 52px; border-radius: 99px; background: var(--acento); color: var(--texto-no-acento); transition: transform var(--m-rapido); }
  .tocar:hover { transform: scale(1.05); }
  .ferr { display: flex; gap: var(--e-sm); margin-bottom: var(--e-md); }
  .ferr .campo { width: 260px; }
  select { height: 40px; padding: 0 var(--e-md); border-radius: 99px; background: var(--hover); color: var(--texto); border: 0; font: inherit; font-size: var(--t-sm); }
  .mais { margin: var(--e-md) var(--e-sm); }
</style>
