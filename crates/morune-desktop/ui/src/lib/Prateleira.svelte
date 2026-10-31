<script lang="ts">
  import Cartao from './Cartao.svelte'
  let { titulo, itens = [] }: { titulo: string; itens?: any[] } = $props()
</script>

{#if itens.length}
  <section class="prateleira">
    <h2 class="secao-titulo">{titulo}</h2>
    <div class="fileira">
      {#each itens as item (item.id)}<Cartao {item} />{/each}
    </div>
  </section>
{/if}

<style>
  .prateleira { margin-top: var(--e-xl); }
  h2 { margin-bottom: var(--e-sm); }
  .fileira {
    display: grid; grid-auto-flow: column; grid-auto-columns: clamp(150px, 15vw, 200px);
    gap: var(--e-xs); overflow-x: auto; margin: 0 calc(-1 * var(--e-sm)); padding-bottom: var(--e-sm);
    scrollbar-width: thin;
  }
</style>
