# O que um ótimo app tem — pesquisa de 22/09/2026

Quatro pesquisas paralelas (movimento, visual, recursos, leveza), cruzadas com o
código de hoje. Cada item leva **situação no Morune** (tem / parcial / falta) e
**custo** para quem está jogando (nenhum / baixo / médio / alto).

**Filtro que manda em tudo:** nada entra se custar CPU, GPU ou disco enquanto um
jogo roda. Item de custo médio ou alto só entra se zerar no portão de tela
cheia (`tela_cheia.rs`), como a aurora já faz.

**Tema de referência: Bruma** — padrão de instalação nova desde 22/09
(`crates/morune-storage/src/config.rs:104`; antes era `midnight`).

---

## 1. Fluidez e resposta

| Item | Número de referência | Morune | Custo |
|---|---|---|---|
| Toda ação mostra reação em < 100 ms, mesmo antes da rede | RAIL 100 ms; Doherty 400 ms | parcial (cartão tem pressed; curtir/play esperam a rede?) | nenhum |
| UI otimista: curtir, pular, pôr na fila mudam na hora e desfazem se falhar | padrão Linear | parcial | nenhum |
| Capa carrega sobre a cor dominante, sem "pulo" | cor hex ≈ grátis; BlurHash não vale numa capa pequena | parcial (`tint.rs` existe; conferir se é usado como fundo antes da capa) | nenhum |
| Esqueleto no lugar de spinner em listas | espera percebida ~30% menor | falta | baixo (estático, sem pulsar em loop) |
| Pré-carregar no passar do mouse (playlist/álbum) | hover ~65 ms antes; 200–300 ms até o clique | falta | baixo (rede, não GPU) |
| Pré-carregar capa e dados da próxima faixa | — | conferir | baixo |
| Custo do "aquecimento" dos primeiros 30 min | — | **aberto** em `PERFORMANCE.md` | alto hoje |

## 2. Animação

Regra já adotada na auditoria de 21/09 e confirmada pela pesquisa: **a
frequência decide.** Ação feita cem vezes por dia não anima ou anima em < 150 ms.

| Item | Referência | Morune | Custo |
|---|---|---|---|
| Durações | Fluent 83/167/250 ms; M3 50–500 ms | tokens 120/200/320 — dentro da faixa | — |
| Curvas: entrar com *ease-out* (Fluent `0,0,0,1`), sair rápido | Fluent 2 | conferir uso | nenhum |
| Transição de página (sobe 6 px, 120 ms) | auditoria P1.1 | falta | baixo |
| Play/pause que se transforma, em vez de trocar o ícone | Spotify, Apple | falta | baixo |
| Coração com escala ao curtir | Spotify | falta | baixo |
| Animação interrompível (novo clique continua de onde parou) | Apple springs | Slint não tem mola nativa — pesquisar | baixo |
| Animar só posição e opacidade, nunca largura/altura | Smashing | sidebar anima largura (auditoria P2.7) | médio |
| Nada anima para sempre com o app parado | regra de ouro | aurora anima; zera em tela cheia e oculto | medido: 2,6% CPU visível |
| Reduzir movimento cobre também parallax e escala grande | MDN / vestibular | tokens zeram; conferir o resto | nenhum |
| Troca de tema com transição curta | better-ui | falta | baixo |

## 3. Visual e UI

| Item | Referência | Morune | Custo |
|---|---|---|---|
| Hierarquia: escala fixa de 6–7 tamanhos; razão 1.25 para destaque, 1.125 para lista | escala modular | tokens existem, `size-display` sem uso (critique) | nenhum |
| Números tabulares em durações e contagens | tabelas de dados | conferir fonte | nenhum |
| Faixa tocando marcada com cor + ícone de onda em **toda** lista, sidebar e fila | Feishin, Spotify | falta (critique P1) | nenhum |
| Vidro só em superfície pequena ou passageira; janela grande = Mica (tinta sem desfoque) | Microsoft Learn Mica/Acrylic | Bruma usa vidro na janela — **decisão sua** | alto se desfoque grande |
| Texto sobre vidro com piso de contraste; ícone inverte claro/escuro; opacidade sobe quando ilegível | Liquid Glass; Apple teve de criar "Tinted" | Bruma sem medição de contraste (critique) | baixo |
| Desfoque ≤ 20 px, nunca animar o raio, no máximo 3–4 camadas de vidro | GPUs integradas | conferir | alto se ignorado |
| Tinta tirada da capa, com piso de luminância | Apple Music 26 (reclamação no escuro) | `tint.rs` existe | baixo (1× por faixa) |
| Raio concêntrico (raio interno = externo − espaçamento) | Apple WWDC25 | já segue (auditoria) | nenhum |
| Estados completos: hover / pressionado / foco / desativado | NN/G | quase; tooltip sem atraso | nenhum |
| Tela vazia com motivo e próximo passo; erro sempre com "tentar de novo" | NN/G | conferir cada página | nenhum |
| Densidade escolhível na lista (compacta / confortável) | Feishin | token `density` sem uso | nenhum |
| Mini player que se ajusta continuamente ao tamanho | Apple Music, Spotify | tem mini player de tamanho fixo | nenhum |
| Polimento consistente vale mais que efeito vistoso | elogio ao Plexamp | — | — |

## 4. Recursos (pensando em quem joga)

**Essencial**

| Item | Morune | Custo |
|---|---|---|
| Atalhos globais configuráveis que funcionam com o jogo na frente — pedido antigo, nunca atendido pelo Spotify | **falta** (só teclas de mídia) | nenhum |
| Controles de mídia do Windows (SMTC) com capa — também alimenta o overlay do Win+G | escrito, falta conferir com música | baixo |
| Rádio / autoplay quando a fila acaba, **separado** da fila normal | código existe em `browse.rs`; conferir se está ligado | baixo |
| Reconectar sozinho quando a rede cai e retomar de onde parou | conferir | nenhum |
| Fila editável, "tocar a seguir", pular sem demora | parcial | nenhum |
| Tocar playlist direto do cartão e da sidebar | falta (critique P1) | nenhum |
| Volume com mudo, roda do mouse e atalho | falta (critique P2) | nenhum |

**Diferencial**

| Item | Morune | Custo |
|---|---|---|
| Modo jogo automático: detecta jogo → baixa prioridade do processo (EcoQoS), zera animação, adia downloads | parcial (movimento zera; prioridade não) | economiza |
| Aviso discreto de faixa nova sobre o jogo (toast ou overlay) | falta | baixo |
| Discord Rich Presence ("ouvindo X") | falta | baixo (conexão local) |
| Crossfade e normalização | normalização tem; crossfade falta | baixo |
| Menu de clique direito em faixas e cartões | tem menu próprio; conferir cobertura | nenhum |
| Voltar / avançar entre páginas (botões do mouse também) | só "Voltar" em detalhe | nenhum |
| Assinatura do instalador (SmartScreen) | falta (roadmap 4.8) | nenhum |

**Luxo / fora** — equalizador, Spotify Connect, cache offline: custo contínuo
maior que o retorno para este público. Letras e podcasts já estão fora por
decisão de 28/08.

## 5. Leveza e robustez

| Item | Morune | Ganho para quem joga |
|---|---|---|
| Thread de áudio no MMCSS (`AvSetMmThreadCharacteristics`): áudio sem falha sem roubar do jogo | **falta** | alto |
| Downloads e decodificação de capa em EcoQoS (baixa prioridade de energia) | **falta** | médio |
| Detectar tela cheia *sem borda* também — `SHQueryUserNotificationState` só pega a exclusiva | `tela_cheia.rs` compara janela da frente com o monitor, o que já cobre as duas | — |
| **Não** mexer na resolução do timer do Windows | conferir se alguma dependência mexe | evita prejudicar o jogo |
| Decodificar capa no tamanho exibido + cache com teto em MB | cache LRU existe; conferir o tamanho de decodificação | alto — candidato para os +68% de RAM |
| Renderizador: femtovg é o mais leve; Skia no Windows chega a 155 MB | usa femtovg | já certo |
| Teto no cache de áudio do librespot | tem limite (`auth.rs:148`) | já certo |
| Liberar a memória ao esconder, uma vez só, não em loop | faz (~2 MB na bandeja) | já certo |
| Medir o impacto no FPS do jogo com PresentMon (1% low) | ferramenta pronta, **falta a sessão** | alto — transforma a promessa em número |
| `SLINT_DEBUG_PERFORMANCE` para ver tempo de quadro | conferir | ferramenta grátis |
| Registro local de travamento (minidump), sem enviar nada | falta | médio |
| Teste de desempenho no CI que falha se piorar | falta | médio |
| Atualização com assinatura verificada (hoje é `.sha256`) | parcial | segurança |

---

## Decisões de 22/09/2026

1. **Bruma é o padrão** em instalação nova. Feito em `config.rs:104`; quem já
   tem outro tema escolhido não muda.
2. **Vidro do Bruma fica como está.** Sem ele o tema perde o aspecto. A regra
   da Microsoft vira só cuidado: não animar o raio do desfoque e não empilhar
   mais camadas de vidro.
3. **Cor da capa no Bruma: acento vivo, fundo quieto** (decisão de design,
   delegada). O Bruma é branco sobre vidro, e a foto do papel de parede é o
   fundo; pintar o fundo inteiro com a capa brigaria com as duas coisas e cria
   o problema de contraste que a Apple teve. Então:
   - a cor da capa aparece na **parte cheia da barra de progresso**, no
     **indicador de "tocando"** das listas e no **brilho atrás da capa** na
     barra do player — este ficou em 6%: com 12% uma capa clara já derruba
     o texto cinza do rodapé para 4,38:1 (o teste de contraste pega);
   - texto, ícones e o botão de play continuam brancos;
   - a cor passa por um piso de claridade para manter contraste ≥ 3:1 sobre o
     vidro — capa preta ou cinza cai de volta no branco do tema;
   - na troca de faixa a cor muda em `motion-slow` (320 ms), uma vez por
     faixa; zera com reduzir movimento e em tela cheia.
   Custo: uma conta por capa, que já existe (`tint.rs`).
4. **Todos os recursos da seção 4 entram.**
   Sobre adiar downloads no modo jogo: o **áudio nunca é adiado**. O que fica
   para depois são capas, listas e pré-carregamento de páginas que ninguém está
   olhando porque o jogo está na frente. A música continua baixando normal, e
   a parte que toca o som ganha prioridade (MMCSS) em vez de perder.

## Comparação com o Spotify (o que falta)

Critério: entra o que não pesa no jogo. "Sem caminho" = o protocolo interno
ainda não tem rota conhecida (ver `morune-spotify/src/catalog.rs`).

**Já tem:** busca, biblioteca, curtidas e curtir, páginas de playlist, álbum e
artista, fila completa (a seguir, reordenar, remover, limpar), aleatório,
repetir, autoplay, normalização, qualidade de áudio, saída de áudio, mini
player, bandeja, teclas de mídia, SMTC, iniciar com o Windows, atualização,
desfazer, fixar playlists, filtro e ordenação, menu de clique direito.

**Falta — entra (custo nenhum ou baixo):**

| Recurso | Observação |
|---|---|
| Criar, renomear e apagar playlist | o Spotify tem isso no centro; hoje o Morune só lê |
| Adicionar faixa a uma playlist / tirar dela | idem; clique direito → "Adicionar à playlist" |
| Salvar álbum e seguir artista | só a leitura existe |
| Histórico ("Tocadas recentemente") | sem caminho no Spotify → guardar localmente, arquivo pequeno gravado no máximo 1× por faixa |
| Buscas recentes e navegar por gênero na busca | buscas recentes é local; gênero depende de rota |
| Timer para dormir | nenhum custo |
| Crossfade | já estava na seção 4 |
| Painel "Tocando agora" (artista, álbum, próximas da fila) | usa dado que já está na memória |
| Copiar link da faixa / álbum / playlist | já existe código de área de transferência; conferir se está no menu |
| Pastas de playlist na barra lateral | conferir se já aparecem |
| Esconder faixa / não tocar este artista no rádio | local |
| Spotify Connect (o Morune aparece como aparelho no celular) | o librespot já faz isso; custo baixo, uma porta de rede ouvindo |

**Sem caminho hoje** (tentar de novo com sonda, sem prometer): mais ouvidas,
histórico do servidor, atividade de amigos.

**Fica fora:**
- Canvas (vídeo em loop atrás da capa), Jam, DJ e Blend — vídeo e sessão em
  grupo contínua custam rede e GPU o tempo todo.
- Download offline — escrita pesada em disco; o cache com teto já cobre o uso
  normal.
- Letras e podcasts — fora desde 28/08. Só voltam se você reabrir a decisão.

## Limites desta pesquisa

- **Reddit bloqueado**: a busca não trouxe resultados do Reddit, e o navegador
  embutido recusa `reddit.com` por segurança. As reclamações vieram do fórum
  oficial do Spotify. Com o seu Chrome dá para ler r/pcgaming e r/truespotify.
- **Material 3** só abre com JavaScript; os números vieram de fontes
  secundárias (batem entre si).
- A pesquisa de visual e a de leveza usaram só os resumos da busca, sem abrir
  cada página. Os números de custo do vidro vêm de blogs, não de medição nossa.
- Os itens "conferir" não foram verificados no código nesta passada.

## Fontes principais

- Latência: [RAIL](https://developer.mozilla.org/en-US/docs/Glossary/RAIL), [Doherty](https://lawsofux.com/doherty-threshold/)
- Movimento: [Fluent 2](https://fluent2.microsoft.design/motion), [Timing e easing no Windows](https://learn.microsoft.com/en-us/windows/apps/design/motion/timing-and-easing), [Emil Kowalski](https://emilkowal.ski/ui/great-animations), [Linear](https://performance.dev/how-is-linear-so-fast-a-technical-breakdown)
- Vidro: [Mica](https://learn.microsoft.com/en-us/windows/apps/design/style/mica), [Acrylic](https://learn.microsoft.com/en-us/windows/apps/design/style/acrylic), [WWDC25 design system](https://developer.apple.com/videos/play/wwdc2025/356/)
- Estados: [NN/G botões](https://www.nngroup.com/articles/button-states-communicate-interaction/), [NN/G tela vazia](https://www.nngroup.com/articles/empty-state-interface-design/)
- Spotify e jogo: [engasgo no jogo](https://community.spotify.com/t5/Desktop-Windows/Spotify-app-makes-my-game-stutter-freeze-till-I-tab-out/td-p/5470691), [atalhos globais](https://community.spotify.com/t5/Closed-Ideas/Desktop-Global-HotKeys-for-Pause-Play-Skip-etc/idi-p/57061)
- Windows: [EcoQoS](https://devblogs.microsoft.com/performance-diagnostics/introducing-ecoqos/), [MMCSS](https://learn.microsoft.com/en-us/windows/win32/procthread/multimedia-class-scheduler-service), [resolução do timer](https://randomascii.wordpress.com/2013/07/08/windows-timer-resolution-megawatts-wasted/), [PresentMon](https://github.com/GameTechDev/PresentMon)
- Slint: [renderizadores](https://docs.slint.dev/latest/docs/slint/guide/backends-and-renderers/backends_and_renderers/), [memória com Skia no Windows](https://github.com/slint-ui/slint/issues/13470)
