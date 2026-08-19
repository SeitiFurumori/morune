# Morune

Cliente de musica nativo para Windows, escrito em Rust. Leve, rapido e
profundamente customizavel.

O objetivo nao e ser mais um player: e ser um player que **o usuario molda**.
Cor, tipografia, forma, movimento e a propria composicao da tela vem de um
pacote de tema declarativo que pode ser importado, exportado e editado sem
recompilar nada.

Sem Electron. Sem Chromium. Sem WebView.

---

## Estado atual

Versao `0.1.0` — MVP executavel. O que ja funciona de verdade, medido nesta
maquina (AMD Ryzen 5 8500G, Windows 11 26200, Radeon 740M):

| | |
|---|---|
| Instalador `.exe` | **4,00 MB** |
| Executavel de release | **9,39 MB** |
| Startup ate o laco de eventos | **18 ms** (mediana de 8 execucoes) |
| Ciclo completo do processo | **1,01 s** (inclui criar contexto OpenGL e encerrar) |
| Working set em repouso | **70,6 MB** (media), pico 71,3 MB |
| CPU em repouso | **0,00%** de um nucleo |
| CPU na bandeja, janela oculta | **0,00%** de um nucleo |
| Testes | 134 passando, `clippy -D warnings` limpo |

O criterio de desempenho do Morune nao e "usar pouca RAM" — e **nao atrapalhar
quem esta jogando**. O que manda e CPU e GPU em segundo plano e a interface
nunca travar, e isso so pode ser medido com reproducao real. Ver
[PERFORMANCE.md](PERFORMANCE.md).

Ja implementado:

- interface nativa completa (barra lateral, paginas, player, fila,
  configuracoes) com **todo pixel derivado do tema**;
- motor de customizacao: esquema versionado, carregamento com fallback seguro,
  heranca entre temas, importacao/exportacao `.musicpack` com defesa contra
  travessia de caminho e bomba de compressao;
- tres temas com aparencia e **composicao** diferentes, trocaveis em execucao;
- configuracao persistente com gravacao atomica e recuperacao de arquivo
  corrompido;
- cofre de credenciais no Gerenciador de Credenciais do Windows;
- fila com shuffle, repeticao, historico real e "tocar a seguir", coberta por
  testes;
- **fechar continua rodando**: a janela some para a bandeja e o aplicativo segue
  vivo, como no Discord. Verificado por script, nao so afirmado;
- **identidade visual propria**: o simbolo da marca aparece na barra lateral, no
  executavel, na janela, na barra de tarefas, na bandeja e no instalador;
- **instalador `.exe` unico** com escolha de disco, sem UAC e sem pre-requisitos.

Escrito, compilando e coberto por teste de unidade, mas **ainda nao exercitado
contra uma conta real** — ver [docs/HANDOFF.md](docs/HANDOFF.md):

- login no Spotify por OAuth com PKCE, sem senha e sem client secret, com o
  token no Gerenciador de Credenciais;
- reproducao sobre a librespot, ligada a fila que ja existia;
- busca, playlists, albuns salvos e artistas seguidos nas telas que ja existiam.

Ainda **nao** implementado — ver [ROADMAP.md](ROADMAP.md):

- capas: nem download nem cache;
- paginacao das telas de conteudo: cada secao traz 50 itens e para;
- telas de detalhe de album, artista e playlist.

Nada aqui e declarado funcionando sem medicao ou teste. Onde falta, esta dito.

---

## Como instalar

Baixe `Morune-<versao>-setup.exe` e execute. Um arquivo so, sem runtime, sem
.NET, sem Visual C++ Redistributable.

**O Windows vai avisar.** O instalador ainda nao e assinado, entao o SmartScreen
mostra "O Windows protegeu o computador" e chama o editor de desconhecido. Nao e
falso positivo: e o comportamento correto para um executavel que ninguem
assinou. Clique em **Mais informacoes → Executar assim mesmo**, ou confira antes
que o arquivo e mesmo o publicado:

```powershell
Get-FileHash .\Morune-0.1.0-setup.exe -Algorithm SHA256
```

O resultado tem de bater com o `.sha256` distribuido junto. Os caminhos para o
instalador passar a ser assinado — dois deles gratuitos — estao em
[docs/assinatura.md](docs/assinatura.md).

O instalador pergunta **em qual disco e em qual pasta** instalar, mostrando o
espaco livre do disco escolhido. Nao pede elevacao: a instalacao e por usuario,
o que e justamente o que torna a escolha de disco livre de verdade — com
instalacao por maquina, apontar para um disco secundario ainda dispararia UAC.

Suas configuracoes e temas ficam sempre em `%APPDATA%\morune`, independente do
disco escolhido, e **nao** sao apagados ao desinstalar, a menos que voce marque
essa opcao.

Para gerar o instalador a partir do codigo:

```bash
.\tools\build-installer.ps1
```

O script recusa empacotar se o executavel nao rodar isolado, com `PATH` reduzido
ao Windows — a garantia de que ninguem vai instalar um aplicativo que nao abre.

## Comportamento ao fechar

Fechar a janela **nao** encerra o Morune: ele vai para a bandeja e continua
tocando, como o Discord. O icone da bandeja tem menu com abrir, tocar/pausar,
anterior, proxima e sair, e clique duplo traz a janela de volta.

Quem prefere o contrario desliga em Configuracoes → Comportamento.

Verificacao automatizada disso (fecha a janela via `WM_CLOSE` e confere que o
processo sobreviveu):

```bash
.\tools\verify-tray.ps1
```

## Como compilar

Requisitos: Rust 1.85+ e, no Windows, uma toolchain MinGW-w64 no `PATH`
(`dlltool` e `as` sao chamados por `rustc` ao gerar bibliotecas de importacao
para o alvo `x86_64-pc-windows-gnu`).

```bash
cargo build --release -p morune-app
```

Nesta maquina o ambiente ja esta preparado; carregue-o antes de qualquer
comando `cargo`:

```bash
. .\tools\env.ps1
```

O binario sai em `target/release/morune.exe`.

### Verificacao

```bash
cargo test --workspace --features morune-theme/hot-reload
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

Medicao real de tamanho, startup e memoria:

```bash
.\tools\measure.ps1
```

Captura de cada tema em PNG, direto do renderizador do Slint:

```bash
.\tools\snapshot.ps1
```

---

## Customizacao

Um tema e um diretorio (ou um `.musicpack`, que e esse diretorio compactado):

```
manifest.toml    identificacao e versao do esquema
theme.toml       cor, tipografia, forma, movimento, efeitos
layout.toml      composicao da tela
assets/          imagens e icones do tema
fonts/           fontes empacotadas
```

Tudo e **declarativo**. Um tema nao contem e nao pode conter codigo executavel —
essa e uma decisao de seguranca permanente, explicada em [SECURITY.md](SECURITY.md).

O que um tema alcanca vai bem alem de cor: posicao da barra lateral, posicao e
altura do player, grade ou lista, densidade, raios, escala tipografica, duracao
das animacoes, transparencia da janela. Os temas `midnight` e `paper` que
acompanham o aplicativo diferem em todos esses eixos, de proposito.

Detalhes e referencia completa de campos: [THEMING.md](THEMING.md).

Pasta de temas do usuario:

```
%APPDATA%\morune\Morune\config\themes\
```

---

## Documentacao

| Arquivo | Conteudo |
|---|---|
| [docs/HANDOFF.md](docs/HANDOFF.md) | **comece aqui**: estado atual, proximo passo, divida tecnica |
| [ARCHITECTURE.md](ARCHITECTURE.md) | camadas, contratos e por que existem |
| [THEMING.md](THEMING.md) | formato de tema e referencia de campos |
| [SECURITY.md](SECURITY.md) | modelo de ameaca, credenciais, validacao de pacotes |
| [docs/assinatura.md](docs/assinatura.md) | por que o SmartScreen avisa, o que custa assinar |
| [PERFORMANCE.md](PERFORMANCE.md) | orcamento, metodo de medicao e numeros reais |
| [ROADMAP.md](ROADMAP.md) | o que vem, em que ordem e por que |
| [CHANGELOG.md](CHANGELOG.md) | historico de versoes |
| [docs/adr/](docs/adr/) | decisoes arquiteturais e seus motivos |

---

## Licenca

[MIT](LICENSE). Use, modifique e redistribua a vontade; so mantenha o aviso de
copyright.

O executavel nao e MIT por inteiro: ele carrega 339 bibliotecas de codigo
aberto, e varias exigem que o aviso de copyright delas acompanhe a distribuicao.
Os avisos vao em [THIRD-PARTY-LICENSES.txt](THIRD-PARTY-LICENSES.txt), gerado
por `tools/licenses.ps1` e instalado junto do aplicativo.

Entre elas, o **Slint** e usado sob a licenca royalty-free, e nao sob a GPL — o
porque esta na [ADR-0005](docs/adr/0005-licenca-e-slint-royalty-free.md).

<a href="https://slint.dev">
  <img alt="#MadeWithSlint" src="https://github.com/slint-ui/slint/raw/master/logo/MadeWithSlint-logo-light.svg" width="140">
</a>

Morune nao e afiliado ao Spotify. O backend de streaming depende de
[librespot](https://github.com/librespot-org/librespot) e exige conta Premium.
