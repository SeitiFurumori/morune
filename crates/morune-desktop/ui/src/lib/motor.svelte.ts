// Ponte com o motor em Rust: o estado chega em JSON (so o que mudou) e as
// acoes voltam pelo comando `acao`, com os mesmos nomes do app em Slint.
import { aplicarTema } from './tema'

type Tauri = {
  core: { invoke: (cmd: string, args?: Record<string, unknown>) => Promise<any> }
  event: { listen: (nome: string, f: (e: { payload: any }) => void) => Promise<() => void> }
}
const T = (window as any).__TAURI__ as Tauri | undefined

export const motor = $state({
  e: {} as Record<string, any>,
  tema: null as any,
  pronto: false,
  visivel: true,
  telaCheia: false,
})

export async function iniciar() {
  if (!T) return
  const r = await T.core.invoke('estado_inicial')
  if (r?.estado) Object.assign(motor.e, r.estado)
  if (r?.tema) {
    motor.tema = r.tema
    aplicarTema(r.tema)
  }
  motor.pronto = true
  T.event.listen('estado', (ev) => Object.assign(motor.e, ev.payload))
  T.event.listen('tema', (ev) => {
    motor.tema = ev.payload
    aplicarTema(ev.payload)
  })
  T.event.listen('visibilidade', (ev) => (motor.visivel = !!ev.payload))
  T.event.listen('tela_cheia', (ev) => (motor.telaCheia = !!ev.payload))
}

export function acao(nome: string, ...args: unknown[]) {
  if (!T) return Promise.resolve()
  return T.core.invoke('acao', { nome, args }).catch((e) => console.error(nome, e))
}

export const Pagina = { Inicio: 0, Busca: 1, Biblioteca: 2, Configuracoes: 3, Fila: 4, Detalhe: 5 } as const
