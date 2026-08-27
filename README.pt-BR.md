<div align="center">
  <img src="assets/brand/morune-logo-system/morune-symbol-128.png" width="88" alt="Símbolo do MORU•NE">
  <h1>MORU•NE</h1>
  <p><strong>Your music. Your way.</strong></p>
  <p>Cliente de Spotify nativo, leve e profundamente customizável para Windows —<br>feito para tocar a sua música enquanto você joga.</p>

  <p>
    <a href="https://github.com/SeitiFurumori/morune/releases"><img src="https://img.shields.io/github/v/release/SeitiFurumori/morune?include_prereleases&amp;label=release" alt="Release"></a>
    <a href="https://github.com/SeitiFurumori/morune/actions/workflows/ci.yml"><img src="https://github.com/SeitiFurumori/morune/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-c05cff.svg" alt="Licença MIT"></a>
    <img src="https://img.shields.io/badge/platform-Windows-6b7280.svg" alt="Plataforma Windows">
    <img src="https://img.shields.io/badge/feito%20com-Rust%20%2B%20Slint-dea584.svg" alt="Rust + Slint">
  </p>

  <p><a href="README.md">🇬🇧 Read in English</a></p>
</div>

<div align="center">
  <img src="assets/screenshots/temas.png" width="880" alt="A mesma tela nos quatro temas: Bruma, Cristal, Pulse e Paper">
  <p><em>Um aplicativo, quatro temas. Repare no Paper (inferior direito): a barra lateral foi para a direita e o player para o topo — tema aqui muda composição, não só cor.</em></p>
</div>

> [!IMPORTANT]
> O MORU•NE está em **alpha**. Já dá para usar, mas ainda tem arestas.
> A reprodução exige uma conta Spotify Premium.

## Por que mais um cliente de Spotify?

O cliente oficial é um aplicativo Electron. Ele fica parado consumindo centenas
de megabytes, acorda a GPU para animar o que ninguém está olhando, e trava por
um segundo quando você faz alt-tab de volta. Quem ouve música jogando sente as
três coisas.

O MORU•NE é construído em torno de um único critério de desempenho: **ser
indistinguível de um processo parado enquanto você joga.** Sem roubar quadro,
sem acordar a GPU, sem disputar CPU e sem travar a janela quando você volta
para ela. Memória é teto para não estourar, não número de vitrine — o método e
as medições reais estão em [PERFORMANCE.md](docs/PERFORMANCE.md), inclusive o
que **ainda não** foi medido.

A segunda razão é o controle sobre a aparência. Quase todo "tema" por aí troca
as cores. Aqui um tema é um pacote declarativo que controla cor, tipografia,
forma, movimento, espaçamento, ícones, fontes e a composição da janela.

## O que ele tem de diferente

- **Nativo e leve** — Rust + [Slint](https://slint.dev). Sem Electron, sem
  Chromium, sem WebView. O instalador tem menos de 4 MB.
- **Feito para o segundo plano** — fecha para a bandeja e continua tocando, com
  menu e controle de volume para quando o jogo está em primeiro plano.
- **Temas que mudam o layout** — posição da barra lateral, posição do player,
  densidade e composição do conteúdo, não só as cores.
- **Seguro por construção** — temas são dados declarativos, nunca código
  executável. Seu token vive no Gerenciador de Credenciais do Windows, nunca
  num arquivo.
- **Acessível** — árvore AccessKit exposta ao Windows, navegação por teclado,
  foco visível e contraste garantido por teste.
- **O seu Spotify, não uma cópia dele** — reprodução, playlists e curtidas usam
  a sua conta. O MORU•NE nunca monta uma biblioteca paralela.

## Capturas

| Bruma (vidro acrílico) | Cristal |
|---|---|
| <img src="assets/screenshots/bruma.png" alt="Tema Bruma"> | <img src="assets/screenshots/cristal.png" alt="Tema Cristal"> |

| Pulse | Paper (barra lateral à direita) |
|---|---|
| <img src="assets/screenshots/pulse.png" alt="Tema Pulse"> | <img src="assets/screenshots/paper.png" alt="Tema Paper"> |

## Instalar

Baixe `Morune-<versão>-setup.exe` em
[Releases](https://github.com/SeitiFurumori/morune/releases) e execute. Não
pede UAC, e você escolhe o disco.

O instalador ainda não tem assinatura digital, então o SmartScreen pode exibir
"Editor desconhecido". Confira o SHA-256 contra o `.sha256` publicado na mesma
release antes de instalar:

```powershell
Get-FileHash .\Morune-0.1.0-setup.exe -Algorithm SHA256
```

Detalhes em [instalação, segurança e assinatura](docs/SIGNING.md).

### Primeiro uso

1. Abra o MORU•NE.
2. Vá em **Configurações → Entrar no Spotify**.
3. Autorize no navegador.
4. Escolha uma música e toque.

O MORU•NE nunca vê a sua senha. O Spotify autentica no navegador e o aplicativo
guarda apenas o token, no cofre do Windows.

## O que já funciona

- login no Spotify via OAuth com PKCE;
- busca por faixas, álbuns, artistas e playlists;
- reprodução, fila, aleatório, repetição, volume e teclas multimídia;
- músicas curtidas sincronizadas com o Spotify;
- biblioteca e playlists com carregamento progressivo;
- fila gerenciável: tocar a seguir, adicionar ao fim, mover, remover, limpar e
  desfazer;
- importar, exportar, pré-visualizar e restaurar temas `.musicpack`;
- mini-player, bandeja do sistema e início opcional com o Windows;
- navegação por teclado, foco visível e acessibilidade nativa via AccessKit;
- layout adaptativo até a janela mínima de 720 × 480.

O que vem a seguir está no [roadmap](docs/ROADMAP.md); as limitações atuais, na
[auditoria de UX](docs/UX_AUDIT.md).

## Temas

Um tema é um diretório ou um arquivo `.musicpack` feito de TOML e recursos:

```text
manifest.toml    identificação e versão do esquema
theme.toml       cor, tipografia, forma, movimento e efeitos
layout.toml      composição da janela
assets/          imagens e ícones opcionais
fonts/           fontes opcionais
```

Todo tema é validado ao carregar: caminho inseguro é recusado, valor absurdo é
limitado e contraste ilegível vira aviso. Um tema pode ser feio de propósito —
não pode ser acidentalmente impossível de ler.

Referência completa: [THEMING.md](docs/THEMING.md).

## Compilar do código

Requisitos: Windows, Rust 1.92+ e MinGW-w64 no `PATH` (`dlltool` e `as`).

```powershell
. .\tools\env.ps1
cargo build --release -p morune-app
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Para gerar o instalador localmente:

```powershell
.\tools\build-installer.ps1
```

Leia antes o [guia de contribuição](CONTRIBUTING.md). Arquitetura, decisões e
processo de publicação estão indexados em [docs/](docs/README.md).

## Licença e marcas

O MORU•NE é distribuído sob a [licença MIT](LICENSE). Avisos de terceiros estão
em [THIRD-PARTY-LICENSES.txt](THIRD-PARTY-LICENSES.txt).

O MORU•NE não é afiliado, associado nem endossado pelo Spotify. O streaming usa
[librespot](https://github.com/librespot-org/librespot) e exige conta Premium.
Spotify é marca da Spotify AB.
