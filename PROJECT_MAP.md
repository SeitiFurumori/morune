# Project Map

## 1. Visão geral

- Otomori (até a 1.0, Morune) é um player de Spotify para Windows: motor em
  Rust e tela web servida pelo WebView2 do Windows, via Tauri 2.
- O workspace separa contratos e modelos (`morune-core`), tema
  (`morune-theme`), persistência (`morune-storage`), integração Spotify
  (`morune-spotify`) e o app (`morune-desktop`). Os nomes dos crates ficaram
  como eram; só o produto mudou de nome.
- `AppState` (`morune-desktop/src/state.rs`) é o motor da tela: sessão,
  navegação, dados, fila e reprodução. A tela web só desenha o estado que
  recebe e pede ações.
- A integração Spotify usa OAuth PKCE, librespot, o protocolo interno do
  Spotify e o Pathfinder do player web.
- Temas são dados TOML (cores, fonte, forma, movimento, efeitos), aplicados
  pela tela como variáveis CSS. Pacotes `.musicpack` para trocar temas.
- Fechada para a bandeja, a tela é destruída e sobra o motor tocando; com jogo
  ou vídeo em tela cheia, toda animação para.

## 2. Stack

| Área | Tecnologia |
|---|---|
| Linguagem | Rust 2021 (rust-version 1.92); TypeScript na tela |
| Tela | Tauri 2 + WebView2; Svelte 5 + Vite 6 em `crates/morune-desktop/ui` |
| Build | Cargo workspace; npm (`npm ci && npm run build` antes do cargo) |
| Concorrência | Tokio multi-thread, canais mpsc/broadcast |
| Áudio | librespot, rodio/cpal e sink próprio |
| Spotify | OAuth 2.0 PKCE, login5, Pathfinder GraphQL, protobuf |
| HTTP | reqwest |
| Persistência | TOML em `%APPDATA%\otomori`, Gerenciador de Credenciais do Windows |
| Plataforma | Windows; toolchain GNU MinGW; crate `windows` para painel de mídia, barra de tarefas, notificações, atalhos e acrílico |
| Empacotamento | NSIS (`installer/otomori.nsi`) via `tools/build-installer.ps1`; release no GitHub Actions |

Dependências locais: `morune-desktop` depende de todos os outros;
`morune-storage` depende de `morune-core` e `morune-theme`; `morune-spotify`
depende de `morune-core`; `morune-core` e `morune-theme` não conhecem a tela.

## 3. Estrutura principal

```
crates/
  morune-core/       modelo, fila, contratos (sem UI, sem rede, sem disco)
  morune-theme/      ThemeSpec, carregamento, validação, .musicpack
  morune-storage/    caminhos (e cópia da pasta do Morune), config, cofre
  morune-spotify/    login, reprodução, catálogo e biblioteca
  morune-desktop/
    src/             o motor: main.rs (fiação, laço, janelas), state.rs
                     (AppState), ui.rs (retrato do estado em JSON), browse.rs,
                     session.rs, módulos do Windows
    ui/src/          a tela: App.svelte, Mini.svelte, MenuBandeja.svelte,
                     paginas/, lib/
    themes/          Lume, Vidro, Grade, Claro (gravados na primeira execução)
    examples/        sondas que conferem dados reais da conta
installer/           otomori.nsi
tools/               build-installer, licenças, assinatura, env.ps1
vendor/              librespot-core com patch local
docs/                arquitetura, temas, ADRs, publicação
```

## 4. Janelas e páginas

| Janela | Arquivo | Papel |
|---|---|---|
| Principal | `ui/src/App.svelte` | lateral, página atual, player |
| Mini player | `ui/src/Mini.svelte` | capa, controles, fixar por cima, buscar |
| Menu da bandeja | `ui/src/MenuBandeja.svelte` | faixa, controles e volume com a janela fechada |

Páginas (`ui/src/paginas/`): Início, Busca, Biblioteca, Detalhe (álbum,
playlist, artista), Fila, Configurações. `MORUNE_START_PAGE` abre direto numa
delas (0 Início, 1 Busca, 2 Biblioteca, 3 Configurações, 4 Fila, 5 Detalhe).

## 5. Componentes da tela

`ui/src/lib/`: `Player` (barra de reprodução; `Onda` desenha a música),
`Lateral`, `Faixas` (lista com arrastar para reordenar), `Prateleira` e
`Cartao`, `Capa`, `Fundo` (imagem, GIF ou vídeo, que para com jogo), `Menu`,
`Marca`, `Icone`. Estado e ajudas: `motor.svelte.ts` (recebe o estado),
`tema.ts` (tema em variáveis CSS), `cores.ts`, `voo.svelte.ts` (capa que voa),
`contexto.svelte.ts`.

## 6. Estado e comunicação

- O motor mora num `RefCell` na linha principal (`com_motor` em `main.rs`).
  Mexer em janela por dentro dele faz o Windows reentrar e travar a tela: isso
  vai para `depois()`.
- Uma linha de fundo acorda o motor a cada 100 ms (`GIRO`); o progresso vai a
  cada 250 ms.
- Comandos da tela: `estado_inicial`, `acao(nome, args)`, `pronta`.
- Eventos para a tela: `estado` (só os campos que mudaram), `tema`,
  `tela_cheia`, `visibilidade`.

## 7. Player

### Superfícies

- `Player.svelte` na janela principal; `Mini.svelte`; `MenuBandeja.svelte`.
- Painel de mídia do Windows (`smtc.rs`), botões na miniatura da barra de
  tarefas (`taskbar.rs`), teclas de atalho (`hotkeys.rs`) e notificação de
  troca de faixa (`notify.rs`).

### Fluxo de comandos e eventos

    clique na tela
      -> acao(nome, args)  (comando Tauri)
      -> AppState
      -> PlaybackEngine::send
      -> canal Tokio
      -> SpotifyEngine
      -> librespot Player

    librespot
      -> SpotifyEngine traduz eventos
      -> broadcast PlayerEvent
      -> AppState::poll_backend (no GIRO)
      -> evento `estado` com o que mudou
      -> Player, Mini e MenuBandeja

Sem conexão, os comandos não vão para o motor morto: a intenção fica na fila
e em `playing`, e `restore_recreated_engine` a refaz quando a sessão volta.

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
| Orquestração de conteúdo | morune-desktop/src/browse.rs | Busca, Home, Library, detalhes, páginas adicionais, artwork e autoplay |
| Ciclo de sessão | morune-desktop/src/session.rs | Restore, login, logout, reconexão e criação do engine |
| Backend runtime | morune-spotify/src/runtime.rs | Runtime Tokio dedicado, sessão compartilhada, catalog/library/artwork e áudio |
| Catálogo Spotify | morune-spotify/src/catalog.rs | Implementa Catalog e Library do core |
| Protocolo interno | morune-spotify/src/internal.rs | Rootlist, playlists, coleções, metadados, artistas e rádio via librespot |
| Busca e mutação | morune-spotify/src/pathfinder.rs, graphql.rs | searchDesktop, addToLibrary e removeFromLibrary |
| Autenticação | morune-spotify/src/auth.rs, token.rs | OAuth PKCE, refresh token e sessão login5 |
| Artwork remoto | morune-spotify/src/artwork.rs | Download autenticado/validado de imagens Spotify |
| Cache de artwork | morune-desktop/src/artwork.rs | Download assíncrono e cache local de capas/avatar |
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

Caminhos dentro de `%APPDATA%\otomori\Otomori`; o cache fica em
`%LOCALAPPDATA%\otomori\Otomori`. Na primeira abertura da 2.0, config e dados
vêm copiados de `%APPDATA%\morune\Morune` (`AppPaths::migrate_from_morune`).

| Dado | Local | Responsável |
|---|---|---|
| Configuração | config/config.toml | morune-storage::Config |
| Temas importados | config/themes/ | morune-theme::loader |
| Refresh token | Windows Credential Manager | morune-storage::CredentialStore |
| Artwork | cache/artwork/ | morune-desktop::ArtworkCache |
| Áudio | cache/audio/ | Spotify/librespot |
| Atualizações | cache/updates/ | updater da aplicação |
| Backgrounds/owned assets | data/backgrounds/ | wallpaper/configuração |
| Log | data/otomori.log | inicialização/tracing |
| Ordem das curtidas, retomar, buscas recentes | data/ordem-curtidas.json, data/retomar.json, data/ | ordem_curtidas.rs, retomar.rs, recentes_busca.rs |
| Favoritos locais | data/favorites.toml | morune-storage::Favorites, sem uso ativo identificado |

## 9. Sistema visual

- O tema chega à tela como JSON (`AppState::tema_json`): o `ThemeSpec` já
  validado, com os ajustes da pessoa (modo liberdade total) por cima.
- `ui/src/lib/tema.ts` traduz o tema em variáveis CSS no `:root` (`--fundo`,
  `--acento-tema`, `--r-md`, `--m-normal`...). Componentes só usam variáveis.
- `effects.material` (`legacy`, `aero`, `liquid`) vira `data-material`; o
  acrílico do Windows é pedido em `material.rs`, e com a transparência do
  Windows desligada o fundo fica sólido.
- A cor da capa entra como acento vivo, brilho e, se o tema pedir
  (`artwork_paints_ui`), no resto da tela.
- O movimento para (`data-parado`, `data-quieta`) com movimento reduzido, tema
  sem movimento, janela minimizada ou algo em tela cheia.

## 10. Dependências entre áreas

- A tela não conhece Spotify nem disco: só o estado em JSON e `acao`.
- `morune-spotify` não conhece a tela; `AppState` junta os dois.
- O tema é o único dado que vai da pasta do usuário até o CSS, e passa por
  `sanitize` antes.

## 11. Arquivos críticos

| Arquivo | Por quê |
|---|---|
| `crates/morune-desktop/src/main.rs` | fiação, laço do motor, janelas, bandeja, comandos |
| `crates/morune-desktop/src/state.rs` | todo o comportamento do app |
| `crates/morune-desktop/src/ui.rs` | o que a tela recebe |
| `crates/morune-desktop/ui/src/lib/motor.svelte.ts` | como a tela recebe |
| `crates/morune-desktop/ui/src/lib/tema.ts` | como o tema vira CSS |
| `crates/morune-storage/src/paths.rs` | pastas e cópia da pasta do Morune |
| `crates/morune-spotify/src/catalog.rs` | de onde vêm os dados |
| `installer/otomori.nsi` | instalar e substituir o Morune |

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
- `layout.toml` e `[scene]` dos temas da 1.0 ainda sao lidos pelo
  `morune-theme`, mas a tela nova os ignora.
- Se houver upgrade do librespot, revisar a necessidade e o comportamento do
  patch em vendor/librespot-core.

