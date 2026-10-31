<script lang="ts">
  import Cartao from '../lib/Cartao.svelte'
  import Icone from '../lib/Icone.svelte'
  import { acao, motor } from '../lib/motor.svelte'
  const e = $derived(motor.e)
  let filtro = $state('')
  const tipos = ['Tudo', 'Playlists', 'Álbuns', 'Artistas']
  const ordens = ['Recentes', 'A–Z']
</script>

<div class="topo">
  <h1 class="titulo-pagina">Biblioteca</h1>
  <label class="campo">
    <Icone nome="busca" tamanho={16} />
    <input placeholder="Buscar na Biblioteca" bind:value={filtro} oninput={() => acao('filter-library', filtro)} />
  </label>
</div>

<div class="ferr">
  <div class="chips">
    {#each tipos as t, i}<button class="chip" class:ativo={e.library_kind === i} onclick={() => acao('set-library-kind', i)}>{t}</button>{/each}
  </div>
  <div class="chips">
    {#each ordens as t, i}<button class="chip" class:ativo={e.library_sort === i} onclick={() => acao('set-library-sort', i)}>{t}</button>{/each}
  </div>
</div>

{#if !e.library_loaded}
  <p class="suave">Carregando sua biblioteca…</p>
{:else if !e.library_items?.length}
  <p class="suave">{e.library_filtering ? 'Nada combina com o filtro.' : 'Sua biblioteca está vazia.'}</p>
{:else}
  <div class="grade">{#each e.library_items as item (item.id)}<Cartao {item} />{/each}</div>
{/if}

<style>
  .topo { display: flex; align-items: center; justify-content: space-between; gap: var(--e-md); }
  .topo .campo { width: 260px; }
  .ferr { display: flex; justify-content: space-between; gap: var(--e-md); margin: var(--e-lg) 0 var(--e-md); }
  .chips { display: flex; flex-wrap: wrap; gap: var(--e-sm); }
  .grade { display: grid; grid-template-columns: repeat(auto-fill, minmax(170px, 1fr)); gap: var(--e-xs); margin: 0 calc(-1 * var(--e-sm)); }
</style>
