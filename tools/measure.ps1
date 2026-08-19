# Medicao de performance do Morune.
#
# Mede o que o PERFORMANCE.md promete, com numeros reais desta maquina:
#   - tamanho do executavel de release
#   - tempo de startup, por dentro (ate o laco de eventos comecar) e por fora
#     (processo inteiro, incluindo carga do binario pelo Windows)
#   - working set em repouso
#
# Uso:  . .\tools\env.ps1 ; .\tools\measure.ps1
# Opcoes: -Runs N (repeticoes de startup), -IdleSeconds N (janela de amostragem)

param(
    [int]$Runs = 10,
    [int]$IdleSeconds = 12,
    [string]$Exe = "$PSScriptRoot\..\target\release\morune.exe"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $Exe)) {
    Write-Error "Executavel nao encontrado em $Exe. Rode: cargo build -p morune-app --release"
}

$exeItem = Get-Item $Exe
Write-Host "== Morune: medicao ==" -ForegroundColor Cyan
Write-Host ("executavel   : {0}" -f $exeItem.FullName)
Write-Host ("tamanho      : {0:N2} MB" -f ($exeItem.Length / 1MB))
Write-Host ("compilado em : {0}" -f $exeItem.LastWriteTime)
Write-Host ""

# --- startup ---
# A primeira execucao paga o cache de arquivo frio do Windows e nao e
# representativa do uso diario; ela e medida a parte em vez de descartada.
$report = Join-Path $env:TEMP "morune-startup.txt"
$env:MORUNE_EXIT_AFTER_STARTUP = "1"
$env:MORUNE_STARTUP_FILE = $report

$internal = @()
$external = @()

for ($i = 0; $i -lt $Runs; $i++) {
    if (Test-Path $report) { Remove-Item $report -Force }

    # `Start-Process -Wait` e obrigatorio: o binario de release e do subsistema
    # "windows", e o operador de chamada do PowerShell nao espera esse tipo de
    # processo -- media-lo com `&` daria alguns milissegundos falsos.
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    Start-Process -FilePath $Exe -Wait | Out-Null
    $sw.Stop()

    if (Test-Path $report) {
        $line = Get-Content $report | Select-String -Pattern "startup_ms=(\d+)"
        if ($line) { $internal += [int]$line.Matches[0].Groups[1].Value }
    }
    $external += $sw.Elapsed.TotalMilliseconds
}

Remove-Item Env:\MORUNE_EXIT_AFTER_STARTUP
Remove-Item Env:\MORUNE_STARTUP_FILE
if (Test-Path $report) { Remove-Item $report -Force }

function Summarize($label, $values, $unit) {
    if ($values.Count -eq 0) { Write-Host "$label : sem amostras"; return }
    $sorted = $values | Sort-Object
    $median = $sorted[[int]($sorted.Count / 2)]
    Write-Host ("{0,-28}: primeiro {1,7:N1} | mediana {2,7:N1} | min {3,7:N1} | max {4,7:N1} {5}" -f `
        $label, $values[0], $median, ($sorted[0]), ($sorted[-1]), $unit)
}

Summarize "startup interno" $internal "ms"
Summarize "startup processo inteiro" $external "ms"
Write-Host "  interno = ate o laco de eventos rodar; processo inteiro inclui carga do binario e saida."
Write-Host ""

# --- memoria em repouso ---
Write-Host "amostrando memoria em repouso por $IdleSeconds s..." -ForegroundColor DarkGray
$proc = Start-Process -FilePath $Exe -PassThru
try {
    Start-Sleep -Seconds 2
    $samples = @()
    for ($i = 0; $i -lt $IdleSeconds; $i++) {
        Start-Sleep -Seconds 1
        $proc.Refresh()
        if ($proc.HasExited) { break }
        $samples += [PSCustomObject]@{
            WorkingSetMB = $proc.WorkingSet64 / 1MB
            PrivateMB    = $proc.PrivateMemorySize64 / 1MB
            CpuSeconds   = $proc.TotalProcessorTime.TotalSeconds
        }
    }

    if ($samples.Count -gt 0) {
        $ws = $samples | Measure-Object WorkingSetMB -Average -Maximum
        $pv = $samples | Measure-Object PrivateMB -Average -Maximum
        $cpuDelta = $samples[-1].CpuSeconds - $samples[0].CpuSeconds
        $span = $samples.Count - 1

        Write-Host ("working set    : media {0,7:N1} MB | pico {1,7:N1} MB" -f $ws.Average, $ws.Maximum)
        Write-Host ("memoria privada: media {0,7:N1} MB | pico {1,7:N1} MB" -f $pv.Average, $pv.Maximum)
        if ($span -gt 0) {
            Write-Host ("cpu em repouso : {0:N2}% de um nucleo" -f (100 * $cpuDelta / $span))
        }
    }
}
finally {
    if (-not $proc.HasExited) {
        $proc.CloseMainWindow() | Out-Null
        Start-Sleep -Milliseconds 800
        if (-not $proc.HasExited) { $proc.Kill() }
    }
}

Write-Host ""
Write-Host "Numeros medidos nesta maquina; registre-os em PERFORMANCE.md com a data." -ForegroundColor DarkGray
