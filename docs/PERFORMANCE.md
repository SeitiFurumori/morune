# Performance

## Regra

Nenhum numero aqui e estimativa. Todo valor desta pagina saiu de
`tools/measure.ps1` nesta maquina, e a data esta registrada. Se uma coluna nao
tem numero, e porque a funcionalidade ainda nao existe — nao porque nao foi
medida.

## O criterio

O Morune existe para tocar musica enquanto o computador faz outra coisa —
tipicamente jogar. Isso define o que conta como bom desempenho aqui, e nao e o
numero que parece obvio.

**Nao e "usar pouca RAM".** Numa maquina com 16 GB, 70 ou 90 MB de working set
nao muda nada para ninguem. Uma meta rigida de MB so criaria pressao para
otimizar o que ja e irrelevante.

**E "nao aparecer".** O Morune tem de ser indistinguivel de um processo parado
enquanto o usuario esta jogando: sem roubar quadro, sem acordar a GPU, sem
disputar CPU, e sem a interface travar quando ele volta para ela — que e
exatamente o que o cliente oficial do Spotify faz de errado.

Dai a ordem de prioridade das metricas abaixo: **CPU e GPU em segundo plano
primeiro, resposta da interface depois, memoria por ultimo** — e memoria com
teto explicito para nao crescer sem limite, nao com meta de vitrine.

## Medicao de 04/09/2026 -- reproducao real, pela primeira vez

Ate aqui todo numero desta pagina era de aplicativo parado. Esta e a primeira
medicao **com musica tocando**, que e o estado em que o Morune passa a vida.

```
cenario        : tocando, janela escondida na bandeja
antes          : 3,41% de um nucleo   (60 s, processo aberto ha ~40 min)
depois         : 1,67% de um nucleo   (3 x 60 s: 1,43 | 1,67 | 1,90)
```

**O que a medicao achou.** Com a janela escondida, a thread da **interface**
gastava 1,93% de um nucleo contra 1,09% da saida de audio: o aplicativo gastava
mais desenhando o que ninguem via do que tocando musica. Tres tarefas rodavam
sem tela na frente -- o relogio de progresso escrevendo na interface 4x por
segundo, o espelhamento de estado a cada mudanca, e duas chamadas ao DWM por
segundo para arredondar canto e aplicar efeito de janela. As tres passaram a
sair cedo quando `window().is_visible()` e falso, e o que se acumulou e
espelhado de uma vez quando a janela volta.

**O que a medicao desmentiu.** Amostras de 24% a 54% de um nucleo, colhidas mais
cedo no mesmo dia, foram lidas como regime e nao eram: e o custo de aquecimento
dos primeiros ~30 minutos -- capa baixando, listas populando, tint
recalculando. A ordem cronologica denuncia (53,8 -> 46,8 -> 23,7 -> 3,4), e a
hipotese de que o misturador do rodio custava caro caiu junto: a saida de audio
aparece com 1,09% quando medida por thread.

**Ressalva.** O "antes" foi medido num processo aberto ha 40 minutos e o
"depois" num processo recem-aberto. A diferenca de 1,7 ponto e grande demais
para ser so isso, e o mecanismo da correcao aponta para onde o ganho apareceu
(a thread da interface saiu do topo), mas a comparacao nao e perfeitamente
controlada.

**Fica em aberto:** o custo de aquecimento. Se alguem abre o Morune e entra num
jogo em seguida -- que e o gesto natural --, pega justamente a meia hora cara.
Ninguem instrumentou o caminho de capas para saber onde esse tempo vai.

## Orcamento

| Metrica | Meta | Medido | Situacao |
|---|---|---|---|
| CPU em repouso, janela visivel | ~0% | **0,23%** (06/09) | cumprido (era 1,48% em 30/08) |
| CPU na bandeja, janela oculta | ~0% | **0,00%** (19/08) | nao remedido em 30/08 |
| GPU em repouso, janela visivel | sem redesenho | **0,00%** | cumprido |
| Interferencia com jogo em tela cheia | imperceptivel | — | ferramenta pronta, falta a sessao |
| Resposta da interface | sem travar, nunca | — | exige sessao real prolongada |
| CPU em reproducao, janela oculta | < 2% | **1,67%** | cumprido (era 3,41% antes de 04/09) |
| CPU em reproducao, janela visivel | < 2% | — | nao medido em regime |
| Startup ate o laco de eventos | < 1 s | **62 ms** | folgado, mas 3x o de 19/08 |
| Ciclo completo do processo | < 1 s | **1.015 ms** | **estourou** (era 509 ms) |
| RAM em repouso, janela visivel | teto, nao vitrine | **132,1 MB** (06/09) | explicado: quase tudo e driver de video |
| RAM na bandeja, janela oculta | o menor possivel | **~2 MB** (06/09) | era 167 MB antes de devolver a memoria |
| RAM em reproducao com capas | crescimento limitado | — | cache em disco limitado; RAM nao medida |
| Tamanho do instalador | < 40 MB | **5,31 MB** (19/08) | nao reempacotado |
| Tamanho do executavel | — | **15,48 MiB** | +1,70 MiB desde 19/08 |
| Dependencia de Chromium | nenhuma | nenhuma | cumprido |

O executavel foi remedido em 20/08/2026 depois de habilitar AccessKit; cresceu
0,21 MiB. A biblioteca local de favoritos acrescentou 0,13 MiB e a gestao da
fila, 0,16 MiB. O tamanho do
instalador acima ainda e o ultimo pacote gerado e deve
ser atualizado na proxima rodada de empacotamento.

As linhas sem numero dependem de uma sessao Premium real, com musica tocando e
um jogo em tela cheia. O teste sintetico nao substitui esse cenario.

## Medicao de 30/08/2026

Mesma maquina de 19/08. Binario de release do dia, **sem login** — a sessao do
Gerenciador de Credenciais nao foi restaurada nesta rodada, entao os numeros sao
do aplicativo parado, sem rede e sem audio.

```
executavel   : target\release\morune.exe
tamanho      : 15,48 MiB (16,23 MB)

startup interno          : primeiro 246,0 | mediana 62,0 | min 59,0 | max 246,0 ms
startup processo inteiro : primeiro 1351,4 | mediana 1014,8 | min 1008,2 | max 1351,4 ms

working set    : media 135,6 MB | pico 137,2 MB
memoria privada: media 162,5 MB | pico 165,1 MB
cpu em repouso : 1,48% de um nucleo   (20 amostras, janela visivel)
gpu em repouso : 0,00% em todos os motores
```

**A GPU e a boa noticia, e e a que o produto mais precisava.** Zero em 3D, Copy
e video ao longo de 20 s com a janela aberta e visivel: parado, o Morune nao
acorda a GPU. Era uma das duas linhas sem numero nenhum desde que a pagina
existe.

**As outras quatro regrediram, e a causa nao foi investigada.** CPU em repouso
saiu de 0,14% para 1,48%, o working set de 78,8 MB para 135,6 MB, o startup
interno de 19 ms para 62 ms e o ciclo completo de 509 ms para 1,01 s. Entre as
duas medicoes ha onze dias de trabalho — AccessKit, atualizador, mini-player,
fundo com desfoque, menu de bandeja proprio, avatar da conta, painel de midia —
e **nenhuma delas foi isolada**. Atribuir a regressao a qualquer uma agora seria
palpite; o que esta registrado e o numero e a data.

O caminho para investigar e o mesmo que ja funcionou uma vez aqui (a bandeja que
reescrevia o menu a cada 150 ms e custava 0,22%): medir de novo depois de
desligar um suspeito de cada vez. O primeiro suspeito e o temporizador de 150 ms
que hoje carrega bandeja, barra de tarefas e painel de midia no mesmo laco.

## Medicao de 19/08/2026


Maquina: AMD Ryzen 5 8500G, 16 GB RAM, Radeon 740M (grafico integrado),
Windows 11 Pro build 26200. Binario de release, perfil `opt-level = "z"`,
`lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `strip = "symbols"`.

```
executavel   : target\release\morune.exe
tamanho      : 13,78 MiB
instalador   : 5,31 MB

startup interno          : primeiro 18,0 | mediana 19,0 | min 17,0 | max 21,0 ms
startup processo inteiro : primeiro 722,4 | mediana 508,7 | min 387,3 | max 722,4 ms

working set    : media  78,8 MB | pico  79,5 MB
memoria privada: media  76,1 MB | pico  76,8 MB
cpu em repouso : 0,14% de um nucleo
```

### O que cada numero significa

**Startup interno (19 ms).** Do inicio de `main` ate o laco de eventos do Slint
executar sua primeira tarefa. Cobre log, caminhos, configuracao, carregamento e
validacao do tema, criacao da janela, aplicacao de todos os tokens e criacao do
icone de bandeja. E o custo do **nosso** codigo.

Foram 8 ms antes da bandeja existir e 16 ms antes da marca visual. Os ~8 ms da
bandeja sao a criacao do icone e do menu no Windows; os ~2 ms seguintes sao a
decodificacao do PNG de 128 px que vira o icone da janela. Todos pagos uma vez.
Ficam registrados aqui em vez de sumidos numa media: uma regressao pequena em
valor absoluto precisa ser visivel para nao virar habito.

**Instalador (5,31 MB).** LZMA sobre um binario de 13,28 MB, 60% de compressao.
Um unico `.exe`, sem runtime nem redistribuivel para instalar antes — verificado
executando o binario sozinho numa pasta vazia com `PATH` reduzido a
`%SystemRoot%\system32`. Carrega tambem `LICENSE` e os 622 KB de
`THIRD-PARTY-LICENSES.txt` (929 KB antes da compressao).

**Ciclo completo do processo (509 ms de mediana).** Medido de fora com `Start-Process
-Wait`: criacao do processo pelo Windows, carga do binario, inicializacao do
backend grafico, criacao do contexto OpenGL, primeiro quadro e encerramento
completo. E a cota honesta; a diferenca em relacao ao numero interno esta em
grande parte no contexto grafico e no teardown, nao na logica do Morune.

Os dois numeros aparecem porque so o primeiro seria autoelogio e so o segundo
esconderia onde esta o custo.

> Nota metodologica: medir com o operador `&` do PowerShell dava ~4 ms, porque
> ele nao espera processos do subsistema "windows". `tools/measure.ps1` usa
> `Start-Process -Wait` e le o tempo interno de um arquivo, ja que o binario de
> release nao tem stdout.

**Working set em repouso (78,8 MB).** Media de 12 amostras de 1 s, comecando 2 s
depois de abrir, com a janela visivel e ociosa. Inclui o driver OpenGL e o
atlas de fontes do renderizador FemtoVG.

Continua pequeno diante do teto pratico do produto. Um valor de 129 MB apareceu numa medicao
anterior com binario de depuracao — a diferenca entre os perfis e grande o
bastante para que medir em `debug` nao signifique nada.

## Decisoes que produziram esses numeros

**Sem Chromium.** Um app Electron equivalente comeca em 150–250 MB de RAM e
80–150 MB de instalador. Essa unica escolha explica a maior parte da margem.

**FemtoVG em vez de Skia.** O renderizador Skia do Slint tem qualidade de texto
melhor, mas pesa dezenas de MB no binario. FemtoVG e Rust puro sobre OpenGL. O
renderizador por software fica compilado como reserva automatica para maquinas
sem GPU utilizavel.

**Sem `std-widgets`.** A interface e construida sobre primitivas (`Rectangle`,
`Text`, `Path`, `TouchArea`, `Flickable`). Isso era necessario para que todo
pixel venha do tema, e como efeito colateral nao carrega a biblioteca de
widgets.

**Icones da interface como caminhos vetoriais.** Nitidez correta em qualquer
escala de DPI do Windows e nenhum bitmap para carregar. Vale para os icones de
navegacao, de player e para o simbolo da marca desenhado na barra lateral.

**Bitmaps so onde o Windows exige.** O sistema nao aceita vetor para icone de
executavel, de janela nem de bandeja. A marca custou **0,24 MB** no binario, e
o custo nao esta onde parece: 0,09 MB sao o `.ico` de quatro tamanhos como
recurso do `.exe` mais os pixels da bandeja, e 0,15 MB sao o icone da janela --
dos quais o PNG e so 4 KB. O resto e o decodificador de PNG do Slint, que o LTO
descartava enquanto nenhuma imagem era carregada em lugar nenhum.

Medido trocando uma coisa de cada vez: 9,15 MB sem marca, 9,24 MB com o `.exe`
e a bandeja, 9,39 MB com o icone da janela.

**Perfil de release agressivo.** `opt-level = "z"` com LTO gordo e uma unidade
de codegen custa ~3 min de compilacao e devolve um binario pequeno. Compilacao
de desenvolvimento usa `opt-level = 0` para o nosso codigo e `2` para as
dependencias, que e o que mantem o ciclo de edicao rapido sem deixar o app
lento em depuracao.

**Imagens escolhidas pelo tamanho de exibicao.** `ImageSet::best_for_width`
pega a menor imagem suficiente, nao a maior disponivel. Numa grade com dezenas
de capas visiveis, a diferenca e a maior fonte isolada de RAM da interface.

**Paginacao obrigatoria.** `Catalog::playlist_tracks` e paginada por contrato.
Carregar uma playlist de 10 mil faixas de uma vez seria o outro caminho facil
para estourar o orcamento.

**Bandeja so escreve quando muda.** A biblioteca de bandeja entrega eventos por
canal, entao ha uma leitura a cada 150 ms. A primeira versao reescrevia o texto
do menu em toda leitura, o que custava **0,22%** de um nucleo em repouso.
Comparar com o ultimo estado antes de escrever devolveu o numero a 0,00%. Vale
registrar o metodo: a regressao so apareceu porque a medicao foi refeita depois
da mudanca, nao porque alguem desconfiou.

## Medicao de 06/09/2026

Mesma maquina. Binario de release do dia, **com login restaurado** e nada
tocando -- ou seja, o cenario mais caro em repouso, e nao o mais barato: ha
sessao aberta com o Spotify.

```
cpu em repouso : 0,23% de um nucleo   (60 s, janela visivel, sessao ativa)
gpu em repouso : 0,00% em todos os motores
working set    : media 132,1 MB | pico 132,1 MB
memoria privada: media 287,5 MB
```

Duas coisas foram achadas, e so uma delas era do aplicativo.

**A saida de audio ficava aberta a toa.** Contando por thread, com o Morune
parado, `cpal_wasapi_out` custava sozinha **0,47% de um nucleo** -- mais da
metade dos 0,78% que o processo inteiro gastava. Um `rodio::OutputStream`
aberto acorda a cada bloco de audio para misturar silencio, e o dispositivo
ficava aberto do inicio ao fim do processo. Agora ele fecha cinco segundos
depois que o som para e reabre no proximo audio -- ver `crates/morune-spotify/src/sink.rs`.
Depois disso o processo inteiro caiu para 0,18%, medido thread a thread.

**A ferramenta de medicao exagerava a CPU em quase tres vezes.**
`tools/measure.ps1` dividia o tempo de CPU pelo *numero de amostras*, como se
cada volta do laco levasse um segundo. Desde 04/09 cada volta tambem le o
contador de GPU, que leva perto de outro segundo -- entao o denominador ficou
menor que o tempo real e toda leitura de CPU saiu inflada. Corrigido para usar
o relogio. **Isto nao explica a medicao de 30/08**, que e anterior a leitura de
GPU e foi feita com a versao sem o defeito.

**O que continua sem resposta:** os 1,48% de 30/08 nao foram reproduzidos, e
aquela rodada foi feita **sem login** -- ou seja, sem nem existir saida de
audio aberta. Entao a causa daquele numero nao e a que foi corrigida aqui. Como
a medicao de hoje esta abaixo do alvo mesmo no cenario mais caro, o caso fica
registrado em vez de perseguido. A RAM, essa sim, continua 68% acima de 19/08
sem causa isolada.

## Medicao de 13/09/2026 -- Aquario

**O Aquario reservava 5,3 GB de memoria privada.** Working set de 558 MB e
5.283 MB privados no release instalado, contra 153 MB e 299 MB do Bruma. Com o
renderizador por software o mesmo tema fica em 100 MB privados: o excesso e do
driver de video. O mapa de regioes mostrou ~160 blocos de 31,9 MB -- exatamente
o tamanho da foto do tema em RGBA (3840 x 2161 x 4). Cada `Vidro` da interface
desenha a copia borrada com `source-clip`, e o driver guarda uma copia da
textura por elemento; com um `Vidro` dentro de cada `Realce` (toda linha de
lista, item de navegacao e playlist), eram 160 copias.

Duas mudancas: a copia borrada passa a ter no maximo 1920 px de largura -- com
24 px de borrao, metade da resolucao e indistinguivel -- e o `Realce` do
material Aero deixa de carregar um `Vidro` (a capsula do Windows 7 ja cobre o
fundo). Resultado, na compilacao de depuracao: **5.300 MB -> 746 MB privados,
working set 254 MB**. O Aquario ainda custa uns 300 MB a mais que o Bruma, que e
o preco de duas texturas grandes de verdade.

**A aurora custa GPU enquanto a janela esta visivel.** A animacao das listras
do vidro Windows 7 redesenha a janela inteira a cada passo. A 20 passos por
segundo, 16% de GPU 3D; a 10, metade. Ela para com a janela na bandeja
(`aurora-viva`), com animacao reduzida e com o portao de tela cheia -- entao
nunca roda atras de um jogo. Numero do release em regime: ver abaixo.

## Riscos conhecidos

**Reabrir pelo atalho com o Morune na bandeja traz uma janela vazia.** Achado
em 06/09/2026 e ainda **nao corrigido**. A segunda instancia chama
`ShowWindow(SW_RESTORE)` direto no `HWND` (`instance.rs`), pelas costas do
Slint: o Windows mostra a janela, o Slint continua achando que ela esta
escondida e nada e desenhado. Aparece um retangulo cinza no tamanho antigo.
Reproduzido tambem em binario sem a devolucao de memoria, entao e anterior a
ela. O caminho pelo icone da bandeja nao foi testado.

**Interferencia com jogo nunca foi medida.** Continua sem dado, mas a falta
agora e so de sessao: o `-Watch` de `tools/measure.ps1` mede CPU e GPU por motor
de um Morune aberto, e a GPU em repouso ja saiu em 0,00%. CPU em repouso 0,14% e um bom sinal, mas nao
prova o que interessa: se o Morune acorda a GPU em segundo plano, quanto custa o
primeiro quadro depois de horas na bandeja, e se algo dele aparece no tempo de
quadro de um jogo em tela cheia. So faz sentido medir com reproducao real
tocando em uma sessao real.

**De onde vem a RAM, medido em 06/09/2026.** Com o renderizador por software
(`SLINT_BACKEND=winit-software`), o mesmo aplicativo com sessao ativa fica em
**65,4 MB** de working set e 35,7 MB de memoria privada. Com o renderizador de
GPU, que e o padrao, sobe para 153,7 MB e 299 MB. A diferenca -- perto de 90 MB
residentes e 260 MB privados -- e contexto e textura do driver de video, e nao
alocacao do Morune. Navegar por treze playlists seguidas manteve o working set
oscilando entre 148 e 175 MB, sem crescimento acumulado: nao ha vazamento.

Na bandeja, essa memoria toda ficava presa. Agora o aplicativo chama
`EmptyWorkingSet` dois segundos depois de esconder a janela e o residente cai
para cerca de 2 MB, voltando sozinho conforme a janela e redesenhada -- 25,6 MB
para a tela inteira de novo. E o cenario que o produto mais precisa acertar:
Morune na bandeja enquanto alguem joga.

**RAM durante navegacao com muitas capas nao foi medida.** O cache em disco ja
tem teto explicito de 48 MB e descarte dos arquivos mais antigos. Falta medir
o cache de imagens do renderizador durante uma sessao longa para confirmar que
o working set tambem estabiliza.

**~0,5 s de ciclo grafico.** Nao foi investigado ainda. Vale medir quanto
disso e criacao do contexto OpenGL no driver Radeon e quanto e o backend winit,
antes de tentar otimizar.

**Reproducao nao foi medida neste ciclo.** A meta de CPU e um criterio, nao um
resultado. So faz sentido medi-la com librespot real tocando.

## Como reproduzir

```bash
. .\tools\env.ps1
cargo build --release -p morune-app
.\tools\measure.ps1 -Runs 10 -IdleSeconds 12
```

A medicao **se recusa a rodar com outro Morune aberto**. Nao e frescura: com uma
instancia viva, cada execucao de startup so traz a janela dela para frente e
sai, e o relogio mediria isso. O numero sairia bonito e seria falso.

### O cenario que so uma sessao real produz

CPU e GPU com musica tocando e um jogo em tela cheia nao dao para montar
sinteticamente — exigem login, Premium e o jogo aberto. Para esse caso a
ferramenta observa em vez de encenar:

```bash
.\tools\measure.ps1 -Watch -IdleSeconds 60 -Rotulo "tocando + jogo"
```

Ela nao abre nem fecha nada: acha o `morune.exe` em execucao e amostra CPU, GPU
por motor (3D, Copy, VideoDecode...) e memoria pelo tempo pedido. O uso de GPU
por processo vem de `\GPU Engine(pid_...)`, somado por motor — a mesma conta da
coluna GPU do Gerenciador de Tarefas. Numa maquina sem esse contador o resultado
sai como "nao medido", e nunca como zero.

Registre o resultado aqui com a data, a maquina e **o cenario**. Numero sem
procedencia nao entra, e CPU sem dizer o que estava tocando nao e procedencia.
