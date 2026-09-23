# Mede se o Morune atrapalha o jogo: FPS medio e 1% low, com e sem o Morune.
#
# E a medicao que falta em docs/PERFORMANCE.md ("Interferencia com jogo em tela
# cheia"). Usa o PresentMon da Intel/GameTechDev, que le os quadros que o jogo
# apresenta sem injetar nada no jogo.
#
# Uso (com o jogo aberto e voce jogando o mesmo trecho nas duas rodadas):
#   .\tools\medir-jogo.ps1 -PresentMon C:\caminho\PresentMon.exe -Jogo RobloxPlayerBeta.exe
#
# O script pede para voce: (1) fechar o Morune e jogar 60 s; (2) abrir o Morune
# tocando musica e jogar mais 60 s. Resultado em bench-out\jogo-<data>.txt.

param(
    [Parameter(Mandatory)][string]$PresentMon,
    [Parameter(Mandatory)][string]$Jogo,
    [int]$Segundos = 60
)

$ErrorActionPreference = "Stop"
$saida = Join-Path $PSScriptRoot "..\bench-out"
New-Item -ItemType Directory -Force $saida | Out-Null
$carimbo = Get-Date -Format "yyyyMMdd-HHmm"

function Rodada([string]$nome) {
    $csv = Join-Path $saida "jogo-$carimbo-$nome.csv"
    & $PresentMon --process_name $Jogo --timed $Segundos --terminate_after_timed `
        --output_file $csv --no_console_stats | Out-Null
    $linhas = Import-Csv $csv
    # Coluna de tempo entre quadros: `MsBetweenPresents` (PresentMon 1.x) ou
    # `FrameTime` (2.x).
    $col = if ($linhas[0].PSObject.Properties.Name -contains "FrameTime") { "FrameTime" } else { "MsBetweenPresents" }
    $ms = $linhas | ForEach-Object { [double]$_.$col } | Where-Object { $_ -gt 0 } | Sort-Object
    $media = ($ms | Measure-Object -Average).Average
    # 1% low: a media do 1% de quadros mais lentos, convertida em FPS.
    $pior = $ms | Select-Object -Last ([math]::Max(1, [int]($ms.Count / 100)))
    $low = 1000 / (($pior | Measure-Object -Average).Average)
    [pscustomobject]@{ Rodada = $nome; Quadros = $ms.Count; FPS = [math]::Round(1000 / $media, 1); Low1 = [math]::Round($low, 1) }
}

Read-Host "Rodada 1: FECHE o Morune, volte para o jogo e aperte Enter aqui (mede $Segundos s)"
$sem = Rodada "sem-morune"
Read-Host "Rodada 2: ABRA o Morune tocando musica, volte para o jogo e aperte Enter (mede $Segundos s)"
$com = Rodada "com-morune"

$relato = Join-Path $saida "jogo-$carimbo.txt"
$texto = @(
    "Jogo: $Jogo   duracao por rodada: $Segundos s",
    ($sem | Format-Table | Out-String),
    ($com | Format-Table | Out-String),
    ("Diferenca: FPS {0:+0.0;-0.0}   1% low {1:+0.0;-0.0}" -f ($com.FPS - $sem.FPS), ($com.Low1 - $sem.Low1))
)
$texto | Set-Content $relato -Encoding utf8
$texto
Write-Host "Salvo em $relato"
