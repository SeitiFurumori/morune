<script lang="ts">
  import Capa from '../lib/Capa.svelte'
  import Cartao from '../lib/Cartao.svelte'
  import Faixas from '../lib/Faixas.svelte'
  import Icone from '../lib/Icone.svelte'
  import Prateleira from '../lib/Prateleira.svelte'
  import { acao, motor } from '../lib/motor.svelte'
  const e = $derived(motor.e)
  let termo = $state(motor.e.search_query ?? '')
  let espera: ReturnType<typeof setTimeout> | undefined
  let campo: HTMLInputElement | undefined = $state()

  // Foco no campo ao abrir a busca. O `autofocus` sozinho perdia para o botao
  // da lateral, que ficava com a marca de foco do teclado.
  $effect(() => {
    campo?.focus()
  })

  function digitar() {
    clearTimeout(espera)
    if (e.categoria_aberta) acao('fechar-categoria')
    espera = setTimeout(() => acao('search', termo), 280)
  }
  function buscar(t: string) {
    termo = t
    acao('search', t)
  }
  const tipos = ['Tudo', 'Músicas', 'Álbuns', 'Artistas', 'Playlists']
  const temRecentes = $derived(!!(e.busca_recentes?.length || e.recent_searches?.length))
</script>

<label class="campo grande">
  <Icone nome="busca" />
  <input bind:this={campo} placeholder="O que você quer ouvir?" bind:value={termo} oninput={digitar}
    onkeydown={(ev) => ev.key === 'Enter' && buscar(termo)} />
  {#if termo}<button title="Limpar" onclick={() => buscar('')}><Icone nome="fechar" tamanho={16} /></button>{/if}
</label>

{#if !e.search_query && e.categoria_aberta}
  <!-- Uma categoria aberta: faixa de cor com o nome e as prateleiras dela. -->
  <header class="cat-cab" style:--cor={e.categoria_cor || 'var(--superficie-alta)'}>
    <button class="voltar" title="Voltar" onclick={() => acao('fechar-categoria')}><Icone nome="voltar" tamanho={20} /></button>
    <h1>{e.categoria_titulo}</h1>
  </header>
  {#if e.categoria_carregando}
    <div class="esqueleto">{#each Array(6) as _}<i></i>{/each}</div>
  {:else if !e.categoria_feed?.length}
    <p class="suave bloco">Nada nesta categoria que o Otomori toque.</p>
  {:else}
    {#each e.categoria_feed as sh, i (sh.title + ':' + i)}<Prateleira titulo={sh.title} itens={sh.items} />{/each}
  {/if}
{:else if !e.search_query}
  {#if temRecentes}
    <section class="bloco">
      <div class="cab"><h2 class="secao-titulo">Buscas recentes</h2>
        <button class="botao" onclick={() => acao('clear-recent-searches')}>Limpar</button></div>
      {#if e.busca_recentes?.length}
        <div class="recentes">
          {#each e.busca_recentes as r, i (r.id + ':' + i)}
            <div class="recente">
              <button class="abre" onclick={() => acao('play-track', r.id)} title={r.title}>
                <Capa src={r.cover} inicial={r.initial} matiz={r.hue} pendente={r.cover_pending}
                  redonda={String(r.id).startsWith('artist/')} tamanho="48px" />
                <span class="txt"><span class="corta">{r.title}</span><small class="suave corta">{r.subtitle || tipoDe(r.id)}</small></span>
              </button>
              <button class="tira" title="Tirar das recentes" onclick={() => acao('esquecer-da-busca', r.id)}><Icone nome="fechar" tamanho={14} /></button>
            </div>
          {/each}
        </div>
      {/if}
      {#if e.recent_searches?.length}
        <div class="chips">
          {#each e.recent_searches as r}<button class="chip" onclick={() => buscar(r)}><Icone nome="busca" tamanho={14} />{r}</button>{/each}
        </div>
      {/if}
    </section>
  {/if}

  {#if e.busca_artistas?.length}
    <Prateleira titulo="Você tem ouvido" itens={e.busca_artistas} />
  {/if}

  {#if e.busca_categorias?.length}
    <section class="bloco">
      <h2 class="secao-titulo cab">Navegar por tudo</h2>
      <div class="categorias">
        {#each e.busca_categorias as c, i (c.id + ':' + i)}
          <button class="categoria" style:--cor={c.subtitle} onclick={() => acao('abrir-categoria', c.id)}>
            <span class="nome">{c.title}</span>
            {#if c.cover}<img src={c.cover} alt="" loading="lazy" decoding="async" onerror={(ev) => ev.currentTarget.remove()} />{/if}
          </button>
        {/each}
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
      <div class="grade">{#each e.search_items as item, i (item.id + ':' + i)}<Cartao {item} />{/each}</div>
    </section>
  {/if}
  {#if !e.search_tracks?.length && !e.search_items?.length}
    <p class="suave bloco">Nada encontrado para “{e.search_query}”.</p>
  {/if}
{/if}

<script lang="ts" module>
  function tipoDe(id: string) {
    const t = String(id).split('/')[0]
    return { artist: 'Artista', album: 'Álbum', playlist: 'Playlist', track: 'Música' }[t] ?? ''
  }
</script>

<style>
  .campo.grande { height: 48px; max-width: 560px; font-size: var(--t-md); }
  .bloco { margin-top: var(--e-xl); }
  .cab { display: flex; align-items: center; justify-content: space-between; margin-bottom: var(--e-md); }
  .chips { display: flex; flex-wrap: wrap; gap: var(--e-sm); margin-top: var(--e-md); }
  .chip { display: inline-flex; align-items: center; gap: 6px; }
  .grade { display: grid; grid-template-columns: repeat(auto-fill, minmax(170px, 1fr)); gap: var(--e-xs); margin: 0 calc(-1 * var(--e-sm)); }

  /* Recentes: fichas compactas com capa, como atalhos. */
  .recentes { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: var(--e-xs); }
  .recente { position: relative; display: flex; align-items: center; border-radius: var(--r-md); background: var(--hover); transition: background var(--m-normal); }
  .recente:hover { background: var(--selecionado); }
  .abre { flex: 1; min-width: 0; display: flex; align-items: center; gap: var(--e-sm); padding: 6px; text-align: left; }
  .txt { min-width: 0; display: flex; flex-direction: column; gap: 2px; font-weight: var(--peso-medio); }
  .txt small { font-weight: normal; font-size: var(--t-sm); }
  .corta { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .tira { display: grid; place-items: center; width: 28px; height: 28px; margin-right: 8px; border-radius: 99px; color: var(--texto-suave); opacity: 0; transition: opacity var(--m-normal); }
  .recente:hover .tira, .tira:focus-visible { opacity: 1; }
  .tira:hover { background: var(--hover); color: var(--texto); }

  /* Categorias: cartao na cor que o Spotify deu, nome em cima e capa no canto. */
  .categorias { display: grid; grid-template-columns: repeat(auto-fill, minmax(190px, 1fr)); gap: var(--e-md); }
  .categoria {
    position: relative; overflow: hidden; aspect-ratio: 16 / 10; border-radius: var(--r-md);
    background: var(--cor); text-align: left; padding: var(--e-md);
    transition: transform var(--m-normal) var(--saida), filter var(--m-normal);
  }
  .categoria:hover { transform: translateY(-2px); filter: brightness(1.08); }
  .categoria .nome { position: relative; z-index: 1; display: block; max-width: 70%; color: #fff; font-weight: var(--peso-forte); font-size: var(--t-lg); line-height: 1.15; text-shadow: 0 1px 8px #0005; }
  .categoria img {
    position: absolute; right: -10%; bottom: -8%; width: 48%; aspect-ratio: 1; object-fit: cover;
    border-radius: 6px; transform: rotate(22deg); box-shadow: 0 6px 18px #0006;
  }

  /* Categoria aberta. */
  .cat-cab {
    display: flex; align-items: center; gap: var(--e-md); margin-top: var(--e-lg);
    padding: var(--e-lg) var(--e-lg); border-radius: var(--r-lg);
    background: linear-gradient(135deg, var(--cor), color-mix(in srgb, var(--cor) 45%, transparent));
  }
  .cat-cab h1 { margin: 0; color: #fff; font-size: var(--t-display, 34px); text-shadow: 0 2px 12px #0005; }
  .voltar { display: grid; place-items: center; width: 40px; height: 40px; border-radius: 99px; background: #0004; color: #fff; }
  .voltar:hover { background: #0007; }
  .esqueleto { display: grid; grid-auto-flow: column; grid-auto-columns: clamp(150px, 15vw, 200px); gap: var(--e-md); margin-top: var(--e-xl); overflow: hidden; }
  .esqueleto i { aspect-ratio: 1; border-radius: var(--r-capa); background: var(--superficie-alta); }

  @media (prefers-reduced-motion: reduce) { .categoria { transition: none; } .categoria:hover { transform: none; } }
</style>
