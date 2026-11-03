<script lang="ts">
  import Faixas from '../lib/Faixas.svelte'
  import Prateleira from '../lib/Prateleira.svelte'
  import { acao, motor } from '../lib/motor.svelte'
  const e = $derived(motor.e)
  const vazio = $derived(!(e.home_feed?.length || e.home_liked?.length || e.home_playlists?.length || e.home_made_for_you?.length))
</script>

{#if !e.logged_in && vazio}
  <div class="vazio">
    <h1 class="titulo-pagina">Nada por aqui ainda</h1>
    <p class="suave">
      {e.auth_url ? 'Conclua o login no navegador. Se a aba sumiu, abra de novo.' : 'Entre na sua conta para ver suas músicas curtidas, o que você mais ouve e suas playlists.'}
    </p>
    <div class="acoes">
      <button class="botao primario" onclick={() => acao('login')}>{e.auth_url ? 'Abrir de novo' : 'Entrar no Spotify'}</button>
      {#if e.auth_url}<button class="botao" onclick={() => acao('copy-auth-url')}>Copiar link</button>
        <button class="botao" onclick={() => acao('cancel-login')}>Cancelar</button>{/if}
    </div>
  </div>
{:else}
  {#if e.home_greeting}<h1 class="titulo-pagina">{e.home_greeting}</h1>{/if}

  <!-- As curtidas abrem a pagina, e nao as tocadas recentemente: tocar uma
       daqui segue pelas curtidas; uma recente tocava sozinha e caia numa
       musica qualquer. As recentes continuam na prateleira do Spotify. -->
  {#if e.home_liked?.length}
    <section class="bloco">
      <div class="titulo-linha">
        <h2 class="secao-titulo">Músicas curtidas</h2>
        <button class="ver suave" onclick={() => acao('play-track', 'liked')}>Ver todas</button>
      </div>
      <Faixas faixas={e.home_liked.slice(0, 6)} />
    </section>
  {/if}

  {#each e.home_feed ?? [] as sh, i (sh.title + ':' + i)}
    <Prateleira titulo={sh.title} itens={sh.items} />
  {:else}
    <Prateleira titulo="Feito para você" itens={e.home_made_for_you} />
    <Prateleira titulo="Suas playlists" itens={e.home_playlists} />
    <Prateleira titulo="Estações" itens={e.home_stations} />
    <Prateleira titulo="Retrospectivas" itens={e.home_retrospectives} />
  {/each}
{/if}

<style>
  .bloco { margin-top: var(--e-xl); }
  .titulo-linha { display: flex; align-items: baseline; justify-content: space-between; margin-bottom: var(--e-sm); }
  .ver { font-size: var(--t-sm); font-weight: var(--peso-medio); }
  .ver:hover { color: var(--texto); text-decoration: underline; }
  .vazio { max-width: 520px; margin: 12vh auto 0; text-align: center; display: grid; gap: var(--e-md); }
  .acoes { display: flex; justify-content: center; gap: var(--e-sm); }
</style>
