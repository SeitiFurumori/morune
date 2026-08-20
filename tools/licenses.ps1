# Reune os avisos de copyright das dependencias em um arquivo so.
#
# Nao e formalidade: MIT, Apache-2.0, BSD e ISC — que sao a quase totalidade do
# que o Morune usa — exigem que o aviso de copyright acompanhe a distribuicao do
# binario. Sem este arquivo, o instalador esta em desacordo com as licencas do
# codigo que ele carrega dentro.
#
# A lista sai do `cargo tree`, e nao do `cargo metadata`, por um motivo que muda
# o resultado: o `metadata` devolve o grafo inteiro possivel, incluindo
# dependencias opcionais que as features ativas nunca ligam. Com ele, o arquivo
# atribuia o Skia, que o Morune nao usa. O `cargo tree` resolve as features de
# verdade. Dai o `metadata` entra so para dizer onde cada pacote esta no disco.
#
# So entram dependencias **normais**: as de build (slint-build, png) e as de
# teste rodam na maquina de quem compila e nao sao distribuidas.
#
# Uso: . .\tools\env.ps1 ; .\tools\licenses.ps1

param(
    [string]$Target = "x86_64-pc-windows-gnu"
)

$ErrorActionPreference = "Stop"
$root = (Resolve-Path "$PSScriptRoot\..").Path
$out = Join-Path $root "THIRD-PARTY-LICENSES.txt"

Push-Location $root
try {
    Write-Host "resolvendo as dependencias que entram no binario..." -ForegroundColor DarkGray
    $tree = cargo tree -e normal --target $Target -p morune-app --prefix none --format "{p}"
    if ($LASTEXITCODE -ne 0) { Write-Error "cargo tree falhou" }

    Write-Host "lendo os caminhos dos pacotes..." -ForegroundColor DarkGray
    $json = cargo metadata --format-version 1 --filter-platform $Target | Out-String
    if ($LASTEXITCODE -ne 0) { Write-Error "cargo metadata falhou" }
} finally { Pop-Location }

$meta = $json | ConvertFrom-Json
$byKey = @{}
foreach ($p in $meta.packages) { $byKey["$($p.name)|$($p.version)"] = $p }

$workspace = @{}
foreach ($p in $meta.packages) {
    if ($meta.workspace_members -contains $p.id) { $workspace["$($p.name)|$($p.version)"] = $true }
}

$keys = @{}
foreach ($line in $tree) {
    if ($line -match '^\s*(\S+)\s+v(\S+)') {
        $key = "$($Matches[1])|$($Matches[2])"
        if (-not $workspace.ContainsKey($key)) { $keys[$key] = $true }
    }
}

$third = $keys.Keys | ForEach-Object { $byKey[$_] } | Where-Object { $_ } |
         Sort-Object { $_.name.ToLower() }, { $_.version }
Write-Host ("dependencias distribuidas: {0}" -f $third.Count) -ForegroundColor DarkGray

# O texto vem do pacote baixado, nao de um modelo: o que a licenca exige
# preservar e o aviso de copyright de cada autor, e ele so existe no arquivo
# real. Um modelo generico de MIT nao substitui nenhum deles.
$wanted = "^(LICEN[CS]E|COPYING|COPYRIGHT|NOTICE|UNLICENSE|(A?GPL|LGPL|MPL)(-|\.|$)|.*-(A?GPL|LGPL|MPL)(-|\.|$))"

function Get-LicenseFiles($package) {
    $dir = Split-Path -Parent $package.manifest_path
    Get-ChildItem $dir -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -match $wanted -and $_.Length -lt 64KB -and $_.Extension -ne ".rs" } |
        Sort-Object Name
}

# Deduplicacao: o texto da Apache-2.0 tem 11 KB e se repete em dezenas de
# pacotes. Sem agrupar, o arquivo passa de 2,8 MB e a maior parte disso e a
# mesma licenca copiada. Textos identicos viram uma entrada so, referenciada por
# numero.
$texts = [ordered]@{}
$entries = @()

foreach ($p in $third) {
    $refs = @()
    foreach ($file in Get-LicenseFiles $p) {
        $body = (Get-Content $file.FullName -Raw).Replace("`r`n", "`n")
        $body = (($body -split "`n") | ForEach-Object { $_.TrimEnd() }) -join "`n"
        $body = $body.TrimEnd()
        if (-not $body) { continue }
        if (-not $texts.Contains($body)) {
            $texts[$body] = [pscustomobject]@{ Number = $texts.Count + 1; Users = @() }
        }
        $texts[$body].Users += ("{0} {1} ({2})" -f $p.name, $p.version, $file.Name)
        $refs += $texts[$body].Number
    }
    $entries += [pscustomobject]@{
        Name = $p.name; Version = $p.version
        License = $(if ($p.license) { $p.license } else { "" })
        Repository = $(if ($p.repository) { $p.repository } else { "" })
        Refs = ($refs | Sort-Object -Unique)
    }
}

$sb = New-Object System.Text.StringBuilder
$rule = "=" * 78
[void]$sb.AppendLine("Avisos de licenca de terceiros -- Morune")
[void]$sb.AppendLine()
[void]$sb.AppendLine("O Morune e distribuido sob a licenca MIT; o texto esta em LICENSE.")
[void]$sb.AppendLine("Este arquivo reune os avisos das bibliotecas de codigo aberto usadas na")
[void]$sb.AppendLine("construcao do executavel, como as licencas delas exigem.")
[void]$sb.AppendLine()
[void]$sb.AppendLine("O Slint aparece abaixo com tres licencas alternativas. O Morune o usa sob a")
[void]$sb.AppendLine("LicenseRef-Slint-Royalty-free-2.0, nao sob a GPL. Ver docs/adr/0005.")
[void]$sb.AppendLine("priority-queue 2.7.0 e usado sob MPL-2.0, nao sob LGPL. O codigo-fonte")
[void]$sb.AppendLine("exato esta em https://crates.io/crates/priority-queue/2.7.0. Ver docs/adr/0006.")
[void]$sb.AppendLine()
[void]$sb.AppendLine(("Gerado por tools/licenses.ps1 em {0}, para o alvo {1}." -f (Get-Date -Format "dd/MM/yyyy"), $Target))
[void]$sb.AppendLine(("{0} dependencias, {1} textos de licenca distintos." -f $entries.Count, $texts.Count))
[void]$sb.AppendLine("Textos identicos aparecem uma vez so, referenciados por numero.")
[void]$sb.AppendLine()

[void]$sb.AppendLine($rule)
[void]$sb.AppendLine("DEPENDENCIAS")
[void]$sb.AppendLine($rule)
[void]$sb.AppendLine()
foreach ($e in $entries) {
    $declared = if ($e.License) { $e.License } else { "licenca nao declarada no manifesto" }
    [void]$sb.AppendLine(("{0} {1} -- {2}" -f $e.Name, $e.Version, $declared))
    if ($e.Repository) { [void]$sb.AppendLine(("    {0}" -f $e.Repository)) }
    if ($e.Refs.Count -gt 0) {
        [void]$sb.AppendLine(("    texto: {0}" -f (($e.Refs | ForEach-Object { "#$_" }) -join ", ")))
    } else {
        [void]$sb.AppendLine("    o pacote nao traz arquivo de licenca proprio; vale a licenca declarada")
    }
    [void]$sb.AppendLine()
}

[void]$sb.AppendLine($rule)
[void]$sb.AppendLine("TEXTOS DE LICENCA")
[void]$sb.AppendLine($rule)
foreach ($body in $texts.Keys) {
    $entry = $texts[$body]
    [void]$sb.AppendLine()
    [void]$sb.AppendLine(("--- texto #{0} ---" -f $entry.Number))
    [void]$sb.AppendLine(("usado por: {0}" -f ($entry.Users -join "; ")))
    [void]$sb.AppendLine()
    [void]$sb.AppendLine($body)
    [void]$sb.AppendLine()
}

# UTF-8 sem BOM: o arquivo e aberto no Bloco de Notas e por ferramentas de linha
# de comando, e o BOM atrapalha as segundas.
[System.IO.File]::WriteAllText($out, $sb.ToString(), (New-Object System.Text.UTF8Encoding $false))

# --- alerta de licenca contagiosa ---
#
# Esta checagem existe porque o problema real aconteceu: o Slint e
# `GPL-3.0-only OR ...`, e isso passou despercebido ate alguem ler a lista
# inteira a mao. Uma dependencia copyleft entra sem barulho e so aparece quando
# ja e tarde -- ou pior, nunca aparece.
#
# A lista de revisadas e curta de proposito: cada nome ali foi olhado e a
# decisao esta escrita num ADR. Qualquer coisa fora dela para o build.
$revisadas = @{
    # Usados sob LicenseRef-Slint-Royalty-free-2.0, nao sob GPL. Ver ADR-0005.
    "slint" = $true; "slint-macros" = $true; "i-slint-core" = $true
    "i-slint-common" = $true; "i-slint-compiler" = $true; "i-slint-core-macros" = $true
    "i-slint-backend-selector" = $true; "i-slint-backend-winit" = $true
    "i-slint-renderer-femtovg" = $true; "i-slint-renderer-software" = $true
    # Dependencia da librespot, usada sem modificacao sob MPL-2.0. Ver ADR-0006.
    "priority-queue" = $true
}

$contagiosas = $entries | Where-Object {
    $_.License -match "GPL|AGPL|SSPL|CDDL|EPL|MS-RL" -and -not $revisadas.ContainsKey($_.Name)
}

$semTexto = $entries | Where-Object { $_.Refs.Count -eq 0 }
Write-Host ("THIRD-PARTY-LICENSES.txt: {0:N0} KB, {1} pacotes, {2} textos distintos" -f `
    ((Get-Item $out).Length / 1KB), $entries.Count, $texts.Count) -ForegroundColor Green
if ($semTexto) {
    Write-Host ("sem arquivo proprio ({0}): {1}" -f $semTexto.Count, (($semTexto | ForEach-Object { $_.Name }) -join ", ")) -ForegroundColor DarkYellow
}
if ($contagiosas) {
    Write-Host ""
    Write-Host "ATENCAO: dependencia com licenca copyleft nao revisada" -ForegroundColor Red
    foreach ($c in $contagiosas) {
        Write-Host ("  {0} {1} -- {2}" -f $c.Name, $c.Version, $c.License) -ForegroundColor Red
    }
    Write-Host "Uma delas pode obrigar o Morune inteiro a mudar de licenca." -ForegroundColor Red
    Write-Host "Decida, escreva um ADR e adicione o nome a lista `$revisadas deste script." -ForegroundColor Red
    exit 1
}
