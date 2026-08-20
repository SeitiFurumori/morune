# Gera o instalador do Morune.
#
# Compila o binario de release, confere que ele roda sem nenhuma DLL externa e
# so entao empacota. A checagem de isolamento vem antes de proposito: um
# instalador que produz um aplicativo que nao abre na maquina do usuario e pior
# que nenhum instalador.
#
# Uso: . .\tools\env.ps1 ; .\tools\build-installer.ps1

param(
    [string]$Version = "0.1.0",
    [switch]$SkipBuild,
    [string]$LicenseTarget = "x86_64-pc-windows-gnu"
)

$ErrorActionPreference = "Stop"
$root = (Resolve-Path "$PSScriptRoot\..").Path
$exe = Join-Path $root "target\release\morune.exe"
$dist = Join-Path $root "dist"

$makensis = @(
    "C:\Program Files (x86)\NSIS\makensis.exe",
    "C:\Program Files\NSIS\makensis.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1

if (-not $makensis) {
    Write-Error "NSIS nao encontrado. Instale com: winget install NSIS.NSIS"
}

if (-not $SkipBuild) {
    Write-Host "compilando release..." -ForegroundColor DarkGray
    Push-Location $root
    try {
        cargo build -p morune-app --release
        if ($LASTEXITCODE -ne 0) { Write-Error "cargo build falhou" }
    } finally { Pop-Location }
}

if (-not (Test-Path $exe)) { Write-Error "Binario nao encontrado: $exe" }

# --- o binario precisa rodar sozinho ---
# Copia para uma pasta isolada e executa com PATH reduzido ao Windows. Se
# faltasse qualquer DLL de runtime (MinGW, por exemplo), o loader encerraria o
# processo antes de ele conseguir gravar o comprovante. O modo de verificacao
# nao abre a janela: runners de CI executam como servico e nao possuem uma area
# de trabalho interativa, portanto abrir UI ali produziria um falso negativo.
Write-Host "verificando se o executavel roda sem dependencias externas..." -ForegroundColor DarkGray
$stage = Join-Path $env:TEMP "morune-isolation-check"
Remove-Item $stage -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $stage | Out-Null
Copy-Item $exe $stage

$saved = $env:PATH
try {
    $env:PATH = "$env:SystemRoot\system32;$env:SystemRoot"
    $env:MORUNE_BINARY_CHECK_FILE = "$stage\binary-check.txt"
    $p = Start-Process -FilePath "$stage\morune.exe" -Wait -PassThru
    if ($p.ExitCode -ne 0 -or -not (Test-Path "$stage\binary-check.txt")) {
        Write-Error "O executavel nao carrega isolado (codigo $($p.ExitCode)). Verifique as DLLs de runtime do pacote."
    }
    Write-Host ("  ok -- " + (Get-Content "$stage\binary-check.txt")) -ForegroundColor Green
} finally {
    $env:PATH = $saved
    Remove-Item Env:\MORUNE_BINARY_CHECK_FILE -ErrorAction SilentlyContinue
    Remove-Item $stage -Recurse -Force -ErrorAction SilentlyContinue
}

# --- avisos de licenca das dependencias ---
# Regerado so quando o Cargo.lock muda: e a unica coisa que altera a lista, e a
# leitura completa do grafo custa meio minuto. O arquivo entra no instalador
# porque as licencas das bibliotecas exigem, nao por cortesia.
$licenses = Join-Path $root "THIRD-PARTY-LICENSES.txt"
$lock = Join-Path $root "Cargo.lock"
if (-not (Test-Path $licenses) -or (Get-Item $licenses).LastWriteTime -lt (Get-Item $lock).LastWriteTime) {
    Write-Host "atualizando os avisos de licenca das dependencias..." -ForegroundColor DarkGray
    & (Join-Path $PSScriptRoot "licenses.ps1") -Target $LicenseTarget
    if ($LASTEXITCODE -ne 0) { Write-Error "geracao dos avisos de licenca falhou" }
}

# --- assinar o executavel ---
# Antes de empacotar, porque o instalador carrega o binario dentro dele: assinar
# depois deixaria o arquivo instalado sem assinatura.
Write-Host "assinando o executavel..." -ForegroundColor DarkGray
& (Join-Path $PSScriptRoot "sign.ps1") -Path $exe
if ($LASTEXITCODE -ne 0) { Write-Error "assinatura do executavel falhou" }

# --- empacotar ---
if (-not (Test-Path $dist)) { New-Item -ItemType Directory -Force $dist | Out-Null }
$out = Join-Path $dist "Morune-$Version-setup.exe"
Remove-Item $out -Force -ErrorAction SilentlyContinue

Write-Host "empacotando com NSIS..." -ForegroundColor DarkGray
& $makensis "/DVERSION=$Version" (Join-Path $root "installer\morune.nsi") | Select-Object -Last 12
if ($LASTEXITCODE -ne 0) { Write-Error "makensis falhou (codigo $LASTEXITCODE)" }
if (-not (Test-Path $out)) { Write-Error "Instalador nao foi gerado em $out" }

Write-Host "assinando o instalador..." -ForegroundColor DarkGray
& (Join-Path $PSScriptRoot "sign.ps1") -Path $out
if ($LASTEXITCODE -ne 0) { Write-Error "assinatura do instalador falhou" }

# --- resumo verificavel ---
# Enquanto nao ha assinatura, o hash e o unico jeito de alguem confirmar que o
# arquivo baixado e o que foi publicado. Vai para um .sha256 ao lado, no formato
# que `sha256sum -c` e `Get-FileHash` entendem.
$hash = (Get-FileHash $out -Algorithm SHA256).Hash.ToLower()
"$hash  $([System.IO.Path]::GetFileName($out))" | Set-Content "$out.sha256" -Encoding ascii

$exeMB = (Get-Item $exe).Length / 1MB
$setupMB = (Get-Item $out).Length / 1MB

Write-Host ""
Write-Host "== instalador pronto ==" -ForegroundColor Cyan
Write-Host ("arquivo    : {0}" -f $out)
Write-Host ("executavel : {0:N2} MB" -f $exeMB)
Write-Host ("instalador : {0:N2} MB  ({1:N0}% de compressao)" -f $setupMB, (100 - 100 * $setupMB / $exeMB))
Write-Host ("sha256     : {0}" -f $hash)

$budget = 40
if ($setupMB -le $budget) {
    Write-Host ("orcamento  : OK -- meta e {0} MB" -f $budget) -ForegroundColor Green
} else {
    Write-Host ("orcamento  : ESTOUROU -- meta e {0} MB" -f $budget) -ForegroundColor Red
    exit 1
}
