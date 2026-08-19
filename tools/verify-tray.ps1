# Verifica o comportamento de fechar para a bandeja.
#
# A afirmacao "fechar a janela nao encerra o Morune" e facil de escrever e facil
# de quebrar sem perceber. Este script fecha a janela do jeito que o Windows
# fecha (WM_CLOSE, o mesmo que o X da barra de titulo envia) e depois checa se o
# processo continua vivo.
#
# Uso: . .\tools\env.ps1 ; .\tools\verify-tray.ps1

param(
    [string]$Exe = "$PSScriptRoot\..\target\release\morune.exe",
    [int]$WarmupSeconds = 4
)

$ErrorActionPreference = "Stop"

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class WinClose {
    [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr hWnd, uint msg, IntPtr wp, IntPtr lp);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    public const uint WM_CLOSE = 0x0010;
}
"@

if (-not (Test-Path $Exe)) { Write-Error "Executavel nao encontrado: $Exe. Rode: cargo build -p morune-app --release" }

$failures = 0
function Check($name, $condition, $detail) {
    if ($condition) {
        Write-Host ("  OK    {0}" -f $name) -ForegroundColor Green
    } else {
        Write-Host ("  FALHA {0} -- {1}" -f $name, $detail) -ForegroundColor Red
        $script:failures++
    }
}

# O padrao de `close_to_tray` e ligado; a configuracao do usuario e preservada.
$configPath = Join-Path $env:APPDATA "morune\Morune\config\config.toml"
$backup = if (Test-Path $configPath) { Get-Content $configPath -Raw } else { $null }

Write-Host "== fechar para a bandeja ==" -ForegroundColor Cyan
$proc = Start-Process -FilePath $Exe -PassThru
try {
    Start-Sleep -Seconds $WarmupSeconds
    $proc.Refresh()
    Check "aplicativo abriu" (-not $proc.HasExited) "saiu com codigo $($proc.ExitCode)"
    if ($proc.HasExited) { return }

    $hwnd = $proc.MainWindowHandle
    Check "janela principal existe" ($hwnd -ne 0) "MainWindowHandle = 0"
    if ($hwnd -eq 0) { return }

    # Mesmo caminho do botao de fechar da barra de titulo.
    [WinClose]::SendMessage($hwnd, [WinClose]::WM_CLOSE, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null
    Start-Sleep -Seconds 3
    $proc.Refresh()

    Check "processo continua vivo apos fechar" (-not $proc.HasExited) `
        "o processo terminou; a reproducao teria parado junto"

    if (-not $proc.HasExited) {
        # `IsWindowVisible` no handle original e a pergunta certa: o processo
        # continua dono da janela, o que importa e ela nao estar na tela.
        $visible = [WinClose]::IsWindowVisible($hwnd)
        Check "janela nao esta mais visivel" (-not $visible) `
            "a janela continua na tela apos WM_CLOSE"

        # Um processo escondido nao pode virar zumbi consumindo CPU.
        $before = $proc.TotalProcessorTime.TotalSeconds
        Start-Sleep -Seconds 3
        $proc.Refresh()
        $cpu = 100 * ($proc.TotalProcessorTime.TotalSeconds - $before) / 3
        Check "cpu baixa em segundo plano" ($cpu -lt 3.0) ("{0:N2}% de um nucleo" -f $cpu)
        Write-Host ("        cpu {0:N2}% | working set {1:N1} MB" -f $cpu, ($proc.WorkingSet64 / 1MB)) -ForegroundColor DarkGray
    }
}
finally {
    if (-not $proc.HasExited) { $proc.Kill(); $proc.WaitForExit() }
    if ($null -ne $backup) { Set-Content $configPath $backup -Encoding utf8 }
}

Write-Host ""
if ($failures -eq 0) {
    Write-Host "tudo passou." -ForegroundColor Green
    exit 0
} else {
    Write-Host "$failures verificacao(oes) falharam." -ForegroundColor Red
    exit 1
}
