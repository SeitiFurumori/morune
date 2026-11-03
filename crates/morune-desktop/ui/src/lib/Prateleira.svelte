<script lang="ts">
  import Cartao from './Cartao.svelte'
  import Icone from './Icone.svelte'
  let { titulo, itens = [] }: { titulo: string; itens?: any[] } = $props()

  // Sem barra de rolagem: setas no canto do titulo, que somem na ponta. A
  // roda do mouse e o arrastar do touchpad continuam rolando de lado.
  let fileira: HTMLElement | undefined = $state()
  let inicio = $state(true)
  let fim = $state(false)
  function medir() {
    if (!fileira) return
    inicio = fileira.scrollLeft < 4
    fim = fileira.scrollLeft + fileira.clientWidth > fileira.scrollWidth - 4
  }
  function rolar(lado: number) {
    fileira?.scrollBy({ left: lado * fileira.clientWidth * 0.85, behavior: 'smooth' })
  }
  $effect(() => {
    itens
    queueMicrotask(medir)
    const ro = new ResizeObserver(medir)
    if (fileira) ro.observe(fileira)
    return () => ro.disconnect()
  })
</script>

{#if itens.length}
  <section class="prateleira">
    <div class="cab">
      <h2 class="secao-titulo">{titulo}</h2>
      {#if !(inicio && fim)}
        <div class="setas">
          <button class="seta" title="Voltar" disabled={inicio} onclick={() => rolar(-1)}><Icone nome="voltar" tamanho={18} /></button>
          <button class="seta" title="Avançar" disabled={fim} onclick={() => rolar(1)}><Icone nome="avancar" tamanho={18} /></button>
        </div>
      {/if}
    </div>
    <div class="fileira" bind:this={fileira} onscroll={medir}>
      {#each itens as item, i (item.id + ':' + i)}<Cartao {item} />{/each}
    </div>
  </section>
{/if}

<style>
  .prateleira { margin-top: var(--e-xl); }
  .cab { display: flex; align-items: center; justify-content: space-between; margin-bottom: var(--e-sm); }
  .setas { display: flex; gap: var(--e-xs); }
  .seta { display: grid; place-items: center; width: 32px; height: 32px; border-radius: 99px; background: var(--hover); color: var(--texto); }
  .seta:hover:not(:disabled) { background: var(--selecionado); }
  .seta:disabled { opacity: 0.3; cursor: default; }
  .fileira {
    display: grid; grid-auto-flow: column; grid-auto-columns: clamp(150px, 15vw, 200px);
    gap: var(--e-xs); overflow-x: auto; margin: 0 calc(-1 * var(--e-sm)); padding-bottom: var(--e-sm);
    scrollbar-width: none;
  }
</style>
