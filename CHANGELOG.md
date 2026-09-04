# Changelog

Formato baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/).
Versionamento semantico.

## [Nao lancado]

### Corrigido

**O aplicativo podia passar semanas rodando um binario velho sem dizer nada**
- **Build local deixa de se anunciar como versao final.** Sem a variavel do
  workflow de publicacao, `build.rs` caia em `v0.1.0` -- e `0.1.0` e, por semver,
  **mais nova** que qualquer `0.1.0-alpha.N`. Um binario compilado na maquina se
  apresentava como a coisa mais nova que existe: a verificacao nunca achava
  lancamento algum e a tela respondia "você já está na versão mais recente" com
  releases novas publicadas. Agora a tag vira `v0.1.0-dev.<hash do commit>`, e a
  tela diz o que isso significa em vez de fingir que esta em dia.
- **A verificacao passa a acontecer sozinha, no maximo uma vez por dia.** So
  havia o botao, e o Morune inicia com o Windows e fica semanas aberto sem que
  ninguem abra as Configuracoes. A marca do "ja verifiquei" fica em disco, e nao
  em memoria, porque o caso que importa e o do processo que reinicia com a sessao
  todo dia. Verificacao que ninguem pediu **nao escreve erro na tela**: sem
  internet, quem so queria ouvir musica receberia "sem resposta do GitHub".
- **O aplicativo percebe quando o proprio executavel muda em disco.** O Windows
  deixa apagar e substituir um `.exe` em uso, e o processo segue vivo com o
  codigo que ja carregou -- foi exatamente o que aconteceu: o Morune rodava de
  uma pasta ja removida, com o dono testando um binario que nao existia mais.
  Agora avisa que a versao nova espera um fechar e abrir. Nao reinicia sozinho:
  derrubar um aplicativo que esta tocando e decisao de quem esta ouvindo.

**A busca ocupava um quinto da largura da janela**
- **A pagina de resultados volta a ocupar a largura toda.** O Flickable dos
  resultados era filho direto do layout da pagina, e um Flickable nessa posicao
  nao tem largura propria para oferecer: o layout de cima resolvia a pagina pela
  largura PREFERIDA do conteudo -- numa lista de faixas, o talo do texto
  elidido. O campo de busca encolhia junto e o resto da janela ficava vazio. Com
  o estado vazio nada disso aparecia, porque ali nao ha Flickable nenhum.
- As linhas das prateleiras de faixas apareciam espremidas contra a esquerda
  pelo mesmo motivo, um nivel abaixo: dentro de um Rectangle -- que nao e
  layout -- um filho sem largura fica com a largura preferida, e nao com a do
  pai.

**Encerrar um Morune travado podia matar outro programa**
- **A instancia travada passa a ser identificada pelo executavel, e nao so pelo
  titulo da janela.** O caminho de recuperacao achava a janela com
  `FindWindowW(None, "Morune")` e oferecia encerrar o dono dela: qualquer
  programa com uma janela de mesmo titulo virava candidato a `TerminateProcess`.
  Agora a varredura e por `EnumWindows` e o processo dono precisa ser um
  `morune.exe`; a checagem e refeita **com o handle ja aberto** antes de
  encerrar, porque entre achar e matar houve uma pergunta ao usuario e nesse
  intervalo um pid pode ser reciclado por outro programa.
- Continua sendo o nome do arquivo, e nao o caminho inteiro: um Morune instalado
  e um recem-compilado sao o mesmo aplicativo em pastas diferentes e disputam o
  mesmo mutex. Exigir caminho identico faria um nao reconhecer o outro e cair no
  aviso de "aberto sem janela" -- um dialogo, ou seja, um aplicativo travado
  esperando um clique.

**Abrir uma playlist eram quatro idas a rede, e duas eram a mesma**
- **O conteudo de uma playlist e lido uma vez.** `Catalog::playlist` (nome e
  tamanho) e `Catalog::playlist_tracks` (as faixas) chamavam os dois o mesmo
  `internal.playlist(id)`, e cada "carregar mais" pedia o protobuf inteiro de
  novo. Agora ha cache por id, com cinco minutos de validade e teto de oito
  playlists -- curto de proposito, para que uma faixa adicionada pelo cliente
  oficial apareca sem reabrir o Morune.
- **Os lotes de metadado saem em paralelo**, com teto de quatro. Eram
  sequenciais: uma pagina de 100 faixas custava duas idas a rede uma depois da
  outra, sendo requisicoes independentes. O teto existe pelo criterio do
  produto -- soltar vinte requisicoes de uma vez numa playlist de mil faixas
  trocaria latencia por um pico de rede e CPU no meio de uma partida.

**Login que travava sem saida**
- **O fluxo interativo de OAuth passa a ser do Morune**, sobre o crate `oauth2`.
  A renovacao silenciosa de token continua na librespot, que funciona bem; o
  que saiu foi o caminho interativo dela, por tres defeitos sentidos em uso
  real.
- **O endereco de autorizacao aparece na tela, com botao de copiar.** A
  librespot so o mandava para `println!`, e num build de release, que nao tem
  console, ele era perdido no instante em que era escrito. Quem tem mais de um
  navegador -- ou mais de um perfil no mesmo -- ficava preso ao que o sistema
  abrisse: sem o link, nao havia como concluir noutro lugar. O contrato de
  `begin_login` sempre prometeu devolver essa URL; agora ele cumpre.
- **A espera tem prazo e botao de cancelar.** Antes era um
  `TcpListener::incoming()` sem tempo limite: fechar o navegador sem concluir
  deixava a porta 5588 tomada ate o processo morrer, e toda tentativa seguinte
  respondia "ja ha um login em andamento". A unica saida era sair pela bandeja.
  Cancelar agora solta a porta, e ha teste que prova isso -- pede a
  autorizacao, desiste, pede de novo.
- O retorno do navegador passa a ter o `state` anti-CSRF conferido. O fluxo
  anterior aceitava qualquer codigo que chegasse na porta.
- **Sair da conta limpa as ofertas de recuperacao.** "Tentar novamente", o
  pedido em voo e o desfazer morriam junto com a sessao mas ficavam guardados;
  sem conta, repetir so falharia de novo.

**Ajustes de audio que existiam no arquivo e nao faziam nada**
- **Qualidade, nivelamento e cache de audio passam a valer.** `bitrate`,
  `normalize` e `audio_cache_mb` estavam no `config.toml`, com padrao e
  documentacao, e **nenhum era lido em lugar nenhum**: o motor usava
  `PlayerConfig::default()` e a sessao era aberta com cache `None`. Na pratica a
  qualidade ficava presa em 160 kbps, o nivelamento nunca acontecia e todo audio
  era baixado de novo a cada reproducao. Uma configuracao que promete e nao
  cumpre e pior que a ausencia dela.
- **Qualidade escolhivel na tela**, entre os tres degraus que o Spotify oferece
  (96, 160 e 320 kbps), com **nivelar o volume entre faixas** ao lado. As duas
  dizem que valem a partir da proxima abertura, porque e a verdade: a
  configuracao do reprodutor da librespot e congelada quando ele nasce, e refazer
  o motor pararia a musica no meio para atender a um ajuste.
- O cache de audio guarda **so audio**. A `Cache` da librespot tambem sabe
  gravar `credentials.json` em texto, e isso fica desligado de proposito: a
  credencial do Morune vive no Gerenciador de Credenciais do Windows.
- As preferencias aplicadas entram no log. Nada na tela nem no som diz em que
  qualidade a faixa chegou, e "mudei e nao senti diferenca" precisa ter resposta.

**Abrir o Morune quando ele ja esta aberto e travado**
- **Uma segunda abertura nao some mais em silencio.** Se a instancia que ja
  existe travou, trazer a janela dela para frente nao tem efeito -- e o processo
  novo encerrava sem janela, sem mensagem e sem uma linha no log. Visto de fora,
  era clicar no atalho e nada acontecer, quantas vezes fosse. Agora o
  travamento e detectado (`IsHungAppWindow`) e vira uma pergunta: encerrar o que
  esta preso e abrir de novo, ou nao. A escolha e de quem esta na frente da
  tela, porque so essa pessoa sabe se aquele processo travou de verdade ou esta
  apenas ocupado tocando.
- **Falha ao abrir o arquivo de log deixou de apagar a sessao inteira.** Ela
  caia calada para a saida padrao, que em release nao existe: a sessao rodava,
  fazia tudo e nao deixava registro nenhum. Ja aconteceu, e transformou um
  travamento numa investigacao sem evidencia. Agora a sessao vai para um arquivo
  proprio, com o numero do processo no nome, e a primeira linha dele diz por que
  o principal ficou para tras.

**O "Tentar novamente" que ficava pregado na tela**
- **O botao passa a pertencer a mensagem que o criou.** Era uma marca solta:
  uma falha a ligava, e qualquer coisa que escrevesse status depois -- um login
  concluido, uma reconexao -- trocava a frase sem desliga-lo. O resultado era
  "Conectado como fulano." com um "Tentar novamente" ao lado, oferecendo repetir
  algo que ninguem sabia mais o que era.
- **E o aviso volta a expirar.** Mensagem com acao nunca some sozinha, de
  proposito -- some-la tiraria do usuario a unica via para aquela acao. Com a
  marca presa, porem, isso pregava a mensagem na tela ate alguem fecha-la a mao,
  que e exatamente o defeito que o relogio de expiracao existe para consertar.
- **Repetir refaz o que falhou**, e nao o que estiver aberto na hora do clique.
  O alvo e registrado quando o pedido sai; antes ele era deduzido da tela, entao
  navegar entre a falha e o clique fazia o botao repetir outra coisa. Uma lista
  que falhou ao abrir tambem passa a ser repetivel -- antes o botao so respondia
  "Volte a abrir o item para tentar novamente".

### Alterado

**Acabamento**
- **Os botoes no preview da barra de tarefas seguem o tema e ficam centrados.**
  Eram desenhados com aritmetica inteira e limites escritos a mao: o triangulo
  de tocar ia de x=11 a x=33 num icone de 32 px, entao saia cortado a direita e
  deslocado, com a borda em escada. Agora as formas sao descritas em
  coordenadas reais, centradas na area util, e rasterizadas com 16 amostras por
  pixel. A cor deixa de ser um violeta fixo e passa a ser o **acento do tema** --
  e nao a cor de texto, porque aqueles botoes ficam sobre a miniatura que o
  *Windows* desenha, cujo fundo segue o tema do sistema. Trocar de tema
  redesenha os icones.
- **O menu da bandeja perde os cantos pretos e passa a ser arredondado pelo
  proprio Windows.** A janela do menu pedia fundo `transparent` contando que o
  cartao arredondado desenhasse sozinho, mas ela nao tem alfa por pixel: o que
  nao era pintado o compositor mostrava como **preto**, e sobravam quatro cantos
  pretos em volta do cartao. Agora a janela pinta o piso e o cartao nao tem raio
  proprio -- a unica curva e a que o `DWMWCP_ROUND` recorta, igual a qualquer
  outra janela solta do Windows 11. A borda morre nos cantos, cortada junto, e e
  a troca certa: uma linha que termina na curva incomoda menos que quatro
  meias-luas escuras.
- **A barra lateral recolhida nao corta mais texto**, e expandir e clicar na
  marca. Havia um chevron logo abaixo do logo, e o rotulo de ajuda dele nascia
  mais largo que os 64 px da barra: comecava em `x` negativo e era cortado pela
  borda esquerda, aparecendo como "ndir barra lateral". O chevron saiu, a marca
  virou o botao -- que e o gesto que se tenta primeiro -- e recolher continua
  onde estava.

**A conta deixa de parecer sobra**
- **Nome de exibicao e foto de verdade.** A barra lateral mostrava `seititm`,
  o identificador tecnico da sessao, porque o `/v1/me` do Web API responde 429
  para este aplicativo. O `user-profile-view` do protocolo interno entrega os
  dois, e a sonda de 19/08/2026 ja tinha confirmado o formato -- so faltava
  ligar. Agora aparece o nome escolhido na conta e a foto do perfil. Quem nao
  tem foto continua com o circulo da inicial, no mesmo tamanho, entao a chegada
  da imagem nao mexe o layout.
- A foto passa pelo mesmo cache de capas, com o mesmo descarte por LRU: e uma
  imagem pequena do mesmo servidor, e um segundo downloader so para ela seria
  duplicar cache, descarte e tratamento de falha. Falhar em buscar o perfil
  **nao** derruba o login -- sem ele, tudo volta ao que era.
- **Na barra lateral**, o identificador era um texto solto colado na borda
  esquerda, num recuo diferente de todo o resto da coluna e em cinza de texto
  secundario. Agora tem a geometria de um item de navegacao -- mesma altura,
  mesmo recuo, mesmo realce sob o cursor -- com um avatar que ocupa exatamente o
  lugar do icone, de modo que o nome cai na mesma coluna de "Inicio", "Buscar" e
  "Biblioteca". Clicar leva as Configuracoes.
- **Nas Configuracoes**, virou um cartao com avatar, o nome em texto primario e
  uma linha dizendo o que aquilo e. Era uma linha de texto secundario com um
  botao ao lado, com o mesmo peso visual de um ajuste qualquer.
- Quem nao tem foto no perfil recebe um circulo com a inicial do nome, sobre o
  acento do tema. Recolhida, a barra lateral mostra so o circulo.

### Adicionado

**Painel de reproducao do Windows (SMTC)**
- **O Morune aparece na sobreposicao de volume, na tela de bloqueio e no
  Ctrl+Alt+Del**, com capa, titulo, artista, album e os botoes de anterior,
  tocar/pausar e proxima. Era o item que faltava do "teclas de midia": as teclas
  do teclado ja funcionavam por `WM_APPCOMMAND`, mas isso so vale com a janela
  em primeiro plano, e o painel do sistema so existe para quem se registra no
  `SystemMediaTransportControls`. Mexer no volume durante um jogo mostrava um
  painel vazio -- ou o do navegador aberto atras.
- Com o painel ativo, as teclas de midia passam a chegar tambem com a janela
  escondida na bandeja.
- Tocar e pausar sao comandos separados, e nao "alternar": o painel do sistema
  tem os dois botoes, e alternar em cima de um "tocar" pausaria justamente o que
  a pessoa acabou de mandar tocar.
- Escreve so quando algo muda. `Update()` atravessa a fronteira do processo ate
  o servico de midia do Windows, e o laco da interface roda a cada 150 ms --
  reescrever a mesma faixa dez vezes por segundo e o mesmo gasto invisivel que
  ja custou 0,22% de um nucleo no menu da bandeja.

**Escolher a saida de audio**
- **A saida de audio deixa de ser um texto fixo dizendo "Padrao do Windows".**
  O campo `output_device` existia no `config.toml` desde o inicio e era o unico
  dos quatro ajustes de audio que nao fazia nada: `sink.rs` abria sempre
  `default_output_device()`. Agora as Configuracoes listam os dispositivos do
  sistema, com "Padrao do Windows" como primeira opcao, e a escolha vale a
  partir da proxima abertura -- como qualidade e nivelamento, e pelo mesmo
  motivo: o dispositivo e aberto quando o reprodutor nasce.
- Um dispositivo que sumiu (fone desconectado) **volta ao padrao com aviso no
  log**, e nao vira erro. Uma escolha que deixou de existir nao pode impedir a
  musica de tocar.
- A lista e relida ao entrar em Configuracoes: um fone conectado depois de abrir
  o aplicativo so apareceria na proxima sessao.

**Medir CPU e GPU de verdade**
- **`tools/measure.ps1 -Watch` mede um Morune que ja esta aberto**, sem abrir
  nem fechar nada, e reporta CPU, GPU por motor (3D, Copy, VideoDecode...) e
  memoria. E o que faltava para responder a metrica mais importante do
  [PERFORMANCE.md](docs/PERFORMANCE.md) e a unica que nunca teve numero: quanto
  o Morune custa com musica tocando e um jogo em tela cheia na frente. Esse
  cenario nao da para montar sinteticamente, entao a ferramenta observa em vez
  de encenar.
- Uso de GPU por processo vem de `\GPU Engine(pid_...)`, somado por motor --
  que e a conta que o Gerenciador de Tarefas mostra. Sem o contador na maquina,
  o resultado e "nao medido", nunca zero: zero inventado viraria numero no
  PERFORMANCE.md.
- **A medicao se recusa a rodar com outro Morune aberto.** Instancia unica faz a
  segunda copia so trazer a janela da primeira para frente e sair -- o relogio
  media isso e chamava de startup. Numero falso que passava por bom.

**Atualizacao pelo proprio aplicativo**
- **Botao "Procurar atualizacoes"** nas configuracoes. Ele consulta os
  lancamentos publicados no GitHub, baixa o instalador da versao seguinte,
  confere o `.sha256` publicado ao lado dele e instala em silencio -- ninguem
  precisa mais voltar ao navegador para atualizar. So verifica quando alguem
  clica: nenhum relogio de fundo e nenhuma requisicao no startup, porque o
  criterio de desempenho do projeto e nao atrapalhar quem esta jogando.
- **Baixar e instalar sao dois cliques**, de proposito. Instalar fecha o
  aplicativo, e fechar o aplicativo interrompe a musica -- isso nao pode
  acontecer sem que a pessoa tenha pedido. O download termina num botao
  "Instalar e reiniciar", nao numa reinicializacao.
- **O hash e conferido antes de qualquer coisa ser executada.** Um arquivo que
  nao bate e apagado na hora, e nada e aberto. Isso protege contra download
  corrompido; **nao** e prova de origem, que continua dependendo da assinatura
  de codigo descrita em `docs/SIGNING.md`.
- **Pre-lancamento so alcanca quem ja esta num.** Quem instalou um alpha recebe
  o alpha seguinte; quem instalou uma versao final nunca e empurrado para
  dentro de um alpha.
- O instalador ganhou a opcao `/RESTART`, que faz o modo silencioso esperar o
  aplicativo sair e reabri-lo no fim. Sem ela a atualizacao terminaria com a
  janela simplesmente sumida. O `/S` sozinho continua sendo instalacao
  silenciosa comum.
- A tag do lancamento passa a ser gravada no executavel. Antes o binario so
  conhecia a versao do `Cargo.toml` (`0.1.0`), que por semver e mais nova que
  qualquer `0.1.0-alpha.N` -- a verificacao concluiria que nao ha nada a fazer
  com tres lancamentos novos no ar.

**Barra lateral**
- **Todas as playlists da conta na barra lateral**, e nao so as suas e as
  editoriais. Daily Mix, Descobertas da Semana, mixes de artista e "Suas
  musicas mais ouvidas" viviam so nas prateleiras do Inicio; quem procurava por
  elas na lateral -- o gesto normal de quem vem do Spotify -- nao as encontrava.
  As prateleiras do Inicio continuam iguais.
- **Fixar playlist no topo**, com o botao direito ou a tecla Menu. A escolha
  fica no `config.toml` (`navigation.pinned_playlists`) e sobrevive a reabrir o
  aplicativo. O menu e desenhado pelo proprio aplicativo, com as cores do tema:
  o menu de contexto nativo do Windows nao aceita estilo.
- Icone `pin` entra na lista de icones que um tema pode substituir.

**Vidro, fundo e a cor do que esta tocando**
- **O vidro passa a valer para a interface inteira.** `Gloss` e `GlassEdge`
  existiam so na barra de reproducao e no aviso de status; agora toda superficie
  do layout usa o componente `Glass` -- barra lateral, cartoes, campos, menus,
  dialogos, linhas de faixa e itens de navegacao sob o cursor.
- **O realce fecha o contorno inteiro, seguindo o raio.** Antes eram duas
  linhas retas recuadas, que terminavam antes da curva e deixavam um corte reto
  no canto. Agora e a borda de um retangulo com o raio do hospedeiro, de uma
  intensidade so -- a direcao da luz fica por conta do `Gloss`, que e
  preenchimento e aceita gradiente de verdade. Sem a quina de baixo o painel
  parecia recorte em papel.
- **O Bruma perdeu o azul.** Vidro nao tem cor propria -- `accent`,
  `accent_hover` e `focus_ring` viraram branco fosco. Com o azul, o botao de
  tocar era o unico objeto da tela que nao parecia da mesma materia.
- **Imagem de fundo**, por tema e por usuario. Decodificada, reduzida e
  desfocada uma unica vez no carregamento -- um fundo que recalculasse desfoque
  a cada repaint disputaria GPU com o jogo pelo resto da sessao.
- **Transparencia e acrilico da janela passam a funcionar.** Eram validados,
  documentados e nunca chegavam a janela. Trocar para um tema opaco agora
  desliga a transparencia: antes ela contaminava todos os temas seguintes ate
  fechar o aplicativo.
- **A barra de reproducao pega a cor da capa que esta tocando.** Escolhida por
  quantidade vezes saturacao, com quase-preto e quase-branco descartados -- um
  encarte de fundo preto devolve a cor do disco, e nao o preto.
- **`gloss` e `border_highlight`**, as duas metades do vidro: a aresta desenha o
  contorno, o gloss desenha a luz caindo sobre ele. Preenchimento estatico, sem
  custo por quadro. Ambos nascem desligados.
- Dois temas de vidro acompanham o aplicativo: **Cristal** e **Bruma**.

**Customizacao**
- **Icones substituiveis por tema**, via `.svg` em `assets/icons/`. O
  `.musicpack` ja aceitava e validava a pasta; ninguem a lia.
- **Medidas dos controles viram token** (`[control]`). O tamanho de botao, de
  icone e do traco deixam de ser valor literal dentro da interface.
- **Fontes empacotadas** (`bundled_font`). Sem isso um tema compartilhado chegava
  errado na casa de quem baixou.
- **Previa de cada tema na lista**, desenhada a partir das cores dele.
- **Recarga do tema ao salvar o arquivo**, opcional.
- **Aba de Aparencia** nas Configuracoes: imagem de fundo, encaixe, opacidade,
  escurecimento, desfoque, transparencia da janela, tamanho da interface e
  reducao de animacoes. `font_scale_override` e `reduce_motion` ja existiam no
  arquivo de configuracao e nao tinham controle nenhum.
- **Botao de parar** na barra de reproducao, opcional e desligado por padrao.
- **Barra de volume no menu da bandeja.** Vale com ou sem faixa tocando: e a
  razao mais comum de abrir a bandeja com o jogo em primeiro plano.

### Corrigido

- **A grade abria sempre com uma coluna a mais do que cabia, e o ultimo cartao
  sangrava pela borda direita.** O numero de colunas so era calculado em
  `changed width` e em `changed sidebar-collapsed`: enquanto ninguem
  redimensionasse a janela valia o default fixo de cinco, e na largura padrao
  cabem quatro. Passou a ser calculado tambem no `init`. O calculo continua em
  handler, e nao como binding declarativo: a largura da janela depende do que o
  conteudo pede, entao a versao declarativa fecha um ciclo em `layoutinfo-h`
  que o Slint aceita mas avisa que pode virar panic em execucao.

- **Inicio, Biblioteca e Fila terminavam cortadas na base, sem nada dizendo que
  havia mais abaixo.** As tres eram `Flickable` direto, e um degrade declarado
  dentro de um Flickable rola junto com o conteudo e some. Viraram `Rectangle`
  com o `Flickable` dentro e o `ScrollFade` por cima -- com a altura preferida
  cortada na raiz, senao a pagina cresce alem da area util e o conteudo fica
  cortado sem nunca chegar a rolar.

- **A interface falava portugues sem acento nenhum.** "Configuracoes",
  "Musicas curtidas", "Voce", "Aparencia" -- 198 textos escritos como se o
  aplicativo nao soubesse escrever a propria lingua, ao lado de nomes de
  playlist do Spotify que chegam acentuados e sempre renderizaram bem. Nao era
  limitacao de fonte nem de renderizador: foi conferido trocando tres palavras
  e capturando a tela. Agora a interface escreve como se escreve.

- **"1 faixas".** As mensagens montavam "{count} faixas" na mao, e a lista de
  faixas carregadas fazia o mesmo. Ha uma funcao para isso agora, e a UI trata
  o singular.

- **Jargao e ingles onde nao cabiam.** "Backend do Spotify indisponivel nesta
  maquina" nao diz o que fazer e usa uma palavra que nao e do usuario: virou
  "Nao foi possivel iniciar o Spotify nesta maquina. Feche e abra o Morune para
  tentar de novo". "Autoplay ligado" virou "Radio ligado", que e o nome que o
  proprio Morune ja usa na mesma tela. E "Fila manual", vocabulario interno,
  saiu das mensagens: para quem usa, a fila e uma so.

- **Erro tecnico despejado na barra de status.** O caso geral de falha no login
  fazia `format!("Nao foi possivel entrar: {other}")`, e o `{other}` e o Debug
  do erro -- endereco, codigo de socket, o que viesse. A frase tecnica vai para
  o log; a barra diz o que a pessoa pode fazer.

- **A ultima playlist da barra lateral aparecia partida ao meio.** A lista
  recebia o que sobrasse de altura, sem relacao com a altura da linha. Agora a
  area util e arredondada para baixo, para o maior multiplo do passo da linha
  que caiba, e o resto fica de fundo da barra.

- **`border` e `scrollbar` reprovavam contraste 3:1 nos quatro temas de fabrica
  e no tema embutido** -- de 1,32:1 a 1,87:1, medido com o alfa resolvido e no
  pior caso de area de trabalho. A validacao de tema rodava a cada boot e nao
  pegava porque so olhava texto; a WCAG 1.4.11 pede os mesmos 3:1 para limite
  grafico e componente de interface. Os novos alfas sao o menor valor que
  alcanca 3:1 em cada tema, e um teste novo reprova qualquer tema de fabrica
  que volte a sair reprovando. **No Bruma e no Cristal o contorno fica
  visivelmente mais marcado** -- e o preco de a borda existir para quem enxerga
  pouco, num tema em que ela e o unico limite entre uma regiao e a seguinte.

- **`tools/snapshot.ps1` morria antes de capturar o primeiro tema** quando o
  cargo escrevia qualquer aviso: com `$ErrorActionPreference = "Stop"`, uma
  linha em stderr vira erro terminante mesmo com o build passando. Quem decide
  se o build passou e o codigo de saida.

- **`tools/screenshot.ps1` podia salvar a janela de outro programa como se
  fosse a captura boa.** A verificacao de primeiro plano usava `-eq` entre dois
  `[IntPtr]`, que no PowerShell 5.1 compara objetos e nao enderecos, e passava
  sempre. E o criterio de captura valida era "algum pixel nao e preto", que
  aceita a janela cinza chapada que o DWM devolve quando ela esta coberta.

- **`tools/screenshot.ps1` nao capturava tema nenhum quando o Morune ja estava
  aberto**, e dizia "o aplicativo saiu antes da captura", que acusa um defeito
  do aplicativo. O que acontecia era a instancia unica: a segunda abertura
  devolve o foco a primeira e sai com codigo 0. Agora o script detecta a
  instancia e explica, com `-Force` para encerra-la.

- **`tools/screenshot.ps1` devolvia janela preta nos temas com acrilico.** Nao
  era corrida de tempo: o vidro e composto pelo DWM a partir do que esta atras
  da janela, e `PrintWindow` pede o desenho so a janela, onde ele nao existe.
  Bruma e Cristal caiam nisso sempre. Ha agora uma leitura da tela como
  alternativa -- que so acontece depois de confirmar que a janela do Morune e
  mesmo a de primeiro plano, para nao repetir o defeito antigo de salvar a
  janela de outro programa como se fosse a captura boa.

- **O menu da bandeja saia branco, com o texto claro sumido dentro dele.** Os
  temas de vidro descrevem as superficies com alfa baixo -- na Bruma,
  `surface_raised` e branco a 18% -- contando com o fundo da janela por tras.
  O cartao do menu nao tinha piso nenhum: agora pinta o fundo do tema sem alfa
  e recebe a superficie de vidro por cima, como qualquer painel. O `Gloss` saiu
  do cartao e ficou so a aresta: o degrade morre em 62% da altura de quem o
  hospeda, o que num painel da janela e uma faixa estreita acima do texto, mas
  num cartao de 240 px cobre justamente a linha do nome da faixa.

- **O pause lia como um retangulo unico.** As barras tinham vao de 2 unidades --
  1,5 px a 18 px, que some no antialiasing. O botao principal do player parecia
  um stop deformado. Todo o conjunto de glifos foi para a mesma grade optica;
  `shuffle` saia do viewbox e era cortado nas pontas, `library` era ilegivel.
- **O fundo da janela era pintado duas vezes**, e o segundo veu cobria a imagem
  de fundo justamente na area de conteudo. Com alfa 0,7 o resultado desenhado
  virava 0,91, e nenhum valor de tema alcancava vidro fino.
- **O verificador de contraste ignorava alfa** e lia `#ffffff0d` como branco
  puro: um tema de vidro legivel acusava 1,00:1. Agora compoe a superficie sobre
  o fundo e mede nos dois piores casos de area de trabalho.
- **A pagina de Configuracoes nao rolava.** O bloco fixo passou a pedir mais
  altura que a janela tem e engolia a lista de temas, sem como chegar nela.
- Icones cheios reservavam espessura de traco invisivel, encolhendo o glifo em
  8%.

**Menu da bandeja**
- **O menu do icone da bandeja passa a ser o Morune.** Era um `HMENU` do
  Windows: o sistema o desenhava, e nao havia onde encaixar cor, tipografia ou
  forma -- o unico pedaco do aplicativo que nao parecia com ele. Agora e uma
  janela propria, com a capa da faixa, titulo e artista, e a mesma linha de
  transporte do player, botao de tocar em cor de destaque incluso. Fecha com
  Esc, com um clique fora ou com um segundo clique no icone.
- A janela do menu e criada ao abrir e descartada ao fechar. Um segundo
  contexto grafico vivo em repouso pesaria na maquina de quem esta jogando, que
  e justamente o que o aplicativo se propoe a nao fazer.

**UX, recuperacao e desktop**
- Importacao de tema com validacao em area temporaria, preview de metadados,
  confirmacao de substituicao, backup e acao Desfazer.
- Limpeza da fila reversivel pelo aviso “Desfazer” e retry explicito para falhas
  repetiveis de Inicio, Biblioteca e Busca.
- Busca automatica com debounce de 350 ms, folha de atalhos `Ctrl+/` e tooltips
  visuais nos controles por icone.
- Mini-player compacto que preserva controles essenciais e restaura tamanho e
  maximizacao anteriores.
- Instancia unica: abrir o Morune novamente restaura e foca a janela existente.
- Persistencia de tamanho/maximizacao, suporte a teclas multimidia e explicacao
  unica ao fechar para a bandeja.
- Configuracoes mostra honestamente que a saida acompanha o dispositivo padrao
  do Windows; selecao interna permanece para quando o backend a suportar.
- Inicializacao opcional com o Windows sincronizada entre instalador e
  Configuracoes. A entrada e por usuario, nao exige UAC, pode ser removida pelo
  mesmo switch e usa a bandeja quando a abertura foi automatica.
- Barra lateral prioriza as playlists abertas recentemente, persiste o historico
  localmente e preserva a ordem original para playlists ainda sem uso.
- O coracao agora adiciona e remove faixas das Musicas curtidas do Spotify, com
  estado completo da conta, confirmacao remota e falha sem falso sucesso. A
  antiga colecao local deixa de aparecer como uma biblioteca concorrente.

**Chrome da janela**
- Barra de título própria, integrada aos temas e à estrutura visual do MORU•NE, com alvos de
  minimizar, maximizar/restaurar e fechar compatíveis com teclado e tecnologia
  assistiva.
- Marca e nome ficam concentrados na sidebar em vez de se repetirem também na
  barra de título; o título nativo continua disponível ao Windows e à tecnologia
  assistiva.
- Arraste da janela, maximização por duplo clique, bordas redimensionáveis e
  conversão de coordenadas por fator de escala para 125%, 150% e 200% de DPI.
- Fechar pela nova barra continua passando pela preferência de fechar para a
  bandeja; a mudança visual não altera o ciclo de vida do player.
- O preview do Morune na barra de tarefas agora oferece controles nativos de
  faixa anterior, tocar/pausar e próxima faixa. O estado do botão central muda
  junto com o player e os três ficam desabilitados quando nada está carregado.

**Radio, detalhes e acabamento do ciclo 2**
- Autoplay configuravel, ligado por padrao: ao fim da fila, a ultima faixa vira
  semente de radio e as recomendacoes sao anexadas sem apagar o historico ou
  repetir faixas ja presentes.
- Parser tolerante para a resposta JSON de `get_radio_for_track`, com falha
  isolada da busca e da navegacao.
- Capas pequenas nas linhas de faixa, reutilizando o cache LRU de 48 MB.
- Colecoes extensas carregam progressivamente, sem os antigos tetos visiveis de
  50/200 faixas. A lista virtualizada mantem custo estavel, filtro e ordenacao
  completam os lotes em segundo plano, e a fila recebe a continuacao sem
  interromper a musica atual.
- Barra lateral usa o rootlist completo em vez de ocultar playlists depois da
  200a.
- Tela de artista com faixas populares por pais e discografia vindas do
  protobuf tipado; abrir um album navega para seu detalhe.
- Playlists removidas da Biblioteca, pois a fonte canonica delas agora e a
  barra lateral.

**Backend de Spotify (`morune-spotify`)**
- Mutacoes autenticadas `addToLibrary` e `removeFromLibrary` pelo Pathfinder v2,
  com fallback para o hash anterior quando o web player gira a consulta
  persistida. Nenhum novo escopo OAuth e pedido.
- Crate nova, implementando os contratos de `morune-core` sobre a librespot 0.8.
  A interface continua guardando `Arc<dyn PlaybackEngine>`, `Arc<dyn Catalog>` e
  `Arc<dyn Authenticator>`: nenhuma tela sabe que o provedor e o Spotify.
- **Login OAuth com PKCE**, sem client secret e sem campo de senha. O usuario
  entra no site do Spotify, no navegador dele, e o Morune so ve o codigo que
  volta. O refresh token vai para o Gerenciador de Credenciais do Windows.
- **Fonte unica de token** (`token.rs`): login e catalogo usam o mesmo token e a
  renovacao acontece num lugar so, sob trava. Um token recusado dentro do prazo
  -- acontece quando a conta revoga o acesso pelo site -- e renovado e a
  requisicao repetida, sem a tela pedir login de novo.
- **Reproducao**: carregar, tocar, pausar, buscar posicao e volume, com o fim de
  faixa ligado a `Queue::next(false)` -- e o que faz "repetir uma" repetir em
  vez de pular. A posicao e interpolada por relogio local entre os avisos da
  librespot, e nao consultada a cada quadro.
- **Volume com curva cubica**, guardado em inteiro atomico porque o misturador
  le esse valor na thread de audio, a cada bloco.
- **Busca** pelo `pathfinder` e **biblioteca** pelo protocolo interno da sessao
  (`spclient`/mercury), depois que o Web API passou a devolver 403 ate para os
  endpoints basicos. Playlists, curtidas, artistas seguidos e capas continuam
  disponiveis sem manter uma segunda pilha de TLS no binario.
- **Mais ouvidos e historico recente** permanecem `Unsupported`: as sondas dos
  caminhos internos candidatos nao encontraram um equivalente estavel. A tela
  degrada sem erro e usa as fontes de biblioteca que foram verificadas.
- `Library` no core ganhou `top_tracks`, `top_artists`, `recently_played` e
  `made_for_you`, todos com implementacao padrao que recusa com `Unsupported`.
  Um provedor de arquivos locais nao tem "mais ouvidos", e obrigar todo backend
  futuro a escrever um `unimplemented` seria pior que um padrao honesto.
- Traducao do GraphQL do `pathfinder` isolada em `graphql.rs`, testavel sem
  rede: item nulo, faixa sem id e arquivo local somem da lista em vez de
  derrubarem a pagina inteira, e as capas saem ordenadas da menor para a maior.

**Contorno da mudanca de 2024 no Web API**
- Em 27/11/2024 o Spotify fechou, para aplicativos novos, `/v1/recommendations`,
  artistas parecidos, vitrine editorial, caracteristicas de faixa e o acesso as
  playlists que ele monta para a conta. Ate hoje nao ha substituto oficial.
- `internal.rs` fala o **protocolo interno** para o que caiu: `get_rootlist` e
  `get_playlist` da librespot, o mesmo caminho que ja entrega o audio. As duas
  respostas sao protobuf tipado pela propria librespot, com nome, dono e tamanho
  decorados junto -- nao ha JSON adivinhado nem endpoint inventado.
- Abrir uma playlist tenta o Web API e cai para o caminho interno em 404 ou 403,
  que e como Descobertas da Semana e Radar de Novidades se apresentam. O
  metadado das faixas volta pelo Web API em lote de 50, porque o protocolo
  interno entrega URIs e uma requisicao por faixa custaria cem numa playlist de
  cem.
- O campo `format` do rootlist separa o que o usuario criou do que o Spotify
  montou; as editoriais, que nao trazem `format`, aparecem pelo dono `spotify`.

**Qualquer pessoa consegue entrar**
- **Conta nao-Premium deixou de matar o aplicativo.** `check_catalogue`, em
  `librespot-core`, chama `exit(1)` ao ver uma conta que nao e Premium: sem
  erro, sem mensagem, a janela sumia. Como a maioria das contas do Spotify e
  gratuita e qualquer pessoa pode clicar em "Entrar", essa era a primeira
  experiencia possivel com o Morune. O login agora pergunta ao `/v1/me` antes de
  entregar a credencial a librespot, e recusa com uma frase que explica o
  motivo.
- `CoreError::AccountPlan`, para "o login funcionou mas o plano nao permite" --
  que nao e o mesmo que credencial recusada, porque entrar de novo nao resolve.
- **Porta de retorno ocupada deixou de ser reportada como falha de rede.** Quem
  tivesse a `5588` em uso era mandado "verificar a internet", que e o lugar
  errado para procurar.
- O perfil passou a vir do `/v1/me`: nome de exibicao escolhido pela pessoa em
  vez do identificador tecnico, e avatar, escolhido no menor tamanho que sirva
  para a barra lateral.
- README ganhou **Como entrar na sua conta**: nao ha cadastro, nao ha servidor
  do Morune no meio, e nao e preciso registrar aplicativo nenhum no Spotify para
  usar nem para compilar.

**Corrigido**
- **Recolher a barra lateral quebrava o visual.** O botao encolhia a caixa para
  a largura de icone, mas os rotulos, o nome "Morune" e o botao de entrar
  seguiam desenhados: eram controlados por `sidebar-labels`, que e escolha do
  tema e nao muda quando o usuario recolhe. O resultado era texto de 210 px
  espremido em 60 px. `Layout` ganhou `sidebar-labels-shown`, que so e verdadeiro
  quando o tema permite **e** a barra nao esta recolhida.
- Recolhida, a marca e o botao de expandir agora empilham, porque nao cabem lado
  a lado; e o botao aparece mesmo em tema com `collapsible = false`, senao nao
  haveria como voltar.
- Recolhida, os icones passam a ser desenhados mesmo em tema com
  `show_icons = false`: sem rotulo e sem icone, a navegacao virava tres linhas
  clicaveis e vazias.

**Interface**
- **Inicio deixou de ser uma grade e virou cinco prateleiras**: Feito para voce,
  Tocadas recentemente, Musicas curtidas, Seus mais ouvidos e Suas playlists.
  Cada uma e independente -- a que falhar chega vazia e as outras aparecem
  igual, porque uma tela inicial que some inteira por causa de uma fonte seria
  pior que uma tela inicial menor.
- Buscar e Biblioteca deixaram de ser vazias: Biblioteca lista playlists, albuns
  salvos e artistas seguidos; a busca devolve faixas.
- Ativar um card carrega o album, a playlist ou as faixas populares do artista
  na fila e comeca a tocar. Ativar uma faixa da busca faz a **lista inteira**
  virar contexto, para que "proxima" continue pelos resultados.
- Cada pedido de catalogo vira tarefa no runtime do backend e e recolhido no
  temporizador que ja atende bandeja e reproducao: a thread da interface nao
  espera rede em nenhum caminho.
- Qualquer lista de faixas visivel na tela vira contexto da fila ao ser clicada,
  entao "proxima" continua pela lista em vez de parar na primeira faixa.
- Sair da conta limpa busca, inicio e biblioteca da tela, junto com a fila.

**Identidade visual, licenciamento e distribuicao**
- Identidade visual aplicada: simbolo da marca na barra lateral (desenhado como
  caminho vetorial, presente tambem com a barra recolhida), icone do executavel,
  da janela, da barra de tarefas, da bandeja e do instalador.
- `tools/make-icon.ps1` gera `assets/brand/morune.ico` a partir dos PNGs do
  sistema de marca, com 16, 32, 128 e 256 px.
- `tools/licenses.ps1` reune os avisos de copyright das 339 dependencias que
  entram no binario em `THIRD-PARTY-LICENSES.txt`, instalado junto do
  aplicativo. MIT, Apache-2.0, BSD e ISC exigem isso; ate agora o instalador
  estava em desacordo com elas. Textos identicos sao agrupados, o que derruba o
  arquivo de 2,8 MB para 622 KB.
- O mesmo script **falha o build** se aparecer dependencia copyleft fora de uma
  lista curta de revisadas. Foi assim que o licenciamento do Slint apareceu.
- Metadados de versao no executavel e no instalador: nome, versao, descricao e
  copyright aparecem nas propriedades do arquivo, no Gerenciador de Tarefas e no
  aviso do SmartScreen, que ate agora mostrava o `.exe` anonimo.
- `tools/sign.ps1` e as duas chamadas no `build-installer.ps1` que assinam
  executavel e instalador. Sem certificado configurado, avisa e nao quebra o
  build; com `MORUNE_SIGN_THUMBPRINT` definido, passa a assinar sem mais nenhuma
  mudanca no pipeline.
- `.sha256` publicado ao lado do instalador, e impresso no fim do build. E o
  unico jeito de conferir o download enquanto nao ha assinatura.
- [docs/SIGNING.md](docs/SIGNING.md): por que o SmartScreen avisa, o que
  assinatura resolve e o que nao resolve, e as quatro formas de assinar — duas
  delas gratuitas (SignPath Foundation e Microsoft Store), com requisitos,
  precos das pagas e fontes conferidas em 18/08/2026.
- Licenca definida: **MIT**, com o texto em [LICENSE](LICENSE). O `Cargo.toml`
  declarava `MIT OR Apache-2.0` sem nenhum arquivo de licenca no repositorio.

### Verificacao
Login, reconexao, reproducao, busca, playlists, curtidas, artistas seguidos,
capas e telas de detalhe foram verificados contra uma conta Premium real em
19/08/2026. Radio/autoplay, capas nas linhas e a tela completa de artista foram
implementados depois dessa rodada e aguardam a reverificacao final descrita em
[handoff histórico](docs/archive/PROJECT_HANDOFF_2026-08.md).

### Adicionado (desempenho)
- **A proxima faixa da fila e adiantada.** Comecar uma faixa custa buscar a
  chave de audio, abrir o fluxo na CDN e montar o decodificador: mediana de
  588 ms, p90 de 996 ms e maximo de 1,7 s, medidos em 41 trocas de uso real. A
  librespot tem `Player::preload` e o caminho de carga a consome direto — inicia
  a reproducao sem ir a rede — mas o Morune nunca chamava. Agora, assim que uma
  faixa comeca a tocar, a seguinte e adiantada. Vale para o botao de proxima e
  para a emenda no fim da musica. O disparo e no inicio da reproducao, e nao no
  carregamento, para duas faixas nao disputarem banda no momento em que a que o
  usuario espera precisa dela.

### Alterado (Inicio)
- **Musicas curtidas viraram um bloco com rolagem propria.** Eram seis faixas
  fixas, o suficiente para dizer "voce curtiu isto" e insuficiente para achar
  algo. Passam a 50, num bloco de oito linhas que rola por dentro. Lista curta
  nao ganha area vazia: o bloco encolhe para caber nela.
- **Suas playlists aparecem no Inicio.** As que voce criou e segue existiam so
  na barra lateral. Ganharam prateleira propria, antes das recomendadas: o que
  o usuario montou vem antes do que o Spotify sugeriu.
- **As prateleiras do Inicio rolam de lado** em vez de quebrar em varias linhas.
  Uma prateleira com trinta itens empurrava tudo que vinha depois para fora da
  tela. O gesto nao briga com o da pagina porque os eixos sao diferentes.

### Corrigido
- **Tema Paper renderizava quebrado.** Tres defeitos somados: `row_height = 38`
  nao cabia as duas linhas da faixa (titulo 15px e artista 13px, entrelinha 1,5,
  somam 42px), entao o texto transbordava e as linhas se encostavam; a trilha da
  barra de progresso usa a cor `border`, e a 9% de preto com 2px ela sumia no
  fundo claro; e `show_times` era ignorado em silencio quando
  `progress_edge_to_edge` estava ligado, deixando o tema sem indicacao alguma de
  posicao. Os dois primeiros sao valores do tema; o terceiro era falha da
  interface, que agora mostra o tempo abaixo dos controles nesse modo.

### Alterado
- **Cantos da janela arredondados.** A janela e `no-frame`, entao o Windows nao
  desenhava borda nem canto e ela ficava um retangulo de canto vivo. Passa a
  pedir ao gerenciador de janelas o raio padrao do sistema, o mesmo das demais
  janelas do Windows 11 — ele recorta a regiao, entao conteudo e sombra
  acompanham a curva. Sem efeito no Windows 10, que nao conhece o atributo. O
  pedido e refeito pelo temporizador de estado da janela, e nao uma vez so: logo
  depois de `show()` o HWND as vezes ainda nao existe -- o mesmo motivo que faz
  a barra de tarefas nascer num temporizador --, e esconder e reabrir a janela
  pode recria-la com o canto padrao.
- **Tocar uma curtida do Inicio usa a colecao inteira como fila.** A prateleira
  guarda seis faixas, e era essa lista que virava o contexto: a fila nascia com
  cinco musicas pela frente. Agora clicar ali abre "Musicas curtidas" por tras,
  sem tirar o usuario do Inicio; a primeira pagina ja comeca a tocar e as
  seguintes entram na fila em lotes.
- **O aviso do canto some sozinho depois de 6 segundos.** "Conectado como
  fulano" e as demais confirmacoes ficavam na tela ate alguem clicar no X, e
  viravam parte do cenario. Aviso que carrega "Desfazer" ou "Tentar novamente"
  continua ate ser respondido: ele nao e so aviso, e o unico lugar de onde essa
  acao pode ser feita.
- **Saida de audio propria, no lugar da `RodioSink` da librespot.** O atraso que
  sobrava nos controles nao era da interface: era o buffer de audio. A
  `RodioSink` mantem cerca de meio segundo decodificado e fazia todo comando
  esperar por ele — `stop()` chamava `sleep_until_end()` antes de pausar, o
  volume era aplicado antes da fila (o audio enfileirado continuava no volume
  antigo) e `seek` nem tocava na fila. O buffer continua do mesmo tamanho, que e
  a folga contra falha de audio com um jogo rodando; o que mudou e que os
  comandos deixaram de esperar por ele. Volume passa a ser aplicado no
  misturador do rodio, que o reaplica ao audio ja enfileirado a cada 5 ms;
  pausar silencia no ato **sem** descartar a fila, entao voltar continua de onde
  o som parou; e trocar de faixa, parar e mover a posicao descartam a fila antes
  do proximo audio sair. Ver `crates/morune-spotify/src/sink.rs`.
- **Controles de reproducao deixaram de esperar.** Volume, tocar/pausar, faixa
  anterior e proxima, e clicar ou arrastar a barra de progresso respondem no
  gesto. Quatro causas somadas explicavam o atraso: cada clique reconstruia a
  interface inteira -- inicio, busca, biblioteca, fila -- e, no meio disso, relia
  a pasta de temas do disco; o slider do volume repetia esse trabalho a cada
  movimento do mouse; tocar/pausar e seek so apareciam depois da ida e volta ate
  a librespot; e a barra de progresso nao andava sozinha, o que fazia um seek
  parecer sem efeito. Agora os controles espelham so a barra, a lista de temas e
  memoizada, a intencao do clique aparece na hora e um relogio de 250 ms mantem
  progresso e tempo decorrido em movimento enquanto ha musica.
- **Capas deixaram de custar um `stat` por espelhamento.**
  `slint::Image::load_from_path` chama `std::fs::metadata` em toda invocacao,
  mesmo com a imagem ja decodificada — e parte da chave do cache do Slint. Sao
  229 us medidos nesta maquina, pagos uma vez por linha de lista e uma vez por
  movimento do mouse na barra de volume. As capas ja convertidas passam a ser
  guardadas pelo caminho, que identifica o conteudo porque o nome do arquivo e o
  hash dele.
- **Arrastar o volume avisa o Rust no maximo uma vez por quadro.** O desenho
  continua acompanhando o ponteiro sem limite; o que passou a ser limitado e o
  aviso que atravessa para o motor. Um mouse de 1000 Hz gerava mil avisos por
  segundo, cada um reescrevendo a barra inteira.
- **"Anterior" reinicia a faixa** passados tres segundos, ou quando nao ha
  historico para onde voltar. Antes o botao nao fazia nada na primeira faixa da
  fila.
- **Criterio de desempenho redefinido.** A meta de "RAM em repouso < 70 MB" saiu:
  numa maquina com 16 GB, 70 ou 90 MB nao muda nada para ninguem. O criterio
  passa a ser nao atrapalhar quem esta jogando — CPU e GPU em segundo plano e
  interface que nunca trava. As tres metricas que passam a mandar ainda nao
  foram medidas, e isso esta dito em [Performance](docs/PERFORMANCE.md).
- **Licenca do Slint escolhida explicitamente.** O Slint e
  `GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR LicenseRef-Slint-Software-3.0`
  e o Morune nao escolhia nenhuma das tres, o que deixava a licenca do
  aplicativo indefinida. Passa a usar a royalty-free, que permite o Morune
  seguir MIT, com atribuicao pelo badge na pagina de download. Ver
  [ADR-0005](docs/adr/0005-licenca-e-slint-royalty-free.md).
- `authors` no `Cargo.toml` passou de "Morune contributors" para o nome do
  autor, coerente com o aviso de copyright do `LICENSE`.
- O icone de bandeja deixou de ser desenhado em codigo e tingido com a cor de
  destaque: agora e o simbolo da marca, fixo. Cor de tema continua valendo para
  tudo dentro da janela.
- Binario de 9,15 MB para 9,39 MB e instalador de 3,83 MB para 4,00 MB, com o
  detalhamento medido em [Performance](docs/PERFORMANCE.md).

### A fazer antes da v1
- Reverificar radio/autoplay e artista contra a conta Premium.
- Medir CPU e GPU em segundo plano com musica tocando e um jogo em tela cheia.
- Instalar o pacote final numa conta limpa do Windows.

## [0.1.0] — 2026-08-18

Primeiro MVP executavel. Prova a arquitetura e o motor de customizacao; ainda
nao toca musica.

### Adicionado

**Core (`morune-core`)**
- Modelo de dominio com ids que carregam o provedor, de modo que `spotify:abc` e
  `local:abc` nunca colidam.
- Fila com shuffle preservando a faixa atual, tres modos de repeticao, historico
  real de reproducao e fila do usuario com prioridade ("tocar a seguir").
- Contrato `PlaybackEngine` dyn-compativel, baseado em comandos e eventos, para
  que a interface nunca espere rede ou disco.
- `NullEngine`, motor sempre valido usado antes do login e em teste.
- Contratos `Catalog`, `Library`, `Authenticator` e `CredentialStore`.
- `AccessToken` sem `Debug` derivado: o segredo nao pode vazar por log.

**Customizacao (`morune-theme`)**
- Esquema de tema versionado em TOML, com valor padrao para todo campo.
- Carregamento que nunca falha: tema corrompido vira o tema embutido mais uma
  lista de diagnosticos.
- `sanitize` que corrige valores impossiveis (`NaN`, fora de faixa, janela quase
  transparente) e devolve avisos, em vez de recusar o tema.
- Aviso de contraste com razao WCAG calculada, sem impedir o tema.
- Heranca entre temas por `based_on`, com limite de profundidade.
- Pacotes `.musicpack` com importacao e exportacao.
- Defesa de importacao: travessia de caminho, prefixo de unidade, fluxo NTFS,
  byte nulo, nomes hostis do Windows, lista de permissao de extensoes, limites
  de tamanho e razao de compressao, extracao atomica.
- Observador de sistema de arquivos para recarga a quente (feature
  `hot-reload`), com agrupamento de eventos e filtro de temporarios de editor.

**Persistencia (`morune-storage`)**
- Configuracao com gravacao atomica e recuperacao de arquivo corrompido para
  `.bak`.
- Caminhos seguindo a convencao do Windows, com modo portatil como reserva.
- Cofre de credenciais sobre o Gerenciador de Credenciais do Windows, verificado
  com ida e volta real ao cofre do sistema.

**Interface (`morune-app`)**
- Janela nativa com barra lateral, paginas Inicio, Buscar, Biblioteca, Fila e
  Configuracoes, e barra de reproducao completa.
- Toda cor, tamanho, raio e duracao vem do tema; nao ha valor visual literal na
  interface.
- Icones como caminhos vetoriais, nitidos em qualquer escala de DPI.
- Troca de tema em execucao, sem recompilar e sem recriar a janela.
- Importar, exportar, duplicar, restaurar e abrir pasta de temas.
- Painel de diagnosticos do tema ativo.
- Temas `paper` e `pulse` embutidos, gravados na primeira execucao e nunca
  sobrescritos depois.

**Segundo plano e instalacao**
- Fechar a janela esconde o Morune na bandeja e mantem o processo vivo, como no
  Discord. Ligado por padrao, desligavel em Configuracoes → Comportamento.
- Icone de bandeja desenhado em codigo, tingido com a cor de destaque do tema
  ativo, com menu de abrir, tocar/pausar, anterior, proxima e sair. Clique duplo
  restaura a janela.
- Sair pela bandeja sempre disponivel; se a bandeja falhar ao ser criada, fechar
  a janela volta a encerrar o aplicativo, para que o processo nunca fique vivo
  sem forma visivel de encerra-lo.
- Instalador `.exe` unico de 3,83 MB, com pagina de escolha de disco e pasta,
  sem UAC (instalacao por usuario), atalhos opcionais, entrada em Aplicativos e
  Recursos e desinstalador que preserva configuracoes e temas por padrao.
- `tools/build-installer.ps1` recusa empacotar se o binario nao rodar isolado
  com `PATH` reduzido ao Windows.

**Qualidade**
- 133 testes cobrindo fila, cores, validacao de tema, seguranca de pacote,
  configuracao e cofre de credenciais, incluindo importacao de pacotes
  maliciosos reais (travessia de caminho, executavel embutido, bomba de
  compressao) montados nos testes de ponta a ponta.
- `clippy -D warnings` limpo em todo o workspace.
- `tools/measure.ps1`: tamanho, startup e memoria medidos de verdade.
- `tools/snapshot.ps1`: cada tema renderizado em PNG pelo proprio renderizador
  do Slint, sem capturar a tela do usuario.
- `tools/verify-tray.ps1`: fecha a janela via `WM_CLOSE` e confere que o
  processo sobreviveu, a janela sumiu e a CPU continua baixa.

### Decisoes registradas
- [ADR-0001](docs/adr/0001-stack.md) — Rust, Slint e nao Electron.
- [ADR-0002](docs/adr/0002-toolchain-windows-gnu.md) — toolchain GNU no Windows.
- [ADR-0003](docs/adr/0003-contrato-de-reproducao.md) — comandos e eventos em
  vez de `async` no trait.
- [ADR-0004](docs/adr/0004-temas-declarativos.md) — temas sao dados, nao codigo.

### Conhecido e nao resolvido
- Reproducao nao implementada; o aplicativo usa `NullEngine`. Fechar para a
  bandeja ja funciona, mas o que continua em segundo plano hoje e o aplicativo,
  nao a musica.
- Minimizar para a bandeja nao existe: o Slint nao expoe o evento de
  minimizacao. So fechar tem esse comportamento.
- O instalador nao e assinado; o SmartScreen avisa na primeira execucao.
- Startup interno subiu de 8 ms para 16 ms com a criacao do icone de bandeja.
- RAM em repouso (70,3 MB) esta na meta sem folga, antes de existir cache de
  capas.
- ~1 s de inicializacao grafica ainda nao investigado.
- `bundled_font` existe no esquema mas fontes empacotadas nao sao registradas.
- Recarga a quente pronta na crate, ainda nao ligada a interface.
