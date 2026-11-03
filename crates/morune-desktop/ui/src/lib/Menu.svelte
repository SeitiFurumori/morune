<script lang="ts">
  import Icone from './Icone.svelte'
  import { acao, motor } from './motor.svelte'
  import { menu, fecharMenu } from './contexto.svelte'

  let submenu = $state(false)
  let el: HTMLDivElement | undefined = $state()
  let pos = $state({ x: 0, y: 0 })

  $effect(() => {
    if (!menu.aberto || !el) return
    submenu = false
    const r = el.getBoundingClientRect()
    pos = {
      x: Math.min(menu.x, innerWidth - r.width - 8),
      y: Math.min(menu.y, innerHeight - r.height - 8),
    }
  })

  function fazer(nome: string, ...args: unknown[]) {
    acao(nome, ...args)
    fecharMenu()
  }
  const f = $derived(menu.faixa)
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && fecharMenu()} />

{#if menu.aberto && f}
  <div class="veu" role="presentation" onclick={fecharMenu} oncontextmenu={(e) => { e.preventDefault(); fecharMenu() }}></div>
  <div class="menu" bind:this={el} style:left="{pos.x}px" style:top="{pos.y}px" role="menu">
    <button role="menuitem" onclick={() => fazer('play-track', f.id)}><Icone nome="tocar" cheio tamanho={16} />Tocar</button>
    <button role="menuitem" onclick={() => fazer('queue-play-next', f.id)}><Icone nome="fila" tamanho={16} />Tocar a seguir</button>
    <button role="menuitem" onclick={() => fazer('queue-enqueue', f.id)}><Icone nome="mais" tamanho={16} />Adicionar à fila</button>
    {#if menu.indiceFila >= 0}
      <button role="menuitem" onclick={() => fazer('queue-remove', menu.indiceFila)}><Icone nome="fechar" tamanho={16} />Tirar da fila</button>
    {/if}
    <hr />
    <button role="menuitem" onclick={() => fazer('toggle-favorite', f.id)}><Icone nome="coracao" cheio={f.favorite} tamanho={16} />{f.favorite ? 'Remover das curtidas' : 'Curtir'}</button>
    <div class="sub" role="presentation" onmouseenter={() => (submenu = true)} onmouseleave={() => (submenu = false)}>
      <button role="menuitem" onclick={() => (submenu = !submenu)}><Icone nome="biblioteca" tamanho={16} />Adicionar à playlist</button>
      {#if submenu}
        <div class="menu lista-pl">
          {#each motor.e.acoes_playlists ?? [] as pl, i (pl.id + ':' + i)}
            <button role="menuitem" class="corta" onclick={() => fazer('add-to-playlist', pl.id, f.id)}>{pl.title}</button>
          {:else}
            <span class="vazio suave">Nenhuma playlist sua</span>
          {/each}
        </div>
      {/if}
    </div>
    {#if motor.e.editable_playlist_open}
      <button role="menuitem" onclick={() => fazer('remove-from-playlist', f.id)}><Icone nome="lixeira" tamanho={16} />Remover desta playlist</button>
    {/if}
    <hr />
    <button role="menuitem" onclick={() => fazer('start-radio', f.id)}><Icone nome="radio" tamanho={16} />Rádio desta música</button>
    {#if f.artist_id}<button role="menuitem" onclick={() => fazer('play-track', f.artist_id)}><Icone nome="pessoa" tamanho={16} />Ir para o artista</button>{/if}
    {#if f.album_id}<button role="menuitem" onclick={() => fazer('play-track', f.album_id)}><Icone nome="disco" tamanho={16} />Ir para o álbum</button>{/if}
    <button role="menuitem" onclick={() => fazer('copy-link', f.id)}><Icone nome="link" tamanho={16} />Copiar link</button>
    <button role="menuitem" onclick={() => fazer('toggle-hidden', f.id)}><Icone nome="olho" tamanho={16} />Não tocar no rádio</button>
  </div>
{/if}

<style>
  .veu { position: fixed; inset: 0; z-index: 50; }
  .menu {
    position: fixed; z-index: 51; min-width: 240px; padding: 6px; border-radius: var(--r-md);
    background: var(--superficie-alta); box-shadow: 0 14px 40px #000a; border: 1px solid var(--borda);
    animation: entra var(--m-rapido) var(--saida);
  }
  button { display: flex; align-items: center; gap: 10px; width: 100%; height: 36px; padding: 0 10px; border-radius: var(--r-sm); font-size: var(--t-sm); text-align: left; }
  button:hover { background: var(--hover); }
  hr { border: 0; height: 1px; background: var(--borda); margin: 4px 6px; }
  .sub { position: relative; }
  .lista-pl { position: absolute; left: calc(100% + 4px); top: -6px; max-height: 320px; overflow: auto; }
  .vazio { display: block; padding: 8px 10px; font-size: var(--t-sm); }
  @keyframes entra { from { opacity: 0; transform: scale(0.97); } to { opacity: 1; transform: none; } }
</style>
