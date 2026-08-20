# Estado do projeto e proximo passo

Documento de retomada. Escrito em 18/08/2026 ao fim do ciclo 1, revisto em
19/08/2026 quando o backend de Spotify ficou de pe, e de novo no mesmo dia
quando ele **precisou ser refeito** -- ver a secao do Web API.

Quem chegar aqui numa sessao nova deve ler este arquivo primeiro, depois
[ARCHITECTURE.md](../ARCHITECTURE.md) e [ROADMAP.md](../ROADMAP.md).

---

## Onde o projeto esta

Ciclo 1 concluido e verificado. Ciclo 2 **verificado contra uma conta Premium
real** em 19/08/2026, depois de o Web API do Spotify parar de responder e o
catalogo inteiro ser reescrito sobre o protocolo interno.

| | |
|---|---|
| Instalador | 5,31 MB, `dist/Morune-0.1.0-setup.exe` |
| Executavel | 13,28 MB, `target/release/morune.exe` |
| Startup interno | 19 ms (mediana de 10 execucoes) |
| RAM em repouso | 78,8 MB de working set medio; pico 79,5 MB |
| Testes | 217 no total; dois exigem uma sessao de logon do Windows para o cofre |
| Clippy | limpo com `-D warnings` |

### Verificado contra a conta real

Login OAuth PKCE com reconexao pelo cofre; reproducao com som saindo; busca;
playlists, curtidas e artistas seguidos; capas; abrir uma playlist, filtrar,
ordenar e tocar a faixa escolhida.

### O que existe agora e nao existia

**Log em arquivo.** O build de release e `windows_subsystem = "windows"` e roda
sem console: tudo que o `tracing` escrevia em `stdout` era descartado em
silencio. Agora vai para `%APPDATA%\morune\Morune\data\morune.log`, com teto
de 4 MB e rotacao. **E o primeiro lugar a olhar quando algo falhar.**

**Copia da librespot em `vendor/`.** Um metodo alterado, para que conta
gratuita nao encerre o processo. Ver [vendor/README.md](../vendor/README.md).

**Catalogo sobre o protocolo interno.** `webapi.rs` e `dto.rs` deixaram de
existir. Busca vai pelo `pathfinder`; playlists, metadado, colecao, capas e
radio vao pela `spclient` e pelo mercury.

**Capas**, com cache em disco de teto explicito e descarte por LRU.

**Playlists na barra lateral**, com filtro, porque o `rootlist` *e* a barra
lateral do Spotify. Musicas curtidas em primeiro.

**Tela de detalhe.** Ativar um card abre a lista em vez de tocar direto, com
filtro e ordenacao.

### Tres defeitos que so a conta real revelou, e que valem como aviso

Os tres tinham a mesma forma: o codigo estava certo isoladamente, e a ligacao
entre duas pontas e que estava errada. Nenhum apareceria em teste de unidade
escrito depois do fato -- todos apareceram na tela.

1. **A extensao do arquivo de cache.** O decodificador do Slint escolhe o
   formato pelo nome, nao pelo conteudo: capas gravadas como `.img` eram JPEG
   validos que nunca abriam. Hoje a extensao sai da assinatura dos bytes.
2. **A ordem do `extended-metadata`.** A resposta nao volta na ordem pedida, e
   o pedido sai em lotes de 50. Acumular na ordem de chegada embaralhava
   qualquer playlist e desfazia a ordenacao por data das curtidas.
3. **A forma textual do alvo.** A lista manda `track/spotify:<id>` e o codigo
   comparava com o id cru: o clique nao achava a faixa e nada acontecia, sem
   erro nenhum. Ha teste de ida e volta agora.

### O que ainda nao existe

- **albuns salvos, mais ouvidos e tocadas recentemente** -- sem caminho
  conhecido, recusam com `Unsupported` e a interface esconde a secao;
- **escrita na conta** -- o Morune so le: nao curte, nao reordena, nao remove.

Radio/autoplay, capas em `TrackRow`, aviso do teto de 200 e a tela completa de
artista foram implementados depois da rodada real de 19/08 e ainda precisam da
reverificacao descrita abaixo.

---

## Antes de qualquer comando

O ambiente de build **nao** esta no `PATH` por padrao. Toda sessao comeca com:

```powershell
. .\tools\env.ps1
```

Sem isso o `cargo` nao existe, ou existe e falha ao linkar. Detalhes e o porque
em [ADR-0002](adr/0002-toolchain-windows-gnu.md).

### Armadilha conhecida

`vergen` esta **fixado em 9.0.6** no `Cargo.lock`. A 9.1 quebra o build do
`librespot-core` (conflito de `vergen-lib` entre 0.1 e 9.1 no `build.rs`). Um
`cargo update` desatento reintroduz o problema. Se voltar a acontecer:

```powershell
cargo update -p vergen --precise 9.0.6
```

---

## Como o backend de Spotify esta montado

Tudo sai de **uma sessao da librespot**. O Web API saiu do desenho.

```text
  morune-app                    morune-spotify           de onde vem
  ----------                    --------------           -----------
  Session  -- login -------->   SpotifyAuthenticator  \
  Browse   -- busca -------->   Pathfinder            |   api-partner
           -- listas ------->   Internal              |   spclient + mercury
           -- capas -------->   SpotifyArtwork        |   i.scdn / pickasso
  AppState -- comandos ---->    SpotifyEngine         /   audio
                                       |
                                  SharedSession
```

- **`token.rs`** guarda o token do OAuth e o refresh no Gerenciador de
  Credenciais. So o **login** usa: depois que a sessao existe, tudo mais e
  assinado pelos tokens que a propria librespot renova (`login5` e o client
  token).
- **`engine.rs`** fala o protocolo da librespot: e de la que sai o audio.
- **`pathfinder.rs` + `graphql.rs`** falam o `api-partner` por GraphQL: e de
  la que sai a **busca**, e so ela. Traz divida -- ver abaixo.
- **`internal.rs`** e o resto: playlists, metadado em lote, colecao, capas,
  album, artista. Protobuf tipado pela propria librespot.
- **`artwork.rs`** baixa capa; quem guarda e o aplicativo, com teto e LRU.
- No aplicativo, **`browse.rs`** e a ponte: cada pedido vira tarefa no runtime
  do backend e o resultado e recolhido no temporizador de 100 ms que ja atende
  bandeja e reproducao. Um pedido de cada vez, e o ultimo ganha -- e o
  comportamento certo para uma caixa de busca.

## O Web API esta fechado para o Morune -- medido em 19/08/2026

O ciclo 2 nao falhou por defeito de codigo. O `api.spotify.com` responde **429
Too Many Requests** na primeira requisicao de qualquer sessao, e o texto abaixo,
escrito antes, partia de uma premissa que a medicao derrubou.

A sonda esta em `crates/morune-spotify/examples/sonda.rs`. Ela exige login real,
e o que se sabe hoje veio de tres rodadas dela:

| Caminho | Resultado |
|---|---|
| `api.spotify.com` com token do OAuth | **429** |
| `api.spotify.com` com token do **login5** | **429** |
| `hm://keymaster/token/authenticated` | **403** |
| `hm://searchview/km/v4/search/` | 404 |
| `spclient searchview/{km/v4, v3, km/v3}` | 404 |
| `collection/v2/paging` na spclient | 404 |
| `play-history/v1`, `recently-played/v3` | 404 |
| **`api-partner.spotify.com/pathfinder/v1/query`, consulta persistida** | **OK, 69 KB de JSON** |
| `pathfinder` com consulta GraphQL crua | 400 |
| `get_rootlist`, `get_track_metadata`, `get_radio_for_track` | OK |
| `hm://collection/collection/{user}` (curtidas) | OK, 20 KB |
| `hm://collection/artist/{user}` | OK |
| `hm://collection/album/{user}` | 404 |
| plano da conta por `get_user_attribute("type")` | OK, ~200 ms apos `connect` |

Tres conclusoes, e nenhuma delas e reversivel por ajuste de codigo:

**Nenhum token abre o Web API.** Nao e cota estourada -- e a primeira chamada da
sessao, e o limite da librespot e de 300 por 30 s. O `Retry-After` volta em
10, 1 e 59 segundos, o que e recusa deliberada. O client ID do cliente oficial
de desktop serve para o protocolo interno e nao serve para o Web API publico.

**Registrar um client ID proprio nao resolve o produto.** Aplicativo novo nasce
em modo de desenvolvimento, onde so entram usuarios adicionados a mao, um a um.
Como o Morune e aberto e a proposta e baixar, entrar e usar, isso reprovaria
todo mundo que nao estivesse na lista. Extended Quota depende de revisao do
Spotify, e um player de desktop de terceiro e justamente a categoria que os
termos do programa restringem. **Decisao: o Web API sai do caminho.**

**O `pathfinder` cobre o buraco.** E por onde o player web busca, aceita o token
do login5 com o client token -- os dois ja existem na sessao -- e devolve album,
artista e faixa em JSON limpo. Ver a divida que ele traz na secao seguinte.

### A divida do pathfinder

A consulta crua e recusada com 400: so passa **consulta persistida**, que e
identificada por um hash SHA-256 acordado entre cliente e servidor. Esse hash
acompanha a versao do player web e **muda sem aviso**. Quando mudar, a busca do
Morune para de responder ate alguem atualizar a constante.

Isso e divida assumida, nao descuido. Nao ha alternativa medida: a consulta
crua nao passa, o `searchview` morreu em todas as versoes testadas, e o Web API
esta fechado. O que a implementacao deve garantir e que a falha seja **local**:
busca que quebra nao pode derrubar Inicio, Biblioteca nem reproducao.

### O que ainda nao tem caminho

Albuns salvos, mais ouvidos e tocadas recentemente responderam 404 em todos os
enderecos testados. Provavelmente existem como operacoes do proprio pathfinder,
que nao foram sondadas ainda. Ate la, sao prateleiras que nao podem ser
prometidas na interface.

### A conta gratuita nao encerra mais o processo

Resolvido com uma copia pontual de `librespot-core` em `vendor/`, ligada por
`[patch.crates-io]`. `check_catalogue` apenas registra o plano no log; o Morune
continua aberto e explica a limitacao quando a reproducao e recusada. O motivo,
a alteracao exata e o procedimento de atualizacao estao em
[`vendor/README.md`](../vendor/README.md).

---

## A mudanca de 2024 no Web API, e o que fazemos a respeito

> **Obsoleto desde 19/08/2026.** Vale como historico do porque `internal.rs`
> existe. A conclusao pratica esta na secao acima: nao e so a lista de 2024 que
> caiu, e o Web API inteiro.

Em 27/11/2024 o Spotify fechou, para aplicativos novos, uma lista de endpoints
que inclui exatamente o que um player usa para nao ser so uma caixa de busca:

| Fechado | O que era |
|---|---|
| `/v1/recommendations` | recomendacao por semente |
| `/v1/artists/{id}/related-artists` | artistas parecidos |
| `/v1/browse/featured-playlists` e categorias | vitrine editorial |
| `/v1/audio-features` e `/v1/audio-analysis` | caracteristicas da faixa |
| `/v1/playlists/{id}` para playlists algoritmicas | Descobertas da Semana, Radar de Novidades |

Ate hoje nao ha substituto oficial. O que o Morune faz:

**Nao depende de nada da lista.** As prateleiras do Inicio saem de
`/v1/me/tracks`, `/v1/me/top/*` e `/v1/me/player/recently-played`, que
continuam abertos e cujos escopos o login ja pede.

**Para as playlists que o Spotify monta para a conta, usa o caminho interno.**
`internal.rs` chama `get_rootlist` e `get_playlist` da librespot -- o mesmo
protocolo que ja entrega o audio. As duas respostas sao protobuf tipado pela
propria librespot, com nome, dono e tamanho decorados junto, entao nao ha JSON
adivinhado. Abrir uma playlist tenta o Web API primeiro e cai para o caminho
interno em 404 ou 403, que e como uma playlist algoritmica se apresenta.

**O `format` separa uma coisa da outra.** Playlist que o usuario criou vem com
`format` vazio; as que o Spotify monta trazem `format` preenchido, e as
editoriais tem dono `spotify`. E o que alimenta a prateleira "Feito para voce".

### A librespot encerra o processo em conta nao-Premium

`check_catalogue`, em `librespot-core`, chama `exit(1)` ao receber o pacote de
produto se o tipo da conta nao for `premium`. Nao ha erro para tratar nem
resultado para inspecionar: o aplicativo some da tela.

Isto importa porque o Morune e aberto e qualquer pessoa pode clicar em "Entrar"
-- e a maioria das contas do Spotify e gratuita. Por isso o login pergunta ao
`/v1/me` **antes** de entregar a credencial a librespot, e recusa com uma frase
que explica o motivo. Ver o cabecalho de `crates/morune-spotify/src/auth.rs`.

**Risco que continua de pe:** `set_user_attributes` tambem chama
`check_catalogue`. Se o Spotify rebaixar o plano no meio de uma sessao ja aberta,
o processo ainda morre. Nao ha como contornar sem alterar a librespot.

### Duas incognitas -- respondidas em 19/08/2026

1. **O client ID do cliente oficial de desktop nao tem acesso estendido.** Ele
   serve ao protocolo interno e leva 429 em qualquer caminho do Web API. Nao
   ha o que testar de novo.
2. **As playlists algoritmicas aparecem sim com `format` preenchido**, e o
   valor e util: `daily-mix`, `discover-weekly`, `blend`, `topic-mix`,
   `inspiredby-mix`, `artist-mix-reader`, `wrapped-*`, `all-time-top-songs-*`,
   `editorial`, `artistsets`. E o que alimenta a classificacao do Inicio --
   ver `PlaylistSummary::kind`.

## Roteiro de verificacao

Feito em 19/08/2026 contra uma conta Premium real. Repetir depois de mexer no
backend -- nesta ordem, porque cada passo depende do anterior.

1. `. .	oolsenv.ps1` e `cargo run --release`.
2. Configuracoes -> **Entrar no Spotify**. O navegador abre; autorize. A barra
   de status deve dizer "Conectado como ...".
3. Feche e abra. Tem de reconectar sozinho, sem navegador: e o refresh token
   vindo do cofre.
4. **Barra lateral**: Musicas curtidas em primeiro, depois as playlists na
   ordem do Spotify. Digitar no filtro esconde o que nao combina.
5. **Inicio**: Musicas curtidas, Feito para voce, Estacoes recomendadas, Seus
   mais ouvidos. Prateleira vazia nao e defeito da tela -- e aquela fonte que
   nao respondeu, e `MORUNE_LOG=debug` diz qual.
6. **Abrir Descobertas da Semana**. E o teste do caminho interno: pelo Web API
   ela responde 404.
7. **Buscar** uma musica: e o unico teste do pathfinder. Se a busca vier vazia
   sem erro, o primeiro suspeito e o hash da consulta persistida.
8. **Abrir uma playlist**: capa, nome, lista. Clicar numa faixa toca **aquela**
   faixa. Filtrar e clicar tem de tocar a faixa filtrada, e nao a da mesma
   posicao na lista sem filtro.
9. **Curtidas**: a primeira tem de ser a mais recente. Foi o defeito mais
   discreto de todos -- a lista parecia certa, so estava desatualizada.
10. **Capas**: parte das playlists mostra imagem, o resto mostra a marca. A
    faixa tocando tem capa na barra de baixo.
11. Deixar uma faixa acabar sozinha, com repeticao em "uma": tem de repetir,
    nao pular. E o unico jeito de verificar o `user_advance = false`.
12. Fechar a janela com "continuar tocando ao fechar" ligado: o som continua e
    a bandeja mostra a faixa.
13. **Medir de novo** com `.	oolsmeasure.ps1` e atualizar
    [PERFORMANCE.md](../PERFORMANCE.md) -- agora com CPU e GPU **tocando**, que
    e o criterio que passou a valer. **Ainda nao foi feito.**

---

## O proximo passo: reverificacao final do ciclo 2

Radio e autoplay estao implementados. Ao fim natural da fila, com a opcao
ligada por padrao, a ultima faixa vira semente; o endpoint devolve uma playlist,
as faixas novas sao anexadas ao contexto e o historico e preservado. Faixas ja
presentes no contexto sao descartadas para evitar ciclos curtos. O pedido de
radio tem canal proprio e nao cancela busca ou navegacao.

O JSON sem esquema publicado e lido por um parser minimo e tolerante a campos
extras. A resposta real gravada na sonda virou fixture conceitual dos testes,
mas ainda e necessario repetir com generos diferentes na conta Premium.

Na mesma rodada foram concluídos:

1. aviso quando a tela mostra apenas as primeiras 200 faixas;
2. capas pequenas em todas as linhas de faixa;
3. discografia e faixas populares vindas diretamente do protobuf tipado do
   artista, sem uma requisicao por album;
4. retirada das playlists da Biblioteca, pois elas ja moram na lateral.

### Roteiro adicional para os recursos novos

1. Com autoplay ligado, tocar ate o fim de uma lista curta: o radio deve
   continuar sem apagar o historico.
2. Desligar autoplay em Configuracoes e repetir: deve parar em "Fim da fila".
3. Abrir uma colecao com mais de 200 faixas: o aviso de recorte deve aparecer.
4. Abrir um artista: faixas populares e discografia devem aparecer; abrir um
   album da faixa horizontal deve trocar para o detalhe do album.
5. Conferir capas em busca, curtidas, detalhe e fila.
6. Repetir com rede indisponivel no fim da fila: a musica termina e a falha do
   radio aparece na barra de status sem derrubar a tela.

## Depois da reverificacao

1. Medir CPU e GPU tocando, com um jogo em tela cheia.
2. Gerar e instalar o pacote final em uma conta limpa do Windows.
3. Decidir se albuns salvos, mais ouvidos e tocadas recentemente justificam
   novas consultas persistidas do pathfinder. Nao implementar continua sendo
   uma resposta valida enquanto nao houver caminho estavel.
---

## Decisoes que dependem do Felipe

Nenhuma delas bloqueia o ciclo 2; todas bloqueiam a publicacao.

**Licenca — resolvida em 18/08/2026.** O Morune e MIT (`LICENSE`), e o Slint e
usado sob a `LicenseRef-Slint-Royalty-free-2.0`, nao sob a GPL, com atribuicao
pelo badge no README. O porque das duas escolhas esta na
[ADR-0005](adr/0005-licenca-e-slint-royalty-free.md). Os avisos das 339
dependencias vao em `THIRD-PARTY-LICENSES.txt`, gerado por `tools/licenses.ps1`
e instalado junto do aplicativo.

**Atencao ao mexer em dependencia.** `tools/licenses.ps1` falha o build se
aparecer uma dependencia copyleft que nao esteja na lista `$revisadas` dele. Nao
contorne adicionando o nome a lista: uma licenca GPL nova pode obrigar o Morune
inteiro a deixar de ser MIT. Decida, escreva um ADR, e so entao adicione.

**Orcamento de RAM — resolvido em 18/08/2026, mudando a pergunta.** O numero
exato de MB nao e criterio. O criterio e **nao atrapalhar quem esta jogando**:
sem roubar quadro, sem acordar a GPU em segundo plano, sem disputar CPU, e sem a
interface travar quando o usuario volta a ela — o defeito do cliente oficial do
Spotify. Os 78,8 MB ficam aceitos como estao.

Duas consequencias praticas, e as duas valem para o ciclo 2:

- o cache de capas nasce com **teto explicito e descarte por LRU**. Feito em
  19/08/2026: 48 MB, em `crates/morune-app/src/artwork.rs`. O valor era livre e
  esta escolhido -- cabem cerca de 1.500 capas de 300 px, que e uma biblioteca
  grande navegada inteira sem baixar duas vezes;
- as metricas que passam a mandar sao **CPU e GPU em segundo plano** e a
  ausencia de travamento da interface. Nenhuma delas foi medida ainda, porque
  nenhuma faz sentido sem reproducao real. Ver o criterio em
  [PERFORMANCE.md](../PERFORMANCE.md).

**Publicacao — decidida em 19/08/2026.** O repositorio fica privado ate haver uma
**v1 pronta**. Nao e indefinicao: e a ordem certa, porque publicar cedo demais
convida issue e pull request antes de o fluxo completo da v1 estar fechado. A
consequencia esta na entrada seguinte -- assinatura de
codigo depende de publicar, entao ela tambem espera a v1.

**Assinatura de codigo.** O instalador nao e assinado e o SmartScreen avisa em
toda instalacao. **Nao e questao de dinheiro:** o SignPath Foundation assina de
graca projetos open source que se qualifiquem, e o Morune ja cumpre a licenca
(MIT) e a ausencia de componente proprietario. Falta o que a candidatura exige e
o projeto ainda nao tem: repositorio publico e uma versao lancada. Ou seja, isto
depende de publicar, nao de comprar -- e publicar ficou para a v1, pela decisao
acima.

O pipeline ja esta pronto para qualquer caminho: `build-installer.ps1` chama
`tools/sign.ps1` no executavel e no instalador, e basta definir
`MORUNE_SIGN_THUMBPRINT`. Enquanto nao ha assinatura, os dois arquivos ja
declaram nome, versao e copyright, e o build publica um `.sha256` ao lado do
instalador. Opcoes, precos e requisitos em [assinatura.md](assinatura.md).

---

## Divida tecnica conhecida

- **O hash da consulta persistida do pathfinder.** E a divida mais provavel de
  cobrar: ele acompanha a versao do player web do Spotify e muda sem aviso.
  Quando mudar, a busca para de responder e o resto continua funcionando --
  isso e de proposito. Fica em `HASH_BUSCA`, em `pathfinder.rs`.
- **A copia da librespot em `vendor/`.** Toda atualizacao dela exige refazer a
  alteracao a mao. Se o projeto original trocar o `exit(1)` por erro -- o
  proprio codigo tem um `TODO` dizendo que deveria --, a copia some.
- **A colecao e lida com protobuf sem esquema publicado.** `collection_items`
  le campo a campo o que a sonda mostrou. Nao ha `.proto` para conferir: se o
  Spotify mudar a numeracao dos campos, a leitura devolve lista vazia em vez
  de erro.
- **~1 s de inicializacao grafica** no ciclo completo do processo. Nao
  investigado. Vale medir quanto e criacao do contexto OpenGL no driver Radeon
  e quanto e o backend winit, antes de tentar otimizar qualquer coisa.
- **Minimizar para a bandeja** nao existe, so fechar. O Slint nao expoe o evento
  de minimizacao; exige alcancar o `HWND` nativo. Faz sentido junto com as
  teclas de midia, no ciclo 4.
- **Recarga a quente** esta pronta em `morune-theme` (feature `hot-reload`, com
  agrupamento de eventos e filtro de temporarios de editor) mas nao esta ligada
  a interface. Hoje o usuario clica em "Recarregar".
- **`begin_login` nao devolve a URL de autorizacao.** A librespot abre o
  navegador sozinha e nao expoe a URL, entao o contrato recebe o endereco de
  retorno no lugar. Se o navegador nao abrir, o usuario fica sem nada para
  copiar. Resolver exige um fluxo proprio sobre a crate `oauth2`.
- **Busca so devolve faixas.** A tela de busca so tem lista de faixas; album,
  artista e playlist nos resultados precisam de uma grade que ainda nao existe
  la.
- **`bundled_font`** existe no esquema de tema mas fontes empacotadas nao sao
  registradas. Fontes instaladas no sistema funcionam por `family`.
- **Icones nao sao substituiveis por tema**; os caminhos vetoriais estao na
  interface.
- **Alvo MSVC** continua valendo pelo tamanho do binario, mas deixou de ser
  bloqueante: o binario GNU roda isolado, sem DLLs do MinGW. Atencao: o icone
  do executavel e embutido com `windres`, que so existe na toolchain GNU. Migrar
  para MSVC exige trocar isso por `rc.exe` ou por uma crate de recursos (ver
  `embed_exe_icon` em `crates/morune-app/build.rs`).
- **Icone da janela precisa vir de `@image-url`.** Imagem montada em codigo
  (`Image::from_rgba8`) nao funciona: o backend winit so reaplica o icone quando
  a chave de cache da imagem muda, e imagens criadas em execucao nao tem chave.
  A janela fica sem icone nenhum, sem erro nem aviso. Custou 0,15 MB de binario
  porque puxa o decodificador de PNG do Slint junto.

---

## Ferramentas de verificacao

Todas ja existem e passam. Usar antes de declarar qualquer coisa pronta.

```powershell
cargo test --workspace --features morune-theme/hot-reload
cargo clippy --workspace --all-targets -- -D warnings
.\tools\measure.ps1          # tamanho, startup, RAM, CPU
.\tools\verify-tray.ps1      # fechar nao encerra o processo
.\tools\snapshot.ps1         # cada tema renderizado em PNG
.\tools\build-installer.ps1  # instalador + checagem de isolamento
.\tools\make-icon.ps1        # regera assets/brand/morune.ico a partir dos PNGs
.\tools\sign.ps1 -Path X     # assina; sem certificado, avisa e nao quebra
.\tools\licenses.ps1         # avisos de terceiros + guarda de copyleft
```

`tools/snapshot.ps1` e `MORUNE_START_PAGE=3` permitem fotografar telas
especificas. As capturas saem em `bench-out/`, que nao entra no git.

---

## Regras do projeto que nao estao no codigo

- **Nada e declarado funcionando sem medicao ou teste.** Onde falta, esta dito,
  inclusive no README.
- **A interface nao pode ter valor visual literal.** Cor, tamanho ou duracao
  escrita direto na interface significa um detalhe que o usuario nao consegue
  customizar — isso e bug, nao omissao.
- **Um tema quebrado nunca impede o aplicativo de abrir.** Vale tambem para
  configuracao corrompida e para bandeja indisponivel.
- **Temas sao dados, nunca codigo.** Ver [ADR-0004](adr/0004-temas-declarativos.md).
