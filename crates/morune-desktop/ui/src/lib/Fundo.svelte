<script lang="ts">
  // O fundo da janela: imagem, GIF ou video escolhido pela pessoa.
  //
  // Como o Wallpaper Engine: com a janela escondida ou jogo em tela cheia, o
  // GIF e o video saem da pagina -- param e largam a memoria -- e voltam de
  // onde estavam. Com "reduzir movimento", o video fica parado no quadro em
  // que estava e o GIF da lugar a nada.
  import { motor } from './motor.svelte'

  const t = $derived(motor.tema ?? {})
  const quieta = $derived(!motor.visivel || motor.telaCheia)
  const parado = $derived(!!t.reduce_motion || t.spec?.motion?.enabled === false)
  let video: HTMLVideoElement | undefined = $state()
  // Onde o video estava quando saiu da pagina.
  let tempo = 0

  $effect(() => {
    const v = video
    if (!v) return
    if (parado) v.pause()
    else v.play().catch(() => {})
  })
</script>

{#if t.fundo && t.fundo_tipo === 'video'}
  {#if !quieta}
    <video
      class="fundo midia"
      src={t.fundo}
      bind:this={video}
      muted
      loop
      playsinline
      preload="auto"
      aria-hidden="true"
      onloadedmetadata={(ev) => {
        const v = ev.currentTarget
        v.currentTime = tempo
        motor.fundoVideo = { src: t.fundo, w: v.videoWidth, h: v.videoHeight }
      }}
      ontimeupdate={(ev) => { tempo = ev.currentTarget.currentTime }}
    ></video>
  {/if}
{:else if t.fundo && t.fundo_tipo === 'gif'}
  {#if !quieta && !parado}<img class="fundo midia" src={t.fundo} alt="" aria-hidden="true" />{/if}
{:else}
  <div class="fundo" aria-hidden="true"></div>
{/if}

<style>
  .fundo {
    position: fixed; inset: 0; z-index: -1; width: 100%; height: 100%;
    background: var(--fundo-imagem) center / var(--fundo-ajuste, cover) no-repeat;
    opacity: var(--fundo-opacidade, 1); filter: blur(var(--fundo-desfoque, 0));
    pointer-events: none;
  }
  .midia { object-fit: var(--fundo-encaixe, cover); }
</style>
