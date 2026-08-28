# Publicacao de releases

O instalador do MORU•NE e compilado pelo GitHub Actions. Binarios nao entram no
historico Git: o workflow `.github/workflows/release.yml` os anexa diretamente
a uma GitHub Release.

## Publicar uma versao de teste

1. Atualize `workspace.package.version` no `Cargo.toml` e o `Cargo.lock`.
2. Confirme localmente os testes e o instalador.
3. Envie o commit para `main`.
4. Crie e envie uma tag anotada com o mesmo numero-base:

```powershell
git tag -a v0.1.0-alpha.1 -m "MORU•NE 0.1.0 alpha 1"
git push origin v0.1.0-alpha.1
```

O sufixo `-alpha.1`, `-beta.1` ou `-rc.1` faz a publicacao ser marcada como
pre-release. Uma tag sem sufixo, como `v0.1.0`, vira a release estavel mais
recente.

O workflow faz, nesta ordem:

1. valida a tag contra a versao do workspace;
2. instala Rust 1.92, MinGW e NSIS num runner Windows;
3. compila em release com o alvo GNU, o mesmo validado no build local;
4. verifica se o executavel abre sem DLLs externas;
5. gera o instalador e seu SHA-256;
6. publica os dois arquivos na release correspondente.

Os testes nao estao nessa lista de proposito: rodam em
`.github/workflows/ci.yml`, a cada push em `main`, e portanto ja passaram no
commit que recebe a tag. Compilar o grafo inteiro em debug so para roda-los
custava 44 minutos dentro da publicacao.

Se qualquer etapa falhar, nenhuma release e publicada. Os arquivos do build
ficam disponiveis por 14 dias na execucao do Actions para diagnostico.

## Ensaiar sem gastar uma tag

Uma tag e definitiva, e um erro de empacotamento so aparece depois de vinte
minutos de compilacao. Para exercitar o mesmo caminho sem publicar nada, va em
**Actions -> Publicar instalador -> Run workflow**, mantenha `dry_run` ligado e
escolha `main`. O ensaio compila, gera o instalador, verifica o isolamento do
executavel e anexa os arquivos a execucao -- sem criar release.

O mesmo botao serve para consertar uma publicacao que morreu no ultimo passo:
escolha a **tag** ja existente em "Use workflow from", desligue `dry_run`, e os
arquivos sao anexados por cima da release daquela versao, sem queimar um numero
de versao novo. Publicar a partir de um branch e recusado logo no inicio, antes
de qualquer compilacao.

## O botao "Procurar atualizacoes"

A tela de configuracoes verifica, baixa e instala a versao seguinte sem que
ninguem precise voltar ao navegador. Tres coisas do processo de publicacao
sustentam isso, e quebrar qualquer uma delas quebra o botao:

**1. A tag entra no binario.** O workflow passa `MORUNE_RELEASE_TAG` ao
`cargo build`, e o `build.rs` a grava no executavel. Sem ela o binario se
apresentaria pela versao do `Cargo.toml` -- `0.1.0`, sem o sufixo de
pre-lancamento --, e como `0.1.0` e *mais nova* que qualquer `0.1.0-alpha.N`,
todo mundo receberia "voce ja esta na versao mais recente" para sempre.

**2. Os dois arquivos precisam estar na release.** O `.exe` e o `.sha256` ao
lado dele. O aplicativo confere o hash antes de executar qualquer coisa e
recusa o arquivo se nao bater. Uma release sem o `.sha256` e, para o botao, uma
release sem instalador.

**3. Pre-lancamento so alcanca quem ja esta num.** Quem instalou um alpha
recebe o alpha seguinte; quem instalou uma versao final so recebe versoes
finais. Publicar `v0.2.0-alpha.1` nao mexe com ninguem que esteja num `v0.1.0`.

Para conferir o caminho inteiro contra a API de verdade, sem interface:

```
cargo test -p morune-app -- --ignored --nocapture atualizacao_de_verdade
```

O instalador ganhou a opcao `/RESTART` para isso: com `/S /RESTART` ele espera
o Morune sair, instala em silencio e reabre o aplicativo no fim. `/S` sozinho
continua sendo uma instalacao silenciosa comum, que nao abre janela nenhuma.

## Acesso dos testadores

Enquanto o repositorio for privado, somente pessoas com acesso ao repositorio
conseguem abrir a release. Em um repositorio publico, a release tambem e
publica: o GitHub nao oferece release publica "somente por link".

## Assinatura digital

Sem um certificado configurado, o workflow publica o instalador sem assinatura
e as notas alertam sobre o SmartScreen. O fluxo de assinatura ja existe em
`tools/sign.ps1`; antes de configurar uma chave no GitHub, siga
`docs/SIGNING.md` e use secrets, nunca arquivos ou senhas versionados.
