<div align="center">
  <img src="assets/brand/morune-logo-system/morune-symbol-128.png" width="88" alt="Símbolo do MORU•NE">
  <h1>MORU•NE</h1>
  <p><strong>Your music. Your way.</strong></p>
  <p>Cliente de música nativo, leve e profundamente customizável para Windows.</p>

  <p>
    <a href="https://github.com/SeitiFurumori/morune/releases"><img src="https://img.shields.io/github/v/release/SeitiFurumori/morune?include_prereleases&amp;label=release" alt="Release"></a>
    <a href="https://github.com/SeitiFurumori/morune/actions/workflows/release.yml"><img src="https://github.com/SeitiFurumori/morune/actions/workflows/release.yml/badge.svg" alt="Build"></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-c05cff.svg" alt="Licença MIT"></a>
    <img src="https://img.shields.io/badge/platform-Windows-6b7280.svg" alt="Plataforma Windows">
  </p>
</div>

> [!IMPORTANT]
> O MORU•NE está em **alpha**. Ele já pode ser usado por testadores, mas ainda
> pode apresentar falhas. A reprodução exige uma conta Spotify Premium.

## Baixar

Acesse [Releases](https://github.com/SeitiFurumori/morune/releases), baixe
`Morune-<versão>-setup.exe` e execute o instalador.

O instalador ainda não possui assinatura digital. Por isso, o Windows
SmartScreen pode exibir “Editor desconhecido”. Compare o SHA-256 do arquivo com
o `.sha256` publicado na mesma release antes de instalar:

```powershell
Get-FileHash .\Morune-0.1.0-setup.exe -Algorithm SHA256
```

Detalhes: [instalação, segurança e assinatura digital](docs/SIGNING.md).

## Por que MORU•NE?

- **Nativo e leve:** Rust + Slint, sem Electron, Chromium ou WebView.
- **Familiar:** busca, biblioteca, fila e player seguem padrões conhecidos de
  aplicativos de música.
- **Customizável de verdade:** temas alteram cores, tipografia, formas,
  movimento e composição da interface.
- **Seguro por construção:** temas são dados declarativos, nunca código
  executável; tokens ficam no Gerenciador de Credenciais do Windows.
- **Integrado ao Spotify:** reprodução, playlists e músicas curtidas usam a
  conta do usuário, sem criar uma biblioteca paralela no MORU•NE.

## O que já funciona

- login Spotify via OAuth com PKCE;
- busca por faixas, álbuns, artistas e playlists;
- reprodução, fila, shuffle, repetição, volume e teclas multimídia;
- músicas curtidas sincronizadas com o Spotify;
- playlists recentes e biblioteca com carregamento progressivo;
- importação, exportação, preview e restauração de temas `.musicpack`;
- mini-player, bandeja do sistema e inicialização opcional com o Windows;
- navegação por teclado, foco visível e acessibilidade nativa via AccessKit;
- layout adaptativo até a janela mínima de 720 × 480.

Veja o que está planejado no [roadmap](docs/ROADMAP.md) e as limitações atuais
na [auditoria de UX](docs/UX_AUDIT.md).

## Primeiros passos

1. Instale e abra o MORU•NE.
2. Vá a **Configurações → Entrar no Spotify**.
3. Autorize o acesso no navegador.
4. Escolha uma música e pressione **Tocar**.

O MORU•NE nunca recebe sua senha. O Spotify autentica a conta no navegador e o
aplicativo guarda apenas o token no cofre do Windows. Ao fechar a janela, o app
continua na bandeja por padrão; esse comportamento pode ser alterado em
**Configurações → Comportamento**.

## Customização

Um tema é um diretório ou arquivo `.musicpack` composto por TOML e recursos:

```text
manifest.toml    identificação e versão do esquema
theme.toml       cor, tipografia, forma, movimento e efeitos
layout.toml      composição da interface
assets/          imagens e ícones opcionais
fonts/           fontes opcionais
```

Consulte a [referência completa de temas](docs/THEMING.md).

## Desenvolvimento

Requisitos: Windows, Rust 1.92+ e MinGW-w64 no `PATH` (`dlltool` e `as`).

```powershell
. .\tools\env.ps1
cargo build --release -p morune-app
cargo test --workspace --features morune-theme/hot-reload
cargo clippy --workspace --all-targets -- -D warnings
```

Para gerar o instalador localmente:

```powershell
.\tools\build-installer.ps1
```

Antes de contribuir, leia o [guia de contribuição](CONTRIBUTING.md). A visão
técnica, decisões arquiteturais e processo de release estão no
[índice de documentação](docs/README.md).

## Licença e marcas

O código do MORU•NE é distribuído sob a [licença MIT](LICENSE). Dependências e
avisos de terceiros estão em [THIRD-PARTY-LICENSES.txt](THIRD-PARTY-LICENSES.txt).

MORU•NE não é afiliado, associado ou endossado pelo Spotify. O backend de
streaming usa [librespot](https://github.com/librespot-org/librespot) e exige
uma conta Premium.
