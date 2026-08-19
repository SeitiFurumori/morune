# Assina um arquivo com o certificado de codigo do Morune.
#
# Enquanto nao existir certificado, este script nao faz nada e devolve sucesso
# -- de proposito. O objetivo e que `build-installer.ps1` ja chame a assinatura
# no lugar certo hoje, para que no dia em que o certificado existir baste
# configurar uma variavel de ambiente, sem mexer no pipeline.
#
# O que e assinatura de codigo e o que ela resolve (e o que nao resolve) esta em
# docs/assinatura.md.
#
# Configuracao, por variavel de ambiente:
#
#   MORUNE_SIGN_THUMBPRINT  impressao digital de um certificado ja instalado no
#                           repositorio de certificados do usuario. E a forma
#                           preferida: nenhum arquivo de chave no disco.
#   MORUNE_SIGN_PFX         caminho de um arquivo .pfx, alternativa quando o
#                           certificado nao esta instalado.
#   MORUNE_SIGN_PFX_PASSWORD  senha do .pfx, se houver.
#   MORUNE_SIGN_TIMESTAMP   servidor de carimbo de tempo. Tem um padrao.
#
# Uso: .\tools\sign.ps1 -Path target\release\morune.exe
#      .\tools\sign.ps1 -Path dist\Morune-0.1.0-setup.exe -Require

param(
    [Parameter(Mandatory = $true)][string]$Path,
    # Falha em vez de avisar quando nao ha como assinar. Para uso no dia em que
    # publicar sem assinatura passar a ser um erro, e nao o estado normal.
    [switch]$Require
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $Path)) { Write-Error "Arquivo nao encontrado: $Path" }

function Resolve-SignTool {
    $inPath = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if ($inPath) { return $inPath.Source }

    # O signtool vem no Windows SDK, que nao esta no PATH por padrao. Pega a
    # versao mais nova instalada.
    $kits = "${env:ProgramFiles(x86)}\Windows Kits\10\bin"
    if (Test-Path $kits) {
        $found = Get-ChildItem "$kits\*\x64\signtool.exe" -ErrorAction SilentlyContinue |
                 Sort-Object FullName -Descending | Select-Object -First 1
        if ($found) { return $found.FullName }
    }
    return $null
}

$thumbprint = $env:MORUNE_SIGN_THUMBPRINT
$pfx = $env:MORUNE_SIGN_PFX
$timestamp = if ($env:MORUNE_SIGN_TIMESTAMP) { $env:MORUNE_SIGN_TIMESTAMP } else { "http://timestamp.digicert.com" }

if (-not $thumbprint -and -not $pfx) {
    $message = "sem certificado configurado; $([System.IO.Path]::GetFileName($Path)) fica sem assinatura"
    if ($Require) { Write-Error $message }
    Write-Host "  $message" -ForegroundColor DarkYellow
    Write-Host "  (defina MORUNE_SIGN_THUMBPRINT ou MORUNE_SIGN_PFX -- ver docs/assinatura.md)" -ForegroundColor DarkGray
    exit 0
}

$signtool = Resolve-SignTool
if (-not $signtool) {
    $message = "certificado configurado mas signtool.exe nao foi encontrado. Instale o Windows SDK."
    if ($Require) { Write-Error $message }
    Write-Host "  $message" -ForegroundColor Red
    exit 0
}

# SHA-256 tanto no resumo do arquivo quanto no carimbo de tempo: SHA-1 nao e
# mais aceito pelo Windows para codigo novo.
$args = @("sign", "/fd", "SHA256", "/td", "SHA256", "/tr", $timestamp)
if ($thumbprint) {
    $args += @("/sha1", $thumbprint)
} else {
    if (-not (Test-Path $pfx)) { Write-Error "MORUNE_SIGN_PFX aponta para um arquivo que nao existe: $pfx" }
    $args += @("/f", $pfx)
    if ($env:MORUNE_SIGN_PFX_PASSWORD) { $args += @("/p", $env:MORUNE_SIGN_PFX_PASSWORD) }
}
$args += $Path

& $signtool @args
if ($LASTEXITCODE -ne 0) { Write-Error "signtool falhou (codigo $LASTEXITCODE)" }

# Conferir a assinatura depois de aplicar: assinar e verificar sao operacoes
# diferentes, e uma cadeia incompleta so aparece na verificacao.
& $signtool @("verify", "/pa", "/all", $Path)
if ($LASTEXITCODE -ne 0) { Write-Error "assinatura aplicada mas nao verifica (codigo $LASTEXITCODE)" }

Write-Host ("  assinado: {0}" -f [System.IO.Path]::GetFileName($Path)) -ForegroundColor Green
