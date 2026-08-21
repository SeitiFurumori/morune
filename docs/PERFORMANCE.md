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

## Orcamento

| Metrica | Meta | Medido | Situacao |
|---|---|---|---|
| CPU em repouso, janela visivel | ~0% | **0,14%** | cumprido |
| CPU na bandeja, janela oculta | ~0% | **0,00%** | cumprido |
| GPU na bandeja | sem redesenho | — | nao medido |
| Interferencia com jogo em tela cheia | imperceptivel | — | nao medido |
| Resposta da interface | sem travar, nunca | — | exige sessao real prolongada |
| CPU em reproducao | < 2% | — | exige reproducao real |
| Startup ate o laco de eventos | < 1 s | **19 ms** | folgado |
| Ciclo completo do processo | < 1 s | **509 ms** | cumprido |
| RAM em repouso | teto, nao vitrine | **78,8 MB** | aceito |
| RAM em reproducao com capas | crescimento limitado | — | cache em disco limitado; RAM nao medida |
| Tamanho do instalador | < 40 MB | **5,31 MB** | folgado |
| Tamanho do executavel | — | **13,78 MiB** | — |
| Dependencia de Chromium | nenhuma | nenhuma | cumprido |

O executavel foi remedido em 20/08/2026 depois de habilitar AccessKit; cresceu
0,21 MiB. A biblioteca local de favoritos acrescentou 0,13 MiB e a gestao da
fila, 0,16 MiB. O tamanho do
instalador acima ainda e o ultimo pacote gerado e deve
ser atualizado na proxima rodada de empacotamento.

As linhas sem numero dependem de uma sessao Premium real, com musica tocando e
um jogo em tela cheia. O teste sintetico nao substitui esse cenario.

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

## Riscos conhecidos

**Interferencia com jogo nunca foi medida.** E a metrica mais importante da
pagina e a unica sem nenhum dado. CPU em repouso 0,14% e um bom sinal, mas nao
prova o que interessa: se o Morune acorda a GPU em segundo plano, quanto custa o
primeiro quadro depois de horas na bandeja, e se algo dele aparece no tempo de
quadro de um jogo em tela cheia. So faz sentido medir com reproducao real
tocando em uma sessao real.

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

Registre o resultado aqui com a data e a maquina. Numero sem procedencia nao
entra.
