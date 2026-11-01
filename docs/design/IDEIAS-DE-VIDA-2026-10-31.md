# Ideias de vida para o Morune — 31/10/2026

Pedido do Felipe depois de ver os temas novos: "a única coisa que tem vida no
app é o brilho do player", e a primeira pesquisa "só trouxe 4 ideias". Este é o
catálogo da segunda pesquisa (subagente, Sonnet). Ver também
[REFERENCIAS-VISUAIS-2026-10-31.md](REFERENCIAS-VISUAIS-2026-10-31.md).

## Regras que valem para tudo

- Animar só `transform` e `opacity`. Nada de largura, altura ou desfoque animado.
- Todo movimento para com jogo em tela cheia e com "reduzir movimento".
- O que se repete centenas de vezes por dia (clique, hover, atalho) quase não
  anima; a vida fica no que é contínuo e lento (cor, respiração) e nas
  transições raras (abrir álbum, trocar de faixa). Animações de interface
  abaixo de 300 ms ([Emil Kowalski](https://emilkowal.ski/ui/you-dont-need-animations)).

Temas: **L** Lume, **V** Vidro, **G** Grade, **C** Claro.

## Catálogo

| # | Área | Ideia | O que a pessoa vê | Custo | Temas |
|---|---|---|---|---|---|
| 1 | Lista | Barras de som na faixa tocando | O número vira 3–4 barrinhas que sobem com o pulso da música | baixo | LVGC |
| 2 | Lista | Régua de cor na linha tocando | Faixa de 2 px na cor da capa que desliza de linha em linha | baixo | LVGC |
| 3 | Lista | Hover que ilumina por perto | Brilho suave seguindo o mouse dentro da linha | médio | LV |
| 4 | Lista | Curtir com estouro leve | O coração preenche com mola curta | baixo | LVGC |
| 5 | Lista | Reordenar a fila com física | Vizinhas abrem espaço com mola ao arrastar | médio | LVC |
| 6 | Lista | Faixa que sai da fila | A linha encolhe e as de baixo sobem | baixo | LVGC |
| 7 | Páginas | Capa que voa para o cabeçalho | A capa do cartão cresce até o topo da página do álbum (View Transitions) | médio | LVC |
| 8 | Páginas | Voltar desfaz o voo | A capa volta ao cartão; a rolagem é mantida | médio | LVGC |
| 9 | Páginas | Troca de página com fade curto | 120 ms de opacidade + 6 px | baixo | LVGC |
| 10 | Páginas | Cabeçalho que encolhe ao rolar | O título grande vira barra fina com a cor da capa | médio | LVC |
| 11 | Lateral | Pílula de seleção que desliza | O destaque do item ativo desliza com mola | baixo | LVGC |
| 12 | Lateral | Playlist tocando com ponto vivo | Ponto pulsando na cor da capa na playlist atual | baixo | LVC |
| 13 | Lateral | Lateral que recolhe para ícones | Atalho recolhe a barra lateral | médio | LVGC |
| 14 | Lateral | Paleta de comandos (Ctrl+K) | Caixa central que acha faixa, playlist ou ação | médio | LVGC |
| 15 | Player | Barra de tempo que engrossa | 3 px em repouso, 6 px sob o mouse, tempo acima do cursor | baixo | LVGC |
| 16 | Player | Preenchimento na cor da capa | O trecho tocado na cor da capa, com degradê | baixo | LVC |
| 17 | Player | Play/pausa que se transforma | O ícone muda de forma de modo contínuo | baixo | LVGC |
| 18 | Player | Título que rola se não couber | Nome longo desliza devagar, só com o player à vista | baixo | LVGC |
| 19 | Player | Capa com inclinação ao pairar | Até 4° seguindo o mouse | médio | LVC |
| 20 | Cartões | Entrada escalonada | Cartões entram em sequência na primeira abertura | baixo | LVC |
| 21 | Cartões | Elevação ao pairar | O cartão sobe 2 px e o botão de tocar aparece com mola | baixo | LVC |
| 22 | Cartões | Pilha de capas na playlist | 3 capas empilhadas que se abrem em leque | médio | LVC |
| 23 | Cor | Cabeçalho do álbum com a cor da capa | Degradê da cor dominante no topo da página | baixo | LVC |
| 24 | Cor | Foco, seleção e rolagem na cor da capa | Anel de foco, seleção e barra de rolagem na cor da faixa | baixo | LVGC |
| 25 | Cor | Cor com contraste garantido | A cor da capa é ajustada antes de virar texto ou botão | baixo | LVGC |
| 26 | Estados | Esqueleto com respiração lenta | Blocos que respiram em vez de varredura brilhante | baixo | LVGC |
| 27 | Estados | Estado vazio com capa fantasma | Frase neutra e uma capa fantasma na cor atual | baixo | LVC |
| 28 | Estados | Aviso que sai sozinho | Pílula no canto que some em 2,5 s | baixo | LVGC |
| 29 | Contexto | Saudação e temperatura por hora | "Boa noite" e fundo levemente mais frio à noite | baixo | LVC |
| 30 | Contexto | Retomar de onde parou | Ao abrir, a última faixa e a posição, com "continuar" | baixo | LVGC |
| 31 | Tipografia | Peso variável no título | O peso sobe ao pairar ou com a rolagem | médio | LVC |
| 32 | Toque | Botões que afundam | Clique encolhe a 0,97 e volta com mola | baixo | LVC |

## Recomendadas primeiro (pela pesquisa)

1, 24, 2, 11, 7, 23, 32 e 30: maior efeito pelo menor custo e risco.

## Armadilhas

1. Animar o que se repete: parece lento na centésima vez do dia.
2. Tudo pulsando junto: vira ruído. Um ou dois lugares por tela.
3. Cor da capa sem controle de contraste.
4. Inclinação 3D e paralaxe em todo cartão: cansa e parece brinquedo.
5. Animar largura, altura ou desfoque: pesa justo com o jogo aberto.

## Limites da pesquisa

Só duas fontes foram lidas de verdade (Emil Kowalski e a documentação de View
Transitions do Chrome). As inspirações dos outros apps vêm do conhecimento
prévio do modelo e não foram conferidas na versão atual de cada app. Nenhum
custo foi medido no Morune.
