# Gera assets/brand/morune.ico a partir dos PNGs do sistema de marca.
#
# O .ico e um artefato derivado, mas fica versionado: o build do executavel e o
# instalador precisam dele, e nenhum dos dois pode depender de ter .NET
# desenhando imagem na hora de compilar.
#
# Cada tamanho vem do desenho responsivo proprio (16, 32 e 128 sao arquivos
# distintos, nao reducoes do master -- ver assets/brand/.../SPECIFICATION.md).
# O 256 e a unica excecao: nao existe desenho nativo, entao ele e uma reducao
# de alta qualidade do master de 512.
#
# As entradas sao DIB de 32 bits, e nao PNG embutido: o NSIS recusa icones com
# entradas comprimidas em PNG nos tamanhos pequenos.

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$brand = Join-Path $PSScriptRoot '..\assets\brand\morune-logo-system'
$out   = Join-Path $PSScriptRoot '..\assets\brand\morune.ico'

function Get-Bitmap([string]$path, [int]$size) {
    $src = [System.Drawing.Image]::FromFile((Resolve-Path $path).Path)
    try {
        if ($src.Width -eq $size -and $src.Height -eq $size) {
            return (New-Object System.Drawing.Bitmap $src)
        }
        $bmp = New-Object System.Drawing.Bitmap $size, $size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $g.PixelOffsetMode   = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        $g.SmoothingMode     = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
        $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
        $g.Clear([System.Drawing.Color]::Transparent)
        $g.DrawImage($src, (New-Object System.Drawing.Rectangle 0, 0, $size, $size))
        $g.Dispose()
        return $bmp
    } finally { $src.Dispose() }
}

# Um DIB de icone: cabecalho com altura dobrada (XOR + AND), pixels BGRA de
# baixo para cima, e a mascara AND zerada porque a transparencia real esta no
# canal alfa.
function Get-Dib([System.Drawing.Bitmap]$bmp) {
    $w = $bmp.Width; $h = $bmp.Height
    $stream = New-Object System.IO.MemoryStream
    $bw = New-Object System.IO.BinaryWriter $stream

    $bw.Write([uint32]40)      # biSize
    $bw.Write([int32]$w)       # biWidth
    $bw.Write([int32]($h * 2)) # biHeight: XOR e AND empilhados
    $bw.Write([uint16]1)       # biPlanes
    $bw.Write([uint16]32)      # biBitCount
    $bw.Write([uint32]0)       # biCompression = BI_RGB
    $bw.Write([uint32]($w * $h * 4))
    $bw.Write([int32]0); $bw.Write([int32]0)
    $bw.Write([uint32]0); $bw.Write([uint32]0)

    $rect = New-Object System.Drawing.Rectangle 0, 0, $w, $h
    $data = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly,
                          [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    try {
        $row = New-Object byte[] ($w * 4)
        for ($y = $h - 1; $y -ge 0; $y--) {
            $ptr = [IntPtr]::Add($data.Scan0, $y * $data.Stride)
            [System.Runtime.InteropServices.Marshal]::Copy($ptr, $row, 0, $row.Length)
            $bw.Write($row)  # Format32bppArgb ja e BGRA em memoria
        }
    } finally { $bmp.UnlockBits($data) }

    $maskRow = [math]::Ceiling($w / 8.0)
    if ($maskRow % 4 -ne 0) { $maskRow += 4 - ($maskRow % 4) }
    $bw.Write((New-Object byte[] ($maskRow * $h)))

    $bw.Flush()
    return ,[byte[]]$stream.ToArray()
}

function Get-Png([System.Drawing.Bitmap]$bmp) {
    $stream = New-Object System.IO.MemoryStream
    $bmp.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
    return ,[byte[]]$stream.ToArray()
}

$sources = @(
    @{ Size = 16;  Path = "$brand\morune-symbol-16.png"  },
    @{ Size = 32;  Path = "$brand\morune-symbol-32.png"  },
    @{ Size = 128; Path = "$brand\morune-symbol-128.png" },
    @{ Size = 256; Path = "$brand\morune-symbol-512.png" }
)

$images = @()
foreach ($s in $sources) {
    $bmp = Get-Bitmap $s.Path $s.Size
    try {
        # 256 px vai comprimido em PNG: em DIB sozinho ele custaria 262 KB, que
        # e mais do que o executavel inteiro pode pagar por um icone. O Windows
        # le entradas PNG desde o Vista e o NSIS desde a 3.
        $bytes = if ($s.Size -ge 256) { Get-Png $bmp } else { Get-Dib $bmp }
        $images += ,@{ Size = $s.Size; Bytes = [byte[]]$bytes }
    } finally { $bmp.Dispose() }
}

$stream = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter $stream
$bw.Write([uint16]0); $bw.Write([uint16]1); $bw.Write([uint16]$images.Count)

$offset = 6 + 16 * $images.Count
foreach ($img in $images) {
    # 256 px se escreve como 0 no campo de um byte.
    $dim = if ($img.Size -ge 256) { 0 } else { $img.Size }
    $bw.Write([byte]$dim); $bw.Write([byte]$dim)
    $bw.Write([byte]0); $bw.Write([byte]0)
    $bw.Write([uint16]1); $bw.Write([uint16]32)
    $bw.Write([uint32]$img.Bytes.Length)
    $bw.Write([uint32]$offset)
    $offset += $img.Bytes.Length
}
foreach ($img in $images) { $bw.Write([byte[]]$img.Bytes, 0, $img.Bytes.Length) }
$bw.Flush()

[System.IO.File]::WriteAllBytes((Join-Path $PSScriptRoot '..\assets\brand\morune.ico'), $stream.ToArray())
Write-Host ("morune.ico: {0} entradas, {1:N0} bytes" -f $images.Count, $stream.Length)
