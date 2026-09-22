# Auditoria de produto — 21/09/2026

Meta da sessao: **trocar de pagina e rolar sem travar, com movimento que se
sente e nao se nota.** Publico: quem joga e faz alt-tab; o Morune tem de ser
rapido e nao pode pesar.

Seis lentes, um passe so: UI, UX, IxD, Visual, Produto. Evidencia: 25 capturas
com a sessao real (`bench-out/homologacao/`, tema Aquario, 1919x1030) e leitura
de `app.slint`, `components.slint`, `theme.slint`, `state.rs`.

Regra de movimento adotada (design-motion-principles, lente Emil primaria,
Jakub secundaria): **frequencia decide.** O que se faz cem vezes por dia nao
anima ou anima em < 150 ms; o que se faz de vez em quando pode ter 200 ms; nada
passa de 300 ms; atalho de teclado nao anima. `reduce-motion` e o portao de
tela cheia ja zeram os tokens — toda animacao nova tem de sair deles.

## O que ja estava certo

- Rolagem com a roda ja e suave: o `Flickable` do Slint 1.17 anima 180 ms e
  soma o que falta quando a roda gira de novo. Nao ha o que fazer aqui.
- Tokens de movimento existem (`motion-fast` 120, `normal` 200, `slow` 320),
  zerados por preferencia e por jogo em tela cheia. A infraestrutura esta
  pronta; o que faltava era usa-la.
- Hover e foco identificam o alvo inteiro; anel de foco; arvore acessivel.
- Desenho a 235 fps; o gargalo nunca foi o renderizador.

## Achados, por ordem de impacto

### P0 — ja corrigidos nesta sessao (antes desta auditoria)

- Cada clique reconstruia todas as listas e a troca de pagina destruia a
  arvore. Corrigido: modelos vivos com diferenca por linha, paginas de dados
  vivas por `visible`, curtidas da Home virtualizadas. Ver `PERFORMANCE.md`.

### P1 — quebra a experiencia

1. **Troca de pagina e teleporte.** Sem transicao nenhuma; o olho perde a
   referencia de onde estava. Corrigir com a coisa mais barata que existe:
   a pagina que entra sobe 6 px e aparece em `motion-fast` (120 ms), `ease-out`;
   a que sai some na hora. Nada de deslizar lateral (custo de camada em tela
   cheia e sensacao de "app de celular").
2. **Ctrl+K abre a Busca mas nao poe o cursor no campo** (`app.slint`,
   `keys.key-pressed`). Na homologacao o texto digitado foi para o nada
   (captura 15). Atalho que so navega e meio atalho.
3. **Setas dos atalhos saem como `â†'`** (captura 22, `app.slint` ~4458): o
   texto do overlay esta em bytes UTF-8 lidos como Latin-1. Bug visivel toda
   vez que alguem abre Ctrl+/.

### P2 — friccao que se sente

4. **Configuracoes em janela larga: rotulo na esquerda, controle a 1500 px na
   direita** (captura 22). A linha inteira e uma caixa de vidro, mas o olho
   nao liga "Nivelar o volume" ao interruptor. Limitar a largura do conteudo
   da pagina de Configuracoes (~880 px, centralizado ou alinhado a esquerda).
5. **Sem feedback ao clicar num cartao ou faixa.** O hover pinta, o clique nao
   responde ate a rede voltar. Um `scale(0.98)` no pressed (`motion-fast`) da
   a resposta tactil que Emil recomenda — e e o unico lugar onde escala cabe.
6. **A mensagem de status entra bonito e some por corte** (`if`). Aceitavel
   pelo custo, documentado no codigo. Manter.
7. **Sidebar recolhe com animacao de largura; o conteudo da pagina salta.**
   A largura da pagina segue a barra sem animar. Como o `animate width` ja
   existe, o conteudo acompanha de graca; conferir se ha `clip` faltando nos
   cartoes durante a animacao (nao reproduzido na captura).

### P3 — visual

8. **Marca d'agua do Morune em cartao sem capa** foi trocada por inicial +
   cor (bom), mas na Home "Suas playlists" oito cartoes seguidos de letra
   gigante (captura 02) viram um alfabeto. Reduzir a inicial para ~30% da
   altura e deixar a cor falar.
9. **Titulo de pagina ("Fila", "Biblioteca") flutua sobre a foto** sem caixa
   (capturas 12, 18); no Aero, tudo mais esta dentro de vidro. Ou entra na
   primeira caixa, ou ganha sombra de texto.
10. **Biblioteca mostra so artistas seguidos** ("Da sua conta") — a pagina
    parece vazia com 7 itens numa tela de 1900 px. Produto: e a Biblioteca ou e
    "Artistas"? Se e Biblioteca, playlists e albuns salvos entram aqui tambem.

## Ordem de execucao proposta

| # | Item | Lente | Custo |
|---|---|---|---|
| 1 | Transicao de pagina (P1.1) | IxD | baixo |
| 2 | Ctrl+K foca o campo (P1.2) | UX | baixo |
| 3 | Setas do overlay (P1.3) | UI | trivial |
| 4 | Pressed nos cartoes e linhas (P2.5) | IxD | baixo |
| 5 | Largura da pagina de Configuracoes (P2.4) | UI | baixo |
| 6 | Inicial menor nos cartoes (P3.8) | Visual | trivial |
| 7 | Titulo de pagina no Aero (P3.9) | Visual | decisao do Felipe |
| 8 | Escopo da Biblioteca (P3.10) | Produto | decisao do Felipe |

Itens 1–6 nao mudam direcao visual; 7 e 8 sao do Felipe.

## Complemento — critique do impeccable (dois avaliadores independentes)

Nota: **24/40** (era 21/40 em 25/08). Snapshot completo em
`.impeccable/critique/2026-09-21T23-30-00Z__crates-morune-app-ui.md`. O que a
primeira passada desta pagina deixou passar:

| Sev. | O que | Onde |
|---|---|---|
| P1 | Tocar uma playlist custa duas paginas; cartao e sidebar nao tem "tocar" | `Card`, `PlaylistNavItem` |
| P1 | "Tocando" fora da barra e so a cor do titulo; sidebar e Fila nao marcam a atual | `TrackListRow` ~590, `QueuePage` |
| P1 | Fila sem saida: nada aceso, sem voltar, `Esc` so em `page == 5` | `keys.key-pressed` |
| P2 | "Continuar tocando ao fechar" e "Iniciar com o Windows" estao sob "Audio" | `SettingsPage` |
| P2 | Volume sem mudo, sem roda, sem atalho | `PlayerBar` ~1690, `Slider` |
| P2 | Coracoes todos cheios na lista de curtidas | `TrackListRow` |
| P2 | Texto cinza sobre vidro no Bruma (vibrancy) — **sem captura, por medir** | `theme.slint` |
| P3 | 28 cores literais fora do tema; play Aero duplicado (`:1524`/`:4740`) | `app.slint` |
| P3 | 2 botoes sem nome acessivel no menu da bandeja (`:4729`, `:4780`); scrim sem papel | `TrayMenuWindow` |
| P3 | 17 `Text` ligados a dado sem `elide`/`wrap` | varios |
| P3 | Tooltip sem atraso; overlay com altura fixa e colunas por espaco; "Sair" sem confirmar | `IconButton`, overlay |

Descartado apos conferir: "curtir reinicia a faixa" era o roteiro clicando na
borda da linha tocando (1790,178), nao no coracao.

Lentes usadas alem do critique: `design-motion-principles` (regra de
frequencia), `apple-design` (resposta no pointer-down, vibrancy sobre vidro,
interrupcao), `better-ui` (troca de tema sem transicao, raio concentrico,
contorno de imagem — os dois ultimos o projeto ja seguia).
