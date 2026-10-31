# O Slint ainda é a escolha certa? — pesquisa de 31/10/2026

Feita para reavaliar a [ADR-0001](../adr/0001-stack.md) depois da nova direção de identidade, que pede vídeo, GIF e efeitos vivos.

## Slint

- **Vídeo:** não tem componente de vídeo ([#10297](https://github.com/slint-ui/slint/issues?q=is%3Aissue+video), fechado). O exemplo com GStreamer só evita cópia no Linux; no Windows, cada quadro passa pela CPU. O FemtoVG-WGPU aceita textura de GPU, um caminho possível, mas ninguém mostrou vídeo assim.
- **GIF animado e desfoque de fundo** entram no 1.19, que ainda não saiu ([CHANGELOG](https://raw.githubusercontent.com/slint-ui/slint/master/CHANGELOG.md)).
- **Recorte com canto arredondado** sempre desenha o que está dentro numa imagem à parte. Está confirmado na [#13007](https://github.com/slint-ui/slint/issues/13007), item 079, e a correção proposta foi recusada. Bate com a suspeita da rolagem.
- **O 1.18 tem correções de desenho:** sombra redesenhada a cada quadro (#12545) e cerca de 12% de ganho na rolagem (#13064).
- **Acrílico como recurso próprio:** está em aberto desde 2023 (#4121). O Morune já aplica por fora.

## Quem compete

1. **Tauri 2 (WebView2).** Vídeo, temas em CSS e rolagem vêm prontos. Mas o WebView2 é o mesmo motor do Chrome. Não há medição confiável de memória no Windows: uma issue antiga ([#5889](https://github.com/tauri-apps/tauri/issues/5889)) mostra consumo parecido com o do Electron. Para tema sem código, seria preciso barrar JavaScript.
2. **Flutter + media-kit.** Tem vídeo pela GPU, mas exige reescrever a interface inteira em Dart e manter uma ponte com o Rust.
3. **Qt/QML.** É o mais maduro, mas QML é código, o que não serve para tema só com dados.
4. **Iced, egui, Freya, GPUI e Dioxus** estão imaturos ou não têm vídeo, temas seguros ou números.

## Não encontrado

- Memória em repouso medida com método claro no Windows, para qualquer uma das opções.
- Player feito em Tauri, Iced ou GPUI com consumo medido.
- Qualquer exemplo de vídeo atrás de uma janela Slint transparente.
