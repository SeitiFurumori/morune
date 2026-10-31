// Menu de contexto das faixas: um so na tela, aberto onde o clique aconteceu.
export const menu = $state({ aberto: false, x: 0, y: 0, faixa: null as any, indiceFila: -1 })

export function abrirMenu(ev: MouseEvent, faixa: any, indiceFila = -1) {
  ev.preventDefault()
  menu.faixa = faixa
  menu.indiceFila = indiceFila
  menu.x = ev.clientX
  menu.y = ev.clientY
  menu.aberto = true
}

export function fecharMenu() {
  menu.aberto = false
}
