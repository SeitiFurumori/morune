<script lang="ts">
  import Capa from './Capa.svelte'
  import Icone from './Icone.svelte'
  import { acao, motor, Pagina } from './motor.svelte'

  const e = $derived(motor.e)
  let filtro = $state('')
  const recolhida = $derived(!!motor.tema?.spec && motor.tema.sidebar_collapsed)

  const nav = [
    { p: Pagina.Inicio, nome: 'Início', icone: 'inicio' },
    { p: Pagina.Busca, nome: 'Buscar', icone: 'busca' },
    { p: Pagina.Biblioteca, nome: 'Biblioteca', icone: 'biblioteca' },
  ]

  function novaPlaylist() {
    const nome = prompt('Nome da nova playlist')
    if (nome?.trim()) acao('create-playlist', nome.trim())
  }
</script>

<aside class="lateral" class:recolhida>
  <div class="marca"><span class="logo"></span><span class="nome">Morune</span></div>
  <!-- A pilula do item ativo e uma so e desliza entre eles com mola. -->
  <nav style:--ativo={nav.findIndex((n) => n.p === e.page)}>
    <span class="pilula" class:some={!nav.some((n) => n.p === e.page)} aria-hidden="true"></span>
    {#each nav as n}
      <button class="item" class:ativo={e.page === n.p} onclick={() => acao('navigate', n.p)}>
        <Icone nome={n.icone} tamanho={22} /><span>{n.nome}</span>
      </button>
    {/each}
  </nav>

  <div class="cab">
    <label class="campo pequeno">
      <Icone nome="busca" tamanho={16} />
      <input placeholder="Filtrar playlists" bind:value={filtro} oninput={() => acao('filter-playlists', filtro)} />
    </label>
    <button class="icone" title="Nova playlist" onclick={novaPlaylist}><Icone nome="mais" tamanho={18} /></button>
  </div>

  <div class="playlists">
    {#each e.sidebar_playlists ?? [] as pl (pl.id)}
      <button class="pl" onclick={() => acao('play-track', pl.id)} title={pl.title}>
        <Capa src={pl.cover} inicial={pl.initial} matiz={pl.hue} pendente={pl.cover_pending} curtidas={pl.id === 'liked'} tamanho="44px" />
        <div class="pl-txt">
          <div class="corta">{pl.title}</div>
          {#if pl.subtitle}<div class="corta suave pl-sub">{pl.subtitle}</div>{/if}
        </div>
      </button>
    {/each}
  </div>

  <div class="rodape">
    <button class="item" class:ativo={e.page === Pagina.Configuracoes} onclick={() => acao('navigate', Pagina.Configuracoes)}>
      <Icone nome="config" tamanho={22} /><span>Configurações</span>
    </button>
    {#if e.logged_in}
      <div class="conta">
        {#if e.account_avatar}<img src={e.account_avatar} alt="" />{:else}<span class="inicial">{e.account_initial}</span>{/if}
        <span class="corta">{e.account_name}</span>
      </div>
    {/if}
  </div>
</aside>

<style>
  .lateral { display: flex; flex-direction: column; min-height: 0; padding: var(--e-md) var(--e-sm); gap: var(--e-xs); }
  .marca { display: flex; align-items: center; gap: 10px; padding: 4px var(--e-sm) var(--e-md); font: var(--peso-forte) var(--t-lg) var(--fonte-titulo); }
  .logo { width: 22px; height: 22px; border-radius: 7px; background: linear-gradient(135deg, var(--acento-tema), color-mix(in srgb, var(--acento-tema) 50%, #fff)); }
  .item { display: flex; align-items: center; gap: var(--e-md); width: 100%; height: 42px; padding: 0 var(--e-sm); border-radius: var(--r-sm); color: var(--texto-suave); font-weight: var(--peso-medio); transition: background var(--m-rapido), color var(--m-rapido); }
  .item:hover { color: var(--texto); background: var(--hover); }
  /* O lugar onde a pessoa esta leva um fundo leve da cor de destaque. */
  .item.ativo { color: var(--texto); background: color-mix(in srgb, var(--acento) 16%, transparent); }
  .item.ativo :global(svg) { color: var(--acento); }
  nav { position: relative; display: flex; flex-direction: column; gap: var(--e-xs); }
  nav .item { position: relative; z-index: 1; }
  nav .item.ativo { background: none; }
  nav .item.ativo:hover { background: none; }
  .pilula {
    position: absolute; left: 0; right: 0; top: 0; height: 42px; border-radius: var(--r-sm);
    background: color-mix(in srgb, var(--acento) 16%, transparent); pointer-events: none;
    transform: translateY(calc(var(--ativo) * (42px + var(--e-xs))));
    transition: transform 460ms cubic-bezier(0.34, 1.3, 0.64, 1), opacity var(--m-rapido);
  }
  .pilula.some { opacity: 0; }
  .cab { display: flex; align-items: center; gap: var(--e-xs); margin-top: var(--e-md); padding: 0 var(--e-xs); }
  .campo.pequeno { height: 34px; flex: 1; font-size: var(--t-sm); }
  .icone { display: grid; place-items: center; width: 34px; height: 34px; border-radius: 99px; color: var(--texto-suave); }
  .icone:hover { background: var(--hover); color: var(--texto); }
  .playlists { flex: 1; min-height: 0; overflow-y: auto; margin-top: var(--e-sm); }
  .pl { display: flex; align-items: center; gap: var(--e-md); width: 100%; padding: 6px var(--e-sm); border-radius: var(--r-sm); text-align: left; transition: background var(--m-rapido); }
  .pl:hover { background: var(--hover); }
  .pl-txt { min-width: 0; font-size: var(--t-sm); font-weight: var(--peso-medio); }
  .pl-sub { font-size: var(--t-xs); font-weight: var(--peso-normal); }
  .rodape { padding-top: var(--e-sm); }
  .conta { display: flex; align-items: center; gap: var(--e-md); padding: 6px var(--e-sm); font-size: var(--t-sm); color: var(--texto-suave); }
  .conta img, .inicial { width: 28px; height: 28px; border-radius: 50%; object-fit: cover; flex: none; }
  .inicial { display: grid; place-items: center; background: var(--selecionado); color: var(--texto); font-weight: var(--peso-forte); }
</style>
