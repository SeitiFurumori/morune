// A capa que voa: ao abrir um album, playlist ou artista, a capa do cartao
// cresce ate o cabecalho da pagina; ao voltar, faz o caminho de volta.
//
// Usa as View Transitions do navegador: ele fotografa a tela antes e depois e
// anima entre as duas o elemento que tem o mesmo nome (`capa-voo`) nos dois
// lados. Durante a troca a tela fica congelada, entao so voa o que de fato
// abre uma pagina, e a espera pela pagina nova tem limite curto.

import { tick } from 'svelte'
import { motor } from './motor.svelte'

/** Id do cartao de onde a capa saiu: ele recebe o nome na volta. */
export const voo = $state({ alvo: '' })

const ABRE_PAGINA = /^(album|playlist|artist|show)\/|^liked$/

function podeVoar() {
  const d = document as any
  return typeof d.startViewTransition === 'function' && document.documentElement.dataset.quieta !== '1'
    && document.documentElement.dataset.parado !== '1'
}

/** Espera o motor trocar de pagina (ou de detalhe), ate `limite` ms. */
function paginaMudou(limite = 700) {
  const antes = `${motor.e.page}|${motor.e.detail_title}`
  const t0 = performance.now()
  return new Promise<void>((ok) => {
    const olhar = () => {
      if (`${motor.e.page}|${motor.e.detail_title}` !== antes || performance.now() - t0 > limite) ok()
      // Nao `requestAnimationFrame`: com a tela congelada pela transicao ele
      // nao roda, e a troca expirava sem nunca terminar.
      else setTimeout(olhar, 16)
    }
    olhar()
  })
}

/** Abre `id` voando a capa a partir do cartao `el`. */
export function abrirVoando(id: string, el: HTMLElement | null, abrir: () => void) {
  if (!ABRE_PAGINA.test(id) || !el || !podeVoar()) {
    abrir()
    return
  }
  voo.alvo = id
  el.style.viewTransitionName = 'capa-voo'
  ;(document as any).startViewTransition(async () => {
    el.style.viewTransitionName = ''
    abrir()
    await paginaMudou()
    await tick()
  })
}

/** Volta do detalhe; a capa do cabecalho volta para o cartao de onde veio. */
export function voltarVoando(voltar: () => void) {
  if (!voo.alvo || !podeVoar()) {
    voltar()
    return
  }
  ;(document as any).startViewTransition(async () => {
    voltar()
    await paginaMudou()
    await tick()
  })
}
