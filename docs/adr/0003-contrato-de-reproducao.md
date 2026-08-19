# ADR-0003 — Reproducao por comandos e eventos, sem `async` no trait

Data: 2026-08-18
Status: aceito

## Contexto

A aplicacao precisa trocar de backend de reproducao em tempo de execucao:
`NullEngine` antes do login, Spotify depois dele, e possivelmente arquivos
locais no futuro. Isso exige guardar `Arc<dyn PlaybackEngine>`, ou seja, um
trait dyn-compativel.

Ao mesmo tempo, quase toda operacao de um player e assincrona: carregar uma
faixa envolve rede, abrir o dispositivo de audio envolve o driver, buscar
posicao envolve o decodificador.

## O problema

`async fn` em trait e estavel desde o Rust 1.75, mas o trait resultante **nao e
dyn-compativel**. Nao da para ter `Arc<dyn PlaybackEngine>` com `async fn play()`.

## Alternativas consideradas

**`#[async_trait]`.** Resolve a dyn-compatibilidade encaixotando cada future.
Custa uma alocacao por chamada e, mais grave, mantem a forma errada: a interface
faria `engine.play().await` e ficaria esperando o backend.

**`Pin<Box<dyn Future>>` escrito a mao.** Mesma forma, sem a macro.

**Comandos e eventos.** O trait fica sincrono e trivialmente dyn-compativel:

```rust
fn send(&self, command: PlayerCommand) -> CoreResult<()>;   // nunca bloqueia
fn snapshot(&self) -> PlayerSnapshot;                       // barato
fn subscribe(&self) -> broadcast::Receiver<PlayerEvent>;
```

## Decisao

Comandos e eventos para `PlaybackEngine`.

Para `Catalog` e `Library`, que de fato precisam **devolver dados**, o retorno e
`Pin<Box<dyn Future>>` escrito a mao — a forma assincrona ali e legitima, e a
dyn-compatibilidade continua sendo requisito.

## Por que essa forma e melhor, e nao so mais conveniente

A interface **nunca** fica presa esperando rede, disco ou dispositivo de audio.
Todo o custo de latencia fica do lado do backend, e a interface so reage a
eventos. Isso vale mais que a economia de alocacoes: e a diferenca entre uma
janela que trava ao apertar play e uma que responde na hora.

`snapshot()` ser barato tambem e parte do contrato: a interface le o estado a
cada quadro.

`user_advance` em `Queue::next` segue a mesma logica de separar intencao de
efeito: com `RepeatMode::One`, so o avanco automatico repete a faixa; clicar em
"proximo" sempre avanca, que e o que o usuario espera.

## Consequencias

O backend fica mais complexo: precisa de uma tarefa propria consumindo comandos
e publicando eventos. Em troca, `NullEngine` cabe em 40 linhas, a interface nao
tem um unico `await`, e testar o contrato nao exige runtime assincrono.

O preco visivel e que uma acao nao retorna sucesso ou falha de imediato: o
resultado chega por `PlayerEvent`. A interface ja e escrita assim.
