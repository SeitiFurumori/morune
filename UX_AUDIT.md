# Auditoria de UX — MORU•NE

Data: 20/08/2026  
Escopo: aplicativo Windows completo, com sessão Spotify Premium real  
Objetivo: reduzir tempo para compreensão e esforço operacional sem descaracterizar o produto

## Resumo executivo

O MORU•NE já tinha uma base visual acima da média para um produto em estágio inicial: identidade própria, hierarquia limpa, navegação estável, player persistente, temas realmente estruturais e mensagens de erro geralmente orientadas à recuperação. O principal problema não era estética. Era a diferença entre o que a interface prometia e o que um usuário conseguia operar.

Foram encontrados quatro riscos P1. Todos estão corrigidos:

- a busca dizia aceitar faixa, álbum e artista, mas consultava somente faixas;
- controles customizados eram essencialmente áreas de mouse, sem árvore acessível real no Windows;
- grades de cinco colunas estouravam na janela mínima;
- salvar/favoritar não concluía o fluxo central de construir uma biblioteca.

O coração agora corresponde ao modelo mental consolidado pelos players: ele
altera “Músicas curtidas” na conta Spotify. O MORU•NE aguarda confirmação remota
antes de preencher ou esvaziar o ícone, evitando falso sucesso e uma biblioteca
local concorrente que o usuário não pediu.

Também foram corrigidas fricções P2 em estados vazios, feedback, configurações, temas, player e navegação por teclado. A direção visual existente foi preservada.

A barra lateral também passou a refletir a intenção recente: “Músicas curtidas”
permanece fixada e as playlists abertas mais recentemente vêm em seguida. As
demais conservam a ordem da conta, evitando reorganização arbitrária.

## Método

Fluxos executados: primeira abertura, login, restauração de sessão, Início, busca autenticada, reprodução, pausa, anterior/próxima, volume, fila, biblioteca, detalhes de conteúdo, configurações, troca de tema, importação/exportação, retorno, bandeja, redimensionamento e inicialização em diferentes escalas.

A avaliação combinou modelo mental, affordance, descoberta, feedback, consistência, hierarquia, proximidade, reconhecimento, prevenção/recuperação de erro, leis de Fitts e Hick, carga cognitiva, arquitetura da informação, fluxo e as dez heurísticas de Nielsen.

Validações realizadas:

- conta Spotify Premium real; a escrita de curtidas requer reverificação final;
- busca “Daft Punk” acionada pela API de acessibilidade do Windows;
- 14 ações de resultado expostas ao UI Automation, com faixa e artista anunciados;
- janela mínima de 720×480 inspecionada visualmente;
- smoke test em escala 100%, 125%, 150% e 200%;
- contraste calculado para os três temas;
- testes automatizados e compilação do aplicativo.

Limites desta rodada: não houve teste humano com Narrador nem monitor secundário com escala mista. O mini-player foi renderizado e inspecionado, mas ainda precisa de uma sessão humana prolongada durante jogo/multitarefa.

## Mapa de severidade

| Severidade | Encontrados | Corrigidos | Pendentes |
|---|---:|---:|---:|
| P0 — bloqueia uso | 0 | 0 | 0 |
| P1 — crítico | 4 | 4 | 0 |
| P2 — fricção relevante | 10 | 9 | 1 |
| P3 — refinamento | 8 | 7 | 1 |
| P4 — cosmético | 2 | 1 | 1 |

## Principais problemas e decisões

### P1 — Busca quebrava a promessa da interface — corrigido

**Problema.** O texto dizia “faixa, álbum ou artista”, mas `SearchKind::Tracks` consultava somente faixas.  
**Impacto.** O usuário concluía que álbuns, artistas e playlists não existiam ou que a busca estava com defeito. Abrir artista e álbum ficava dependente de descoberta indireta.  
**Princípios.** Correspondência sistema/mundo real, consistência, prevenção de erro, confiança.  
**Solução.** Busca combinada por faixas, álbuns, artistas e playlists; resultados separados por tipo; estados distintos para inicial, carregando e nenhum resultado.  
**Justificativa.** A interface agora entrega exatamente o contrato que comunica e reduz caminhos indiretos.

### P1 — Aplicativo não era exposto à acessibilidade do Windows — corrigido

**Problema.** Os controles eram `TouchArea`; o recurso de acessibilidade do Slint também estava desativado no build. UI Automation encontrava a janela, mas nenhum descendente.  
**Impacto.** Narrador, automação assistiva e navegação por teclado não conseguiam operar o produto.  
**Princípios.** Acessibilidade, flexibilidade e eficiência, controle do usuário.  
**Solução.** AccessKit habilitado; botões, switches, sliders, navegação, cards e faixas receberam papel, nome, estado, foco visível e Enter/Espaço; sliders aceitam setas.  
**Justificativa.** A acessibilidade passou de intenção no código para uma árvore realmente consumida pelo Windows.

### P1 — Grade fixa quebrava em janelas estreitas — corrigido

**Problema.** Início e biblioteca sempre desenhavam cinco cards de 168 px, mesmo com apenas cerca de 448 px úteis na janela mínima.  
**Impacto.** Conteúdo cortado, ações fora da janela e sensação de produto não adaptativo.  
**Princípios.** Design responsivo, prevenção de erro, carga cognitiva, legibilidade.  
**Solução.** Número de colunas recalculado a cada redimensionamento, preservando tamanho legível dos cards.  
**Justificativa.** A estrutura se reorganiza; os cards não são comprimidos até perder legibilidade.

### P1 — Salvar e favoritar não existiam — corrigido

**Problema.** Não há ação de curtir, salvar álbum ou seguir artista, embora Biblioteca e padrões de players criem essa expectativa.  
**Impacto.** Um fluxo central de música termina sem conclusão e a biblioteca parece somente decorativa.  
**Princípios.** Modelo mental, controle e liberdade, completude do fluxo.  
**Solução.** Coração no player e nas linhas ligado às “Músicas curtidas” do
Spotify; estado acessível; remoção pelo mesmo controle; todos os IDs curtidos
carregados para representar corretamente itens antigos; confirmação remota antes
da mudança visual; erro preserva o estado anterior.

**Justificativa.** O coração produz o efeito que usuários de música esperam e a
mesma biblioteca aparece nos demais clientes Spotify. Não há duas coleções com
o mesmo símbolo e significados diferentes.

### P2 — Estados vazios não ofereciam próxima ação — corrigido

**Problema.** Início deslogado, Biblioteca vazia e Fila vazia explicavam o estado, mas deixavam o usuário parado.  
**Impacto.** Aumentava procura e cliques na navegação.  
**Princípios.** Onboarding contextual, reconhecimento, recuperação.  
**Solução.** CTAs contextuais: entrar, explorar músicas e buscar música.  
**Justificativa.** O estado vazio passa a ensinar pelo próximo passo real.

### P2 — Mensagem de status permanente e sem controle — corrigido

**Problema.** “Conectado como…” permanecia sobre o conteúdo indefinidamente.  
**Impacto.** Obstrução visual e sensação de estado transitório inacabado.  
**Princípios.** Visibilidade do sistema, controle e liberdade, minimalismo.  
**Solução.** A mensagem agora pode ser dispensada e tem ação acessível.  
**Justificativa.** Mantém feedback sem sequestrar espaço permanentemente.

### P2 — Tema ativo dependia de cor e ações competiam com a linha — corrigido

**Problema.** A seleção era comunicada só pelo fundo; a área clicável da linha podia interceptar Duplicar/Exportar.  
**Impacto.** Ambiguidade para baixa visão e cliques com resultado inesperado.  
**Princípios.** Consistência, feedback, prevenção de erro, não depender de cor.  
**Solução.** Selo textual “Ativo”, semântica de estado e camadas de clique independentes.  
**Justificativa.** Estado e ação ficam reconhecíveis e previsíveis.

### P2 — Configurações não cabiam com segurança na largura mínima — corrigido

**Problema.** Quatro ações de tema ocupavam uma única linha.  
**Impacto.** Corte ou compressão em janelas estreitas e texto escalado.  
**Princípios.** Adaptação, legibilidade, Fitts.  
**Solução.** Ações divididas em dois grupos, com explicação curta sobre aplicação imediata e retorno ao padrão.  
**Justificativa.** Reduz densidade sem esconder funções importantes.

### P2 — Estados de shuffle/repeat e controles do player eram ambíguos — corrigido parcialmente

**Problema.** Ícones pequenos e sem nome; estado ativo dependia principalmente da cor.  
**Impacto.** Usuários novos precisavam adivinhar fila, repetição e aleatório.  
**Princípios.** Affordance, feedback, Fitts, reconhecimento.  
**Solução.** Nome acessível contextual, fundo ativo, foco visível e alvos acionáveis por teclado.  
**Solução adicional.** Tooltips visuais usam os mesmos nomes contextuais da árvore acessível. Alvos continuam com área maior que o glifo; modos densos permanecem como escolha explícita do tema.
**Justificativa.** A operação assistiva está clara; a descoberta visual ainda pode melhorar.

### P2 — Fila não podia ser gerenciada — corrigido

**Problema.** A fila permite escolher uma faixa, mas não remover, reordenar, limpar, tocar a seguir ou adicionar ao fim.  
**Impacto.** O usuário enxerga estado sem ter controle sobre ele.  
**Princípios.** Controle e liberdade, modelo mental, recuperação de erro.  
**Solução.** Ações “Tocar a seguir” e “Adicionar ao fim” nas linhas; fila dividida entre inserções manuais e continuação da lista atual; mover para cima/baixo, remover e limpar somente onde há controle real. Todos os alvos têm nome e estado acessíveis.

**Justificativa.** A fila virou ferramenta de decisão sem sugerir que a ordem original de álbum ou playlist foi editada. Separar as camadas reduz erro e explica por que alguns itens são gerenciáveis.

### P2 — Importação de customização era irreversível — corrigido

**Problema.** Trocar tema é imediato e importar não mostra previamente o alcance da mudança.  
**Impacto.** Experimentação — diferencial do MORU•NE — parece arriscada para iniciantes.  
**Princípios.** Controle, prevenção e recuperação de erro, progressive disclosure.  
**Solução.** O pacote é validado integralmente numa área temporária e mostra nome, autor, versão, descrição e aviso de substituição antes da confirmação. Tema substituído recebe backup privado e a aplicação oferece Desfazer. Pacote recusado ou cancelado não altera o tema atual.
**Justificativa.** Aumenta liberdade percebida sem tornar a tela principal complexa.

### P2 — Mini-player e saída de áudio não eram visíveis — corrigido parcialmente

**Problema.** Não há mini-player nem seletor/indicação de saída.  
**Impacto.** Uso durante jogos e multitarefa — contexto central do produto leve — perde eficiência.  
**Princípios.** Flexibilidade, Fitts, visibilidade do sistema.  
**Solução.** Mini-player compacto preserva faixa, capa, favorito, play/pause, anterior/próxima, progresso e retorno, restaurando tamanho/maximização anteriores. Configurações explicita “Padrão do Windows”. A troca de dispositivo dentro do app continua pendente porque o backend ainda segue a seleção do sistema.
**Justificativa.** Entrega controle frequente com baixa ocupação de tela.

### P3/P4 — refinamentos

| Problema | Impacto | Estado / solução |
|---|---|---|
| Busca exige Enter | Um passo extra e pouco feedback durante digitação | Corrigido: debounce de 350 ms, com Enter preservado |
| Atalhos não são descobertos | Exige memorização externa | Corrigido: folha `Ctrl+/` e dicas em tooltips |
| Voltar tinha pouca equivalência de teclado | Navegação mais lenta | Corrigido: Escape e Alt+← no detalhe |
| Títulos truncados sem revelação | Informação pode ficar inacessível visualmente | Parcial: ações de faixa revelam título completo; falta tooltip direto no texto |
| Ações ativas só por cor | Baixa visão perde estado | Corrigido em tema, navegação, shuffle e repeat |
| Texto secundário pequeno | Pode cansar em densidade alta | Contraste aprovado; reavaliar tamanho em teste humano a 200% |
| Toast sem expiração automática | Pode continuar ocupando espaço | Parcial: agora dispensável; classificar mensagens antes de aplicar TTL |
| Espaçamento e iconografia | Pequenas diferenças entre áreas | Preservados: sistema atual é coerente e próprio |
| Barra de título nativa não acompanhava os temas | A primeira faixa visual parecia pertencer a outro produto | Corrigido: title bar temática sem repetir a marca da sidebar, controles acessíveis, arraste, duplo clique, resize e correção de DPI |
| Ausência de ajuda contextual | Usuário avançado descobre atalhos por tentativa | Corrigido: referência curta sob demanda, sem tour obrigatório |

## Heurísticas de Nielsen

1. **Visibilidade do estado:** player persistente e estados de reprodução são bons; busca ganhou loading/empty/result; toast ganhou dispensa.  
2. **Sistema e mundo real:** terminologia musical é natural; busca agora corresponde ao texto.  
3. **Controle e liberdade:** voltar, troca de tema, curtidas sincronizadas e fila manual estão cobertos; a ordem do contexto permanece protegida.
4. **Consistência e padrões:** tokens, iconografia e agora também o chrome da janela seguem a identidade do produto; os controles preservam nomes e ações esperados no Windows.
5. **Prevenção de erros:** camadas de clique de tema corrigidas; importação valida, confirma, faz backup e pode ser desfeita.
6. **Reconhecimento:** sidebar e player são reconhecíveis; ícones críticos têm nome acessível e tooltip visual.
7. **Flexibilidade e eficiência:** teclado, resize, mini-player, teclas de mídia e referência de atalhos estão cobertos.
8. **Estético e minimalista:** bom; não foram adicionadas animações ou superfícies desnecessárias.  
9. **Recuperação de erros:** limpar fila e importar tema têm Desfazer; falhas repetíveis de Início, Biblioteca e Busca oferecem retry explícito.
10. **Ajuda:** onboarding contextual e a folha de atalhos explicam no momento de necessidade.

## Comparação com padrões maduros

- **Spotify e Apple Music:** reforçam busca ampla, favorito próximo da faixa e fila editável. O MORU•NE atende os três, sincronizando o coração com o Spotify e distinguindo fila manual de contexto.
- **YouTube Music:** mantém player e fila como continuidade da navegação e oferece atalhos no desktop. O player persistente do MORU•NE está alinhado; falta referência de atalhos e mini-player.
- **Windows 11:** exige Tab lógico, foco visível, Enter/Espaço e nomes para controles customizados. A árvore AccessKit agora segue esse modelo.
- **Discord:** torna `Ctrl+,`, voltar e navegação por teclado padrões descobríveis. O MORU•NE implementa parte do vocabulário, mas ainda precisa uma folha de atalhos.
- **Steam:** é uma boa referência de densidade e uso paralelo a jogos. O MORU•NE preserva leveza e baixa ornamentação; o próximo ganho contextual é o mini-player.

Referências: [atalhos do Spotify](https://support.spotify.com/uk/article/keyboard-shortcuts/), [fila no Apple Music para Windows](https://support.apple.com/guide/music-windows/queue-up-your-songs-musb1e6d1c76/windows), [favoritos no Apple Music](https://support.apple.com/guide/music-windows/mark-items-as-favorites-musa407e11b8/windows), [interações de teclado no Windows](https://learn.microsoft.com/en-us/windows/apps/develop/input/keyboard-interactions), [navegação de foco no Windows](https://learn.microsoft.com/en-us/windows/apps/develop/input/focus-navigation), [atalhos e navegação do Discord](https://support.discord.com/hc/en-us/articles/31232432266647-Discord-Commands-Shortcuts-and-Navigation-Guide), [YouTube Music no desktop](https://support.google.com/youtubemusic/answer/9231765?co=GENIE.Platform%3DDesktop&hl=en).

## Antes e depois

| Antes | Depois |
|---|---|
| Busca apenas faixas apesar da promessa | Faixas, álbuns, artistas e playlists |
| UI Automation via Windows: 0 descendentes | Controles e resultados nomeados e focáveis |
| Cinco cards fixos em qualquer largura | Colunas recalculadas no resize |
| Estado vazio terminava em texto | Estado vazio oferece próxima ação |
| Tema ativo somente por cor | “Ativo” + estado semântico |
| Mensagem permanente sem controle | Botão acessível para dispensar |
| Switch só no pequeno alvo | Linha inteira acionável, sem duplicar foco |
| Player icon-only para tecnologia assistiva | Ações e estados anunciados por nome |
| Coração criava favoritos somente no Morune | Coração sincroniza com “Músicas curtidas” do Spotify |
| Fila era apenas uma lista de leitura | Inserções manuais reordenáveis, removíveis e separadas do contexto |
| Barra de título genérica e separada do tema | Chrome integrado ao MORU•NE, com controles de janela previsíveis e acessíveis |
| Importação substituía tema sem inspeção ou volta | Preview de metadados, confirmação, backup e Desfazer |
| Limpar fila era definitivo | Aviso com Desfazer restaura a ordem removida |
| Busca aguardava Enter | Busca automática com debounce e Enter preservado |
| Atalhos existiam apenas na memória/documentação | Folha `Ctrl+/` dentro do produto |
| Janela sempre ocupava a interface completa | Mini-player compacto com retorno ao estado anterior |
| Abrir novamente criava outro player | Instância única traz a janela existente para frente |
| Inicialização com o sistema só aparecia no instalador | Preferência reversível também em Configurações; abertura automática fica na bandeja |

## Pontos fortes preservados

- identidade visual própria e consistente;
- player persistente e fácil de localizar;
- navegação lateral simples, com Configurações corretamente secundária;
- temas alteram composição, não só paleta;
- redução de movimento já faz parte dos tokens;
- capas têm fallback estável, sem layout shift;
- contraste mínimo aprovado: Midnight 7,05:1, Pulse 6,32:1 e Paper 4,60:1 para texto secundário no pior fundo comum;
- mensagens de erro do backend geralmente incluem recuperação;
- janela e bandeja seguem o modelo mental de aplicativo de música para Windows.
- o preview da barra de tarefas oferece anterior, tocar/pausar e próxima sem
  obrigar o usuário a restaurar a janela, preservando contexto e reduzindo cliques.
- a identidade do produto aparece uma vez na sidebar; a barra de título não
  repete nome e símbolo imediatamente acima dela.

## Próximos passos priorizados

1. Permitir seleção de dispositivo no app quando o backend expuser enumeração/troca segura (P2).
2. Fazer teste humano com Narrador, teclado-only e 200% de escala em monitor secundário.
3. Testar rede lenta/offline, nomes extremos, CJK/RTL e listas extensas.
4. Integrar SMTC completo para metadados/capa no painel de mídia do Windows; as teclas de mídia já funcionam.
5. Assinar instalador e executável para remover o maior atrito externo restante: SmartScreen.

O critério de aceite permanece: um usuário novo deve chegar de abertura a busca e reprodução sem precisar aprender a arquitetura do aplicativo.
