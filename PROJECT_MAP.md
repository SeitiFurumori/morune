# Project Map

## 1. Visão geral

- Morune é um cliente nativo de música para Windows, implementado em Rust.
- A interface é declarativa e compilada com Slint; não há frontend web, CSS ou WebView.
- O workspace separa contratos e modelos (morune-core), tema (morune-theme), persistência (morune-storage), integração Spotify (morune-spotify) e composição da aplicação (morune-app).
- AppState é o orquestrador central da UI, sessão, navegação, dados, fila e reprodução.
- A integração Spotify usa OAuth PKCE, librespot, protocolo interno Spotify e Pathfinder HTTP.
- A reprodução é assíncrona: comandos entram em SpotifyEngine e eventos retornam por broadcast para a aplicação.
- Temas e layouts são configuráveis por TOML, com suporte a packs .musicpack, herança e hot reload.
- Configurações, credenciais, cache de artwork/áudio e estado de janela são tratados localmente.
- A janela pode ser ocultada para a bandeja; o backend continua sendo processado enquanto a UI está oculta.

## 2. Stack

| Área | Tecnologia |
|---|---|
| Linguagem | Rust 2021; rust-version 1.92 |
| Build e pacotes | Cargo; workspace com resolver 2; Cargo.lock |
| UI | Slint 1.17, backend Winit, renderers FemtoVG/software, AccessKit |
| Concorrência | Tokio multi-thread, futures, canais mpsc/broadcast |
| Áudio | librespot, rodio/cpal e MoruneSink customizado |
| Spotify | OAuth 2.0 PKCE, login5/librespot, Pathfinder GraphQL/JSON, protobuf |
| HTTP | reqwest, http e cliente autenticado compartilhado |
| Serialização | serde, serde_json, TOML |
| Persistência | arquivos TOML, diretórios de aplicação e Windows Credential Manager |
| Plataforma | Windows; toolchain GNU MinGW; tray, mídia, taskbar e startup integrados |
| Empacotamento | scripts de build, recursos Windows e instalador em installer/ |

Dependências locais:

morune-app depende de morune-core, morune-theme, morune-storage e
morune-spotify. morune-storage depende de morune-core e morune-theme;
morune-spotify depende de morune-core; morune-core e morune-theme não
dependem da UI.

Dependências externas mais relevantes: slint, slint-build, tokio, serde, toml,
reqwest, tracing, anyhow, thiserror, directories, rfd, tray-icon,
raw-window-handle, windows, librespot-*, rodio, oauth2, protobuf, sha2 e open.

## 3. Estrutura principal

    Cargo.toml / Cargo.lock
    ├── crates/
    │   ├── morune-core/src/
    │   │   ├── lib.rs, model.rs, catalog.rs, playback.rs, queue.rs
    │   │   ├── auth.rs
    │   │   └── error.rs
    │   ├── morune-theme/src/
    │   │   ├── lib.rs, spec.rs, tokens.rs, layout.rs, loader.rs
    │   │   ├── manifest.rs, pack.rs, icons.rs, color.rs
    │   │   └── watch.rs
    │   ├── morune-storage/src/
    │   │   ├── lib.rs, paths.rs, config.rs
    │   │   ├── credentials.rs
    │   │   └── favorites.rs
    │   ├── morune-spotify/src/
    │   │   ├── lib.rs, runtime.rs, engine.rs, catalog.rs
    │   │   ├── auth.rs, token.rs, internal.rs
    │   │   ├── pathfinder.rs, graphql.rs, artwork.rs
    │   │   └── sink.rs
    │   └── morune-app/
    │       ├── build.rs, Cargo.toml
    │       ├── src/main.rs, state.rs, session.rs, browse.rs
    │       ├── src/theme_bridge.rs, artwork.rs, wallpaper.rs, tint.rs
    │       ├── src/tray.rs, tray_menu.rs, update.rs
    │       ├── src/taskbar.rs, smtc.rs, startup.rs
    │       └── ui/app.slint, components.slint, theme.slint
    ├── assets/                         # recursos de marca e distribuição
    ├── docs/                           # arquitetura e documentação
    ├── installer/                      # empacotamento/instalador
    ├── plans/                          # planos de trabalho
    ├── tools/                          # ferramentas auxiliares
    └── vendor/librespot-core/          # patch local do librespot

morune-app é o binário morune, com entrada em crates/morune-app/src/main.rs.
O build script compila a UI Slint, injeta a tag de release e prepara recursos
de bandeja/Windows. A compilação principal usa crates/morune-app/ui/app.slint.

target/, dist/ e bench-out/ são saídas geradas e não fazem parte do mapa de
implementação.

## 4. Rotas e páginas

Não há roteador por URL. A navegação usa o enum Page em state.rs, cujo valor
numérico é contrato com a propriedade page do Slint.

| Rota/Tela | Arquivo | Função |
|---|---|---|
| page = 0 — Home | crates/morune-app/ui/app.slint | Prateleiras de recomendações, estações, retrospectivas, playlists e curtidas |
| page = 1 — Search | crates/morune-app/ui/app.slint | Campo de busca, faixas e cards de álbuns/artistas/playlists |
| page = 2 — Library | crates/morune-app/ui/app.slint | Álbuns salvos e artistas seguidos, com estados vazio/login |
| page = 3 — Settings | crates/morune-app/ui/app.slint | Conta, áudio, aparência, janela, temas, diagnóstico e atualização |
| page = 4 — Queue | crates/morune-app/ui/app.slint | Fila manual, contexto atual, reordenação, remoção, limpeza e undo |
| page = 5 — Detail | crates/morune-app/ui/app.slint | Detalhe de álbum, playlist, artista ou curtidas; faixas paginadas |
| Mini-player | crates/morune-app/ui/app.slint | Modo compacto da mesma janela/player, controlado pelo estado de janela |
| Menu da bandeja | crates/morune-app/ui/app.slint | Janela separada com faixa atual, play/pause, anterior, próxima e volume |

AppState abre detalhes a partir de tags de destino (track, album, playlist,
artist e liked). Search, Home, Library, Detail e Context são solicitações
internas de dados, não rotas independentes.

## 5. Componentes principais

| Componente | Arquivo | Responsabilidade |
|---|---|---|
| AppWindow | crates/morune-app/ui/app.slint | Janela raiz, composição de páginas, callbacks e propriedades da UI |
| SidebarShell / Sidebar | crates/morune-app/ui/app.slint | Navegação, conta, playlists, filtros e colapso da barra lateral |
| PlayerBar | crates/morune-app/ui/app.slint | Artwork, faixa atual, transporte, seek, volume, shuffle/repeat e fila |
| HomePage | crates/morune-app/ui/app.slint | Composição das prateleiras da Home |
| SearchPage | crates/morune-app/ui/app.slint | Resultados de busca |
| LibraryPage | crates/morune-app/ui/app.slint | Biblioteca e artistas seguidos |
| DetailPage | crates/morune-app/ui/app.slint | Cabeçalho, filtros, ordenação e lista progressiva do detalhe |
| QueuePage / ManagedQueueShelf | crates/morune-app/ui/app.slint | Fila gerenciável, próximos itens e ações de fila |
| SettingsPage | crates/morune-app/ui/app.slint | Configurações e gerenciamento de temas |
| Card / CardCollection | crates/morune-app/ui/app.slint | Cards e coleções de entidades |
| TrackList / TrackListRow | crates/morune-app/ui/app.slint | Listas de faixas e ações por faixa |
| ScrollableCardShelf / TrackShelf | crates/morune-app/ui/app.slint | Prateleiras horizontais de cards ou faixas |
| TrayMenuWindow | crates/morune-app/ui/app.slint | Superfície compacta de controles da bandeja |
| Controles compartilhados | crates/morune-app/ui/components.slint | Botões, toggles, slider, busca, artwork, materiais e superfícies |
| Theme / Layout / Brand | crates/morune-app/ui/theme.slint | Globals visuais consumidos pelos componentes |

## 6. Estado global

AppState em crates/morune-app/src/state.rs concentra o estado de aplicação e é
manipulado no fluxo de callbacks da thread da UI. Não há Redux, store,
Context ou hooks de frontend.

Principais grupos de estado:

- Navegação: Page, detalhe aberto, filtro/ordenação de detalhe, status, retry
  e undo.
- Sessão: Session, usuário, avatar, login/logout, falha e conexão perdida.
- Player: Arc<dyn PlaybackEngine>, snapshot/event receiver, volume,
  reprodução, seek otimista, artwork atual e cor dominante.
- Fila: morune_core::Queue, origem/contexto, shuffle, repeat, histórico,
  itens manuais e autoplay.
- Conteúdo: resultados de busca, Home, Library, detalhes, cards, liked IDs,
  carregamento progressivo e pendências.
- Configuração: Config, tema carregado, overrides, wallpaper, janela,
  dispositivos de saída e updater.

Os dados são projetados para propriedades e modelos Slint por push_to_ui;
atualizações rápidas do player usam push_playback. A comunicação assíncrona
chega por canais e é coletada por polling no loop da aplicação.

Session em crates/morune-app/src/session.rs mantém o ciclo de vida do backend
Spotify e seleciona NullEngine antes do login ou quando não existe engine
autenticado.

## 7. Player

### Superfícies

- PlayerBar concentra controles na janela principal.
- TrayMenuWindow expõe controles reduzidos quando a aplicação está na bandeja.
- A UI também recebe integração de mídia, teclas e taskbar pelos módulos Windows
  de morune-app/src/.

### Fluxo de comandos e eventos

    Slint callback
      -> main.rs
      -> AppState
      -> PlaybackEngine::send
      -> canal Tokio
      -> SpotifyEngine
      -> librespot Player / MoruneSink

    librespot
      -> SpotifyEngine traduz eventos
      -> broadcast PlayerEvent
      -> AppState::poll_backend
      -> push_playback / push_to_ui
      -> PlayerBar e TrayMenuWindow

O polling do backend ocorre aproximadamente a cada 100 ms. O tick visual de
posição ocorre aproximadamente a cada 250 ms, apenas quando a janela está
visível e há reprodução. O backend continua sendo consultado quando a janela
está oculta para preservar sessão, eventos e bandeja.

### Contratos

- morune-core/src/playback.rs define PlaybackEngine, PlayerCommand,
  PlayerEvent, PlayerSnapshot, PlaybackState e AudioSettings.
- SpotifyEngine em morune-spotify/src/engine.rs recebe comandos por canal não
  bloqueante e publica eventos por broadcast.
- AppState decide o avanço real da fila; Next e Previous não são delegados
  como decisão de contexto ao engine.
- Load, Preload, play/pause, stop, seek, volume, shuffle e repeat são
  convertidos em comandos do engine.
- EndOfTrack retorna ao AppState, que chama Queue::next(false); quando a fila
  termina, o autoplay pode solicitar uma rádio do contexto atual.
- O engine usa MoruneSink/rodio/cpal para dispositivo e volume, e cria o
  player librespot com bitrate, normalização e cache configurados.
- A configuração persiste volume, shuffle, repeat, autoplay, normalização,
  bitrate, tamanho de cache e dispositivo de saída.
- A configuração de áudio do engine é efetivamente definida na criação;
  mudanças que exigem novo engine dependem do fluxo de reconexão/recriação.
- O engine Spotify declara seek/volume/posição precisa e gapless desabilitado.

### Fila e seleção

morune-core/src/queue.rs mantém o contexto atual, uma permutação para shuffle,
posição corrente, fila manual, histórico limitado a 200 itens, repeat e
origem (Album, Playlist, Artist, Search ou Custom).

play_track em AppState reaproveita o contexto quando a faixa já está na fila;
para Home, busca, liked e detalhes, cria ou seleciona o contexto correspondente.
preload_next adianta o próximo item sem alterar a posição.

## 8. Dados e APIs

### Serviços e contratos

| Área | Arquivo(s) | Função |
|---|---|---|
| Orquestração de conteúdo | morune-app/src/browse.rs | Busca, Home, Library, detalhes, páginas adicionais, artwork e autoplay |
| Ciclo de sessão | morune-app/src/session.rs | Restore, login, logout, reconexão e criação do engine |
| Backend runtime | morune-spotify/src/runtime.rs | Runtime Tokio dedicado, sessão compartilhada, catalog/library/artwork e áudio |
| Catálogo Spotify | morune-spotify/src/catalog.rs | Implementa Catalog e Library do core |
| Protocolo interno | morune-spotify/src/internal.rs | Rootlist, playlists, coleções, metadados, artistas e rádio via librespot |
| Busca e mutação | morune-spotify/src/pathfinder.rs, graphql.rs | searchDesktop, addToLibrary e removeFromLibrary |
| Autenticação | morune-spotify/src/auth.rs, token.rs | OAuth PKCE, refresh token e sessão login5 |
| Artwork remoto | morune-spotify/src/artwork.rs | Download autenticado/validado de imagens Spotify |
| Cache de artwork | morune-app/src/artwork.rs | Download assíncrono e cache local de capas/avatar |
| Persistência | morune-storage/src/paths.rs, config.rs, credentials.rs | Caminhos, TOML e credencial do sistema |

### Spotify e autenticação

- A sessão compartilhada (SharedSession) é reutilizada por autenticação,
  engine, catálogo e artwork.
- OAuth usa PKCE, listener local em 127.0.0.1:5588/login, estado CSRF,
  escopos de streaming, biblioteca, playlists, perfil, top/recentes e follows.
- O refresh token é salvo no Windows Credential Manager como
  Morune:spotify.refresh_token; não há senha do usuário no aplicativo.
- A busca usa https://api-partner.spotify.com/pathfinder/v1/query, operação
  persistida searchDesktop, com limite efetivo de até 20 itens por solicitação.
- A mutação de curtidas usa Pathfinder v2 e addToLibrary/removeFromLibrary.
- Rootlist, playlists, coleções de faixas/artistas, metadados e rádio usam
  endpoints/protocolos internos do Spotify/librespot.
- O backend implementa playlists salvas, faixas salvas, IDs de faixas salvas,
  artistas seguidos, rootlist e recomendações classificadas.
- Álbuns salvos, top tracks/artists e recently played estão marcados como não
  suportados no backend Spotify atual.

### Conteúdo e cache

- Browse expõe Card, Home, Detail, Outcome e paginações para a UI.
- Busca suporta faixas, álbuns, artistas e playlists.
- Detalhes de playlists e curtidas são carregados em páginas; playlists usam
  páginas de até 100 faixas.
- O cache interno do Spotify é em memória, com TTLs e limites próprios para
  playlists/metadados. O backend não possui um banco local dedicado.
- Artwork baixado pela aplicação usa cache em disco; o áudio usa o diretório
  de cache configurado pelo backend.
- Não há SQLite, servidor local de dados ou camada de banco de dados.
- morune-storage/src/favorites.rs define uma coleção local TOML de faixas,
  mas o AppState atualmente usa liked_ids e Library::set_track_saved no
  Spotify. A integração do módulo local de favoritos não está evidenciada no
  fluxo ativo da UI.

### Persistência local

| Dado | Local padrão | Responsável |
|---|---|---|
| Configuração | config/config.toml | morune-storage::Config |
| Temas importados | config/themes/ | morune-theme::loader |
| Refresh token | Windows Credential Manager | morune-storage::CredentialStore |
| Artwork | cache/artwork/ | morune-app::ArtworkCache |
| Áudio | cache/audio/ | Spotify/librespot |
| Atualizações | cache/updates/ | updater da aplicação |
| Backgrounds/owned assets | data/backgrounds/ | wallpaper/configuração |
| Log | data/morune.log | inicialização/tracing |
| Favoritos locais | data/favorites.toml | morune-storage::Favorites, sem uso ativo identificado |

## 9. Sistema visual

- A camada visual é composta por theme.slint, components.slint, app.slint e
  morune-app/src/theme_bridge.rs.
- Theme, Layout e Brand são globals Slint; os componentes consomem
  propriedades semânticas, não cores fixas por página.
- ThemeSpec combina manifest, cores, tipografia, formas, controles, motion,
  efeitos, background e layout.
- Tokens incluem cores semânticas, família/tamanho/peso de fonte, escala,
  espaçamento, raios, bordas, progress/scrollbar, duração/easing, opacidade,
  acrylic, blur, sombra, tint e background.
- `effects.material` distingue `legacy`, `aero` e `liquid`, independentemente
  de `gloss`. Pacotes sem esse campo preservam a seleção antiga. A ponte
  `theme_bridge.rs` traduz a família para `Theme.material-kind` no Slint.
- `morune-app/src/optics.rs` prepara lentes de borda sobre o wallpaper por
  `Theme.refract-background` e `LensRegion`. O cache tem limite de 4 MiB/12
  entradas e cada imagem tem no máximo 320 px por lado. `SceneGlass` usa essa
  lente quando existe wallpaper interno. Sem imagem, `SceneGlass` mantém só
  a orla e não repinta o fundo. Bruma pede Acrylic ao DWM; `main.rs` informa
  `AppWindow.native-backdrop-active` e a janela usa uma tinta de 12% quando o
  sistema aceita o material. Sem suporte ou com o portão de tela cheia ativo,
  usa a base de fallback do tema. Não captura a tela nem refrata
  conteúdo dinâmico da interface. Aquário usa gel vetorial e cena fotográfica;
  O campo central permanece transparente nos dois temas. A proteção de leitura
  fica nos itens, títulos e painéis; `Theme.glass-tint` ajusta a densidade do
  vidro à imagem amostrada. A ponte reaplica a proteção ao mudar preferências.
  As margens são consideradas pela grade.
- Layouts configuram posição/largura/colapso da sidebar, posição/altura e
  controles do player, densidade e modo de conteúdo (grid/list/compact).
- Temas TOML podem herdar outro tema; o loader sanitiza, valida contraste,
  aplica fallback para midnight e suporta packs .musicpack.
- O build fornece fontes bundled opcionalmente; ícones podem ser substituídos
  por arquivos do tema.
- reduce_motion zera as durações de motion aplicadas pelo bridge.
- Wallpaper suporta imagem, escala, opacidade, tint e blur; artwork atual pode
  fornecer cor dominante para a superfície do player.
- O build compila a UI com o estilo Slint fluent-dark, mas o visual efetivo é
  definido pelo sistema próprio de tokens e componentes.
- A responsividade não usa breakpoints CSS: AppWindow calcula colunas de cards
  a partir da largura disponível, sidebar e Layout.card-width.
- Tamanho mínimo documentado: 720x480 na janela normal e 600x120 no
  mini-player; sidebar e layout podem ser reposicionados pelo tema.

## 10. Dependências entre áreas

    AppWindow / callbacks Slint
      -> main.rs
      -> AppState
         -> Session
            -> SpotifyBackend
               -> SpotifyAuthenticator / SpotifyCatalog / SpotifyArtwork
               -> SpotifyEngine
                  -> librespot + MoruneSink

    AppState
      -> Browse
         -> Catalog / Library / Artwork
            -> SpotifyCatalog
               -> Internal + Pathfinder + SharedSession

    AppState
      -> morune_core::Queue
      -> PlaybackEngine
      -> PlayerEvent
      -> push_playback
      -> PlayerBar / TrayMenuWindow

    Config / AppPaths / CredentialStore
      -> AppState, Session, ArtworkCache, temas e áudio

    ThemeSpec
      -> theme_bridge.rs
      -> Theme / Layout / Brand
      -> Sidebar, páginas, cards, listas e PlayerBar

Regra arquitetural documentada: morune-core contém modelos e contratos sem
UI, Spotify ou I/O; morune-app faz a composição e wiring; o Spotify não acessa
diretamente a interface.

## 11. Arquivos críticos

### Essenciais

1. Cargo.toml — workspace, features, dependências e patch do librespot.
2. crates/morune-app/src/main.rs — inicialização, timers e wiring da aplicação.
3. crates/morune-app/src/state.rs — estado central e projeção para Slint.
4. crates/morune-app/src/session.rs — ciclo de vida da sessão Spotify.
5. crates/morune-app/src/browse.rs — orquestração de busca, conteúdo e artwork.
6. crates/morune-app/src/theme_bridge.rs — ponte entre tema TOML e globals Slint.
7. crates/morune-app/ui/app.slint — janela, páginas e composição estrutural.
8. crates/morune-app/ui/components.slint — controles e superfícies compartilhados.
9. crates/morune-app/ui/theme.slint — globals de tema/layout/marca.
10. crates/morune-core/src/playback.rs — contrato de comandos/eventos do player.
11. crates/morune-core/src/queue.rs — contexto, fila manual, shuffle e repeat.
12. crates/morune-spotify/src/runtime.rs — runtime e recursos Spotify compartilhados.
13. crates/morune-spotify/src/engine.rs — engine de áudio e tradução de eventos.
14. crates/morune-spotify/src/catalog.rs — implementação de catálogo/biblioteca.
15. crates/morune-storage/src/config.rs — configuração persistida da aplicação.

### Secundários

- Core: model.rs, catalog.rs, auth.rs, error.rs, lib.rs.
- Spotify: auth.rs, token.rs, internal.rs, pathfinder.rs, graphql.rs,
  artwork.rs, sink.rs, error.rs.
- Storage: paths.rs, credentials.rs, favorites.rs, lib.rs.
- Theme: spec.rs, tokens.rs, layout.rs, loader.rs, manifest.rs, pack.rs,
  icons.rs, color.rs, watch.rs.
- App: artwork.rs, wallpaper.rs, tint.rs, update.rs, tray.rs, tray_menu.rs,
  taskbar.rs, smtc.rs, startup.rs, bundled.rs, build.rs e módulos específicos
  de Windows/snapshot.
- UI/documentação: docs/ARCHITECTURE.md, documentação de temas, temas
  distribuídos em crates/morune-app/themes/ e recursos em assets/.

## 12. Pontos que precisam de investigação posterior

- Confirmar se morune-storage::Favorites é legado, preparação para modo local
  ou fluxo planejado; hoje o favorito visível é sincronizado com Spotify.
- Definir o comportamento esperado para álbuns salvos, top/recentes e outras
  capacidades que o contrato core prevê, mas o backend Spotify marca como
  não suportadas.
- Verificar a estabilidade dos hashes/formatos Pathfinder e dos endpoints
  internos usados por busca, mutações e coleções.
- Confirmar quais mudanças de áudio devem recriar o engine e quais podem ser
  aplicadas ao engine existente.
- Validar a semântica final de playlists classificadas como recomendações,
  estações e retrospectivas a partir do rootlist.
- Investigar consumo efetivo de tokens de layout declarados no tema quando
  houver trabalho visual futuro.
- Se houver upgrade do librespot, revisar a necessidade e o comportamento do
  patch em vendor/librespot-core.

### Escopo deste mapeamento

Foram mapeados: workspace e stack, crates, entrypoints, páginas, componentes
estruturais, estado, sessão, player, fila, serviços Spotify, autenticação,
persistência, cache, temas, layout e dependências entre áreas.

Não foram abertos integralmente: arquivos gerados (target/, dist/,
bench-out/), dependências vendorizadas em detalhe, lockfile completo,
imagens/assets individualmente, testes e auxiliares não necessários para o
fluxo estrutural, além dos módulos de plataforma e atualização que não alteram
o desenho central descrito acima.

Nenhuma alteração além deste arquivo foi realizada.
