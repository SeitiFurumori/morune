# Texto para pedir um tema

Cole na ferramenta de design junto com os seis HTML. Preencha a linha em branco.

Na Claude Design: template **Blank** (não "UI mockups" -- esse devolve telas
novas, que e o oposto do que se quer), `+` para anexar os seis arquivos, e colar
o texto abaixo.

---

Anexei seis páginas HTML da interface do **Morune**, um tocador de música para
Windows. Comece por `contrato.html` e `modelo.html` — eles definem o que existe
e o que não existe neste produto.

Quero uma proposta de tema:

> **____________________________________________________________**

**Regras que não são negociáveis:**

- Um tema só **troca valores**. Não move, não cria e não remove elemento.
- **Não existe sombra projetada nem gradiente em superfície.** Contraste vem de
  linha (`border`), sempre.
- **Nada pode custar repaint por quadro** — o app toca enquanto a pessoa joga.
- Contraste mínimo **3:1** de `text` sobre `background`, `sidebar_background` e
  `player_background`, e de `text_on_accent` sobre `accent`. Isso é medido no
  carregamento, não é questão de gosto.
- Fonte: só família já instalada no Windows ou de licença livre (SIL OFL). **Um
  nome só** — lista separada por vírgula não funciona aqui.

**Devolva `janela.html` inteiro, com o bloco `:root` reescrito** — todos os
valores novos, as 19 cores e todas as medidas. Assim eu vejo o tema aplicado e
extraio os valores direto, sem tradução.

Se preferir devolver só os valores, use o formato do `modelo.html`
(`theme.toml`). Os dois servem; o `:root` é mais rápido de conferir.

Se algum efeito que você quiser não couber nas chaves disponíveis, **diga qual** —
não improvise por cima do mock.

---

## Ideias para a linha em branco

Descreva **intenção**, não valores — a ferramenta escolhe melhor que um hex
ditado:

- escuro e quente, papel envelhecido, para ouvir à noite
- alto contraste de verdade, para quem enxerga mal
- vidro sobre a área de trabalho, frio e sóbrio
- claro, quase branco, tipografia grande, quase sem cor
- retrô de aparelho de som: âmbar sobre preto, cantos retos
- monocromático, uma cor só e escalas de cinza
