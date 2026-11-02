// O tema chega como dados (tokens do theme.toml ja resolvidos) e vira
// variaveis de CSS. Nada do tema roda como codigo.
import { acento, tingir, vezesAlfa } from './cores'

function cor(c: string | undefined, padrao: string) {
  return typeof c === 'string' && c.startsWith('#') ? c : padrao
}

/** Mesma cor, sem transparencia: para quando o vidro do Windows nao esta ligado. */
function opaca(c: string) {
  return c.length === 9 ? c.slice(0, 7) : c
}

/**
 * As cores como estao na tela agora, ja com destaque, paleta e transparencia
 * aplicados. "Salvar como tema" manda isto: so a tela sabe fazer essas contas.
 */
export const coresNaTela: Record<string, string> = {}

/** Parametros da mola da troca de faixa, vindos do tema. */
export const mola = { rigidez: 200, atrito: 14, ligada: true }

/**
 * Faz `el` entrar com mola: sai de `x0` px e assenta em zero, passando um
 * pouco do ponto conforme o atrito. So `transform`, que roda na GPU.
 */
export function entrarComMola(el: HTMLElement, x0 = -24) {
  if (!mola.ligada || document.documentElement.dataset.quieta === '1') return
  let x = x0, v = 0, antes = performance.now()
  const passo = (agora: number) => {
    const dt = Math.min(0.032, (agora - antes) / 1000)
    antes = agora
    v += (-mola.rigidez * x - mola.atrito * v) * dt
    x += v * dt
    if (Math.abs(x) > 0.05 || Math.abs(v) > 0.05) {
      el.style.transform = `translateX(${x.toFixed(2)}px)`
      requestAnimationFrame(passo)
    } else el.style.transform = ''
  }
  requestAnimationFrame(passo)
}

/** Estouro curto ao curtir: reinicia a animacao `.estoura` do app.css. */
export function estourar(el: HTMLElement) {
  el.classList.remove('estoura')
  void el.offsetWidth
  el.classList.add('estoura')
}

export function aplicarTema(t: any) {
  const s = t?.spec
  if (!s) return
  const r = document.documentElement.style
  const c = s.color ?? {}
  // Com o acrilico do Windows ligado, o fundo do tema deixa ver o que esta
  // atras da janela. Sem ele (tema opaco, ou transparencia desligada no
  // Windows), o fundo vai solido -- senao apareceria o vazio da janela.
  const vidro = !!t.vidro_ativo
  const set = (k: string, v: string | number) => r.setProperty(k, String(v))
  const claro = s.manifest?.appearance === 'light'

  // O que a pessoa escolheu por cima do tema: paleta muda o tom dos fundos,
  // paineis multiplica a opacidade deles, janela da a opacidade do fundo.
  const tom = (hex: string) => (t.paleta ? tingir(hex, t.paleta) : hex)
  const painel = (hex: string) => (t.paineis > 0 ? vezesAlfa(tom(hex), t.paineis) : tom(hex))
  let fundo = tom(cor(c.background, '#0b0d12'))
  if (t.janela_opacidade > 0) fundo = vezesAlfa(opaca(fundo), t.janela_opacidade)
  set('--fundo', vidro ? fundo : opaca(fundo))
  // Texto em cima de um botao cheio com a cor do texto: a cor do fundo, mas
  // sempre solida (no Vidro o fundo e translucido e o texto sumiria).
  set('--sobre-texto', opaca(fundo))
  coresNaTela.background = fundo
  coresNaTela.surface = painel(cor(c.surface, '#12151d'))
  coresNaTela.surface_raised = tom(cor(c.surface_raised, '#1a1e29'))
  coresNaTela.sidebar_background = painel(cor(c.sidebar_background, '#0d1016'))
  coresNaTela.player_background = painel(cor(c.player_background, '#0f1219'))
  set('--superficie', coresNaTela.surface)
  set('--superficie-alta', coresNaTela.surface_raised)
  set('--lateral', coresNaTela.sidebar_background)
  set('--player', coresNaTela.player_background)
  set('--texto', cor(c.text, '#eceff4'))
  set('--texto-suave', cor(c.text_muted, '#939cb0'))
  const escolhido = t.acento ? acento(t.acento, claro) : null
  set('--texto-no-acento', escolhido?.sobre ?? cor(c.text_on_accent, '#111111'))
  set('--acento-tema', escolhido?.cor ?? cor(c.accent, '#8b63f6'))
  set('--acento-hover', escolhido?.hover ?? cor(c.accent_hover, '#a383ff'))
  coresNaTela.accent = escolhido?.cor ?? cor(c.accent, '#8b63f6')
  coresNaTela.accent_hover = escolhido?.hover ?? cor(c.accent_hover, '#a383ff')
  coresNaTela.text_on_accent = escolhido?.sobre ?? cor(c.text_on_accent, '#111111')
  // Cor escolhida vence a cor viva da capa (App.svelte).
  document.documentElement.dataset.acentoFixo = escolhido || t.livre?.cores?.accent ? '1' : '0'
  set('--borda', cor(c.border, '#ffffff22'))
  set('--borda-destaque', cor(c.border_highlight, cor(c.border, '#ffffff22')))
  r.colorScheme = s.manifest?.appearance === 'light' ? 'light' : 'dark'
  set('--hover', cor(c.hover, '#ffffff0f'))
  set('--selecionado', cor(c.selected, '#ffffff18'))
  set('--foco', cor(c.focus_ring, '#ffffff'))
  set('--perigo', cor(c.danger, '#ff7b7b'))
  set('--sucesso', cor(c.success, '#5ed6a4'))

  const ty = s.typography ?? {}
  const escala = t.font_scale > 0 ? t.font_scale : ty.scale || 1
  set('--fonte', `"${ty.family || 'Segoe UI'}", "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif`)
  set('--fonte-titulo', `"${ty.display_family || ty.family || 'Segoe UI'}", "Segoe UI Variable Display", "Segoe UI", system-ui, sans-serif`)
  for (const k of ['xs', 'sm', 'md', 'lg', 'xl', 'display']) {
    const v = ty[`size_${k}`]
    if (v) set(`--t-${k}`, `${v * escala}px`)
  }
  set('--peso-normal', ty.weight_normal || 400)
  set('--peso-medio', ty.weight_medium || 600)
  set('--peso-forte', ty.weight_bold || 700)
  set('--altura-linha', ty.line_height || 1.45)
  set('--espaco-letra', `${ty.letter_spacing || 0}px`)

  const sh = s.shape ?? {}
  set('--r-sm', `${sh.radius_sm ?? 8}px`)
  set('--r-md', `${sh.radius_md ?? 12}px`)
  set('--r-lg', `${sh.radius_lg ?? 18}px`)
  set('--r-capa', `${sh.radius_artwork ?? 8}px`)
  set('--e-xs', `${sh.spacing_xs ?? 4}px`)
  set('--e-sm', `${sh.spacing_sm ?? 8}px`)
  set('--e-md', `${sh.spacing_md ?? 14}px`)
  set('--e-lg', `${sh.spacing_lg ?? 22}px`)
  set('--e-xl', `${sh.spacing_xl ?? 34}px`)
  // Forma: o mesmo app com outro desenho, nao so outra cor.
  set('--borda-painel', `${sh.border_width ?? 0}px`)
  set('--trilho', `${sh.progress_thickness ?? 4}px`)
  const ct = s.control ?? {}
  set('--r-botao', `${Math.min(ct.button_radius ?? 999, 999)}px`)

  const m = s.motion ?? {}
  const parado = t.reduce_motion || m.enabled === false
  const vel = m.speed || 1
  set('--m-rapido', parado ? '0ms' : `${(m.duration_fast ?? 120) / vel}ms`)
  set('--m-normal', parado ? '0ms' : `${(m.duration_normal ?? 200) / vel}ms`)
  set('--m-lento', parado ? '0ms' : `${(m.duration_slow ?? 320) / vel}ms`)
  // Mola da troca de faixa, lida pelo player. Parado = sem mola.
  mola.rigidez = m.spring_stiffness || 200
  mola.atrito = m.spring_damping || 14
  mola.ligada = !parado

  const e = s.effects ?? {}
  // A capa manda: o brilho em volta dela e o fundo tirado dela.
  set('--brilho-capa', Math.max(0, Math.min(1, e.artwork_glow ?? 0.45)))
  set('--fundo-capa', e.artwork_tint ? Math.max(0, Math.min(1, e.artwork_tint_strength ?? 0)) : 0)
  set('--vidro-desfoque', `${Math.max(0, t.vidro_desfoque ?? e.backdrop_blur ?? 0)}px`)
  document.documentElement.dataset.material = String(e.material ?? 'legacy').toLowerCase()
  document.documentElement.dataset.vidro = vidro ? '1' : '0'
  document.documentElement.dataset.parado = parado ? '1' : '0'
  document.documentElement.dataset.capaPinta = e.artwork_paints_ui ? '1' : '0'

  // GIF e video vao num elemento proprio (Fundo.svelte), que para com jogo.
  set('--fundo-imagem', t.fundo && t.fundo_tipo === 'imagem' ? `url("${t.fundo}")` : 'none')
  set('--fundo-ajuste', ['cover', 'contain', 'auto', '100% 100%'][t.fundo_ajuste] ?? 'cover')
  set('--fundo-encaixe', ['cover', 'contain', 'none', 'fill'][t.fundo_ajuste] ?? 'cover')
  set('--fundo-opacidade', t.fundo_opacidade ?? 1)
  set('--fundo-desfoque', `${t.fundo_desfoque ?? 0}px`)
}
