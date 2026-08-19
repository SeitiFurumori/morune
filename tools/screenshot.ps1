# Captura a janela do Morune para inspecao visual.
#
# Existe porque testes automatizados provam que o tema carrega, mas nao provam
# que a tela ficou legivel. Uma captura por tema e a verificacao mais barata
# disso.
#
# Captura apenas o retangulo da janela do aplicativo, nunca a tela inteira: o
# resto do desktop e assunto de quem esta usando a maquina.
#
# Uso: . .\tools\env.ps1 ; .\tools\screenshot.ps1 -Theme paper -Out .\bench-out\paper.png

param(
    [string]$Theme = "",
    [string]$Out = "$PSScriptRoot\..\bench-out\morune.png",
    [int]$WarmupSeconds = 4,
    [string]$Exe = "$PSScriptRoot\..\target\release\morune.exe"
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Runtime.InteropServices;
public struct RECT { public int Left, Top, Right, Bottom; }
public class Win32 {
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT r);
    [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr hWnd, int attr, out RECT r, int size);
}
"@

if (-not (Test-Path $Exe)) { Write-Error "Executavel nao encontrado: $Exe" }

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
    [Win32]::SetForegroundWindow($hwnd) | Out-Null
    Start-Sleep -Milliseconds 900

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

    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $gfx = [System.Drawing.Graphics]::FromImage($bmp)
    $gfx.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size $w, $h))
    $bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
    $gfx.Dispose(); $bmp.Dispose()

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
