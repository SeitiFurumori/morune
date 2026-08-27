# Retomada — ciclo de correção de UI

Aberto em 25/08/2026. Substitui a conversa: ler isto antes de continuar.

## Por que este ciclo existe

Felipe olhou o aplicativo rodando e disse que a interface parecia "fraca,
trabalho de iniciante". Um critique com dois avaliadores independentes
confirmou a impressão e localizou a causa. O relatório completo está em
`.impeccable/critique/2026-08-25T02-51-13Z__crates-morune-app-ui-app-slint.md`.

Nota Nielsen: **21/40**. O gargalo não é acessibilidade nem lógica — essas são
o ponto forte do projeto. É hierarquia visual e ruído.

## As três causas da impressão de "iniciante"

1. **Conteúdo cortado em três regiões ao mesmo tempo, nos quatro temas, sem
   nenhum indicador de rolagem.** Última faixa serrada no meio do texto, item
   de playlist reduzido a um retângulo órfão, grade de cartões cortada na base.
2. **O símbolo do Morune aparece 13 vezes na mesma tela** como placeholder de
   capa faltando. A marca virou sinônimo de ausência. E `Artwork` usa o mesmo
   desenho para "carregando" e "sem capa" — significados opostos.
3. **A hierarquia contradiz a tese do produto.** A faixa tocando, que é o que
   importa a quem faz alt-tab do jogo, é 14px no canto inferior esquerdo;
   cinco retângulos cinzas vazios ocupam o maior bloco da tela.

## Já aplicado (conferido na tela em 26/08 — ver a seção seguinte)

- `app.slint` — `TrackShelf.visible-height` agora soma os sete vãos entre as
  oito linhas. Era `row-height * 8`; faltavam 28px, exatamente a faixa serrada.
- `app.slint` — `Card.height` vem de `conteudo.preferred-height` em vez de
  `card-width + 56px`. Os 56px não acompanhavam fonte nem entrelinha do tema,
  e cortavam os descendentes do título no Paper.
- `app.slint` — a célula vazia do `CardCollection` perdeu a altura fixa;
  acompanha a linha.
- `components.slint` — componente novo `ScrollFade`: degradê que diz "há mais
  abaixo" sem ocupar altura. Recebe a cor de base porque fundo de página,
  de barra lateral e de painel são tokens diferentes.
- `ScrollFade` ligado na prateleira de faixas (base `background`) e na lista de
  playlists da barra lateral (base `sidebar-background`), os dois só quando há
  o que rolar.
- `tools/screenshot.ps1` — **bug corrigido**: usava `CopyFromScreen` depois de
  `SetForegroundWindow`, que o Windows recusa vindo de processo em segundo
  plano. Gerava captura da janela que estivesse por cima, com aparência de
  sucesso. Agora usa `PrintWindow` e confere o resultado antes de salvar.

## Verificado na tela em 26/08/2026

Os quatro temas foram capturados e conferidos (`bench-out/fix3-*.png`,
`bench-out/biblioteca-cristal.png`). O que a captura provou:

- **Faixa serrada: corrigido nos quatro temas.** A oitava linha da prateleira
  aparece inteira. Ressalva: no Paper o `ScrollFade` esmaece bastante o nome do
  artista da ultima linha — legivel, mas no limite.
- **Grade cortada na base: corrigido.** Medido pixel a pixel no Cristal: onde
  antes havia corte chapado em 75,75,81, ha agora degrade 60→53→46→39→31 ate o
  fundo. Exigiu trocar as tres paginas roladas de `Flickable` para `Rectangle`
  com `Flickable` dentro, e cortar a altura preferida na raiz.
- **Ultimo cartao sangrando pela borda direita: corrigido e verificado.** A
  Biblioteca abre com quatro colunas exatas, sem redimensionar a janela. A
  causa era o `init` ausente, nao a conta de colunas.
- **Item de playlist cortado ao meio na barra lateral: corrigido.** A area da
  lista e arredondada para baixo, para o maior multiplo do passo da linha que
  caiba; o resto fica de fundo da barra. Verificado nos quatro temas: a lista
  termina em "Senhor" inteiro, sem meia linha embaixo.
- **As 13 marcas d'agua: continuam**, como esperado — o item P1 nao foi tocado.

Achado colateral: a captura do Cristal mostra conteudo do desktop atraves da
janela (o acrilico amostrando o que esta atras). E a evidencia visual da
decisao pendente sobre o Cristal ficar opaco.

- **Contraste de `border` e `scrollbar`: corrigido nos quatro temas e no tema
  embutido**, com portao automatico. Os valores medidos antes iam de 1,32:1 a
  1,87:1; os novos alfas sao o **menor** que alcanca 3:1 em cada tema, para
  mexer no visual o minimo que o criterio exige. A checagem de contraste da
  crate so olhava texto -- foi por isso que oito reprovacoes conviveram anos
  com uma validacao que roda a cada boot. Agora cobre limite grafico e
  componente de interface (WCAG 1.4.11), e um teste novo reprova qualquer tema
  de fabrica que saia reprovando.

Verificacoes do projeto apos as mudancas: `cargo fmt --check` limpo, 286 testes
passando, `clippy -D warnings` limpo, e o aviso de laco de binding que a
primeira tentativa introduziu foi eliminado.

## Sobre confiar nas capturas

`tools/screenshot.ps1` le a janela pelo sistema, e isso tem um limite que nao
da para contornar: **janela translucida deixa passar o que estiver atras, e o
que chega ao arquivo e a composicao, nao a interface.** Nesta sessao ele
devolveu, sem erro, a janela de um navegador que estava atras -- com a
verificacao de primeiro plano passando, porque `-eq` entre dois `[IntPtr]` no
PowerShell 5.1 compara objetos e nao enderecos.

**O caminho confiavel e `tools/snapshot.ps1`**, que pede o quadro ao proprio
renderizador do Slint. Foi por ele que as correcoes acima foram conferidas.
Ele tambem estava quebrado: `$ErrorActionPreference = "Stop"` transformava
qualquer aviso de lint do cargo em erro terminante, e o script morria antes de
capturar o primeiro tema. Os dois foram corrigidos.

Para ver o vidro como o DWM o compoe, o `screenshot.ps1` continua sendo a unica
via -- so nao serve como prova sozinho.

## Onde parou

A verificação visual foi feita. **Dois dos três cortes sumiram**; o item de
playlist cortado ao meio na barra lateral continua e voltou para a fila como
P0.

A janela preta na captura **não era corrida de tempo**, como esta página supôs
antes: janela com acrílico volta vazia do `PrintWindow` sempre, porque o vidro
é composto pelo DWM a partir do que está atrás e o `PrintWindow` pede o desenho
só à janela. Bruma e Cristal caíam nisso toda vez. O script agora tem leitura
da tela como alternativa, condicionada a confirmar antes que a janela do Morune
é a de primeiro plano.

Não há mais P0 aberto. Próximo passo: os P1 — separar "carregando" de "sem
capa" e acabar com as 13 marcas d'água.

## Fila, na ordem

| | O quê | Onde |
|---|---|---|
| ~~P0~~ | ~~Clipping: faixa serrada e grade cortada~~ **verificado em 26/08** | `app.slint` |
| ~~P0~~ | ~~Item de playlist cortado ao meio na barra lateral~~ **feito em 26/08** | `app.slint` |
| ~~P0~~ | ~~Contraste de `border` e `scrollbar`~~ **feito em 26/08, com teste** | `themes/*/theme.toml` |
| P1 | Separar "carregando" de "sem capa"; matar as 13 marcas d'água | `components.slint` |
| P1 | Accent reservado a duas coisas, não quatro | `app.slint` |
| P1 | Ligar `show-hero` com a faixa tocando em `size-display` | `app.slint` |

~~Também pendente da grade: o último cartão sangra pela borda direita.~~
Corrigido e verificado em 26/08: a conta de colunas estava certa, faltava rodar
no `init` — sem ele valia o default fixo de cinco até alguém redimensionar.

## Decisões que são do Felipe

1. **Tokens declarados e mortos.** `Theme.size-display`, `Theme.motion-slow`,
   `Layout.show-hero`, `hero-height` e `density` têm **zero usos** em qualquer
   linha de UI; `view-mode` declara três modos e tem um efeito. Implementar ou
   cortar — deixar como estão é o que faz temas "não fazerem nada".
2. **O tema padrão.** Felipe informou que **Bruma é o Liquid Glass, Cristal
   não** — mas `cristal/theme.toml` está com `acrylic = true` e
   `window_opacity = 0.94`, o que contradiz a intenção. Confirmar se o Cristal
   deve ficar opaco. Isso importa porque acrílico amostra o desktop, não o jogo
   em tela cheia: sobre um jogo, colapsa em cinza chapado.

## Ressalvas honestas

- **O contraste de Bruma e Cristal não foi medido de verdade.** Os números são
  alpha-over simples, sem o blur e o tint do acrílico. Falta ler o pixel
  renderizado.
- O comentário em `themes/bruma/theme.toml:2-4` afirma corrigir um contraste de
  2,79:1; pelo cálculo disponível o valor atual dá 2,85:1 e continua
  reprovando AA. Sujeito à ressalva acima.
- **O detector do Impeccable não lê `.slint`** (extensão ausente de
  `SCANNABLE_EXTENSIONS`). Ali `exit 0` significa "nenhum arquivo lido", não
  "limpo". Toda a varredura da UI foi manual.

## O que o critique achou de bom, e convém não quebrar

A disciplina de tokens é real: 100% dos `font-size` usam token, zero cores
nomeadas do Slint. A acessibilidade está acima da média: 44 propriedades
`accessible-*`, rótulos compostos úteis, Tab funcional, anel de foco em quatro
lugares. O problema nunca foi a fundação — é que quase nenhum token expressivo
é usado.
