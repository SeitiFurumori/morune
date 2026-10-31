# ADR-0007 — A tela passa a ser web, no Tauri; o motor continua em Rust

Data: 2026-10-31
Status: aceito (substitui a parte de interface da [ADR-0001](0001-stack.md))

## Contexto

A ADR-0001 escolheu o Slint pela meta de 70 MB em repouso, antes de existir a
identidade do produto. Três coisas mudaram:

1. **A rolagem nunca ficou fluida.** Num monitor de 240 Hz, as listas do Slint
   pausam de 30 a 40 ms a cada giro da roda: entre 1 e 16 pausas por segundo,
   entre 57 e 225 quadros. O desenho leva 2 a 3 ms, e o Slint chega a 235
   quadros redesenhando sem parar. Desligar o recorte arredondado, as capas e
   o vidro e criar as linhas de antemão não mudou nada. A pausa está na
   rolagem do próprio Slint.
2. **A identidade nova pede vídeo, GIF e efeitos vivos** (entrevista de
   31/10, `docs/design/ENTREVISTA-IDENTIDADE-2026-10-31.md`). O Slint não
   toca vídeo.
3. **Ver uma mudança custava de 15 a 50 minutos de compilação.**

O critério de desempenho também ficou mais claro (Felipe, 31/10): aberto e em
uso pode consumir um pouco; minimizado ou na bandeja, não pode atrapalhar
quem está jogando. É o modelo do Wallpaper Engine.

## Medição que decidiu

Protótipo em `D:\Users\Furumori\Documents\morune-prototipo-tauri`, com 1000
faixas com capa, vidro com desfoque e vídeo de fundo, medido no PC do Felipe
(RTX 3060, 240 Hz) em 31/10/2026:

| Estado | Protótipo com vídeo | Protótipo sem vídeo |
|---|---|---|
| Rolando | 214–240 quadros, nenhuma pausa | 240 quadros, nenhuma pausa |
| Aberto parado | 15% de um núcleo, 448 MB | 0,9%, 397 MB |
| Minimizado | 11% (o vídeo continuava tocando) | 0,9% |
| Bandeja (tela destruída) | 0%, 40 MB, 1 processo | 0%, 28 MB |

Destruir a tela na bandeja fecha os processos do WebView2 de verdade.

## Decisão

- **A tela é feita em web e roda no Tauri 2,** sobre o WebView2 do Windows.
- **O motor continua em Rust e não muda:** Spotify, áudio, fila, armazenamento
  e temas. A tela só mostra o estado e pede ações.
- **Regras de segundo plano, obrigatórias:**
  - na bandeja, a tela é destruída e sobra só o motor;
  - minimizado, vídeos e animações param e a tela é suspensa;
  - com um jogo em tela cheia, vale a regra que já existe: tudo que se mexe
    vai a zero.
- **Tema continua sendo só dados e mídia** ([ADR-0004](0004-temas-declarativos.md)).
  O CSS do tema é gerado pelo app a partir dos tokens, e a política de
  segurança da página bloqueia qualquer script vindo de tema.

## Consequências

- Com a tela aberta, a memória fica na casa de centenas de MB, contra dezenas
  no Slint. É aceito pelo critério novo; em segundo plano o número volta a
  dezenas.
- A interface inteira é refeita. O motor e as regras de negócio do
  `morune-app` são reaproveitados.
- O Morune 1.0 (Slint) continua instalado e recebendo correções até a versão
  nova estar pronta e testada pelo Felipe.
- Mudanças visuais passam a aparecer em segundos, com recarga na hora.
