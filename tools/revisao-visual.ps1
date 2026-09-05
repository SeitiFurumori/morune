# Revisao visual: captura o Morune como ele aparece PARA QUEM USA.
#
# ---------------------------------------------------------------------------
# Por que este script existe
# ---------------------------------------------------------------------------
#
# Os dois scripts anteriores nao viam o que quebra. `snapshot.ps1` pede o quadro
# ao renderizador, o que e otimo para provar que o tema carrega -- mas ele roda
# uma configuracao minima, sem conta e no tamanho padrao da janela.
# `screenshot.ps1` captura a janela de verdade, porem uma tela so, sem
# navegacao e sem cursor.
#
# Tres defeitos seguidos passaram exatamente por essa fresta, e os tres tem a
# mesma forma:
#
# 1. **Margem do fundo no meio do conteudo.** So aparecia com a janela
#    MAXIMIZADA: `image-fit: cover` corta o eixo que sobra, e a proporcao da
#    janela padrao esconde o corte.
# 2. **Sombra do cartao terminando em linha reta.** So aparece com CARTOES na
#    tela, e cartao so existe com conta conectada.
# 3. **Realce de hover sem relacao com o tema.** So aparece com o CURSOR sobre
#    uma linha, e captura nenhuma tinha cursor.
#
# Ou seja: nao foi falta de atencao em tres ocasioes, foi um instrumento que nao
# media essas tres coisas. Este script mede.
#
# ---------------------------------------------------------------------------
# O que ele faz de diferente
# ---------------------------------------------------------------------------
#
# - **Usa o aplicativo instalado**, com a sessao real e os dados reais. Listas,
#   cartoes, capas e o rodape tocando so existem assim.
# - **No tamanho de janela que a pessoa usa**, e nao no padrao do tema. O
#   padrao e 1919x1030, que e a janela maximizada nesta maquina.
# - **Navega entre as paginas** e captura cada uma.
# - **Poe o cursor sobre os alvos** e captura o estado de hover.
# - **Copia da tela, e nao da janela.** PrintWindow devolve o desenho do
#   aplicativo isolado; com `window_opacity` abaixo de 1 -- que e o caso -- o que
#   a pessoa ve e a COMPOSICAO com o que esta atras. Capturar da tela e a unica
#   forma de ver o que ela ve. O risco conhecido dessa escolha (capturar a
#   janela errada) e coberto conferindo o primeiro plano antes de copiar.
# - **Preserva a configuracao.** A troca de tema reescreve so a linha `theme`, e
#   o arquivo original volta no fim. `snapshot.ps1` substitui o config inteiro
#   por um minimo, o que apagaria conta e preferencias.
#
# Uso:
#   .\tools\revisao-visual.ps1
#   .\tools\revisao-visual.ps1 -Themes aquario,bruma -Size 1919x1030

param(
    [string[]]$Themes = @("aquario", "bruma"),
    [string]$Size = "1919x1030",
    [string]$Exe = "$env:LOCALAPPDATA\Programs\Morune\morune.exe",
    [string]$OutDir = "$PSScriptRoot\..\bench-out\revisao",
    [int]$WarmupSeconds = 7
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Runtime.InteropServices;
public struct RECT { public int Left, Top, Right, Bottom; }
public class RV {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int a, out RECT r, int s);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint f);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, IntPtr e);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, IntPtr pid);
    [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool c);
    [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
}
"@

$LARGURA, $ALTURA = $Size.Split("x") | ForEach-Object { [int]$_ }

# As cenas.
#
# Cada uma e: para onde clicar antes (em coordenadas de CLIENTE, relativas ao
# canto da janela), onde deixar o cursor, e quanto esperar. `hover` sem `click`
# so move o cursor. Os alvos sao a barra lateral, cujas posicoes sao fixas
# porque o layout do tema define a largura e a altura de cada item.
$CENAS = @(
    @{ nome = "01-inicio";            click = @(90, 121);  hover = $null;        espera = 1800 }
    @{ nome = "02-inicio-hover-faixa"; click = $null;      hover = @(700, 245);  espera = 900 }
    @{ nome = "03-inicio-hover-cartao"; click = $null;     hover = @(350, 720);  espera = 900 }
    @{ nome = "04-biblioteca";        click = @(90, 209);  hover = @(700, 300);  espera = 1800 }
    @{ nome = "05-configuracoes";     click = @(90, 855);  hover = $null;        espera = 1800 }
    @{ nome = "06-config-fim";        click = $null;       hover = @(700, 860);  espera = 900 }
)

function Get-Retangulo {
    param([IntPtr]$Hwnd)
    $r = New-Object RECT
    $tam = [System.Runtime.InteropServices.Marshal]::SizeOf($r)
    # DWMWA_EXTENDED_FRAME_BOUNDS (9): GetWindowRect inclui a borda invisivel de
    # redimensionamento do Windows 11 e traria uma faixa do que esta atras.
    if ([RV]::DwmGetWindowAttribute($Hwnd, 9, [ref]$r, $tam) -ne 0) {
        [RV]::GetWindowRect($Hwnd, [ref]$r) | Out-Null
    }
    return $r
}

# Traz a janela ao primeiro plano de verdade.
#
# O Windows recusa SetForegroundWindow vindo de um processo que nao tem o foco,
# a menos que ele compartilhe a fila de entrada da janela que tem. AttachThreadInput
# cria esse vinculo pelo tempo do pedido.
function Set-Frente {
    param([IntPtr]$Hwnd)
    $meu = [RV]::GetCurrentThreadId()
    $dele = [RV]::GetWindowThreadProcessId([RV]::GetForegroundWindow(), [IntPtr]::Zero)
    [RV]::AttachThreadInput($meu, $dele, $true) | Out-Null
    [RV]::ShowWindow($Hwnd, 5) | Out-Null
    $ok = [RV]::SetForegroundWindow($Hwnd)
    [RV]::AttachThreadInput($meu, $dele, $false) | Out-Null
    Start-Sleep -Milliseconds 400
    return $ok
}

function Invoke-Clique {
    param([int]$X, [int]$Y)
    [RV]::SetCursorPos($X, $Y) | Out-Null
    Start-Sleep -Milliseconds 120
    [RV]::mouse_event(0x0002, 0, 0, 0, [IntPtr]::Zero)  # LEFTDOWN
    Start-Sleep -Milliseconds 60
    [RV]::mouse_event(0x0004, 0, 0, 0, [IntPtr]::Zero)  # LEFTUP
}

# Copia da TELA, com o primeiro plano conferido antes.
#
# Uma captura errada silenciosa e pior que nenhuma: se a janela do Morune nao
# estiver na frente, este script falha em vez de gravar a janela de outro
# programa. Foi assim que a versao antiga do `screenshot.ps1` gravou a janela
# errada parecendo ter dado certo.
function Save-Captura {
    param([IntPtr]$Hwnd, [string]$Caminho)

    if (-not (Test-MesmoHwnd ([RV]::GetForegroundWindow()) $Hwnd)) {
        Set-Frente $Hwnd | Out-Null
    }
    if (-not (Test-MesmoHwnd ([RV]::GetForegroundWindow()) $Hwnd)) {
        Write-Warning "janela do Morune nao esta em primeiro plano; captura pulada: $Caminho"
        return $false
    }

    $r = Get-Retangulo $Hwnd
    $w = $r.Right - $r.Left
    $h = $r.Bottom - $r.Top
    if ($w -le 0 -or $h -le 0) { Write-Warning "retangulo invalido ${w}x${h}"; return $false }

    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size($w, $h)))
    $g.Dispose()
    $bmp.Save($Caminho, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    return $true
}

# `-eq` entre dois [IntPtr] no PowerShell 5.1 compara os objetos, nao os
# enderecos: a comparacao direta sai sempre falsa e a verificacao vira enfeite.
function Test-MesmoHwnd {
    param([IntPtr]$A, [IntPtr]$B)
    return $A.ToInt64() -eq $B.ToInt64()
}

if (-not (Test-Path $Exe)) { Write-Error "Executavel nao encontrado: $Exe" }
if (-not (Test-Path $OutDir)) { New-Item -ItemType Directory -Force $OutDir | Out-Null }

$configPath = Join-Path $env:APPDATA "morune\Morune\config\config.toml"
if (-not (Test-Path $configPath)) { Write-Error "Config nao encontrado: $configPath" }
$backup = Get-Content $configPath -Raw

try {
    foreach ($tema in $Themes) {
        Write-Host "== $tema ==" -ForegroundColor Cyan

        foreach ($p in @(Get-Process -Name morune -ErrorAction SilentlyContinue)) {
            try { $p.Kill() } catch { }
            $p.WaitForExit(5000) | Out-Null
        }
        Start-Sleep -Milliseconds 500

        # So a linha do tema muda. O resto da configuracao -- conta, opacidade,
        # preferencias de audio -- fica intacto.
        $novo = $backup -replace '(?m)^theme\s*=\s*".*"$', "theme = `"$tema`""
        if ($novo -eq $backup) { Write-Error "nao achei a linha 'theme =' no config" }
        Set-Content $configPath $novo -Encoding utf8

        $proc = Start-Process -FilePath $Exe -PassThru
        Start-Sleep -Seconds $WarmupSeconds
        $proc.Refresh()
        if ($proc.HasExited) { Write-Error "o aplicativo saiu antes da captura (codigo $($proc.ExitCode))" }
        if ($proc.MainWindowHandle -eq 0) { Write-Error "a janela principal nao apareceu" }
        $hwnd = $proc.MainWindowHandle

        Set-Frente $hwnd | Out-Null
        # SWP_NOMOVE nao: a janela vai para 0,0 para as coordenadas de cliente
        # baterem com as da tela sem conta nenhuma.
        [RV]::SetWindowPos($hwnd, [IntPtr]::Zero, 0, 0, $LARGURA, $ALTURA, 0x0040) | Out-Null
        Start-Sleep -Milliseconds 1200

        $r = Get-Retangulo $hwnd

        foreach ($cena in $CENAS) {
            if ($null -ne $cena.click) {
                Invoke-Clique ($r.Left + $cena.click[0]) ($r.Top + $cena.click[1])
            }
            if ($null -ne $cena.hover) {
                [RV]::SetCursorPos(($r.Left + $cena.hover[0]), ($r.Top + $cena.hover[1])) | Out-Null
            }
            Start-Sleep -Milliseconds $cena.espera

            $arq = Join-Path $OutDir "$tema-$($cena.nome).png"
            if (Save-Captura $hwnd $arq) {
                Write-Host ("  {0}" -f (Split-Path -Leaf $arq)) -ForegroundColor Green
            }
        }

        try { $proc.Kill() } catch { }
        $proc.WaitForExit(5000) | Out-Null
    }
}
finally {
    Set-Content $configPath $backup -Encoding utf8
    Write-Host "configuracao restaurada" -ForegroundColor DarkGray
}
