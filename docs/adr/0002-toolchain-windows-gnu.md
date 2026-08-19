# ADR-0002 — Toolchain GNU no Windows durante o desenvolvimento

Data: 2026-08-18
Status: aceito, com migracao prevista

## Contexto

A maquina de desenvolvimento nao tinha Rust, MSVC, Windows SDK nem cmake, e o
disco `C:` tinha 18,9 GB livres. O pedido explicito foi "quero que seja leve".

Duas rotas:

**MSVC.** Alvo padrao de release no Windows: binarios menores, sem DLLs de
runtime ao lado do executavel, compatibilidade maxima com crates que trazem
C/C++. Exige VS Build Tools mais Windows SDK, cerca de 5–8 GB, boa parte
obrigatoriamente em `C:`.

**GNU (`x86_64-pc-windows-gnu`).** Toolchain do rustup em ~825 MB, sem Visual
Studio. Alvo de segunda classe no Windows.

## Decisao

Toolchain GNU, instalada em `D:\rust` com `RUSTUP_HOME` e `CARGO_HOME`
redirecionados. MSVC fica como alvo de release, no ciclo 4 do roteiro.

A escolha foi validada por medicao, nao por suposicao: um crate de sondagem com
`librespot 0.8`, `slint 1.17`, `symphonia`, `zip`, `notify`, `rusqlite` com
SQLite embutido e a crate `windows` compila inteiro nesta toolchain.

## O que a sondagem custou

Tres problemas reais apareceram, e nenhum deles teria aparecido em MSVC:

**1. `dlltool.exe` nao encontrado.** O `rustc` chama `dlltool` ao gerar
bibliotecas de importacao para crates que usam `raw-dylib` (`windows-sys`). O
binario existe dentro da toolchain, em
`lib/rustlib/x86_64-pc-windows-gnu/bin/self-contained`, mas nao entra no `PATH`.

**2. `dlltool` falhando mesmo encontrado.** Ele invoca `as` internamente, e o
diretorio `self-contained` nao tem um. Resolvido instalando WinLibs
(MinGW-w64 GCC 14.1, ~923 MB) e colocando seu `bin` no `PATH`. Total do ambiente
fica em ~1,8 GB, contra 5–8 GB do MSVC.

**3. `librespot-core 0.8.0` nao compila com `vergen 9.1.0`.** O `vergen-gitcl`
que ele usa depende de `vergen-lib 0.1`, enquanto `vergen 9.1.0` passou a usar
`vergen-lib 9.1`; as duas versoes coexistem e o `build.rs` quebra num limite de
trait. Resolvido fixando `vergen` em 9.0.6 no `Cargo.lock`. **Essa fixacao
precisa sobreviver a qualquer `cargo update`.**

## Consequencias

O ambiente ficou 4× menor e o MVP compila hoje. Em troca:

- o alvo tem menos rodagem no Windows, e problemas como os tres acima devem ser
  esperados ao adicionar dependencias com C;
- `tools/env.ps1` e obrigatorio antes de qualquer `cargo`, e o `PATH` do MinGW
  precisa estar la.

Uma preocupacao que **nao** se confirmou: esperava-se ter de distribuir
`libgcc_s_seh-1.dll` e `libwinpthread-1.dll` ao lado do executavel. O binario
gerado roda isolado, verificado copiando o `.exe` sozinho para uma pasta vazia e
executando com `PATH` reduzido a `%SystemRoot%\system32`. Essa checagem virou
etapa obrigatoria de `tools/build-installer.ps1`, para que uma dependencia nova
que quebre isso apareca na hora de empacotar e nao na maquina de quem instalou.

A migracao para MSVC e uma troca de toolchain no rustup mais uma recompilacao;
o codigo nao muda. Ela continua valendo pelo tamanho do binario e pela rodagem
do alvo, mas deixou de ser bloqueante para o primeiro instalador.
