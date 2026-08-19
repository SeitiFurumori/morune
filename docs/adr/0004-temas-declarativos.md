# ADR-0004 — Temas sao dados, nao codigo

Data: 2026-08-18
Status: aceito

## Contexto

O diferencial do produto e a customizacao profunda. A tentacao natural e dar ao
autor de temas o maximo de poder possivel: CSS, script, ou arquivos `.slint`
dentro do pacote.

Ao mesmo tempo, um `.musicpack` circula pela internet. Um usuario vai baixar um
tema bonito de um forum e abrir. Se abrir um tema puder executar codigo, o
aplicativo virou um vetor de distribuicao de malware com aparencia amigavel.

## Alternativas consideradas

**`.slint` dentro do pacote.** Liberdade total de layout, com o
`slint-interpreter` carregando a interface em tempo de execucao. Mas um arquivo
`.slint` nao e dado: tem expressoes, ligacoes reativas, callbacks e estado. Nao
executa processo nem abre socket, mas consegue consumir CPU, montar strings e
reagir a entrada. E um formato **ativo**, e um formato ativo perde a condicao de
declarativo aos poucos, nunca de uma vez.

**Script embutido (Lua, Rhai) com API restrita.** Poder maximo, e o modelo de
seguranca inteiro passa a depender do sandbox. Sandbox e trabalho permanente,
nao uma tarefa concluida, e nao ha equipe para manter isso.

**Tokens tipados em TOML.** O autor escolhe entre eixos que oferecemos. Um tema
e um conjunto de cores, numeros, enums e nomes de arquivo. Nao ha nada nele que
execute, e isso nao depende de sandbox nenhum.

## Decisao

O formato base de tema e **estritamente declarativo**. Cores, numeros, enums,
nomes de arquivo. Sem script, sem expressao, sem ponto de extensao executavel.

Para que a limitacao nao vire pobreza, os eixos oferecidos vao muito alem de
cor: posicao da barra lateral, posicao e altura do player, grade contra lista,
densidade, raios por categoria, escala tipografica, duracao e curva de
animacao, transparencia da janela, visibilidade de cada peca do player.

O tema `paper` existe como prova disso: comparado ao embutido, muda cor,
tipografia, forma, densidade, modo de exibicao, **lado** da barra lateral e
**posicao** do player. Nenhuma linha de codigo envolvida.

Layout arbitrario via `slint-interpreter` continua previsto, mas atras do
Developer Mode, com aviso explicito de que sai do formato declarativo, e nunca
para um pacote que o usuario acabou de baixar.

## Consequencias

O modelo de seguranca fica simples de explicar e de manter: nao ha sandbox a
defender, porque nao ha execucao. As defesas se concentram todas na extracao do
pacote — travessia de caminho, tipos de arquivo, bomba de compressao — que sao
problemas conhecidos com solucao conhecida.

Em troca, um autor nao consegue inventar uma tela que nao previmos, e cada novo
eixo de customizacao exige uma propriedade nova na interface. E um custo real e
recorrente, aceito conscientemente.

Duas consequencias de projeto seguem daqui:

- **A interface nao pode ter valor visual literal.** Se uma cor ou tamanho esta
  escrito na interface em vez de vir do tema, aquele detalhe simplesmente nao e
  customizavel. Isso e tratado como bug, nao como omissao.
- **Todo campo precisa de padrao e de faixa valida.** Um tema parcial tem de
  carregar, e um valor absurdo tem de ser corrigido com aviso em vez de recusar
  o tema — porque um tema quebrado nunca pode impedir o aplicativo de abrir.
