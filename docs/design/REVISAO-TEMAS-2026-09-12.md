# Correção visual de Aquário e Bruma — 12/09/2026

## Problema confirmado

As capturas do aplicativo confirmaram a crítica do usuário: Aquário cobria a
fotografia com um campo quase branco; Bruma tinha aparência de painel opaco.
Os testes anteriores validavam legibilidade, mas não a intenção visual.

Aquário usava um suporte central com alfa 240/255 (94%). Bruma usava 237/255
no campo e 232/255 (91%) no vidro. A lente estava ativa, mas era encoberta
pelo preenchimento colocado depois dela.

## Correção

- Campo central transparente; a cena ocupa novamente a área principal.
- Proteção de leitura localizada em títulos, faixas, cartões e painéis.
- Aquário 3.1.0: suportes claros com base de 60% e tintas mais escuras.
- Bruma 2.1.0: vidro com base de 20%, cuja densidade só aumenta se a imagem
  exigir para preservar a leitura. Os itens usam uma base independente.
- `readable_tint` calcula uma densidade a partir da imagem e dos estados de
  seleção, hover e iluminação. Cenas escuras não recebem automaticamente a
  proteção necessária para um fundo branco. O cálculo acompanha mudanças de
  cena/geometria ou preferências; não há timer novo.
- A refração continua limitada ao wallpaper do aplicativo; não é uma
  implementação de refração do desktop ou do conteúdo dinâmico.

## Arquivos alterados neste recorte

| Arquivo | Motivo |
| --- | --- |
| `crates/morune-app/themes/aquario/theme.toml` | Densidade e tintas para preservar a fotografia. |
| `crates/morune-app/themes/aquario/manifest.toml` | Versão 3.1.0. |
| `crates/morune-app/themes/bruma/theme.toml` | Vidro transparente e remoção do tint global. |
| `crates/morune-app/themes/bruma/manifest.toml` | Versão 2.1.0. |
| `crates/morune-app/ui/app.slint` | Campo aberto e proteção localizada em conteúdo e títulos. |
| `crates/morune-app/ui/components.slint` | Densidade adaptativa aplicada sobre a lente. |
| `crates/morune-app/ui/theme.slint` | Contrato `glass-tint`. |
| `crates/morune-app/src/optics.rs` | Cálculo da proteção mínima da cena. |
| `crates/morune-app/src/theme_bridge.rs` | Aplicar proteção ao carregar imagem e alterar preferências. |
| `crates/morune-app/src/bundled.rs` | Testar contraste composto e preservação de transparência em cenas escuras. |
| `tools/revisao-visual.ps1` | Capturar visão geral sem cliques fixos e usar PrintWindow com limites apropriados. |
| `PROJECT_MAP.md` | Atualizar a composição e o contrato visual. |
| Este documento | Registrar a crítica, correção e evidência. |

## Verificação

235 testes passaram (143 do app, 84 de temas, 8 de empacotamento), com dois
testes ignorados. Clippy com `-D warnings` e `git diff --check` passaram.
O aviso preexistente de librespot permanece fora do escopo. O formatador
encontrou um bloqueio temporário de arquivo; a execução posterior concluiu.

As primeiras tentativas de captura pela tela falharam por falta de primeiro
plano. O script passou a preferir o quadro da própria janela, sem capturar
outros aplicativos. A configuração pessoal é restaurada ao final das rodadas.
Os registros anteriores estão em `bench-out/temas-20260912-antes-dpi/`.
O build otimizado da correção principal concluiu. As capturas finais da página
inicial estão em `bench-out/temas-20260912-final/`: a fotografia do Aquário
fica exposta e o traço do fundo de Bruma atravessa a lateral. A revisão também
corrigiu o posicionamento do suporte dos títulos e excluiu a borda especular
da amostragem que define a densidade do vidro.

A conferência de configurações em 960×720 encontrou texto pequeno sobre
detalhes da água no Aquário. Foi acrescentado um suporte translúcido apenas
ao formulário de configurações, sem restaurar o preenchimento global.
O build release desse último ajuste concluiu em 14m02s; os 143 testes do app
passaram novamente (dois ignorados), assim como Clippy com `-D warnings` e
`git diff --check`. As capturas atualizadas estão em
`bench-out/temas-20260912-final-settings/`: a proteção melhora a leitura dos
textos pequenos e preserva a imagem sob o formulário. A configuração pessoal
foi restaurada pelo script ao terminar.

O executável release foi conferido como `Windows GUI`. O atributo de
subsystem agora vale também para builds de depuração, evitando que qualquer
binário iniciado pelo Explorer abra um console auxiliar. A cópia entregue para
uso diário fica em `dist/Morune.exe`; `target/ci/morune.exe` continua sendo
apenas artefato de testes. O release final foi recompilado com sucesso em
10m18s; `dist/Morune.exe` tem 21.089.280 bytes e também foi conferido como
`Windows GUI`.

Após essa alteração, os testes do app passaram novamente: 143 passaram e 2
foram ignorados. O único aviso continua sendo a expectativa de lint não
utilizada já existente em `vendor/librespot-core`.

Não foi feita medição de FPS nesta rodada. A captura estática não comprova
fluidez, nem equivale a uma validação completa de reprodução e navegação.
