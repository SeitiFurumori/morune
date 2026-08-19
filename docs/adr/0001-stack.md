# ADR-0001 — Rust com Slint, sem Electron

Data: 2026-08-18
Status: aceito

## Contexto

O produto precisa ser um cliente de musica para Windows com startup abaixo de
1 s, menos de 70 MB de RAM em repouso, instalador abaixo de 40 MB e customizacao
profunda. Nada disso e alcancavel com um runtime de navegador embutido.

## Alternativas consideradas

**Electron / Tauri com WebView.** Electron comeca em 150–250 MB de RAM e
80–150 MB de instalador: fora do orcamento por uma ordem de grandeza. Tauri usa
o WebView2 do sistema e e bem mais leve, mas ainda carrega um motor de
navegador, o consumo depende de um componente que nao controlamos, e o modelo de
customizacao viraria CSS injetado — o que abre exatamente a porta de "tema com
codigo" que queremos fechada.

**C# com WinUI 3.** Boa integracao com o Windows, mas exige o runtime .NET e o
Windows App SDK, o que sozinho ja consome boa parte do orcamento de instalador,
e o consumo de RAM de uma app WinUI tipica fica acima da meta.

**C++ com Qt.** Atinge o desempenho, mas o licenciamento comercial complica um
projeto que quer ser aberto, o setup e pesado, e a seguranca de memoria fica por
conta da disciplina.

**Rust com egui.** Modo imediato, muito leve, mas redesenha a cada quadro — ruim
para CPU em repouso — e o estilo e programatico, o que dificulta o objetivo de
customizacao declarativa.

**Rust com Slint.** Interface declarativa com propriedades reativas, que e
exatamente o formato que o motor de temas precisa alimentar. Renderizacao por
GPU com reserva por software. API estavel desde a 1.0. Licenca permitindo uso em
projeto aberto.

## Decisao

Rust com Slint 1.17, renderizador FemtoVG e reserva por software.

Sem `std-widgets`: a interface e construida sobre `Rectangle`, `Text`, `Path`,
`TouchArea` e `Flickable`. Isso era necessario para que todo pixel venha do
tema, e tem o efeito colateral de nao carregar a biblioteca de widgets.

Skia foi descartado como renderizador: a qualidade de texto e melhor, mas o peso
no binario nao cabe no orcamento.

## Consequencias

Boas, medidas em 18/08/2026: executavel de 9,07 MB, 8 ms ate o laco de eventos,
70,3 MB em repouso, 0% de CPU ociosa.

Ruins: a comunidade do Slint e menor que a do ecossistema web, ha menos
componentes prontos, e a customizacao fica limitada aos eixos que expomos em vez
de ser CSS livre. A ultima e uma troca deliberada, discutida na
[ADR-0004](0004-temas-declarativos.md).
