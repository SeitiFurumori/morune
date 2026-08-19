# Changelog

Formato baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/).
Versionamento semantico.

## [Nao lancado]

### Adicionado
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
- [docs/assinatura.md](docs/assinatura.md): por que o SmartScreen avisa, o que
  assinatura resolve e o que nao resolve, e as quatro formas de assinar — duas
  delas gratuitas (SignPath Foundation e Microsoft Store), com requisitos,
  precos das pagas e fontes conferidas em 18/08/2026.
- Licenca definida: **MIT**, com o texto em [LICENSE](LICENSE). O `Cargo.toml`
  declarava `MIT OR Apache-2.0` sem nenhum arquivo de licenca no repositorio.

### Alterado
- **Criterio de desempenho redefinido.** A meta de "RAM em repouso < 70 MB" saiu:
  numa maquina com 16 GB, 70 ou 90 MB nao muda nada para ninguem. O criterio
  passa a ser nao atrapalhar quem esta jogando — CPU e GPU em segundo plano e
  interface que nunca trava. As tres metricas que passam a mandar ainda nao
  foram medidas, e isso esta dito em [PERFORMANCE.md](PERFORMANCE.md).
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
  detalhamento medido em [PERFORMANCE.md](PERFORMANCE.md).

### A fazer no proximo ciclo
- Backend de reproducao `morune-spotify` sobre librespot 0.8.
- Login OAuth PKCE com token no Gerenciador de Credenciais.
- Busca, biblioteca e capas ligadas as telas existentes.

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
