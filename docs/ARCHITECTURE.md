# Arquitetura

## Regra que organiza tudo

**A logica do Spotify nunca toca a interface.** Entre os dois existe um conjunto
de contratos em `morune-core` que nenhuma das pontas pode contornar.

Isso nao e purismo. E o que torna possivel:

- testar fila, shuffle e repeticao sem rede, sem disco e sem janela;
- abrir a tela com dados de demonstracao (`MORUNE_SNAPSHOT_*`) para capturar
  temas sem precisar de conta nem de credencial;
- trocar de provedor de musica sem reescrever nenhuma tela;
- abrir o aplicativo mesmo sem sessao, sem backend e com tema corrompido.

---

## Camadas

```
morune-desktop    o app: motor de tela em Rust + tela web (Tauri, Svelte)
   |  depende de
morune-spotify    login, reproducao, catalogo e biblioteca sobre librespot
morune-storage    configuracao, caminhos, cofre de credenciais
morune-theme      esquema de tema, carregamento, pacotes .musicpack
   |  depende de
morune-core       modelo, fila, contratos. Zero UI, zero Spotify, zero I/O.
```

As setas apontam sempre para baixo. `morune-core` nao conhece nenhuma das
outras; `morune-theme` nao conhece a tela; `morune-desktop` e a unica que
conhece todas.

### `morune-core`

O que e dominio de verdade e o que todo backend precisa satisfazer.

| Modulo | Papel |
|---|---|
| `model` | `Track`, `Album`, `Artist`, `Playlist` e ids com provedor embutido |
| `queue` | fila, shuffle, repeticao, historico, "tocar a seguir" |
| `playback` | contrato `PlaybackEngine` + `NullEngine` |
| `catalog` | contratos `Catalog` e `Library` |
| `auth` | contratos `Authenticator` e `CredentialStore`, tipo `AccessToken` |
| `error` | `CoreError` classificado em auth / transitorio / permanente |

Duas decisoes explicam a forma dos contratos:

**Ids carregam o provedor.** `TrackId { provider, id }` significa que
`spotify:abc` e `local:abc` nunca colidem em cache, banco ou fila. Um modelo
"do Spotify" teria travado o projeto no primeiro backend.

**`PlaybackEngine` nao tem `async fn`.** O trait precisa ser dyn-compativel para
que a aplicacao guarde `Arc<dyn PlaybackEngine>` e troque de backend em
execucao. Comandos sao enfileirados (`send`), o estado e lido barato
(`snapshot`) e o resultado chega por evento (`subscribe`). A interface nunca
espera rede, disco ou dispositivo de audio.

`Catalog` e `Library`, que de fato precisam devolver dados, usam
`Pin<Box<dyn Future>>` escrito a mao pelo mesmo motivo: `async fn` em trait
ainda nao e dyn-compativel.

**`NullEngine` existe de proposito.** Sem ele, "nao ha motor de reproducao"
viraria um `Option` que toda tela teria de tratar — e o lugar onde nascem os
panics. Com ele a aplicacao sempre tem um motor valido: antes do login, com o
backend em falha, e em teste.

### `morune-theme`

Motor de customizacao. Garante tres coisas a quem esta acima:

1. `loader::load` **nunca falha**. Tema corrompido vira o tema embutido mais uma
   lista de diagnosticos.
2. Todo `ThemeSpec` entregue ja passou por `sanitize`, entao a interface nao
   valida nada.
3. Nada de dentro de um `.musicpack` e escrito em disco antes de passar por
   `sanitize_entry_path` e `check_entry_allowed`.

O caminho de validacao roda **inclusive no tema embutido**, em todo boot. Um
teste falha se o tema embutido produzir qualquer aviso. Assim o caminho de erro
e exercitado sempre, e nao so quando ja e tarde.

### `morune-storage`

Configuracao, caminhos e segredos.

- gravacao atomica (arquivo temporario + renomeacao) para que uma queda no meio
  nao trunque a configuracao;
- configuracao corrompida vira `.bak` antes de ser substituida pelos padroes;
- segredos vao para o Gerenciador de Credenciais do Windows, nunca para arquivo
  nosso.

### `morune-desktop`

O app. Duas metades que conversam por JSON:

- **o motor** (`src/`, Rust) — `state.rs` guarda tudo que a tela mostra e
  todas as acoes; `ui.rs` e o retrato desse estado, serializado para a tela;
  `main.rs` e fiacao, laco do motor e janelas; os modulos do Windows (`smtc`,
  `taskbar`, `notify`, `hotkeys`, `startup`, `instance`, `tela_cheia`,
  `material`) ficam cada um no seu arquivo;
- **a tela** (`ui/`, Svelte + Vite) — so desenha e pede. `lib/motor.svelte.ts`
  recebe o estado, `lib/tema.ts` aplica o tema como variaveis CSS, e
  `paginas/` tem uma pagina por tela.

A tela fala com o motor por tres comandos do Tauri: `estado_inicial` (o
retrato inteiro, uma vez), `acao(nome, args)` (tudo que a pessoa faz) e
`pronta`. O motor responde por eventos: `estado` leva **so os campos que
mudaram** desde o ultimo envio, `tema` leva o tema quando ele troca, e
`tela_cheia` e `visibilidade` dizem quando parar de animar.

**O motor mora num `RefCell` na linha principal** (`com_motor`). Mexer na
janela por dentro dele -- tamanho, maximizar, criar janela -- faz o Windows
chamar o tratador na hora, que pede o motor de novo. Isso e feito fora, com
`depois()` (`run_on_main_thread`).

`bundled.rs` grava os temas de fabrica na primeira execucao e so os atualiza
quando o tema instalado ainda e o que o app pos la.

---

## Como um tema chega na tela

```
manifest.toml + theme.toml
        |  serde
   ThemeSpec  (tipado, com padrao para todo campo)
        |  sanitize  -> Vec<ThemeWarning>
   ThemeSpec valido  + ajustes da pessoa (liberdade total)
        |  AppState::tema_json
   evento `tema`
        |  ui/src/lib/tema.ts
   variaveis CSS no :root -- a tela inteira reage
```

Trocar de tema e mandar o evento de novo. Nao ha recarga de pagina nem
recriacao de janela, e a reproducao nao e tocada.

**Por que tokens e nao CSS dentro do pacote.** CSS ou HTML no pacote daria
liberdade total de layout, mas e formato ativo: seletores alcancam qualquer
coisa, e `url()` busca na rede. O tema continua sendo dado -- cores, forma,
fonte, movimento, fundo -- e a tela decide o layout. Ver
[ADR-0004](adr/0004-temas-declarativos.md).

---

## Vida do processo

O app nao morre quando a janela fecha. Isso muda tres coisas:

**Fechar para a bandeja destroi a tela.** Sobra so o motor, tocando. A janela
volta inteira quando a pessoa clica na bandeja. Minimizada, a tela e suspensa;
com jogo ou video em tela cheia, toda animacao vai a zero. E isso que deixa o
app parado enquanto alguem joga (ADR-0007).

**O motor gira sozinho.** Uma linha de fundo acorda o motor a cada 100 ms
(`GIRO`): recolhe respostas do Spotify, eventos de reproducao, bandeja, teclas
de midia e botoes da barra de tarefas, e manda a diferenca para a tela, se
houver tela.

**Sair precisa ser sempre alcancavel.** Se a bandeja falhar ao ser criada, o
app volta a encerrar ao fechar. Um processo vivo, invisivel e sem forma de
encerrar seria pior que perder a funcionalidade.

## Ordem de inicializacao

Deliberada, e parte do orcamento de startup. Nada que dependa de rede acontece
antes da janela aparecer.

1. caminhos e log — sem I/O pesado
2. temas de exemplo, se faltarem
3. configuracao
4. tema, com fallback garantido
5. janela
6. **so entao** autenticacao, biblioteca, cache

---

## `morune-spotify`

Implementa os quatro contratos do core sem que nenhuma tela saiba disso. O
contrato foi fechado e testado contra `NullEngine` antes desta crate existir, e
nao mudou por causa dela — que era o ponto de te-lo escrito primeiro.

| Modulo | Papel |
|---|---|
| `token` | fonte unica de token: obtem, guarda no cofre e renova sob trava |
| `auth` | `Authenticator`: OAuth PKCE, restauracao de sessao, logout |
| `engine` | `PlaybackEngine` sobre a librespot, em runtime tokio proprio |
| `pathfinder` + `graphql` | busca por consulta persistida do player web |
| `internal` | playlists, colecao, metadado, artista e radio pelo protocolo interno |
| `artwork` | download validado de capas |
| `catalog` | `Catalog` e `Library` sobre os caminhos anteriores |
| `runtime` | dono do runtime e ponto de entrada da crate |

**Tudo nasce de uma sessao da librespot.** Audio e catalogo interno usam a
sessao diretamente; apenas a busca passa pelo `api-partner`/Pathfinder, assinada
pelos tokens que essa mesma sessao mantem. O Web API publico saiu do desenho
depois de responder 429 para qualquer token medido.

**O canal interno faz o que o Web API parou de fazer.** Playlists, curtidas,
artistas seguidos, capas, detalhes e radio continuam disponiveis pelo protocolo
do cliente oficial. O criterio para usar um caminho ou outro e onde o dado foi
medido, nao preferencia de implementacao.

**A interface nunca espera rede.** Comandos de reproducao entram por canal e nao
devolvem resultado; quem quer saber o que aconteceu assina os eventos. Consultas
de catalogo viram tarefa no runtime do backend e sao recolhidas no giro de
100 ms do motor, que ja atende bandeja e reproducao.

---

## O que ainda nao existe

Albuns salvos, historico recente e estatisticas de mais ouvidos nao possuem um
caminho estavel conhecido. Escrita na conta tambem esta fora do escopo atual.
Ver [ROADMAP.md](ROADMAP.md).
