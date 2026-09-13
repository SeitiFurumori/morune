# Bruma sobre o desktop e controles translúcidos do Aquário

## Pedido e correção

Bruma deve mostrar a área de trabalho desfocada, não uma imagem do próprio
aplicativo. A versão anterior não atendia: Acrylic desligado, fundo opaco e
wallpaper preenchendo a janela. Agora o tema pede Acrylic, não define imagem
e mantém opacidade global 1 para preservar texto, ícones e capas.

O retorno do DWM alimenta `AppWindow.native-backdrop-active`. Com material
aceito, a janela usa 12% de tinta; quando indisponível ou suspenso pelo portão
de tela cheia, usa o fundo de fallback. A lente de wallpaper não acrescenta
camadas de tinta quando não há imagem. O desfoque é do compositor do Windows;
não há captura contínua do desktop nem simulação de refração de outras janelas.

Aquário usa a fotografia fornecida pelo usuário em 13/09, preservada no asset
`assets/cidade-aero.png`. O carregador limita a imagem em memória a 3840 px
por lado, como já fazia com outros wallpapers. Curtir, aleatório, repetição e play usam vidro
claro com contorno e ícones escuros. Progresso e volume usam preenchimento
escuro translúcido. Os estados e comandos de teclado são preservados.

Referência de plataforma: [DWMSBT_TRANSIENTWINDOW](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwm_systembackdrop_type).

## Arquivos deste recorte

- `crates/morune-app/themes/aquario/theme.toml` e `manifest.toml`: foco em
  petrol, versão 3.3.0 e novo fundo padrão.
- `crates/morune-app/themes/aquario/assets/cidade-aero.png`: imagem fornecida,
  sem alteração do original.
- `crates/morune-app/themes/bruma/theme.toml`: Acrylic, remoção do wallpaper,
  base e borda de fallback.
- `crates/morune-app/themes/bruma/manifest.toml`: versão 2.2.0 e descrição.
- `crates/morune-app/src/main.rs`: propagar sucesso do material para a janela.
- `crates/morune-app/ui/app.slint`: fundo nativo condicionado ao DWM e play claro.
- `crates/morune-app/ui/components.slint`: controles claros e remoção da tinta
  duplicada em `SceneGlass` sem wallpaper.
- `crates/morune-app/src/bundled.rs`: verificar que Bruma pede Acrylic sem imagem,
  mantendo as verificações existentes de contraste e opacidade do conteúdo.
- `tools/revisao-visual.ps1`: opção `DesktopComposition` para capturar o quadro
  composto em tela; `PrintWindow` não comprova o fundo nativo.
- `tools/probe-backdrop.ps1`: janela independente de teste com fundo alternado.
- `PROJECT_MAP.md`: contrato entre DWM e janela.
- Este documento: escopo, evidências e limites.

## Validação

A primeira tentativa de fundo sempre transparente reprovou contraste sem
compositor; isso motivou separar o material ativo do fallback. Os 143 testes
do app e o lint passaram após esse ajuste.

A prova nativa de 13/09 está em `bench-out/vidro-nativo-comprovacao-20260913/`.
Entre as duas capturas, uma janela atrás do Morune muda de azul para vermelho;
o fundo do Bruma acompanha a mudança, com a divisão entre branco e cor
desfocada. Não há wallpaper no Bruma. As primeiras tentativas do teste não
eram conclusivas: a janela auxiliar precisava de um processo com seu próprio
loop de eventos, e a versão antiga do PowerShell bloqueava o script. O teste
agora usa o PowerShell atual e verifica se a janela auxiliar realmente abriu.

Com a nova imagem do Aquário, o texto secundário precisou ficar um pouco mais
escuro (`#0c2530`) para preservar contraste sobre os pixels mais escuros,
sem aumentar a opacidade dos painéis. O teste usa a mesma imagem limitada em
memória que a interface, não uma versão reduzida apenas para o teste.
Os 143 testes passaram novamente (dois ignorados), assim como Clippy com
`-D warnings`. A foto incorporada foi conferida por SHA-256 e é idêntica ao
arquivo fornecido. O build release final concluiu em 7m23s. `git diff --check`
e a sintaxe dos dois scripts de captura também passaram.

As capturas da entrega estão em `bench-out/temas-entrega-20260913/`: Aquário
usa a cidade fornecida e controles claros; Bruma não repinta as antigas faixas
escuras de rolagem sobre o material nativo. A configuração pessoal foi
restaurada depois da revisão.

`dist/Morune.exe` foi atualizado e conferido por hash contra o release. Tem
45.243.904 bytes e subsistema `Windows GUI`. A versão anterior foi preservada
em `dist/backups/Morune-20260913-012759.exe`. O novo executável abriu com sua
janela, diretamente da pasta `dist`, sem preparar o ambiente de compilação.
Não foi feito benchmark de FPS nem uma reprodução completa de áudio nesta
rodada; a comprovação visual cobre material, imagem e aparência dos controles.
