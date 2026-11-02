<script lang="ts">
  import { acao, motor } from '../lib/motor.svelte'
  import { ACENTOS, PALETAS, amostraAcento, amostraPaleta } from '../lib/cores'
  import { coresNaTela } from '../lib/tema'
  const e = $derived(motor.e)
  const t = $derived(motor.tema ?? {})
  // Video de fundo acima de 1440p: a placa de video paga por cada quadro, e
  // quem sente o PC pesado deve saber que e o video, nao o Morune.
  const videoPesado = $derived.by(() => {
    const v = motor.fundoVideo
    return t.fundo_tipo === 'video' && v.src === t.fundo && v.w * v.h > 2560 * 1440 ? v : null
  })
  // `paineis` guardado e a opacidade (0 = a do tema); o controle mostra
  // "quanta transparencia", de 0 a 1.
  const paineis = $derived(t.paineis > 0 ? (1 - t.paineis) / 0.8 : 0)
  const mexeu = $derived(!!(t.acento || t.paleta || t.paineis > 0 || t.janela_opacidade > 0))
  // Modo liberdade total.
  const livre = $derived(t.livre ?? { cores: {} })
  const temaAtivo = $derived((e.themes ?? []).find((x: any) => x.active))
  const CORES = [
    ['background', 'Fundo da janela'], ['surface', 'Painel principal'], ['sidebar_background', 'Lateral'],
    ['player_background', 'Player'], ['surface_raised', 'Cartões e menus'], ['accent', 'Destaque'],
    ['text', 'Texto'], ['text_muted', 'Texto secundário'], ['border', 'Bordas'],
  ]
  // [chave, rotulo, min, max, passo, valor do tema, unidade]
  const NUMEROS = $derived([
    ['cantos', 'Cantos arredondados', 0, 32, 1, t.spec?.shape?.radius_lg ?? 14, 'px'],
    ['espaco', 'Espaçamento', 0.5, 1.6, 0.05, 1, '×'],
    ['borda', 'Borda dos painéis', 0, 4, 0.5, t.spec?.shape?.border_width ?? 0, 'px'],
    ['velocidade', 'Velocidade das animações', 0.4, 2.5, 0.1, 1, '×'],
    ['brilho_capa', 'Brilho em volta da capa', 0, 1, 0.05, t.spec?.effects?.artwork_glow ?? 0.45, ''],
    ['fundo_capa', 'Fundo tingido pela capa', 0, 1, 0.05, t.spec?.effects?.artwork_tint_strength ?? 0, ''],
  ] as [string, string, number, number, number, number, string][])
  const corAtual = (k: string) => livre.cores?.[k] ?? String(t.spec?.color?.[k] ?? '#000000').slice(0, 7)
  const AJUSTES = ['Cobrir a janela', 'Caber inteiro', 'Tamanho original', 'Esticar']
  let nomeTema = $state('')
  function salvarTema() {
    acao('salvar-tema', nomeTema, { ...coresNaTela })
    nomeTema = ''
  }
  const qualidades = ['Normal (96 kbps)', 'Alta (160 kbps)', 'Muito alta (320 kbps)']
  const sono = [
    { m: 0, t: 'Desligado' }, { m: 15, t: '15 min' }, { m: 30, t: '30 min' }, { m: 60, t: '1 hora' }, { m: -1, t: 'No fim da faixa' },
  ]
</script>

<h1 class="titulo-pagina">Configurações</h1>

<section class="modulo">
  <h2 class="secao-titulo">Conta</h2>
  {#if e.logged_in}
    <div class="linha"><span>Conectado como <b>{e.account_name}</b></span><button class="botao" onclick={() => acao('logout')}>Sair</button></div>
  {:else}
    <div class="linha"><span class="suave">Você não está conectado.</span><button class="botao primario" onclick={() => acao('login')}>Entrar no Spotify</button></div>
  {/if}
</section>

<section class="modulo">
  <h2 class="secao-titulo">Aparência</h2>
  <div class="temas">
    {#each e.themes ?? [] as t (t.id)}
      <button class="tema" class:ativo={t.active} onclick={() => acao('select-theme', t.id)}>
        <div class="previa" style:background={t.preview_background}>
          <span style:background={t.preview_sidebar}></span>
          <span style:background={t.preview_surface}><i style:background={t.preview_accent}></i></span>
        </div>
        <div class="corta">{t.name}</div>
      </button>
    {/each}
  </div>
  <div class="bloco">
    <span>Cor de destaque</span>
    <div class="amostras" role="radiogroup" aria-label="Cor de destaque">
      <button class="amostra tema-cor" class:ativo={!t.acento} role="radio" aria-checked={!t.acento} title="A do tema" onclick={() => acao('set-accent', '')}><span>Aa</span></button>
      {#each ACENTOS as a (a.id)}
        <button class="amostra" class:ativo={t.acento === a.id} role="radio" aria-checked={t.acento === a.id} title={a.nome}
          style:background={amostraAcento(a.id)} onclick={() => acao('set-accent', a.id)}></button>
      {/each}
    </div>
  </div>
  <div class="bloco">
    <span>Paleta</span>
    <div class="paletas" role="radiogroup" aria-label="Paleta">
      <button class="paleta" class:ativo={!t.paleta} role="radio" aria-checked={!t.paleta} onclick={() => acao('set-palette', '')}>
        <i class="tema-cor"></i>Do tema</button>
      {#each PALETAS as p (p.id)}
        <button class="paleta" class:ativo={t.paleta === p.id} role="radio" aria-checked={t.paleta === p.id} onclick={() => acao('set-palette', p.id)}>
          <i style:background={amostraPaleta(p.id)}></i>{p.nome}</button>
      {/each}
    </div>
  </div>
  <label class="linha"><span>Transparência da janela<small class="suave">Deixa ver a área de trabalho atrás, desfocada.</small></span>
    <input type="range" min="0" max="1" step="0.01" value={e.window_opacity} oninput={(ev) => acao('set-window-opacity', Number(ev.currentTarget.value))} /></label>
  <label class="linha"><span>Transparência dos painéis<small class="suave">Deixa ver a imagem de fundo e o que está atrás.</small></span>
    <input type="range" min="0" max="1" step="0.01" value={paineis} oninput={(ev) => acao('set-panel-opacity', Number(ev.currentTarget.value))} /></label>
  {#if mexeu}
    <div class="linha"><span class="suave">Cor e transparência fora do padrão do tema.</span>
      <button class="botao" onclick={() => acao('reset-appearance')}>Voltar ao tema</button></div>
  {/if}
  <div class="linha"><span>Fundo {e.background_name ? `(${e.background_name})` : ''}<small class="suave">Imagem, GIF ou vídeo. O que se mexe para com jogo em tela cheia.</small></span>
    <span class="grupo"><button class="botao" onclick={() => acao('choose-background')}>Escolher…</button>
      {#if e.has_background}<button class="botao" onclick={() => acao('clear-background')}>Tirar</button>{/if}</span></div>
  {#if videoPesado}
    <div class="aviso-video">
      <b>Este vídeo é {videoPesado.h >= 2160 ? '4K' : 'maior que 1440p'} ({videoPesado.w}×{videoPesado.h}).</b>
      A placa de vídeo trabalha a cada quadro enquanto ele toca, e o PC pode ficar mais pesado por causa do vídeo, não do Morune.
      A mesma cena em 1080p fica quase igual na janela e gasta bem menos. Com jogo em tela cheia, ele para sozinho.
    </div>
  {/if}
  <label class="linha"><span>Tamanho do texto</span>
    <input type="range" min="0" max="1" step="0.01" value={e.font_scale} oninput={(ev) => acao('set-font-scale', Number(ev.currentTarget.value))} /></label>
  <label class="linha"><span>Reduzir movimento</span>
    <input type="checkbox" checked={e.reduce_motion} onchange={(ev) => acao('set-reduce-motion', ev.currentTarget.checked)} /></label>
</section>

<section class="modulo">
  <details class="livre">
    <summary><span><b>Liberdade total</b><small class="suave">Cada cor, forma, fonte e movimento. Para quem quer mexer em tudo.</small></span></summary>
    <div class="livre-corpo">
      <h3>Fundo</h3>
      {#if e.has_background}
        <label class="linha"><span>Encaixe</span>
          <select value={e.background_fit} onchange={(ev) => acao('set-background-fit', Number(ev.currentTarget.value))}>
            {#each AJUSTES as a, i}<option value={i}>{a}</option>{/each}</select></label>
        <label class="linha"><span>Opacidade do fundo</span>
          <input type="range" min="0" max="1" step="0.01" value={e.background_opacity} oninput={(ev) => acao('set-background-opacity', Number(ev.currentTarget.value))} /></label>
        <label class="linha"><span>Desfoque do fundo<small class="suave">Num vídeo, desfocar custa a cada quadro.</small></span>
          <input type="range" min="0" max="1" step="0.01" value={e.background_blur} oninput={(ev) => acao('set-background-blur', Number(ev.currentTarget.value))} /></label>
      {:else}
        <div class="linha"><span class="suave">Escolha um fundo em Aparência, logo acima, para ajustar aqui.</span></div>
      {/if}

      <h3>Cores</h3>
      <div class="cores">
        {#each CORES as [k, rotulo] (k)}
          <label class="cor">
            <input type="color" value={corAtual(k)} onchange={(ev) => acao('set-livre-cor', k, ev.currentTarget.value)} />
            <span>{rotulo}</span>
            {#if livre.cores?.[k]}
              <button class="volta" title="Voltar à do tema" onclick={(ev) => { ev.preventDefault(); acao('set-livre-cor', k, '') }}>Do tema</button>
            {/if}
          </label>
        {/each}
      </div>

      <h3>Forma e movimento</h3>
      {#each NUMEROS as [k, rotulo, min, max, passo, padrao, unidade] (k)}
        {@const valor = livre[k] ?? padrao}
        <label class="linha"><span>{rotulo} <span class="suave">{unidade === '×' ? `${Number(valor).toFixed(2)}×` : unidade === 'px' ? `${valor} px` : `${Math.round(valor * 100)}%`}</span></span>
          <span class="grupo">
            {#if livre[k] != null}<button class="volta" onclick={(ev) => { ev.preventDefault(); acao('set-livre-numero', k, null) }}>Do tema</button>{/if}
            <input type="range" {min} {max} step={passo} value={valor} oninput={(ev) => acao('set-livre-numero', k, Number(ev.currentTarget.value))} />
          </span></label>
      {/each}

      <h3>Fonte</h3>
      <label class="linha"><span>Texto</span>
        <select value={livre.fonte ?? ''} onchange={(ev) => acao('set-livre-fonte', false, ev.currentTarget.value)}>
          <option value="">Do tema</option>
          {#each t.fontes ?? [] as f}<option value={f} style:font-family={f}>{f}</option>{/each}</select></label>
      <label class="linha"><span>Títulos</span>
        <select value={livre.fonte_titulo ?? ''} onchange={(ev) => acao('set-livre-fonte', true, ev.currentTarget.value)}>
          <option value="">A mesma do texto</option>
          {#each t.fontes ?? [] as f}<option value={f} style:font-family={f}>{f}</option>{/each}</select></label>

      <h3>Guardar e compartilhar</h3>
      <div class="linha"><span>Salvar tudo como um tema novo<small class="suave">Entra na lista de temas, com o fundo junto.</small></span>
        <span class="grupo">
          <input class="nome" type="text" placeholder="Nome do tema" maxlength="40" bind:value={nomeTema}
            onkeydown={(ev) => { if (ev.key === 'Enter') salvarTema() }} />
          <button class="botao primario" onclick={salvarTema}>Salvar</button>
        </span></div>
      <div class="linha"><span>Pacote de tema<small class="suave">Um arquivo para mandar para alguém ou trazer de outro PC.</small></span>
        <span class="grupo">
          {#if temaAtivo}<button class="botao" onclick={() => acao('export-theme', temaAtivo.id)}>Exportar “{temaAtivo.name}”…</button>{/if}
          <button class="botao" onclick={() => acao('import-theme')}>Importar…</button>
        </span></div>
      {#if Object.keys(livre.cores ?? {}).length || NUMEROS.some(([k]) => livre[k] != null) || livre.fonte || livre.fonte_titulo}
        <div class="linha"><span class="suave">Há ajustes do modo livre por cima do tema.</span>
          <button class="botao" onclick={() => acao('reset-livre')}>Desfazer todos</button></div>
      {/if}
    </div>
  </details>
</section>

<section class="modulo">
  <h2 class="secao-titulo">Reprodução</h2>
  <label class="linha"><span>Qualidade do áudio</span>
    <select value={e.bitrate} onchange={(ev) => acao('set-bitrate', Number(ev.currentTarget.value))}>
      {#each qualidades as q, i}<option value={i}>{q}</option>{/each}</select></label>
  <label class="linha"><span>Saída de áudio</span>
    <select value={e.output_device} onchange={(ev) => acao('set-output-device', ev.currentTarget.value)}>
      <option value="">Padrão do sistema</option>
      {#each e.output_devices ?? [] as d}<option value={d}>{d}</option>{/each}</select></label>
  <label class="linha"><span>Nivelar volume entre faixas</span>
    <input type="checkbox" checked={e.normalize} onchange={(ev) => acao('set-normalize', ev.currentTarget.checked)} /></label>
  <label class="linha"><span>Áudio mono</span>
    <input type="checkbox" checked={e.mono} onchange={(ev) => acao('set-mono', ev.currentTarget.checked)} /></label>
  <label class="linha"><span>Continuar tocando músicas parecidas quando a fila acabar</span>
    <input type="checkbox" checked={e.autoplay} onchange={(ev) => acao('set-autoplay', ev.currentTarget.checked)} /></label>
  <label class="linha"><span>Timer para dormir</span>
    <select onchange={(ev) => acao('set-sleep', Number(ev.currentTarget.value))}>
      {#each sono as s}<option value={s.m}>{s.t}</option>{/each}</select></label>
  <div class="linha"><span>Cache de áudio</span><button class="botao" onclick={() => acao('clear-cache')}>Limpar</button></div>
</section>

<section class="modulo">
  <h2 class="secao-titulo">Sistema</h2>
  <label class="linha"><span>Fechar manda para a bandeja</span>
    <input type="checkbox" checked={e.close_to_tray} onchange={(ev) => acao('set-close-to-tray', ev.currentTarget.checked)} /></label>
  <label class="linha"><span>Iniciar com o Windows</span>
    <input type="checkbox" checked={e.start_with_windows} onchange={(ev) => acao('set-start-with-windows', ev.currentTarget.checked)} /></label>
  <label class="linha"><span>Aviso do Windows ao trocar de faixa</span>
    <input type="checkbox" checked={e.track_notifications} onchange={(ev) => acao('set-track-notifications', ev.currentTarget.checked)} /></label>
  <div class="linha"><span>{e.update_title || 'Atualizações'} <span class="suave">{e.update_detail}</span></span>
    <button class="botao" onclick={() => acao('check-update')}>Procurar</button></div>
</section>

<style>
  .modulo { margin-top: var(--e-xl); max-width: 760px; display: grid; gap: var(--e-xs); }
  .modulo h2 { margin-bottom: var(--e-sm); }
  .linha { display: flex; align-items: center; justify-content: space-between; gap: var(--e-md); min-height: 48px; padding: 0 var(--e-md); border-radius: var(--r-md); background: var(--hover); }
  .grupo { display: flex; gap: var(--e-sm); }
  select { height: 34px; padding: 0 var(--e-sm); border-radius: var(--r-sm); background: var(--selecionado); color: var(--texto); border: 0; font: inherit; font-size: var(--t-sm); }
  input[type='checkbox'] { width: 18px; height: 18px; accent-color: var(--acento); }
  input[type='range'] { width: 180px; accent-color: var(--acento); }
  .temas { display: grid; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); gap: var(--e-sm); margin-bottom: var(--e-sm); }
  .tema { display: grid; gap: 6px; padding: 6px; border-radius: var(--r-md); text-align: left; font-size: var(--t-sm); outline: 2px solid transparent; }
  .tema.ativo { outline-color: var(--acento); }
  .linha small { display: block; font-size: var(--t-xs, 12px); }
  .linha > span:has(small) { padding: var(--e-sm) 0; }
  .bloco { display: grid; gap: var(--e-sm); padding: var(--e-md); border-radius: var(--r-md); background: var(--hover); }
  .amostras { display: flex; flex-wrap: wrap; gap: var(--e-sm); }
  .amostra { width: 34px; height: 34px; border-radius: 99px; outline: 2px solid transparent; outline-offset: 3px; display: grid; place-items: center; }
  .amostra.ativo, .paleta.ativo { outline-color: var(--texto); }
  .tema-cor { background: var(--acento-tema); color: var(--texto-no-acento); font-size: 11px; font-weight: var(--peso-forte); }
  .paletas { display: flex; flex-wrap: wrap; gap: var(--e-sm); }
  .paleta { display: inline-flex; align-items: center; gap: 8px; height: 34px; padding: 0 14px 0 6px; border-radius: 99px; background: var(--selecionado); font-size: var(--t-sm); outline: 2px solid transparent; outline-offset: 2px; }
  .paleta i { width: 22px; height: 22px; border-radius: 99px; box-shadow: inset 0 0 0 1px #ffffff26; }
  .paleta i.tema-cor { background: var(--fundo); }
  .aviso-video { padding: var(--e-sm) var(--e-md); border-radius: var(--r-md); background: color-mix(in srgb, var(--acento) 12%, transparent); font-size: var(--t-sm); color: var(--texto-suave); line-height: 1.5; }
  .aviso-video b { display: block; color: var(--texto); font-weight: var(--peso-forte); }
  .livre { border-radius: var(--r-md); background: var(--hover); }
  .livre summary { display: flex; align-items: center; min-height: 56px; padding: var(--e-sm) var(--e-md); cursor: pointer; list-style: none; }
  .livre summary::-webkit-details-marker { display: none; }
  .livre summary::after { content: ''; margin-left: auto; width: 8px; height: 8px; border-right: 2px solid var(--texto-suave); border-bottom: 2px solid var(--texto-suave); transform: rotate(45deg); transition: transform var(--m-normal); }
  .livre[open] summary::after { transform: rotate(225deg); }
  .livre summary small { display: block; font-size: var(--t-xs, 12px); }
  .livre-corpo { display: grid; gap: var(--e-xs); padding: 0 var(--e-sm) var(--e-md); }
  .livre-corpo .linha { background: var(--selecionado); }
  .livre h3 { margin: var(--e-lg) var(--e-sm) var(--e-xs); font-size: var(--t-sm); font-weight: var(--peso-forte); color: var(--texto-suave); text-transform: uppercase; letter-spacing: 0.06em; }
  .livre h3:first-child { margin-top: 0; }
  .cores { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: var(--e-xs); }
  .cor { display: flex; align-items: center; gap: var(--e-sm); min-height: 48px; padding: 0 var(--e-md); border-radius: var(--r-md); background: var(--selecionado); font-size: var(--t-sm); }
  .cor input { width: 30px; height: 30px; padding: 0; border: 0; border-radius: 99px; background: none; cursor: pointer; flex: none; }
  .cor input::-webkit-color-swatch-wrapper { padding: 0; }
  .cor input::-webkit-color-swatch { border: 1px solid var(--borda); border-radius: 99px; }
  .cor span { flex: 1; }
  .volta { font-size: var(--t-xs, 12px); color: var(--texto-suave); padding: 4px 8px; border-radius: var(--r-sm); }
  .volta:hover { color: var(--texto); background: var(--hover); }
  .nome { height: 34px; width: 180px; padding: 0 var(--e-sm); border-radius: var(--r-sm); background: var(--selecionado); color: var(--texto); border: 0; font: inherit; font-size: var(--t-sm); }
  .previa { display: grid; grid-template-columns: 1fr 3fr; gap: 4px; height: 70px; padding: 6px; border-radius: var(--r-sm); }
  .previa span { border-radius: 4px; position: relative; }
  .previa i { position: absolute; left: 8px; bottom: 8px; width: 28px; height: 6px; border-radius: 3px; }
</style>
