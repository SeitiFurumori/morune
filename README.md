# Morune

Cliente de musica nativo para Windows, escrito em Rust. Leve, rapido e
profundamente customizavel.

O objetivo nao e ser mais um player: e ser um player que **o usuario molda**.
Cor, tipografia, forma, movimento e a propria composicao da tela vem de um
pacote de tema declarativo que pode ser importado, exportado e editado sem
recompilar nada.

Premissa de produto: **Your music. Your way.** A experiencia basica permanece
simples; liberdade e profundidade aparecem quando o usuario decide procura-las.
O coracao segue o modelo mental conhecido: ele altera as Musicas curtidas da
conta Spotify, sem criar uma segunda biblioteca concorrente no Morune.

Sem Electron. Sem Chromium. Sem WebView.

---

## Estado atual

Versao `0.1.0` — MVP executavel. O que ja funciona de verdade, medido nesta
maquina (AMD Ryzen 5 8500G, Windows 11 26200, Radeon 740M):

| | |
|---|---|
| Instalador `.exe` | **5,31 MB** |
| Executavel de release | **13,78 MiB** |
| Startup ate o laco de eventos | **19 ms** (mediana de 10 execucoes) |
| Ciclo completo do processo | **509 ms** (mediana; inclui criar contexto OpenGL e encerrar) |
| Working set em repouso | **78,8 MB** (media), pico 79,5 MB |
| CPU em repouso | **0,14%** de um nucleo |
| CPU na bandeja, janela oculta | **0,00%** de um nucleo |
| Testes | 224, `clippy -D warnings` limpo |

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
- curtidas sincronizadas com a biblioteca Spotify pelo player e pelas listas,
  com confirmacao remota antes de alterar o estado visual;
- cofre de credenciais no Gerenciador de Credenciais do Windows;
- fila com shuffle, repeticao, historico real, tocar a seguir, adicionar ao fim,
  reordenar, remover e limpar, coberta por testes;
- **fechar continua rodando**: a janela some para a bandeja e o aplicativo segue
  vivo, como no Discord. Verificado por script, nao so afirmado;
- **identidade visual propria**: o simbolo da marca aparece na barra lateral, no
  executavel, na janela, na barra de tarefas, na bandeja e no instalador;
- **acessibilidade nativa do Windows**: controles expostos via AccessKit, foco
  visivel, operacao por Enter/Espaco e sliders por setas;
- **layout adaptativo**: grades reorganizam colunas durante o redimensionamento,
  inclusive na janela minima de 720x480;
- **instalador `.exe` unico** com escolha de disco, sem UAC e sem pre-requisitos.
- inicializacao opcional com o Windows, configuravel tanto no instalador quanto
  no aplicativo; quando automatica, abre discretamente na bandeja.

Verificado contra uma conta Spotify Premium real em 19/08/2026 — ver
[docs/HANDOFF.md](docs/HANDOFF.md):

- login no Spotify por OAuth com PKCE, sem senha e sem client secret, com o
  token no Gerenciador de Credenciais;
- reproducao sobre a librespot, ligada a fila que ja existia;
- busca por faixas, albuns, artistas e playlists; musicas curtidas e artistas
  seguidos;
- coracao sincronizado com as Musicas curtidas do Spotify pelo caminho
  autenticado usado pelo web player;
- capas com cache LRU de 48 MB;
- playlists abertas recentemente no topo da barra lateral, com historico local
  persistente; telas de detalhe com filtro e ordenacao.

As playlists que o Spotify monta para a conta — Descobertas da Semana, Radar de
Novidades — nao sao mais acessiveis pelo Web API desde a
[mudanca de novembro de 2024](https://developer.spotify.com/blog/2024-11-27-changes-to-the-web-api).
O Morune as busca pelo mesmo protocolo interno que ja usa para tocar.

Implementado depois dessa verificacao e aguardando uma nova rodada na conta
real:

- radio e autoplay configuravel quando a fila acaba;
- capas nas linhas de faixa;
- discografia e faixas populares na tela de artista;
- aviso quando uma lista e limitada as primeiras 200 faixas.

Ainda sem caminho conhecido no protocolo acessivel: albuns salvos, historico
recente e estatisticas de mais ouvidos. Essas secoes ficam escondidas em vez de
mostrar dados inventados.

Nada aqui e declarado funcionando sem medicao ou teste. Onde falta, esta dito.
A auditoria de produto e usabilidade, com severidades, evidencias e pendencias,
esta em [UX_AUDIT.md](UX_AUDIT.md).

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

Na tela de componentes, **Abrir com o Windows** e opcional e vem desmarcado. A
mesma escolha pode ser alterada depois em **Configuracoes → Iniciar com o
Windows**, sem reinstalar e sem permissao de administrador.

Suas configuracoes e temas ficam sempre em `%APPDATA%\morune`, independente do
disco escolhido, e **nao** sao apagados ao desinstalar, a menos que voce marque
essa opcao.

---

## Como entrar na sua conta

Abra **Configuracoes → Entrar no Spotify**. O navegador abre no site do Spotify,
voce autoriza, e pronto. O Morune nunca ve sua senha: o login e OAuth com PKCE,
e o unico segredo que chega aqui e um token, guardado no Gerenciador de
Credenciais do Windows. Nao ha cadastro, nao ha servidor do Morune no meio, e
nao ha nada para configurar antes.

**Precisa ser Premium.** Nao e escolha do Morune: o Spotify so entrega audio
para contas Premium, e nenhum cliente aberto contorna isso. Uma conta gratuita e
recusada no login, com essa explicacao na tela.

**Se o navegador nao abrir sozinho**, o login espera a autorizacao em
`127.0.0.1:5588`. Essa porta precisa estar livre, porque e nela que o Spotify
devolve a resposta — o endereco de retorno e fixo e nao pode ser sorteado.

Voce nao precisa registrar nenhum aplicativo no Spotify para usar o Morune, nem
para compila-lo.

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
| [docs/RELEASING.md](docs/RELEASING.md) | como uma tag gera e publica o instalador pelo GitHub Actions |
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
