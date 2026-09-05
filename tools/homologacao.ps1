# Homologacao: percorre o Morune inteiro pelo mouse e pelo teclado.
#
# Diferente do `revisao-visual.ps1`, que so olha telas paradas, este exercita o
# COMPORTAMENTO: tocar, pausar, pular, curtir, buscar, abrir playlist, trocar
# tema, mexer no volume. Cada passo grava uma captura e o roteiro guarda o
# pedaco do log que o aplicativo escreveu naquele intervalo -- e a leitura dos
# dois juntos que diz se o passo funcionou.
#
# Nao afirma sozinho que passou. Ele produz a evidencia; quem julga le as
# capturas e o log. Um script que dissesse "OK" sem olhar pixel nenhum seria
# exatamente o instrumento cego que este arquivo existe para substituir.
#
# Uso:
#   .\tools\homologacao.ps1
#   .\tools\homologacao.ps1 -Tema bruma

param(
    [string]$Tema = "aquario",
    [string]$Size = "1919x1030",
    [string]$Exe = "$env:LOCALAPPDATA\Programs\Morune\morune.exe",
    [string]$OutDir = "$PSScriptRoot\..\bench-out\homologacao",
    [int]$WarmupSeconds = 9
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing
# SendKeys mora em System.Windows.Forms; sem carregar, os passos de teclado
# morrem no meio do roteiro em vez de na primeira linha.
Add-Type -AssemblyName System.Windows.Forms

Add-Type @"
using System;
using System.Runtime.InteropServices;
public struct RECT { public int Left, Top, Right, Bottom; }
public class HG {
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
    [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, IntPtr extra);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern short VkKeyScanW(char c);
}
"@

$LARGURA, $ALTURA = $Size.Split("x") | ForEach-Object { [int]$_ }
$logPath = Join-Path $env:APPDATA "morune\Morune\data\morune.log"

# O roteiro.
#
# Coordenadas em cliente, medidas nas capturas de 1903x1014 -- que e o que o DWM
# entrega quando se pede 1919x1030. `espera` e o tempo ATE a captura, e existe
# porque quase tudo aqui e assincrono: rede, decodificacao, animacao.
$PASSOS = @(
    @{ n = "01-abertura";        acao = "nada";                          espera = 1200 }
    @{ n = "02-tocar-faixa";     acao = "dblclick"; alvo = @(520, 119);  espera = 6000 }
    @{ n = "03-pausar";          acao = "click";    alvo = @(951, 947);  espera = 1500 }
    @{ n = "04-retomar";         acao = "click";    alvo = @(951, 947);  espera = 2500 }
    @{ n = "05-proxima";         acao = "click";    alvo = @(1003, 947); espera = 5000 }
    @{ n = "06-anterior";        acao = "click";    alvo = @(899, 947);  espera = 5000 }
    @{ n = "07-aleatorio";       acao = "click";    alvo = @(855, 947);  espera = 1200 }
    @{ n = "08-repetir";         acao = "click";    alvo = @(1048, 947); espera = 1200 }
    @{ n = "09-volume";          acao = "click";    alvo = @(1830, 964); espera = 1200 }
    @{ n = "10-curtir";          acao = "click";    alvo = @(1790, 178); espera = 2000 }
    @{ n = "11-descurtir";       acao = "click";    alvo = @(1790, 178); espera = 2000 }
    @{ n = "12-fila";            acao = "click";    alvo = @(1737, 964); espera = 1800 }
    # Sair da Fila pela barra lateral, e nao clicando de novo no botao da fila:
    # ele chama `navigate(4)`, que estando ja na Fila nao muda nada -- um passo
    # que nunca poderia falhar nao testa nada.
    @{ n = "13-sair-da-fila";    acao = "click";    alvo = @(90, 113);   espera = 2500 }
    @{ n = "14-buscar";          acao = "tecla";    vk = 0x4B; ctrl = $true;  espera = 2000 }
    @{ n = "15-digitar";         acao = "texto";    texto = "radiohead"; espera = 5000 }
    @{ n = "16-resultado-hover"; acao = "hover";    alvo = @(600, 200);  espera = 900 }
    @{ n = "17-biblioteca";      acao = "click";    alvo = @(90, 201);   espera = 3000 }
    @{ n = "18-playlist";        acao = "click";    alvo = @(110, 388);  espera = 4000 }
    @{ n = "19-detalhe-hover";   acao = "hover";    alvo = @(600, 400);  espera = 900 }
    @{ n = "20-tocar-detalhe";   acao = "dblclick"; alvo = @(600, 400);  espera = 5000 }
    @{ n = "21-config";          acao = "click";    alvo = @(90, 847);   espera = 2000 }
    # 0xBF e a tecla que carrega "/" no layout americano. Num teclado ABNT2 ela
    # nao e "/", entao este passo prova o atalho so em layout US -- se ele nao
    # abrir o dialogo, isso NAO e prova de defeito.
    @{ n = "22-atalhos";         acao = "tecla";    vk = 0xBF; ctrl = $true;  espera = 1500 }
    @{ n = "23-fechar-atalhos";  acao = "tecla";    vk = 0x1B;                espera = 1200 }
    @{ n = "24-inicio";          acao = "click";    alvo = @(90, 113);   espera = 2500 }
    @{ n = "25-hover-cartao";    acao = "hover";    alvo = @(350, 700);  espera = 900 }
)

function Get-Retangulo {
    param([IntPtr]$H)
    $r = New-Object RECT
    $tam = [System.Runtime.InteropServices.Marshal]::SizeOf($r)
    if ([HG]::DwmGetWindowAttribute($H, 9, [ref]$r, $tam) -ne 0) {
        [HG]::GetWindowRect($H, [ref]$r) | Out-Null
    }
    return $r
}

function Test-MesmoHwnd {
    param([IntPtr]$A, [IntPtr]$B)
    return $A.ToInt64() -eq $B.ToInt64()
}

function Set-Frente {
    param([IntPtr]$H)
    $meu = [HG]::GetCurrentThreadId()
    $dele = [HG]::GetWindowThreadProcessId([HG]::GetForegroundWindow(), [IntPtr]::Zero)
    [HG]::AttachThreadInput($meu, $dele, $true) | Out-Null
    [HG]::ShowWindow($H, 5) | Out-Null
    [HG]::SetForegroundWindow($H) | Out-Null
    [HG]::AttachThreadInput($meu, $dele, $false) | Out-Null
    Start-Sleep -Milliseconds 400
}

function Invoke-Clique {
    param([int]$X, [int]$Y, [int]$Vezes = 1)
    [HG]::SetCursorPos($X, $Y) | Out-Null
    Start-Sleep -Milliseconds 140
    for ($i = 0; $i -lt $Vezes; $i++) {
        [HG]::mouse_event(0x0002, 0, 0, 0, [IntPtr]::Zero)
        Start-Sleep -Milliseconds 50
        [HG]::mouse_event(0x0004, 0, 0, 0, [IntPtr]::Zero)
        if ($i -lt $Vezes - 1) { Start-Sleep -Milliseconds 90 }
    }
}

# Teclado por `keybd_event`, e nao por `SendKeys`.
#
# `SendKeys` posta mensagens na fila da janela em foco, e a janela do Slint nao
# as consumiu: na primeira rodada os passos de Ctrl+K, de digitacao e de Ctrl+/
# nao produziram mudanca nenhuma na tela -- e o roteiro seguiu adiante como se
# tivessem funcionado, que e pior do que falhar. `keybd_event` injeta no fluxo de
# entrada do sistema, um nivel abaixo, e chega em qualquer janela.
function Invoke-Tecla {
    param([byte]$Vk, [bool]$Ctrl = $false, [bool]$Shift = $false)
    if ($Ctrl) { [HG]::keybd_event(0x11, 0, 0, [IntPtr]::Zero) }
    if ($Shift) { [HG]::keybd_event(0x10, 0, 0, [IntPtr]::Zero) }
    [HG]::keybd_event($Vk, 0, 0, [IntPtr]::Zero)
    Start-Sleep -Milliseconds 30
    [HG]::keybd_event($Vk, 0, 2, [IntPtr]::Zero)
    if ($Shift) { [HG]::keybd_event(0x10, 0, 2, [IntPtr]::Zero) }
    if ($Ctrl) { [HG]::keybd_event(0x11, 0, 2, [IntPtr]::Zero) }
    Start-Sleep -Milliseconds 60
}

function Invoke-Texto {
    param([string]$Texto)
    foreach ($c in $Texto.ToCharArray()) {
        $m = [HG]::VkKeyScanW($c)
        if ($m -eq -1) { continue }
        Invoke-Tecla ([byte]($m -band 0xFF)) $false ((($m -shr 8) -band 1) -eq 1)
    }
}

# A captura NAO pede o primeiro plano.
#
# `Set-Frente` usa `AttachThreadInput` para poder chamar `SetForegroundWindow`, e
# amarrar/desamarrar filas de entrada entre dois processos a cada passo custa
# eventos: na primeira rodada, o clique em "Inicio" logo depois de uma captura
# sumia, e o roteiro seguia como se a navegacao do aplicativo estivesse quebrada.
# Tres reproducoes isoladas -- os mesmos cliques, sem captura entre eles --
# navegaram sempre. O defeito era do instrumento, e o instrumento estava
# acusando o aplicativo, que e o pior modo de errar.
#
# A janela ja foi trazida a frente uma vez, no comeco. Se ela perdeu o primeiro
# plano, o certo e avisar e pular: capturar a janela de outro programa seria pior
# que nao capturar.
function Save-Captura {
    param([IntPtr]$H, [string]$Caminho)
    if (-not (Test-MesmoHwnd ([HG]::GetForegroundWindow()) $H)) {
        Write-Warning "sem primeiro plano; captura pulada: $Caminho"
        return
    }
    $r = Get-Retangulo $H
    $w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size($w, $h)))
    $g.Dispose()
    $bmp.Save($Caminho, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

if (-not (Test-Path $Exe)) { Write-Error "Executavel nao encontrado: $Exe" }
if (Test-Path $OutDir) { Remove-Item "$OutDir\*" -Force -ErrorAction SilentlyContinue }
else { New-Item -ItemType Directory -Force $OutDir | Out-Null }

$configPath = Join-Path $env:APPDATA "morune\Morune\config\config.toml"
$backup = Get-Content $configPath -Raw
$padrao = '(?m)^theme\s*=\s*"[^"]*"'
if (-not [regex]::IsMatch($backup, $padrao)) { Write-Error "nao achei a linha 'theme =' no config" }

try {
    foreach ($p in @(Get-Process -Name morune -ErrorAction SilentlyContinue)) {
        try { $p.Kill() } catch { }
        $p.WaitForExit(5000) | Out-Null
    }
    Start-Sleep -Milliseconds 500

    Set-Content $configPath ($backup -replace $padrao, "theme = `"$Tema`"") -Encoding utf8

    # Marca onde o log estava: tudo depois disto e desta rodada.
    $logAntes = if (Test-Path $logPath) { (Get-Item $logPath).Length } else { 0 }

    $proc = Start-Process -FilePath $Exe -PassThru
    Start-Sleep -Seconds $WarmupSeconds
    $proc.Refresh()
    if ($proc.HasExited) { Write-Error "o aplicativo saiu antes de comecar (codigo $($proc.ExitCode))" }
    if ($proc.MainWindowHandle -eq 0) { Write-Error "a janela principal nao apareceu" }
    $hwnd = $proc.MainWindowHandle

    Set-Frente $hwnd
    [HG]::SetWindowPos($hwnd, [IntPtr]::Zero, 0, 0, $LARGURA, $ALTURA, 0x0040) | Out-Null
    Start-Sleep -Milliseconds 1200
    $r = Get-Retangulo $hwnd
    Write-Host ("janela {0}x{1}" -f ($r.Right - $r.Left), ($r.Bottom - $r.Top)) -ForegroundColor DarkGray

    $marcas = @()
    foreach ($passo in $PASSOS) {
        $t0 = (Get-Date).ToString("HH:mm:ss.fff")
        switch ($passo.acao) {
            "click"    { Invoke-Clique ($r.Left + $passo.alvo[0]) ($r.Top + $passo.alvo[1]) 1 }
            "dblclick" { Invoke-Clique ($r.Left + $passo.alvo[0]) ($r.Top + $passo.alvo[1]) 2 }
            "hover"    { [HG]::SetCursorPos(($r.Left + $passo.alvo[0]), ($r.Top + $passo.alvo[1])) | Out-Null }
            "tecla"    { Invoke-Tecla $passo.vk ([bool]$passo.ctrl) $false }
            "texto"    { Invoke-Texto $passo.texto }
            "nada"     { }
        }
        Start-Sleep -Milliseconds $passo.espera
        Save-Captura $hwnd (Join-Path $OutDir "$($passo.n).png")
        $marcas += "$t0  $($passo.n)  ($($passo.acao))"
        Write-Host "  $($passo.n)" -ForegroundColor Green
    }

    $marcas | Set-Content (Join-Path $OutDir "roteiro.txt") -Encoding utf8

    # O log desta rodada, e so ele.
    $fs = [System.IO.File]::Open($logPath, 'Open', 'Read', 'ReadWrite')
    $fs.Seek($logAntes, 'Begin') | Out-Null
    $sr = New-Object System.IO.StreamReader($fs)
    $sr.ReadToEnd() | Set-Content (Join-Path $OutDir "log.txt") -Encoding utf8
    $sr.Close(); $fs.Close()

    try { $proc.Kill() } catch { }
    $proc.WaitForExit(5000) | Out-Null
}
finally {
    Set-Content $configPath $backup -Encoding utf8
    Write-Host "configuracao restaurada" -ForegroundColor DarkGray
}
