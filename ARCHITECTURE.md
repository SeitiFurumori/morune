# Arquitetura

## Regra que organiza tudo

**A logica do Spotify nunca toca a interface.** Entre os dois existe um conjunto
de contratos em `morune-core` que nenhuma das pontas pode contornar.

Isso nao e purismo. E o que torna possivel:

- testar fila, shuffle e repeticao sem rede, sem disco e sem janela;
- rodar a interface contra implementacoes falsas para capturar temas em PNG
  (`tools/snapshot.ps1`) sem precisar de conta nem de credencial;
- trocar de provedor de musica sem reescrever nenhuma tela;
- abrir o aplicativo mesmo sem sessao, sem backend e com tema corrompido.

---

## Camadas

```
morune-app        interface Slint, estado de tela, fiacao
   |  depende de
morune-spotify    login, reproducao, catalogo e biblioteca sobre librespot
morune-storage    configuracao, caminhos, cofre de credenciais
morune-theme      esquema de tema, carregamento, pacotes .musicpack
   |  depende de
morune-core       modelo, fila, contratos. Zero UI, zero Spotify, zero I/O.
```

As setas apontam sempre para baixo. `morune-core` nao conhece nenhuma das
outras; `morune-theme` nao conhece Slint; `morune-app` e a unica que conhece
todas.

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

### `morune-app`

Interface e fiacao.

- `ui/*.slint` — a interface. Nenhuma cor, tamanho ou duracao literal: se um
  valor nao esta em `Theme` ou `Layout`, ele nao e customizavel, e isso e um bug.
- `theme_bridge.rs` — **o unico lugar** que conhece o formato de tema e o
  formato da interface ao mesmo tempo.
- `state.rs` — estado de tela e as acoes que a interface dispara.
- `main.rs` — so fiacao e ordem de inicializacao.
- `bundled.rs` — temas de exemplo embutidos, gravados na primeira execucao e
  nunca sobrescritos depois.

---

## Como um tema chega na tela

```
theme.toml + layout.toml
        |  serde
   ThemeSpec  (tipado, com padrao para todo campo)
        |  sanitize  -> Vec<ThemeWarning>
   ThemeSpec valido
        |  theme_bridge::apply
   globais Theme e Layout do Slint
        |
   a interface inteira reage
```

Trocar de tema e re-executar a etapa `apply`. Nao ha recompilacao, nao ha
recriacao de janela, e o estado de reproducao nao e tocado.

**Por que globais tipadas e nao `.slint` dentro do pacote.** Um `.slint` no
pacote daria liberdade total de layout, mas um arquivo `.slint` tem expressoes,
ligacoes e callbacks — e um formato ativo, nao um dado. Comeca declarativo e
some dessa condicao aos poucos. O conjunto de globais cobre os eixos que
importam (posicao, tamanho, densidade, forma, movimento, visibilidade de cada
peca) com um formato que e inequivocamente dado. Layout arbitrario via
`slint-interpreter` esta previsto atras do Developer Mode — ver
[ADR-0004](docs/adr/0004-temas-declarativos.md).

---

## Vida do processo

O aplicativo nao morre quando a janela fecha. Isso muda tres coisas:

**O laco de eventos e explicito.** `Window::run()` termina quando a janela some,
que e exatamente o que nao pode acontecer. O `main` faz `window.show()` seguido
de `slint::run_event_loop_until_quit()`; quem encerra o laco e `quit_event_loop`,
chamado pelo item "Sair" da bandeja.

**Fechar e uma decisao, nao um fim.** `on_close_requested` consulta a
configuracao e sempre esconde a janela; a diferenca esta em encerrar o laco ou
nao.

**Sair precisa ser sempre alcancavel.** Se a bandeja falhar ao ser criada, o
aplicativo volta a encerrar ao fechar. Um processo vivo, invisivel e sem forma
de encerrar seria pior que perder a funcionalidade.

A bandeja entrega eventos por canal, entao ha uma leitura a cada 150 ms num
`slint::Timer`. O temporizador precisa continuar vivo: descartado, a bandeja
para de responder — por isso `wire_tray` devolve o `Timer` em vez de solta-lo.

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
| `webapi` | transporte HTTP autenticado do Web API |
| `dto` | JSON do Spotify -> modelo do core, sem rede e testavel |
| `catalog` | `Catalog` e `Library` sobre os dois anteriores |
| `runtime` | dono do runtime e ponto de entrada da crate |

**Sao dois canais, e nao um por escolha.** O protocolo que a librespot fala
entrega audio; busca e biblioteca so existem no Web API, que e HTTP comum. Os
dois usam a mesma `Session` e o mesmo token: duas conexoes gastariam dois slots
de dispositivo na conta, e o Spotify derrubaria uma delas.

**A interface nunca espera rede.** Comandos de reproducao entram por canal e nao
devolvem resultado; quem quer saber o que aconteceu assina os eventos. Consultas
de catalogo viram tarefa no runtime do backend e sao recolhidas no temporizador
de 100 ms que ja atende bandeja e reproducao.

---

## O que ainda nao existe

Capas (download e cache), paginacao das telas de conteudo e telas de detalhe de
album, artista e playlist. Ver [ROADMAP.md](ROADMAP.md).
