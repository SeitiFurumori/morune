<script lang="ts">
  // Barra de progresso com o desenho da musica: tracos altos onde ela e forte,
  // baixos onde e quieta. A parte tocada leva a cor de destaque, e um brilho
  // corre de um lado para o outro no ritmo do som.
  //
  // O desenho vem do motor (`picos`, de 0 a 1) quando ele souber. Sem isso, a
  // barra se desenha enquanto a musica toca, com o volume medido agora -- o
  // que ainda nao tocou fica baixo e apagado.
  //
  // Canvas e nao DOM: sao ~140 tracos a 60 quadros por segundo. E so anima
  // tocando, com a janela visivel e sem jogo em tela cheia (`data-quieta`).
  let {
    progresso = 0,
    picos = [],
    nivel = 0.5,
    tocando = false,
    faixa = '',
  }: { progresso?: number; picos?: number[]; nivel?: number; tocando?: boolean; faixa?: string } = $props()

  const N = 140
  let tela: HTMLCanvasElement | undefined = $state()
  // Desenho feito ao vivo, um valor por traco.
  let vivo = new Float32Array(N)
  let ultimaFaixa = ''
  let quadro = 0
  let fase = 0
  let antes = 0

  $effect(() => {
    if (faixa !== ultimaFaixa) {
      vivo = new Float32Array(N)
      ultimaFaixa = faixa
    }
  })

  // O volume medido entra no traco onde a musica esta agora.
  $effect(() => {
    if (!tocando || picos.length) return
    const i = Math.min(N - 1, Math.floor(progresso * N))
    vivo[i] = Math.max(vivo[i], 0.18 + 0.82 * Math.min(1, nivel))
  })

  function altura(i: number) {
    if (picos.length) {
      // Reamostra os picos do motor para o numero de tracos, pelo maior valor.
      const a = Math.floor((i / N) * picos.length)
      const b = Math.max(a + 1, Math.floor(((i + 1) / N) * picos.length))
      let m = 0
      for (let k = a; k < b; k++) m = Math.max(m, picos[k] ?? 0)
      return 0.06 + 0.94 * m
    }
    return vivo[i] || 0.12
  }

  function desenhar(agora: number) {
    const c = tela
    if (!c) return
    const dpr = window.devicePixelRatio || 1
    const w = c.clientWidth, h = c.clientHeight
    if (c.width !== Math.round(w * dpr) || c.height !== Math.round(h * dpr)) {
      c.width = Math.round(w * dpr)
      c.height = Math.round(h * dpr)
    }
    const g = c.getContext('2d')
    if (!g) return
    g.setTransform(dpr, 0, 0, dpr, 0, 0)
    g.clearRect(0, 0, w, h)

    const estilo = getComputedStyle(c)
    const acento = estilo.getPropertyValue('--acento').trim() || '#8b63f6'
    const texto = estilo.getPropertyValue('--texto').trim() || '#ffffff'
    const dt = antes ? Math.min(0.05, (agora - antes) / 1000) : 0
    antes = agora
    // O brilho atravessa a barra em ~2,4 s, mais depressa quando o som cresce.
    if (tocando) fase = (fase + dt * (0.32 + 0.25 * nivel)) % 1.3

    const passo = w / N
    const largura = Math.max(1.5, passo * 0.55)
    const feita = progresso * N
    for (let i = 0; i < N; i++) {
      const x = i * passo + (passo - largura) / 2
      // Distancia ate o brilho, que vai de -0,15 a 1,15 para entrar e sair.
      const d = Math.abs(i / N - (fase - 0.15))
      const luz = tocando ? Math.max(0, 1 - d / 0.08) * (0.4 + 0.6 * nivel) : 0
      const alto = Math.min(1, altura(i) * (1 + 0.25 * luz)) * h
      const y = (h - alto) / 2
      if (i < feita) {
        g.fillStyle = acento
        g.globalAlpha = 0.85 + 0.15 * luz
      } else {
        g.fillStyle = texto
        g.globalAlpha = (picos.length ? 0.28 : 0.16) + 0.4 * luz
      }
      g.beginPath()
      g.roundRect(x, y, largura, alto, largura / 2)
      g.fill()
    }
    g.globalAlpha = 1
  }

  function laco(agora: number) {
    desenhar(agora)
    const quieta = document.documentElement.dataset.quieta === '1' || document.documentElement.dataset.parado === '1'
    if (tocando && !quieta) quadro = requestAnimationFrame(laco)
    else quadro = 0
  }

  // Redesenha quando algo muda; anima so enquanto toca.
  $effect(() => {
    progresso; picos; tocando; faixa
    cancelAnimationFrame(quadro)
    antes = 0
    quadro = requestAnimationFrame(laco)
    return () => cancelAnimationFrame(quadro)
  })
</script>

<canvas class="onda" bind:this={tela} aria-hidden="true"></canvas>

<style>
  .onda { display: block; width: 100%; height: 100%; }
</style>
