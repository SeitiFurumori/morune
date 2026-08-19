# ADR-0005 — Morune sob MIT, Slint sob a licenca royalty-free

Data: 2026-08-18
Status: aceito

## Contexto

Ate hoje o `Cargo.toml` declarava `MIT OR Apache-2.0` e nao havia nenhum arquivo
de licenca no repositorio. Duas coisas estavam erradas nisso, e a segunda so
apareceu ao montar os avisos de terceiros.

**A primeira:** sem arquivo de licenca, o padrao legal e "todos os direitos
reservados". Codigo visivel nao e codigo liberado, e publicar assim seria o pior
dos dois mundos — mostrar tudo e nao deixar ninguem usar.

**A segunda:** o Slint nao e permissivo. Ele e distribuido sob
`GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR LicenseRef-Slint-Software-3.0`,
e o Morune linka o Slint dentro do executavel. Escolher uma das tres nao e
opcional: sem escolha explicita, a licenca do aplicativo fica indefinida, e a
opcao que vale por omissao seria a mais restritiva.

## Alternativas consideradas

**GPL-3.0-only para tudo.** Coerente e sem custo de atribuicao: o Slint permite,
e o Morune inteiro vira copyleft. Em troca, qualquer pessoa que distribua uma
versao modificada e obrigada a abrir o codigo dela. Isso protege o trabalho, mas
fecha a porta para uso comercial e e mais pesado de conviver do que o projeto
precisa hoje.

**MIT com Slint sob a royalty-free.** O Slint permite explicitamente que a
aplicacao seja MIT, ou ate proprietaria, e nao cobra royalties. A contrapartida
e atribuicao visivel: o widget `AboutSlint` numa tela "Sobre" alcancavel pelo
menu principal, **ou** o badge de atribuicao numa pagina publica de download.

**MIT com a licenca comercial do Slint.** Resolveria a atribuicao pagando. Nao
faz sentido: o projeto e aberto e nao tem receita.

## Decisao

O Morune e **MIT**, com o texto em `LICENSE`.

O Slint e usado sob a **`LicenseRef-Slint-Royalty-free-2.0`**, e a atribuicao e
cumprida pelo **badge na pagina publica de download** — hoje o README do
repositorio, que e de onde o instalador vai ser baixado.

O widget `AboutSlint` foi recusado por um motivo de arquitetura, nao de gosto:
ele vem de `std-widgets`, e o projeto nao usa a biblioteca de widgets de
proposito — a interface e construida sobre primitivas para que todo pixel venha
do tema (ver [ADR-0004](0004-temas-declarativos.md)). Trazer a biblioteca
inteira para exibir um selo contrariaria isso e ainda entregaria um widget que
nao segue o tema ativo.

## Consequencias

**A atribuicao passa a ser condicao de publicar.** Enquanto nada e distribuido,
nao ha o que atribuir. No momento em que o repositorio for publicado ou um
instalador for entregue a alguem, o badge precisa estar na pagina de download.
Se um dia o Morune for distribuido por um canal sem pagina — uma loja, um link
direto — a atribuicao volta a exigir a tela "Sobre", e a decisao acima muda.

**A licenca royalty-free nao vale para sistema embarcado.** Nao e o caso hoje e
nao esta no roteiro, mas fecha essa porta enquanto valer esta decisao.

**O binario nao e MIT por inteiro, e o repositorio precisa dizer isso.** O
codigo do Morune e MIT; o executavel carrega 339 bibliotecas com licencas
proprias, e varias delas exigem que o aviso de copyright acompanhe a
distribuicao. Dai o `THIRD-PARTY-LICENSES.txt`, gerado por `tools/licenses.ps1`
e instalado junto do aplicativo.
