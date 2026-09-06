# Medicao de performance do Morune.
#
# Mede o que o PERFORMANCE.md promete, com numeros reais desta maquina:
#   - tamanho do executavel de release
#   - tempo de startup, por dentro (ate o laco de eventos comecar) e por fora
#     (processo inteiro, incluindo carga do binario pelo Windows)
#   - working set em repouso
#   - CPU e GPU, com o cenario descrito por quem mede
#
# Uso:  . .\tools\env.ps1 ; .\tools\measure.ps1
# Opcoes: -Runs N (repeticoes de startup), -IdleSeconds N (janela de amostragem)
#
# -Watch mede um Morune **que ja esta aberto**, sem abrir nem fechar nada. E o
# modo que responde a metrica mais importante do PERFORMANCE.md e a unica que
# nunca teve numero: quanto o Morune custa com musica tocando e um jogo em tela
# cheia na frente. Esse cenario nao da para montar sinteticamente -- exige uma
# sessao real, com login e Premium --, entao a ferramenta se limita a observar.
#
#   .\tools\measure.ps1 -Watch -IdleSeconds 60 -Rotulo "tocando + jogo"

param(
    [int]$Runs = 10,
    [int]$IdleSeconds = 12,
    [switch]$Watch,
    [string]$Rotulo = "em repouso",
    [string]$Exe = "$PSScriptRoot\..\target\release\morune.exe"
)

$ErrorActionPreference = "Stop"

# --- GPU por processo -------------------------------------------------------
#
# O Windows publica uso de GPU por processo em `\GPU Engine(...)`, com uma
# instancia por motor: 3D, Copy, VideoDecode, VideoEncode. O nome da instancia
# carrega o pid, e um processo aparece em varias -- somar as do mesmo motor e o
# que o Gerenciador de Tarefas mostra na coluna GPU.
#
# **Por que somar, e nao pegar a maior.** Um redesenho da interface toca 3D e
# Copy no mesmo quadro. A pergunta do projeto e "o Morune acorda a GPU?", e para
# isso o total e que responde.
#
# Devolve $null quando o contador nao existe (maquina sem WDDM 2.0, ou driver
# que nao o publica). Ausencia de contador vira "nao medido", nunca zero: um
# zero inventado aqui viraria numero no PERFORMANCE.md.
function Get-GpuPercent([int]$ProcessId) {
    $paths = @()
    try {
        $paths = (Get-Counter -ListSet 'GPU Engine' -ErrorAction Stop).PathsWithInstances |
            Where-Object { $_ -like "*(pid_${ProcessId}_*" -and $_ -like '*Utilization Percentage' }
    }
    catch { return $null }

    if ($paths.Count -eq 0) { return @{} }

    try {
        $amostra = Get-Counter -Counter $paths -ErrorAction Stop
    }
    catch { return @{} }

    $porMotor = @{}
    foreach ($v in $amostra.CounterSamples) {
        if ($v.Path -match 'engtype_([A-Za-z0-9]+)') {
            $motor = $Matches[1]
            if (-not $porMotor.ContainsKey($motor)) { $porMotor[$motor] = 0.0 }
            $porMotor[$motor] += $v.CookedValue
        }
    }
    return $porMotor
}

# Amostra CPU, GPU e memoria de um processo ja em execucao.
function Measure-Process($proc, [int]$Segundos, [string]$Rotulo) {
    Write-Host "amostrando $Rotulo por $Segundos s (pid $($proc.Id))..." -ForegroundColor DarkGray

    $samples = @()
    $gpu = @{}
    $gpuAmostras = 0
    $gpuIndisponivel = $false

    for ($i = 0; $i -lt $Segundos; $i++) {
        Start-Sleep -Seconds 1
        $proc.Refresh()
        if ($proc.HasExited) { break }
        $samples += [PSCustomObject]@{
            Quando       = Get-Date
            WorkingSetMB = $proc.WorkingSet64 / 1MB
            PrivateMB    = $proc.PrivateMemorySize64 / 1MB
            CpuSeconds   = $proc.TotalProcessorTime.TotalSeconds
        }

        $leitura = Get-GpuPercent -ProcessId $proc.Id
        if ($null -eq $leitura) { $gpuIndisponivel = $true }
        else {
            $gpuAmostras++
            foreach ($motor in $leitura.Keys) {
                if (-not $gpu.ContainsKey($motor)) { $gpu[$motor] = @() }
                $gpu[$motor] += $leitura[$motor]
            }
        }
    }

    if ($samples.Count -eq 0) { Write-Host "sem amostras"; return }

    $ws = $samples | Measure-Object WorkingSetMB -Average -Maximum
    $pv = $samples | Measure-Object PrivateMB -Average -Maximum
    $cpuDelta = $samples[-1].CpuSeconds - $samples[0].CpuSeconds
    # Tempo real decorrido, e nao o numero de amostras: cada volta do laco dorme
    # um segundo e AINDA le o contador de GPU, que sozinho leva perto de outro.
    # Contando amostra como se fosse segundo, a mesma CPU aparecia quase tres
    # vezes maior -- foi o que fez o gasto em repouso parecer 1,48%.
    $span = ($samples[-1].Quando - $samples[0].Quando).TotalSeconds

    Write-Host ("working set    : media {0,7:N1} MB | pico {1,7:N1} MB" -f $ws.Average, $ws.Maximum)
    Write-Host ("memoria privada: media {0,7:N1} MB | pico {1,7:N1} MB" -f $pv.Average, $pv.Maximum)
    if ($span -gt 0) {
        Write-Host ("cpu ($Rotulo): {0:N2}% de um nucleo" -f (100 * $cpuDelta / $span))
    }

    if ($gpuIndisponivel) {
        Write-Host "gpu            : contador indisponivel nesta maquina (nao medido)"
    }
    elseif ($gpu.Keys.Count -eq 0) {
        Write-Host ("gpu ($Rotulo): nenhum motor em uso em {0} amostras" -f $gpuAmostras)
    }
    else {
        foreach ($motor in ($gpu.Keys | Sort-Object)) {
            $m = $gpu[$motor] | Measure-Object -Average -Maximum
            Write-Host ("gpu {0,-12}: media {1,6:N2}% | pico {2,6:N2}%" -f $motor, $m.Average, $m.Maximum)
        }
    }
}

# --- modo observador --------------------------------------------------------
if ($Watch) {
    $abertos = @(Get-Process -Name morune -ErrorAction SilentlyContinue)
    if ($abertos.Count -eq 0) {
        Write-Error "Nenhum morune.exe aberto. Abra o Morune, ponha musica para tocar e rode de novo."
    }
    if ($abertos.Count -gt 1) {
        Write-Error "Ha $($abertos.Count) processos morune.exe. Feche os extras: a medicao seria de qual?"
    }

    Write-Host "== Morune: medicao de um processo ja aberto ==" -ForegroundColor Cyan
    Write-Host ("pid          : {0}" -f $abertos[0].Id)
    Write-Host ("aberto desde : {0}" -f $abertos[0].StartTime)
    Write-Host ""
    Measure-Process -proc $abertos[0] -Segundos $IdleSeconds -Rotulo $Rotulo
    Write-Host ""
    Write-Host "Nada foi aberto nem fechado. Registre em PERFORMANCE.md com a data e o cenario." -ForegroundColor DarkGray
    return
}

if (-not (Test-Path $Exe)) {
    Write-Error "Executavel nao encontrado em $Exe. Rode: cargo build -p morune-app --release"
}

# Instancia unica: com um Morune ja aberto, cada execucao de startup abaixo
# apenas traz a janela dele para frente e sai. O relogio mediria isso, e nao o
# startup -- numero falso que passaria por bom. Ver `instance.rs`.
$jaAberto = @(Get-Process -Name morune -ErrorAction SilentlyContinue)
if ($jaAberto.Count -gt 0) {
    Write-Error ("Ha {0} Morune aberto ({1}). Feche antes de medir: com uma instancia viva, o startup medido nao e startup -- e a segunda copia desistindo. Para medir o que ja esta aberto, use -Watch." -f $jaAberto.Count, ($jaAberto.Path -join ', '))
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

# --- CPU, GPU e memoria em repouso ---
$proc = Start-Process -FilePath $Exe -PassThru
try {
    # Dois segundos de folga: o primeiro quadro e a criacao do contexto OpenGL
    # nao sao repouso, e entrariam na media como se fossem.
    Start-Sleep -Seconds 2
    Measure-Process -proc $proc -Segundos $IdleSeconds -Rotulo "em repouso"
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
