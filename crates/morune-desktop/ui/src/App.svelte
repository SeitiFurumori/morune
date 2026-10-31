<script lang="ts">
  import Lateral from './lib/Lateral.svelte'
  import Player from './lib/Player.svelte'
  import Menu from './lib/Menu.svelte'
  import Icone from './lib/Icone.svelte'
  import Inicio from './paginas/Inicio.svelte'
  import Busca from './paginas/Busca.svelte'
  import Biblioteca from './paginas/Biblioteca.svelte'
  import Detalhe from './paginas/Detalhe.svelte'
  import Fila from './paginas/Fila.svelte'
  import Configuracoes from './paginas/Configuracoes.svelte'
  import { acao, motor, Pagina } from './lib/motor.svelte'

  const e = $derived(motor.e)
  let conteudo: HTMLElement | undefined = $state()

  // A cor viva da capa vira o acento; sem capa, o do tema.
  $effect(() => {
    document.documentElement.style.setProperty('--acento', e.live_accent || 'var(--acento-tema)')
  })
  // Escondida ou com jogo em tela cheia: nada se mexe.
  $effect(() => {
    document.documentElement.dataset.quieta = !motor.visivel || motor.telaCheia ? '1' : '0'
  })
  // Trocar de pagina volta ao topo.
  $effect(() => {
    e.page
    e.detail_title
    conteudo?.scrollTo({ top: 0 })
  })

  function teclas(ev: KeyboardEvent) {
    const alvo = ev.target as HTMLElement
    if (alvo.tagName === 'INPUT' || alvo.tagName === 'SELECT' || alvo.tagName === 'TEXTAREA') return
    if (ev.code === 'Space') { ev.preventDefault(); acao('toggle-play') }
    else if (ev.ctrlKey && ev.key === 'ArrowRight') acao('next-track')
    else if (ev.ctrlKey && ev.key === 'ArrowLeft') acao('previous-track')
    else if (ev.shiftKey && ev.key === 'ArrowRight') acao('seek-by', 10)
    else if (ev.shiftKey && ev.key === 'ArrowLeft') acao('seek-by', -10)
    else if (ev.ctrlKey && ev.key === 'ArrowUp') acao('nudge-volume', 0.05)
    else if (ev.ctrlKey && ev.key === 'ArrowDown') acao('nudge-volume', -0.05)
    else if (ev.ctrlKey && ev.key.toLowerCase() === 'l') { ev.preventDefault(); acao('navigate', Pagina.Busca) }
    else if (ev.ctrlKey && ev.key.toLowerCase() === 's') { ev.preventDefault(); acao('toggle-shuffle') }
    else if (ev.ctrlKey && ev.key.toLowerCase() === 'r') { ev.preventDefault(); acao('cycle-repeat') }
    else if (ev.key === 'Escape' && e.page === Pagina.Detalhe) acao('detail-back')
  }
  // Botoes laterais do mouse voltam do Detalhe, como num navegador.
  function mouse(ev: MouseEvent) {
    if (ev.button === 3 && e.page === Pagina.Detalhe) acao('detail-back')
  }
</script>

<svelte:window onkeydown={teclas} onmouseup={mouse} />

<div class="fundo" aria-hidden="true"></div>
<div class="app" class:carregando={!motor.pronto}>
  <div class="superficie lateral-wrap"><Lateral /></div>
  <main class="superficie conteudo" bind:this={conteudo}>
    <div class="pagina">
      {#key e.page}
        <div class="entra">
          {#if e.page === Pagina.Busca}<Busca />
          {:else if e.page === Pagina.Biblioteca}<Biblioteca />
          {:else if e.page === Pagina.Configuracoes}<Configuracoes />
          {:else if e.page === Pagina.Fila}<Fila />
          {:else if e.page === Pagina.Detalhe}<Detalhe />
          {:else}<Inicio />{/if}
        </div>
      {/key}
    </div>
  </main>
  <div class="superficie player-wrap"><Player /></div>
</div>

{#if e.status_message}
  <div class="aviso" role="status">
    <span>{e.status_message}</span>
    {#if e.undo_available}<button class="botao" onclick={() => acao('undo-last')}>Desfazer</button>{/if}
    {#if e.retry_available}<button class="botao" onclick={() => acao('retry-last')}>Tentar de novo</button>{/if}
    <button class="fecha" title="Fechar" onclick={() => acao('dismiss-status')}><Icone nome="fechar" tamanho={16} /></button>
  </div>
{/if}

<Menu />

<style>
  .fundo {
    position: fixed; inset: 0; z-index: -1;
    background: var(--fundo-imagem) center / cover no-repeat;
    opacity: var(--fundo-opacidade, 1); filter: blur(var(--fundo-desfoque, 0));
  }
  .app {
    display: grid; grid-template-columns: 280px minmax(0, 1fr); grid-template-rows: minmax(0, 1fr) 88px;
    gap: var(--e-sm); height: 100vh; padding: var(--e-sm);
    transition: opacity var(--m-normal);
  }
  .app.carregando { opacity: 0; }
  .superficie { border-radius: var(--r-lg); min-height: 0; }
  :global([data-material='liquid']) .superficie, :global([data-material='aero']) .superficie {
    backdrop-filter: blur(var(--vidro-desfoque));
  }
  .lateral-wrap { background: var(--lateral); display: flex; flex-direction: column; overflow: hidden; }
  .conteudo { background: var(--superficie); overflow-y: auto; scroll-behavior: smooth; }
  .pagina { padding: var(--e-xl); max-width: 1500px; }
  .player-wrap { grid-column: 1 / 3; background: var(--player); }
  .entra { animation: entra var(--m-normal) var(--saida); }
  @keyframes entra { from { opacity: 0; transform: translateY(6px); } to { opacity: 1; transform: none; } }
  .aviso {
    position: fixed; left: 50%; bottom: 104px; transform: translateX(-50%); z-index: 40;
    display: flex; align-items: center; gap: var(--e-md); padding: 8px 8px 8px 18px; border-radius: 99px;
    background: var(--texto); color: var(--fundo); font-size: var(--t-sm); font-weight: var(--peso-medio);
    box-shadow: 0 10px 30px #0008; animation: entra var(--m-normal) var(--saida);
  }
  .aviso .botao { background: color-mix(in srgb, var(--fundo) 12%, transparent); height: 30px; }
  .fecha { display: grid; place-items: center; width: 30px; height: 30px; border-radius: 99px; }
  :global([data-quieta='1'] *) { animation-play-state: paused !important; transition: none !important; }
</style>
