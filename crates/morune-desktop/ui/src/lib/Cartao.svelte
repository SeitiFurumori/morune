<script lang="ts">
  import Capa from './Capa.svelte'
  import Icone from './Icone.svelte'
  import { acao, motor, Pagina } from './motor.svelte'
  import { abrirVoando, voo } from './voo.svelte'
  let { item, redonda = false }: { item: any; redonda?: boolean } = $props()
  const artista = $derived(redonda || String(item.id).startsWith('artist/'))
  let arte: HTMLElement | undefined = $state()
  // Na volta do detalhe, este cartao e o destino da capa que voou dele.
  const destino = $derived(voo.alvo === item.id && motor.e.page !== Pagina.Detalhe)
</script>

<button class="cartao" onclick={() => abrirVoando(String(item.id), arte ?? null, () => acao('play-track', item.id))} title={item.title}>
  <div class="arte" bind:this={arte} style:view-transition-name={destino ? 'capa-voo' : undefined}>
    <Capa src={item.cover} inicial={item.initial} matiz={item.hue} pendente={item.cover_pending}
      redonda={artista} curtidas={item.id === 'liked'} />
    <span class="tocar" aria-hidden="true"><Icone nome="tocar" cheio tamanho={22} /></span>
  </div>
  <div class="nome corta">{item.title}</div>
  {#if item.subtitle}<div class="sub suave">{item.subtitle}</div>{/if}
</button>

<style>
  .cartao { display: flex; flex-direction: column; gap: 4px; padding: var(--e-sm); border-radius: var(--r-md); text-align: left; min-width: 0; transition: background var(--m-normal); }
  .cartao:hover { background: var(--hover); }
  .arte { position: relative; margin-bottom: var(--e-sm); }
  .arte :global(.capa) { box-shadow: 0 8px 22px #0006; }
  .tocar {
    position: absolute; inset: 0; display: grid; place-items: center; border-radius: var(--r-capa);
    background: #0007; color: #fff; opacity: 0; transition: opacity var(--m-normal) var(--saida);
  }
  .cartao:hover .tocar { opacity: 1; }
  .nome { font-weight: var(--peso-medio); }
  .sub { font-size: var(--t-sm); display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
</style>
