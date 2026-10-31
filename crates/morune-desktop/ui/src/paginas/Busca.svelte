<script lang="ts">
  import Cartao from '../lib/Cartao.svelte'
  import Faixas from '../lib/Faixas.svelte'
  import Icone from '../lib/Icone.svelte'
  import { acao, motor } from '../lib/motor.svelte'
  const e = $derived(motor.e)
  let termo = $state(motor.e.search_query ?? '')
  let espera: ReturnType<typeof setTimeout> | undefined

  function digitar() {
    clearTimeout(espera)
    espera = setTimeout(() => acao('search', termo), 280)
  }
  function buscar(t: string) {
    termo = t
    acao('search', t)
  }
  const tipos = ['Tudo', 'Músicas', 'Álbuns', 'Artistas', 'Playlists']
</script>

<label class="campo grande">
  <Icone nome="busca" />
  <!-- svelte-ignore a11y_autofocus -->
  <input placeholder="O que você quer ouvir?" bind:value={termo} oninput={digitar} autofocus
    onkeydown={(ev) => ev.key === 'Enter' && buscar(termo)} />
  {#if termo}<button title="Limpar" onclick={() => buscar('')}><Icone nome="fechar" tamanho={16} /></button>{/if}
</label>

{#if !e.search_query}
  {#if e.recent_searches?.length}
    <section class="bloco">
      <div class="cab"><h2 class="secao-titulo">Buscas recentes</h2>
        <button class="botao" onclick={() => acao('clear-recent-searches')}>Limpar</button></div>
      <div class="chips">
        {#each e.recent_searches as r}<button class="chip" onclick={() => buscar(r)}>{r}</button>{/each}
      </div>
    </section>
  {/if}
{:else if e.searching}
  <p class="suave bloco">Buscando…</p>
{:else}
  <div class="chips bloco">
    {#each tipos as t, i}
      <button class="chip" class:ativo={e.search_kind === i} onclick={() => acao('set-search-kind', i)}>{t}</button>
    {/each}
  </div>
  {#if e.search_tracks?.length}
    <section class="bloco"><h2 class="secao-titulo">Músicas</h2><Faixas faixas={e.search_tracks} /></section>
  {/if}
  {#if e.search_items?.length}
    <section class="bloco">
      <div class="grade">{#each e.search_items as item (item.id)}<Cartao {item} />{/each}</div>
    </section>
  {/if}
  {#if !e.search_tracks?.length && !e.search_items?.length}
    <p class="suave bloco">Nada encontrado para “{e.search_query}”.</p>
  {/if}
{/if}

<style>
  .campo.grande { height: 48px; max-width: 560px; font-size: var(--t-md); }
  .bloco { margin-top: var(--e-xl); }
  .cab { display: flex; align-items: center; justify-content: space-between; margin-bottom: var(--e-md); }
  .chips { display: flex; flex-wrap: wrap; gap: var(--e-sm); }
  .grade { display: grid; grid-template-columns: repeat(auto-fill, minmax(170px, 1fr)); gap: var(--e-xs); margin: 0 calc(-1 * var(--e-sm)); }
</style>
