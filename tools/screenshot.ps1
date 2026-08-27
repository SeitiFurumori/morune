# Captura a janela do Morune para inspecao visual.
#
# **Para conferir tema, prefira `tools/snapshot.ps1`.** Ele pede o quadro ao
# proprio renderizador do Slint e nao depende de nada que esteja na tela. Este
# script aqui le a janela pelo sistema, e isso tem um limite que nao da para
# contornar: uma janela translucida deixa passar o que estiver atras dela, e o
# que chega ao arquivo e a composicao, nao a interface. Com outra janela por
# tras, a captura sai com o conteudo dela -- ja aconteceu, e as verificacoes
# aqui reduzem o risco sem elimina-lo.
#
# Este script continua util para o que o snapshot nao alcanca: o vidro real,
# como o DWM o compoe, e o comportamento da janela no ambiente de verdade.
#
# Existe porque testes automatizados provam que o tema carrega, mas nao provam
# que a tela ficou legivel. Uma captura por tema e a verificacao mais barata
# disso.
#
# Captura apenas o retangulo da janela do aplicativo, nunca a tela inteira: o
# resto do desktop e assunto de quem esta usando a maquina.
#
# Usa PrintWindow, que pede o conteudo a propria janela, em vez de copiar
# pixels da tela. A versao anterior copiava da tela depois de chamar
# SetForegroundWindow -- e o Windows recusa esse pedido vindo de um processo em
# segundo plano, entao a captura saia com a janela que estivesse por cima,
# parecendo ter dado certo. Uma captura errada silenciosa e pior que nenhuma,
# e por isso o resultado tambem e conferido antes de virar arquivo.
#
# Uso: . .\tools\env.ps1 ; .\tools\screenshot.ps1 -Theme paper -Out .\bench-out\paper.png

param(
    [string]$Theme = "",
    [string]$Out = "$PSScriptRoot\..\bench-out\morune.png",
    [int]$WarmupSeconds = 4,
    [string]$Exe = "$PSScriptRoot\..\target\release\morune.exe",
    [switch]$Force
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Runtime.InteropServices;
public struct RECT { public int Left, Top, Right, Bottom; }
public class Win32 {
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdc, uint flags);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT r);
    [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr hWnd, int attr, out RECT r, int size);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, IntPtr pid);
    [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint from, uint to, bool attach);
    [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
}
"@

if (-not (Test-Path $Exe)) { Write-Error "Executavel nao encontrado: $Exe" }

# Uma captura so vale se tiver interface desenhada dentro.
#
# O criterio anterior era "algum pixel nao e preto", e ele deixou passar uma
# janela cinza chapada -- que e o que o DWM devolve para uma janela com
# acrilico que esta atras de outra. Cor unica nunca e a interface do Morune:
# ha barra lateral, painel, texto e capas, sempre mais de um tom.
function Test-TemInterface {
    param([System.Drawing.Bitmap]$Imagem, [int]$Largura, [int]$Altura)

    $tons = @{}
    for ($x = 0; $x -lt $Largura; $x += 20) {
        for ($y = 0; $y -lt $Altura; $y += 20) {
            $p = $Imagem.GetPixel($x, $y)
            # Agrupa em faixas de 8 para nao contar ruido de antialiasing como
            # variedade.
            $tons["$([math]::Floor($p.R / 8)),$([math]::Floor($p.G / 8)),$([math]::Floor($p.B / 8))"] = $true
        }
    }
    return $tons.Count -ge 8
}

# Comparacao de HWND por valor. `-eq` entre dois [IntPtr] no PowerShell 5.1
# compara os objetos, nao os enderecos: a verificacao de primeiro plano passava
# sempre, e o script salvou a janela de outro programa como se fosse a captura
# boa -- o defeito antigo, de volta por outra porta.
function Test-MesmoHwnd {
    param([IntPtr]$A, [IntPtr]$B)
    return $A.ToInt64() -eq $B.ToInt64()
}

# O Morune e instancia unica: uma segunda abertura devolve o foco a janela que
# ja existe e sai com codigo 0. Sem esta checagem o script capturava esse
# codigo 0 e dizia "o aplicativo saiu antes da captura", que acusa um defeito
# do aplicativo em vez de apontar a instancia aberta -- e nenhum tema era
# capturado. Fechar para a bandeja mantem o processo vivo, entao a janela nao
# estar visivel nao quer dizer que nao ha instancia.
$existente = @(Get-Process -Name morune -ErrorAction SilentlyContinue)
if ($existente.Count -gt 0) {
    $pids = ($existente | ForEach-Object { $_.Id }) -join ", "
    if (-not $Force) {
        Write-Error "Ja ha uma instancia do Morune rodando (PID $pids). Ela impede a captura: a segunda abertura apenas devolve o foco a primeira. Feche-a pela bandeja ou rode com -Force."
    }
    foreach ($p in $existente) {
        # CloseMainWindow so manda o aplicativo para a bandeja; aqui o processo
        # precisa terminar mesmo, para liberar o mutex de instancia unica.
        #
        # `Kill` devolve "Acesso negado" quando o processo ja esta terminando --
        # e o caso de uma captura anterior nesta mesma rodada. Esperar a saida
        # cobre os dois casos sem transformar um processo moribundo em erro.
        try { $p.Kill() } catch { }
        $p.WaitForExit(5000) | Out-Null
    }
    Start-Sleep -Milliseconds 500
}

$outDir = Split-Path -Parent $Out
if (-not (Test-Path $outDir)) { New-Item -ItemType Directory -Force $outDir | Out-Null }

# Troca o tema ativo direto na configuracao. Mais confiavel que automatizar
# cliques, e exercita exatamente o caminho que o aplicativo usa ao abrir.
$configPath = Join-Path $env:APPDATA "morune\Morune\config\config.toml"
$backup = $null
if ($Theme -ne "") {
    if (Test-Path $configPath) {
        $backup = Get-Content $configPath -Raw
        $new = $backup -replace '(?m)^theme\s*=\s*".*"$', "theme = `"$Theme`""
        if ($new -eq $backup) { $new = $backup + "`n[appearance]`ntheme = `"$Theme`"`n" }
        Set-Content $configPath $new -Encoding utf8
    } else {
        New-Item -ItemType Directory -Force (Split-Path -Parent $configPath) | Out-Null
        Set-Content $configPath "version = 1`n[appearance]`ntheme = `"$Theme`"`n" -Encoding utf8
    }
}

$proc = Start-Process -FilePath $Exe -PassThru
try {
    Start-Sleep -Seconds $WarmupSeconds
    $proc.Refresh()
    if ($proc.HasExited) { Write-Error "O aplicativo saiu antes da captura (codigo $($proc.ExitCode))." }
    if ($proc.MainWindowHandle -eq 0) { Write-Error "A janela principal nao apareceu." }

    $hwnd = $proc.MainWindowHandle
    [Win32]::ShowWindow($hwnd, 5) | Out-Null   # SW_SHOW
    # Nao pedimos o foco: PrintWindow le a janela mesmo coberta, e roubar o
    # foco de quem esta usando a maquina seria justamente o que este produto
    # promete nao fazer.
    Start-Sleep -Milliseconds 1500

    # DWMWA_EXTENDED_FRAME_BOUNDS: GetWindowRect inclui a borda invisivel de
    # redimensionamento no Windows 10/11 e capturaria uma faixa do que estiver
    # atras da janela.
    $r = New-Object RECT
    $size = [System.Runtime.InteropServices.Marshal]::SizeOf($r)
    if ([Win32]::DwmGetWindowAttribute($hwnd, 9, [ref]$r, $size) -ne 0) {
        [Win32]::GetWindowRect($hwnd, [ref]$r) | Out-Null
    }

    $w = $r.Right - $r.Left
    $h = $r.Bottom - $r.Top
    if ($w -le 0 -or $h -le 0) { Write-Error "Retangulo de janela invalido: ${w}x${h}" }

    # A janela existe antes de ter desenhado o primeiro quadro, e nesse
    # intervalo o PrintWindow devolve um retangulo todo preto. O tempo ate o
    # primeiro quadro varia com o tema e com o que a maquina esta fazendo, entao
    # nao da para resolver com uma espera fixa: tentamos ate vir conteudo.
    # Sem isso, o arquivo vazio passa por captura boa -- o defeito que este
    # script ja teve, em outra forma.
    $bmp = $null
    $tentativas = 12
    for ($i = 1; $i -le $tentativas; $i++) {
        if ($null -ne $bmp) { $bmp.Dispose() }
        $bmp = New-Object System.Drawing.Bitmap $w, $h
        $gfx = [System.Drawing.Graphics]::FromImage($bmp)
        $hdc = $gfx.GetHdc()
        # PW_RENDERFULLCONTENT (2): sem esta flag, janelas desenhadas pela GPU
        # voltam vazias.
        $ok = [Win32]::PrintWindow($hwnd, $hdc, 2)
        $gfx.ReleaseHdc($hdc)
        $gfx.Dispose()
        if (-not $ok) { $bmp.Dispose(); Write-Error "PrintWindow falhou para a janela do aplicativo." }

        if (Test-TemInterface $bmp $w $h) { break }
        if ($i -eq $tentativas) { $bmp.Dispose(); $bmp = $null; break }
        Start-Sleep -Milliseconds 700
    }

# Janela com acrilico volta preta no PrintWindow por definicao, nao por corrida
# de tempo: o vidro e composto pelo DWM a partir do que esta atras da janela, e
# PrintWindow pede o desenho so a janela, que ali nao existe. Bruma e Cristal
# caem sempre neste caso. A saida e ler os pixels da tela -- que foi a origem
# do defeito antigo deste script, entao a diferenca aqui e a verificacao: so
# copiamos depois de confirmar que a janela do Morune e mesmo a de primeiro
# plano. Sem essa confirmacao, preferimos falhar a salvar a janela errada.
    if ($null -eq $bmp) {
        # SetForegroundWindow e recusado a processo em segundo plano, a menos
        # que ele compartilhe a fila de entrada da janela que tem o foco.
        # AttachThreadInput cria esse vinculo pelo tempo do pedido.
        # SetForegroundWindow e recusado a processo em segundo plano, a menos
        # que ele compartilhe a fila de entrada da janela que tem o foco.
        # AttachThreadInput cria esse vinculo pelo tempo do pedido. O Windows
        # ainda recusa em varias situacoes, dai a insistencia.
        $veio = $false
        for ($tentativa = 1; $tentativa -le 5 -and -not $veio; $tentativa++) {
            $meu = [Win32]::GetCurrentThreadId()
            $dono = [Win32]::GetWindowThreadProcessId([Win32]::GetForegroundWindow(), [IntPtr]::Zero)
            $atou = $false
            if ($dono -ne 0 -and $dono -ne $meu) { $atou = [Win32]::AttachThreadInput($meu, $dono, $true) }
            [Win32]::ShowWindow($hwnd, 9) | Out-Null   # SW_RESTORE
            [Win32]::SetWindowPos($hwnd, [IntPtr]::Zero, 0, 0, 0, 0, 0x0043) | Out-Null  # TOP|NOMOVE|NOSIZE|SHOWWINDOW
            [Win32]::SetForegroundWindow($hwnd) | Out-Null
            if ($atou) { [Win32]::AttachThreadInput($meu, $dono, $false) | Out-Null }
            Start-Sleep -Milliseconds 900
            $veio = Test-MesmoHwnd ([Win32]::GetForegroundWindow()) $hwnd
        }

        if (-not $veio) {
            Write-Error "Captura pela tela abortada: a janela do Morune nao veio para o primeiro plano, e copiar a tela agora salvaria a janela de outro programa. Minimize as outras janelas e rode de novo."
        }

        $bmp = New-Object System.Drawing.Bitmap $w, $h
        $gfx = [System.Drawing.Graphics]::FromImage($bmp)
        $gfx.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size($w, $h)))
        $gfx.Dispose()

        if (-not (Test-TemInterface $bmp $w $h)) {
            $bmp.Dispose()
            Write-Error "Captura invalida: a leitura da tela nao trouxe interface nenhuma (cor unica). A janela provavelmente esta coberta."
        }
        "aviso        : capturado pela tela (janela com acrilico; PrintWindow devolve vazio)"
    }

    $bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()

    "captura salva : $Out ({0}x{1})" -f $w, $h
    "janela        : $($proc.MainWindowTitle)"
    "working set   : {0:N1} MB" -f ($proc.WorkingSet64 / 1MB)
}
finally {
    if (-not $proc.HasExited) {
        $proc.CloseMainWindow() | Out-Null
        Start-Sleep -Milliseconds 800
        if (-not $proc.HasExited) { $proc.Kill() }
    }
    if ($null -ne $backup) { Set-Content $configPath $backup -Encoding utf8 }
}
