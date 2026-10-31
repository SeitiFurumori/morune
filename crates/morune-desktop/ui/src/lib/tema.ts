// O tema chega como dados (tokens do theme.toml ja resolvidos) e vira
// variaveis de CSS. Nada do tema roda como codigo.

function cor(c: string | undefined, padrao: string) {
  return typeof c === 'string' && c.startsWith('#') ? c : padrao
}

/** Mesma cor, sem transparencia: para quando o vidro do Windows nao esta ligado. */
function opaca(c: string) {
  return c.length === 9 ? c.slice(0, 7) : c
}

export function aplicarTema(t: any) {
  const s = t?.spec
  if (!s) return
  const r = document.documentElement.style
  const c = s.colors ?? {}
  const vidro = !!s.effects?.acrylic
  const set = (k: string, v: string | number) => r.setProperty(k, String(v))

  set('--fundo', vidro ? cor(c.background, '#0b0d12') : opaca(cor(c.background, '#0b0d12')))
  set('--superficie', cor(c.surface, '#12151d'))
  set('--superficie-alta', cor(c.surface_raised, '#1a1e29'))
  set('--lateral', cor(c.sidebar_background, '#0d1016'))
  set('--player', cor(c.player_background, '#0f1219'))
  set('--texto', cor(c.text, '#eceff4'))
  set('--texto-suave', cor(c.text_muted, '#939cb0'))
  set('--texto-no-acento', cor(c.text_on_accent, '#111111'))
  set('--acento-tema', cor(c.accent, '#8b63f6'))
  set('--acento-hover', cor(c.accent_hover, '#a383ff'))
  set('--borda', cor(c.border, '#ffffff22'))
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

  const m = s.motion ?? {}
  const parado = t.reduce_motion || m.enabled === false
  const vel = m.speed || 1
  set('--m-rapido', parado ? '0ms' : `${(m.duration_fast ?? 120) / vel}ms`)
  set('--m-normal', parado ? '0ms' : `${(m.duration_normal ?? 200) / vel}ms`)
  set('--m-lento', parado ? '0ms' : `${(m.duration_slow ?? 320) / vel}ms`)

  const e = s.effects ?? {}
  set('--vidro-desfoque', `${Math.max(0, t.vidro_desfoque ?? e.backdrop_blur ?? 0)}px`)
  document.documentElement.dataset.material = String(e.material ?? 'legacy').toLowerCase()
  document.documentElement.dataset.vidro = vidro ? '1' : '0'
  document.documentElement.dataset.parado = parado ? '1' : '0'

  set('--fundo-imagem', t.fundo ? `url("${t.fundo}")` : 'none')
  set('--fundo-opacidade', t.fundo_opacidade ?? 1)
  set('--fundo-desfoque', `${t.fundo_desfoque ?? 0}px`)
}
