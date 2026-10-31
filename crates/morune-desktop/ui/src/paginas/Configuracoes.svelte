<script lang="ts">
  import { acao, motor } from '../lib/motor.svelte'
  const e = $derived(motor.e)
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
  <div class="linha"><span>Imagem de fundo {e.background_name ? `(${e.background_name})` : ''}</span>
    <span class="grupo"><button class="botao" onclick={() => acao('choose-background')}>Escolher…</button>
      {#if e.has_background}<button class="botao" onclick={() => acao('clear-background')}>Tirar</button>{/if}</span></div>
  <label class="linha"><span>Tamanho do texto</span>
    <input type="range" min="0" max="1" step="0.01" value={e.font_scale} oninput={(ev) => acao('set-font-scale', Number(ev.currentTarget.value))} /></label>
  <label class="linha"><span>Reduzir movimento</span>
    <input type="checkbox" checked={e.reduce_motion} onchange={(ev) => acao('set-reduce-motion', ev.currentTarget.checked)} /></label>
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
  .previa { display: grid; grid-template-columns: 1fr 3fr; gap: 4px; height: 70px; padding: 6px; border-radius: var(--r-sm); }
  .previa span { border-radius: 4px; position: relative; }
  .previa i { position: absolute; left: 8px; bottom: 8px; width: 28px; height: 6px; border-radius: 3px; }
</style>
