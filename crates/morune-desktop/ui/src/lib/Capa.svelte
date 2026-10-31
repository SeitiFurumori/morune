<script lang="ts">
  // Capa da faixa, album ou playlist. Sem imagem, uma ficha com a inicial na
  // cor do nome -- a mesma cor sempre, para a coisa ser reconhecivel de relance.
  let {
    src = '',
    inicial = '',
    matiz = 0,
    pendente = false,
    redonda = false,
    curtidas = false,
    tamanho = '100%',
  }: {
    src?: string
    inicial?: string
    matiz?: number
    pendente?: boolean
    redonda?: boolean
    curtidas?: boolean
    tamanho?: string
  } = $props()
  let falhou = $state(false)
  $effect(() => {
    src
    falhou = false
  })
</script>

<div class="capa" class:redonda style:width={tamanho} style:--h={matiz}>
  {#if curtidas}
    <div class="curtidas">
      <svg viewBox="0 0 24 24" fill="#fff"><path d="M12 20s-7.5-4.6-7.5-10A4.3 4.3 0 0 1 12 7.4 4.3 4.3 0 0 1 19.5 10C19.5 15.4 12 20 12 20z" /></svg>
    </div>
  {:else if src && !falhou}
    <img {src} alt="" loading="lazy" decoding="async" onerror={() => (falhou = true)} />
  {:else}
    <div class="ficha" class:pendente><span>{inicial}</span></div>
  {/if}
</div>

<style>
  .capa { aspect-ratio: 1; border-radius: var(--r-capa); overflow: hidden; flex: none; background: var(--superficie-alta); container-type: inline-size; }
  .capa.redonda { border-radius: 50%; }
  img { width: 100%; height: 100%; object-fit: cover; }
  .ficha {
    width: 100%; height: 100%; display: grid; place-items: center;
    background: linear-gradient(135deg, hsl(var(--h) 45% 34%), hsl(calc(var(--h) + 30) 40% 22%));
    color: #fff; font-weight: var(--peso-forte); font-size: 38cqw;
  }
  .ficha.pendente { background: var(--superficie-alta); }
  .ficha.pendente span { opacity: 0; }
  .curtidas { width: 100%; height: 100%; display: grid; place-items: center; background: linear-gradient(135deg, #4a2bff, #a7c7e7); }
  .curtidas svg { width: 44%; height: 44%; }
</style>
