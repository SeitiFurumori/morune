// Cores que a pessoa escolhe por cima do tema: destaque e paleta.
//
// Tudo em OKLCH, onde "claridade" e a que o olho ve. Assim uma cor pronta tem a
// mesma forca em qualquer matiz, e trocar a paleta muda o tom do fundo sem
// mexer em quanto ele e claro -- o tema continua legivel como foi desenhado.

type Oklch = { l: number; c: number; h: number; a: number }

/** Cores de destaque prontas. A claridade vem do tema (claro ou escuro). */
export const ACENTOS = [
  { id: 'violeta', nome: 'Violeta', h: 295 },
  { id: 'azul', nome: 'Azul', h: 258 },
  { id: 'ciano', nome: 'Ciano', h: 215 },
  { id: 'verde', nome: 'Verde', h: 158 },
  { id: 'lima', nome: 'Lima', h: 128 },
  { id: 'ambar', nome: 'Âmbar', h: 78 },
  { id: 'coral', nome: 'Coral', h: 32 },
  { id: 'rosa', nome: 'Rosa', h: 352 },
]

/**
 * Paletas prontas: o tom dos fundos e paineis. `c` e o quanto de cor entra
 * nos cinzas -- pouco, para o fundo nao brigar com a capa.
 */
export const PALETAS = [
  { id: 'grafite', nome: 'Grafite', h: 260, c: 0.004 },
  { id: 'noite', nome: 'Noite', h: 262, c: 0.035 },
  { id: 'oceano', nome: 'Oceano', h: 220, c: 0.035 },
  { id: 'musgo', nome: 'Musgo', h: 150, c: 0.03 },
  { id: 'brasa', nome: 'Brasa', h: 38, c: 0.032 },
  { id: 'ameixa', nome: 'Ameixa', h: 320, c: 0.035 },
]

function hexParaOklch(hex: string): Oklch | null {
  const m = /^#([0-9a-f]{6})([0-9a-f]{2})?$/i.exec(hex)
  if (!m) return null
  const n = parseInt(m[1], 16)
  const lin = (v: number) => {
    v /= 255
    return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4
  }
  const r = lin(n >> 16), g = lin((n >> 8) & 255), b = lin(n & 255)
  const l_ = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b)
  const m_ = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b)
  const s_ = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b)
  const L = 0.2104542553 * l_ + 0.793617785 * m_ - 0.0040720468 * s_
  const A = 1.9779984951 * l_ - 2.428592205 * m_ + 0.4505937099 * s_
  const B = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.808675766 * s_
  return {
    l: L,
    c: Math.hypot(A, B),
    h: ((Math.atan2(B, A) * 180) / Math.PI + 360) % 360,
    a: m[2] ? parseInt(m[2], 16) / 255 : 1,
  }
}

function oklchParaHex({ l, c, h, a }: Oklch): string {
  // Fora do que a tela mostra, reduz a cor ate caber, mantendo a claridade.
  for (let cc = c; ; cc *= 0.9) {
    const A = cc * Math.cos((h * Math.PI) / 180), B = cc * Math.sin((h * Math.PI) / 180)
    const l_ = (l + 0.3963377774 * A + 0.2158037573 * B) ** 3
    const m_ = (l - 0.1055613458 * A - 0.0638541728 * B) ** 3
    const s_ = (l - 0.0894841775 * A - 1.291485548 * B) ** 3
    const rgb = [
      4.0767416621 * l_ - 3.3077115913 * m_ + 0.2309699292 * s_,
      -1.2684380046 * l_ + 2.6097574011 * m_ - 0.3413193965 * s_,
      -0.0041960863 * l_ - 0.7034186147 * m_ + 1.707614701 * s_,
    ]
    const cabe = rgb.every((v) => v >= -0.001 && v <= 1.001)
    if (cabe || cc < 0.001) {
      const g = (v: number) => {
        v = Math.max(0, Math.min(1, v))
        v = v <= 0.0031308 ? 12.92 * v : 1.055 * v ** (1 / 2.4) - 0.055
        return Math.round(v * 255).toString(16).padStart(2, '0')
      }
      const alfa = a < 1 ? Math.round(Math.max(0, a) * 255).toString(16).padStart(2, '0') : ''
      return '#' + rgb.map(g).join('') + alfa
    }
  }
}

/** A cor de destaque escolhida, pronta para o tema claro ou escuro. */
export function acento(id: string, claro: boolean) {
  const p = ACENTOS.find((x) => x.id === id)
  if (!p) return null
  // No claro, mais escuro para o destaque continuar legivel sobre branco.
  const l = claro ? 0.56 : 0.76
  const c = claro ? 0.17 : 0.15
  return {
    cor: oklchParaHex({ l, c, h: p.h, a: 1 }),
    hover: oklchParaHex({ l: l + (claro ? -0.05 : 0.06), c, h: p.h, a: 1 }),
    sobre: claro ? '#ffffff' : '#111111',
  }
}

/** Amostra para o botao da cor, igual nos dois temas. */
export function amostraAcento(id: string) {
  const p = ACENTOS.find((x) => x.id === id)
  return p ? oklchParaHex({ l: 0.7, c: 0.15, h: p.h, a: 1 }) : '#888888'
}

/** Amostra para o botao da paleta: o fundo dela num tema escuro. */
export function amostraPaleta(id: string) {
  const p = PALETAS.find((x) => x.id === id)
  return p ? oklchParaHex({ l: 0.3, c: p.c * 2.2, h: p.h, a: 1 }) : '#333333'
}

/** Leva um fundo do tema para o tom da paleta, com a mesma claridade e alfa. */
export function tingir(hex: string, paleta: string) {
  const p = PALETAS.find((x) => x.id === paleta)
  const o = hexParaOklch(hex)
  if (!p || !o) return hex
  // Perto do branco a mesma quantidade de cor parece muito mais forte: no
  // tema claro, menos da metade.
  const c = o.l > 0.8 ? p.c * 0.4 : p.c
  return oklchParaHex({ ...o, c, h: p.h })
}

/** Multiplica a opacidade da cor. */
export function vezesAlfa(hex: string, f: number) {
  const o = hexParaOklch(hex)
  if (!o) return hex
  const base = hex.length === 9 ? parseInt(hex.slice(7), 16) / 255 : 1
  return hex.slice(0, 7) + Math.round(Math.max(0, Math.min(1, base * f)) * 255).toString(16).padStart(2, '0')
}
