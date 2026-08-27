# Verificacao visual dos temas.
#
# Abre o Morune uma vez por tema, deixa o Slint renderizar e pede que o proprio
# aplicativo salve a janela em PNG. A captura sai do renderizador, nao da tela:
# funciona com a janela em segundo plano e nunca grava nada que esteja atras
# dela.
#
# Uso: . .\tools\env.ps1 ; .\tools\snapshot.ps1
#      . .\tools\env.ps1 ; .\tools\snapshot.ps1 -Themes midnight,paper

param(
    [string[]]$Themes = @("midnight", "paper", "pulse", "cristal", "bruma"),
    [string]$OutDir = "$PSScriptRoot\..\bench-out",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$root = Resolve-Path "$PSScriptRoot\.."
$exe = Join-Path $root "target\debug\morune.exe"

if (-not $SkipBuild) {
    Write-Host "compilando com a feature snapshot..." -ForegroundColor DarkGray
    Push-Location $root
    try {
        # `$ErrorActionPreference = "Stop"` transforma qualquer linha que o
        # cargo escreva em stderr num erro terminante -- inclusive um aviso de
        # lint, que nao impede build nenhum. O script inteiro morria antes de
        # capturar um tema so. Quem decide se o build passou e o codigo de
        # saida, e nada mais.
        $anterior = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        cargo build -p morune-app --features snapshot 2>&1 | Out-Null
        $codigo = $LASTEXITCODE
        $ErrorActionPreference = $anterior
        if ($codigo -ne 0) { Write-Error "cargo build falhou (codigo $codigo)." }
    }
    finally { Pop-Location }
}

if (-not (Test-Path $exe)) { Write-Error "Executavel nao encontrado: $exe" }
if (-not (Test-Path $OutDir)) { New-Item -ItemType Directory -Force $OutDir | Out-Null }

$configPath = Join-Path $env:APPDATA "morune\Morune\config\config.toml"
$backup = if (Test-Path $configPath) { Get-Content $configPath -Raw } else { $null }

try {
    foreach ($theme in $Themes) {
        # Trocar o tema pela configuracao exercita o mesmo caminho que o
        # aplicativo usa ao abrir, e nao depende de automatizar cliques.
        New-Item -ItemType Directory -Force (Split-Path -Parent $configPath) | Out-Null
        Set-Content $configPath "version = 1`n`n[appearance]`ntheme = `"$theme`"`n" -Encoding utf8

        $out = Join-Path $OutDir "$theme.png"
        if (Test-Path $out) { Remove-Item $out -Force }

        $env:MORUNE_SNAPSHOT = $out
        & $exe 2>&1 | Where-Object { $_ -match "snapshot=|falha" } | ForEach-Object { Write-Host "  $_" }
        Remove-Item Env:\MORUNE_SNAPSHOT

        if (Test-Path $out) {
            $kb = (Get-Item $out).Length / 1KB
            Write-Host ("{0,-10} -> {1} ({2:N0} KB)" -f $theme, $out, $kb) -ForegroundColor Green
        } else {
            Write-Host ("{0,-10} -> FALHOU" -f $theme) -ForegroundColor Red
        }
    }
}
finally {
    if ($null -ne $backup) { Set-Content $configPath $backup -Encoding utf8 }
    elseif (Test-Path $configPath) { Remove-Item $configPath -Force }
}
