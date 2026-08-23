# Direitos em temas

O que se pode e o que nao se pode fazer ao criar um tema do Morune inspirado na
aparencia de outro produto -- em especial o "Liquid Glass" da Apple, que e o
pedido mais comum.

> Isto nao e parecer juridico. E um levantamento das fontes primarias, feito
> para o projeto tomar decisoes informadas. Para uso comercial, confirme com
> alguem qualificado.

---

## Resumo

| | |
|---|---|
| Imitar a **aparencia** (vidro translucido, desfoque, cantos, brilho na borda) | **Pode.** |
| Usar as **fontes** SF Pro / SF Mono / SF Symbols | **Nao pode.** |
| Empacotar qualquer arquivo da Apple num `.musicpack` | **Nao pode.** |
| Chamar o tema de "Liquid Glass", "Aqua", "iOS", "macOS" | **Nao faca.** |
| Copiar uma tela especifica da Apple pixel a pixel | **Zona de risco.** |

A linha que separa as duas colunas e simples: **arquivo e nome sao deles;
tecnica e estilo nao sao de ninguem.**

---

## 1. A aparencia em si nao e protegida

Direito autoral protege **expressao concreta** -- codigo, imagens, arquivos --,
nao a ideia por tras nem uma tecnica de renderizacao. "Superficie translucida
com desfoque do que esta atras, borda clara e cantos generosos" e um estilo, e
estilo nao se registra.

E ha muita anterioridade: vidro fosco em interface e mais velho que a Apple
nesse formato. Windows Aero (2006), Acrylic e Mica do Windows 11, KDE, GNOME, e
uma decada inteira de "glassmorphism" na web. O proprio efeito que o Morune usa
vem do **DWM do Windows**, chamado pela API publica que existe exatamente para
isso -- nao ha nada da Apple no caminho.

## 2. O que a licenca da Apple diz, palavra por palavra

A [licenca dos Apple Design Resources][adr] e o documento que rege as fontes SF,
os SF Symbols e os arquivos de interface. Os trechos que importam:

> "you are granted a limited, non-transferable, non-exclusive license to use the
> Apple Design Resources **solely for creating mock-ups of user interfaces
> designed for use in software products that run only on** Apple's macOS, iOS,
> watchOS, tvOS, and/or visionOS operating system software"

> "The grants set forth in this License do not permit you to, and you agree not
> to, install, use or run the Apple Design Resources for the purpose of creating
> mock-ups of user interfaces to be used in software products running on **any
> non-Apple operating system software**."

> "**You may not embed the Apple Design Resources in any software programs or
> other products.**"

Tres consequencias diretas para o Morune:

1. **As fontes SF estao fora.** O Morune roda em Windows. Nem para maquete.
2. **Nada de empacotar.** A proibicao de embutir e absoluta, independente de
   plataforma. Um `.musicpack` com SF Pro dentro viola a licenca, e como o
   Morune e MIT e os pacotes circulam, quem distribui responde.
3. **SF Symbols seguem a mesma regra.** Nao sao substitutos de icone validos em
   `assets/icons/`.

**O que a licenca nao diz** e tao relevante quanto: procurei por clausulas sobre
*imitar*, *trade dress* ou *semelhanca confusa* e **nao existem**. A licenca
restringe **os arquivos**, nao a linguagem visual. Quem nunca baixou os Design
Resources nao esta sequer vinculado a ela.

## 3. Patente de desenho: o que realmente cobre

A Apple registra desenhos de interface -- a mais conhecida e a
[USD604,305][d604], que esteve no centro do caso contra a Samsung.

O ponto pratico: **uma patente de desenho protege o que esta desenhado nas
figuras**, exceto o que aparece em linha tracejada. A D604,305 cobria o arranjo
especifico de uma grade de icones, com o retangulo do aparelho em tracejado
justamente para nao fazer parte da reivindicacao.

Ou seja: elas alcancam **uma tela especifica, com aquele arranjo ornamental**.
Nao alcancam "usar translucidez", "usar desfoque" nem "usar cantos arredondados".
O risco existe para quem reproduz uma tela reconhecivel elemento a elemento --
nao para quem faz um tocador de musica com painel de vidro.

## 4. Marca: o unico ponto onde o Morune escorregaria facil

"Liquid Glass", "Aqua", "iOS", "macOS", a maca -- sao marcas. Usar como **nome
de tema** ou como argumento de divulgacao ("o visual da Apple no seu PC") e o
erro mais provavel e o mais facil de evitar.

Descreva o efeito, nao a origem: *vidro*, *cristal*, *fosco*, *translucido*.

## 5. Tipografia: o que usar no lugar

A escolha certa e **[Inter][inter]**, sob a SIL Open Font License. Foi desenhada
para tela pelas mesmas razoes que a SF, tem eixo de tamanho optico como a SF, e
a OFL permite redistribuir -- inclusive dentro de um `.musicpack`.

Sem instalar nada, o **Segoe UI Variable** do Windows 11 ja da um resultado
proximo e e a fonte do sistema onde o Morune roda.

Outras OFL seguras para empacotar: Public Sans, Roboto, Fira Sans, IBM Plex,
JetBrains Mono.

## 6. Regra para quem escreve tema

Antes de por um arquivo em `fonts/` ou `assets/` de um `.musicpack`, uma
pergunta so:

> **Eu tenho direito de redistribuir este arquivo?**

Se a resposta nao for um sim claro, o arquivo nao entra. OFL, Apache e CC0
respondem sim. Fontes de sistema, fontes compradas e qualquer coisa vinda de um
instalador de sistema operacional respondem nao.

---

[adr]: https://developer.apple.com/support/downloads/terms/apple-design-resources/Apple-Design-Resources-License-20230621-English.pdf
[d604]: https://patents.google.com/patent/USD604305
[inter]: https://rsms.me/inter/
