# Estado do projeto e proximo passo

Documento de retomada. Escrito em 18/08/2026 ao fim do ciclo 1, e revisto em
19/08/2026, quando o backend de Spotify ficou de pe.

Quem chegar aqui numa sessao nova deve ler este arquivo primeiro, depois
[ARCHITECTURE.md](../ARCHITECTURE.md) e [ROADMAP.md](../ROADMAP.md).

---

## Onde o projeto esta

Ciclo 1 concluido e verificado. Ciclo 2 **escrito e compilando, ainda nao
verificado** -- ver o bloqueio logo abaixo.

| | |
|---|---|
| Instalador | 4,00 MB, `dist/Morune-0.1.0-setup.exe` (medido no ciclo 1) |
| Executavel | 9,39 MB, `target/release/morune.exe` (medido no ciclo 1) |
| Startup interno | 18 ms |
| RAM em repouso | 70,6 MB |
| CPU em repouso | 0,00% |
| Testes | 196, todos passando |
| Clippy | limpo com `-D warnings` |

**Verificado:** interface completa, tres temas trocaveis em execucao,
import/export `.musicpack`, configuracao persistente, cofre de credenciais do
Windows, fechar para a bandeja mantendo o processo vivo, instalador com escolha
de disco, identidade visual em todos os lugares que o Windows mostra o
aplicativo.

**Escrito e coberto por teste de unidade, mas nunca exercitado contra o
Spotify:** login OAuth PKCE, reproducao sobre a librespot, busca, e as cinco
prateleiras do Inicio -- Feito para voce, Tocadas recentemente, Musicas
curtidas, Seus mais ouvidos e Suas playlists -- alem da Biblioteca com
playlists, albuns salvos e artistas seguidos. O `NullEngine` continua sendo o motor ate o
login dar certo -- ele aceita preferencias e recusa reproducao de forma
explicita, entao a interface nunca fica sem motor.

**O que ainda nao existe:** radio e autoplay, capas (nem download nem cache),
paginacao das telas de conteudo, e tela propria de album, artista ou playlist --
ativar um card toca direto, sem passar por uma tela de detalhe.

### O bloqueio

O login e interativo e exige uma conta Premium. Nenhuma linha do ciclo 2 pode
ser declarada funcionando antes de o Felipe entrar com a conta dele e ouvir som
sair. O roteiro minimo de verificacao esta em
[Como verificar o ciclo 2](#como-verificar-o-ciclo-2).

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

Duas conexoes, uma sessao so:

```text
  morune-app                       morune-spotify
  ----------                       --------------
  Session   -- login/logout -->    SpotifyAuthenticator ---+
  Browse    -- busca/lista -->     SpotifyCatalog ------+  |  mesma
  AppState  -- comandos ---->      SpotifyEngine -----+ |  |  Session
                                                      v v  v
                                          librespot (audio) + Web API (catalogo)
```

- **`token.rs`** e a fonte unica de token. Login e catalogo pedem token ao mesmo
  lugar, a renovacao acontece sob trava, e o refresh token mora no Gerenciador
  de Credenciais. Sem isso, o catalogo renovaria por conta propria e a conta
  acumularia tokens vivos.
- **`engine.rs`** fala o protocolo da librespot: e de la que sai o audio.
- **`catalog.rs` + `webapi.rs` + `dto.rs`** falam o Web API por HTTP: e de la
  que saem busca e biblioteca. Nao e escolha -- o protocolo da librespot nao tem
  busca. O cliente HTTP e o da propria sessao, para nao duplicar a pilha de TLS
  no binario.
- **`internal.rs`** fala o protocolo interno para o que o Web API deixou de
  entregar. Ver a secao abaixo.
- No aplicativo, **`browse.rs`** e a ponte: cada pedido vira tarefa no runtime
  do backend e o resultado e recolhido no temporizador de 100 ms que ja atende
  bandeja e reproducao. Um pedido de cada vez, e o ultimo ganha -- e o
  comportamento certo para uma caixa de busca.

## A mudanca de 2024 no Web API, e o que fazemos a respeito

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

### Duas incognitas que so o primeiro login resolve

1. **O client ID que usamos e o do cliente oficial de desktop.** Aplicativos com
   acesso estendido anterior a 2024 nao foram afetados. Se esse client ID for
   tratado assim, os endpoints fechados podem simplesmente responder -- vale
   testar `/v1/recommendations` uma vez antes de assumir que morreu.
2. **Se as playlists algoritmicas aparecem no rootlist com `format`
   preenchido.** O caminho esta escrito e compila; qual campo o servidor
   preenche de verdade so se ve com uma conta na mao.

## Como verificar o ciclo 2

Nesta ordem, porque cada passo depende do anterior:

1. `. .\tools\env.ps1` e `cargo run --release`.
2. Configuracoes -> **Entrar no Spotify**. O navegador abre; autorize. A barra
   de status deve dizer "Conectado como ...".
3. Feche e abra o aplicativo. Tem de reconectar sozinho, sem navegador: e o
   refresh token vindo do cofre.
4. **Inicio** deve mostrar cinco prateleiras: Feito para voce, Tocadas
   recentemente, Musicas curtidas, Seus mais ouvidos e Suas playlists. Uma
   prateleira vazia nao e defeito da tela: e aquela fonte que nao respondeu, e o
   log em `MORUNE_LOG=debug` diz qual.
5. **Abrir Descobertas da Semana** na prateleira "Feito para voce". E o teste do
   caminho interno: pelo Web API ela responde 404.
6. **Biblioteca**: playlists, albuns salvos e artistas seguidos.
7. **Buscar** uma musica e clicar nela: som. Depois "proxima" tem de andar pelos
   resultados da busca, e nao parar. O mesmo vale para clicar numa faixa de
   "Musicas curtidas" ou "Tocadas recentemente": a prateleira inteira vira a
   fila.
8. Clicar num card de album ou playlist: toca a primeira faixa, e a Fila mostra
   o resto.
9. Deixar uma faixa acabar sozinha, com repeticao em "uma": tem de repetir, nao
   pular. E o unico jeito de verificar o `user_advance = false`.
10. Fechar a janela com "continuar tocando ao fechar" ligado: o som continua e a
    bandeja mostra a faixa.
11. **Medir de novo** com `.\tools\measure.ps1` e atualizar
   [PERFORMANCE.md](../PERFORMANCE.md) -- agora com CPU e GPU **tocando**, que e
   o criterio que passou a valer.

O que falhar aqui e trabalho do proximo ciclo, nao defeito de projeto: nada
disto passou por uma conta real ainda.

## Depois disso

1. **Radio e autoplay.** E o que sobra de "recomendacao" depois de 2024, e o
   caminho e o interno: `spclient.get_radio_for_track` e `get_apollo_station`,
   que a librespot ja expoe. Resolve duas coisas de uma vez -- "tocar
   parecidas" a partir de uma faixa, e o silencio quando a fila acaba.
   **Atencao:** diferente do rootlist, essas duas respondem JSON sem tipo na
   librespot. O formato precisa ser visto uma vez com uma conta real antes de
   escrever o parser, senao vira adivinhacao.
2. **Capas**: download, cache em disco com teto explicito e descarte por LRU,
   escolha pelo tamanho de exibicao via `ImageSet::best_for_width`. O modelo ja
   carrega as URLs e ja as entrega ordenadas da menor para a maior; falta o
   cache e um lugar na interface para elas -- hoje `CardItem` e `TrackRow` nao
   tem campo de imagem.
3. **Paginacao**: `Catalog::playlist_tracks` ja e paginado e nao e usado pela
   interface. Uma playlist de mil faixas hoje chega cortada nas primeiras 100.
   As prateleiras do Inicio tambem param no que cabe numa fileira.
4. **Telas de detalhe** de album, artista e playlist. Hoje ativar um card toca
   direto, o que resolve ouvir mas nao resolve navegar.

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
Spotify. Os 70,6 MB ficam aceitos como estao.

Duas consequencias praticas, e as duas valem para o ciclo 2:

- o cache de capas nasce com **teto explicito e descarte por LRU**. O que nao
  pode e crescer sem limite; o valor do teto e livre;
- as metricas que passam a mandar sao **CPU e GPU em segundo plano** e a
  ausencia de travamento da interface. Nenhuma delas foi medida ainda, porque
  nenhuma faz sentido sem reproducao real. Ver o criterio em
  [PERFORMANCE.md](../PERFORMANCE.md).

**Assinatura de codigo.** O instalador nao e assinado e o SmartScreen avisa em
toda instalacao. **Nao e questao de dinheiro:** o SignPath Foundation assina de
graca projetos open source que se qualifiquem, e o Morune ja cumpre a licenca
(MIT) e a ausencia de componente proprietario. Falta o que a candidatura exige e
o projeto ainda nao tem: repositorio publico e uma versao lancada. Ou seja, isto
depende de publicar, nao de comprar.

O pipeline ja esta pronto para qualquer caminho: `build-installer.ps1` chama
`tools/sign.ps1` no executavel e no instalador, e basta definir
`MORUNE_SIGN_THUMBPRINT`. Enquanto nao ha assinatura, os dois arquivos ja
declaram nome, versao e copyright, e o build publica um `.sha256` ao lado do
instalador. Opcoes, precos e requisitos em [assinatura.md](assinatura.md).

---

## Divida tecnica conhecida

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
- **Artistas seguidos sao paginados por cursor** no Web API, e o contrato
  `Library` e por deslocamento. A implementacao caminha os cursores ate chegar
  ao deslocamento pedido, com teto de 40 requisicoes. Funciona para quem segue
  centenas de artistas; para milhares, nao.
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
