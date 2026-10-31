# Referências visuais e espaço do Morune — 31/10/2026

Estudo para a conversa 3 da [entrevista de identidade](ENTREVISTA-IDENTIDADE-2026-10-31.md).
[P] = fonte primária lida; [S] = fonte secundária ou julgamento sem fonte aberta.

## O que é "da moda" e combina com sóbrio e polido

A hipótese se confirmou em parte. Nenhuma fonte fala em "sóbrio" como tendência.
O que as fontes mostram é **material, profundidade e movimento com física**, e
isso combina com uma interface polida e minimalista.

- **Apple Liquid Glass (2025):** vidro translúcido que tira a cor do conteúdo em volta e reage ao movimento. Os controles ficam numa camada própria, por cima do conteúdo. Ou seja: vidro nos controles, não no conteúdo [P: [Apple Newsroom](https://www.apple.com/newsroom/2025/06/apple-introduces-a-delightful-and-elegant-new-software-design/)].
- **Material 3 Expressive (Google):** cor, forma, tamanho, movimento com molas. Teve até 87% de preferência entre 18 e 24 anos, **mas** uma playlist que largou o formato de lista conhecido ficou pior de usar, e parte das pessoas preferiu um design mais calmo [P: [design.google](https://design.google/library/expressive-material-design-google-research)].
- **Windows 11 (Mica):** o material lê o papel de parede uma vez só, pensando no desempenho, e vira cor sólida quando a transparência está desligada, a bateria está em economia ou a janela está inativa. A recomendação é uma camada de base e outra de conteúdo [P: [Microsoft Learn](https://learn.microsoft.com/en-us/windows/apps/design/style/mica)].
- **Apple Music (iOS 26):** mais profundidade na tela "tocando agora", com a capa animada. Uma resenha achou que a estética ficou acima da utilidade [S].
- **Nothing:** a marca inteira sai de uma ideia só, a matriz de pontos, aplicada em tudo [S].

**Leitura para o Morune:** "da moda" quer dizer material com regra (vidro só
nos controles), movimento com física e cor tirada da capa. Enfeite seria pôr
brilho e reflexo em tudo ou desfocar a tela inteira.

## Referências

- **Plexamp:** o caráter vem do fundo desfocado tirado da capa, de mais de uma dúzia de visualizadores e de 4 temas, numa janela só [P/S]. Dá para aprender que um fundo vivo tirado da capa vira assinatura. O Morune já tem o brilho da capa.
- **Apple Music:** a capa manda e o resto some [S].
- **Cider:** modo imersivo com a capa em tela cheia e loja de plugins, mas a identidade própria é fraca [P: [cider.sh](https://cider.sh)].
- **Nothing:** uma regra de forma coerente vale mais que sete paletas [S].
- **Teenage Engineering:** a identidade nasce de restrições, com poucos controles e regras rígidas [S].
- **Wallpaper Engine:** imagem, vídeo e site como fundo. **Pausa sozinho com jogo ou app maximizado e descarrega da memória** [P: [wallpaperengine.io](https://www.wallpaperengine.io/en)]. É o modelo exato do modo "liberdade total" e da promessa de não pesar.
- **Rainmeter:** a comunidade faz tudo. Sem curadoria, a personalização total vira bagunça [P: [rainmeter.net](https://rainmeter.net)]. Reforça a ideia de dois níveis.

## Mapa

| | Padronizado | Personalizável |
|---|---|---|
| **Vivo** | Apple Music, Plexamp | Wallpaper Engine, Rainmeter (não são players) |
| **Parado** | Spotify, Tidal | Cider, Winamp/foobar |

**Espaço vazio:** um player vivo e personalizável com acabamento de produto.
Quem personaliza (Wallpaper Engine, Rainmeter) não é player, e os players
polidos têm poucos temas, que mudam sobretudo a cor. O Morune cabe aí: poucos
temas fechados, cada um com forma e movimento próprios, mais uma porta aberta
tipo Wallpaper Engine. As posições no mapa são julgamento, não medição.

## Princípios propostos

**Resposta do Felipe (31/10):** ficam **1, 2 e 3**. O 4 (vidro só nos
controles) não foi marcado e fica fora da lista por enquanto. O 5 (pular) "não é
princípio": continua valendo como requisito de uso (conversa 2), mas não como
princípio de direção. O quanto de movimento entra nos temas prontos ficou para
depois de ele ver os efeitos numa demonstração.

1. **A capa manda, a interface cede.** O brilho do player usa a cor da capa; texto e botões ficam em tom neutro.
2. **Cada tema, uma forma.** Trocar de tema muda os cantos, os botões e o material, não só a paleta.
3. **Movimento com peso.** A troca de faixa anda com mola, nunca em linha reta, e tudo para com jogo em tela cheia ou com "reduzir movimento" ligado.
4. **Vidro só nos controles.** Uma camada de base e uma de conteúdo; cor sólida quando o sistema pede economia.
5. **Pular é o gesto principal.** A próxima faixa fica sempre à mão; a busca abre por atalho já com o teclado.

**Evitar:** vidro e reflexo em tudo; mudar de lugar o que já é usado só por
moda (a lição do M3 com a playlist); desfoque animado ou desfoque de tela inteira.

## Movimento nos temas prontos (decidido em 31/10)

Depois de ver o [Laboratório de movimento](https://claude.ai/artifact/LVPwRAyRJAvNjt8AJWYGk1),
o Felipe escolheu: **entram mola na troca de faixa, brilho da capa, brilho que
respira com a música e fundo tirado da capa. Fica fora o visualizador de pontos**
("não vejo encaixando"). Vídeo e GIF continuam só no modo liberdade total. Tudo
para com jogo em tela cheia e com "reduzir movimento" ligado.

Consequência técnica: o brilho que respira precisa do nível do áudio vindo do
motor (hoje a tela não recebe isso). O custo tem de ser medido com jogo aberto.

## Ideias de "tema com vida" e custo provável

Os custos são estimativas sem medição. Medir no PC do Felipe com jogo aberto antes de fechar.

| Ideia | Muda | Custo |
|---|---|---|
| Brilho da capa, trocando de cor só na troca de faixa | cor/material | quase zero |
| Forma por tema (cantos, pílula ou reto, espessura) | forma | zero (variáveis CSS) |
| Grão ou textura fixa | material | muito baixo |
| Mola na troca de faixa (só `transform` e `opacity`) | movimento | baixo |
| Brilho que "respira" com o volume da música | movimento | baixo a médio; precisa do nível do áudio |
| Vidro desfocado só no painel de controles | material | médio |
| Fundo desfocado da capa, feito uma vez por faixa | material | baixo |
| Visualizador de pontos em área pequena, 15–30 quadros | forma + movimento | médio |
| Vídeo ou GIF de fundo (liberdade total) | movimento | alto; pausar e descarregar com jogo |

## Limites

- Não abriram: Plexamp (403), Tidal (403), Lively Wallpaper (redireciona sem conteúdo); a página de movimento do M3 e a de materiais da Apple só em resumo.
- Não pesquisados: Winamp e foobar modernos, Fluent 2 em detalhe, estúdios de design.
- Nothing, Teenage Engineering e Apple Music no iOS 26 vêm só de imprensa e blogs. Nenhum custo foi medido.
- **Decisão do Felipe:** quanto movimento entra nos 4 temas fechados e quanto fica só no modo livre. A sugestão do estudo é mola e brilho nos fechados, e vídeo só no livre.
