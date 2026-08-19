# Ambiente de build local do Morune (Windows, toolchain GNU).
#
# A toolchain fica em D: porque C: tem pouco espaco livre nesta maquina.
# O diretorio do MinGW precisa estar no PATH: rustc chama `dlltool` e `as`
# ao gerar bibliotecas de importacao para o target x86_64-pc-windows-gnu.
$env:RUSTUP_HOME = "D:\rust\.rustup"
$env:CARGO_HOME  = "D:\rust\.cargo"
$env:PATH        = "D:\rust\winlibs\mingw64\bin;D:\rust\.cargo\bin;$env:PATH"
