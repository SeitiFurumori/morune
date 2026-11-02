# Entrevista de marca do Morune — 01/11/2026

Parte 3 da ordem de 31/10 (app usável → interface → marca). A personalidade já
saiu da [entrevista de identidade](ENTREVISTA-IDENTIDADE-2026-10-31.md):
marcante, vivo e atual, mas limpo, sóbrio, polido e um pouco frio; universal,
não gamer. Entrevistado: Felipe.

## Conversa 1 — nome e símbolo

**O nome.** "Morune" veio de uma tentativa de marca a partir do sobrenome dele
(Furumori / Komori). Ele não gosta: "ficou meio sem significado, mas foi o
melhor que pude".

**O símbolo de hoje** (onda roxa em M, revisão de 29/09). "Satisfeito, pois
estava pior", mas olhar para ele não desperta nada.

**Onde a marca mais aparece para ele.** "Lá embaixo e à esquerda": a barra de
tarefas do Windows. (Confirmar.)

**Marcas que ele admira.** BOSS, Nike, NVIDIA.

### O que isso diz

- **O nome está aberto,** mas trocar tem custo: a v1.0 saiu em 27/09 com esse
  nome (repositório, página de download, atualização automática, pasta de
  configuração de quem já instalou). É decisão dele, com esse custo à vista.
- **O símbolo atual não é ponto de partida,** só referência do que evitar
  (lembra a Deezer e não diz nada).
- **O lugar que manda é o ícone pequeno da barra de tarefas** (16 a 32 px).
  O símbolo precisa funcionar ali antes de funcionar grande.
- **As três marcas têm o mesmo traço:** confiança sem explicação. Preto e
  branco forte (BOSS), um sinal só que se reconhece de longe (o swoosh da Nike),
  um símbolo geométrico com uma cor só, de tecnologia (o olho verde da NVIDIA).
  Nenhuma usa degradê, mascote ou enfeite. Bate com "sóbrio, polido, marcante".

**Aprofundamento.**

- *Nome:* "vamos tentar trocar; se nada der certo, mantém."
- *Onde aparece:* barra de tarefas e dentro do app.
- *O que pega nas três marcas:* "símbolo forte ou identidade de marca forte".
- *Cor:* roxo é a cor favorita dele; "se achar brega, podemos discutir".

### O que isso diz (2)

- **Troca de nome com rede de segurança:** procurar um nome novo; Morune é o
  plano B, não um fracasso.
- **O símbolo tem de aguentar sozinho,** sem o nome do lado, como o swoosh. É o
  que aparece na barra de tarefas e no canto do app.
- **Roxo não é brega, mas é cheio no setor:** Deezer, Twitch, Discord (quase),
  Apple Music já usou. Foi o roxo com degradê que puxou o logo atual para a
  Deezer. Caminhos: um roxo menos comum (mais frio, quase azul, ou mais
  escuro), ou o símbolo em preto e branco com o roxo só de acento. Decidir
  depois de ver.

## Conversa 2 — nome

- *Língua:* depende da pronúncia; nada longo ou complexo. Português pode sair
  brega dependendo da palavra.
- *O que sugere:* nada direto, só um som forte (como Nike, BOSS).
- *Tamanho:* curto; até duas palavras ("tipo Hugo Boss").
- *Nomes de que gosta pelo som:* Discord, LinkedIn, Jason Bourne, Steam.

### O que isso diz

- **Os quatro têm consoante dura e uma sílaba forte** (Dis-CORD, BOURNE,
  STEAM). Nenhum é fofo ou terminado em vogal aberta.
- **Três são palavras reais em inglês** usadas de um jeito novo (discord,
  steam, bourne); o nome não precisa ser inventado, precisa soar firme.
- **Filtro de pronúncia:** tem de soar igual em português e inglês e não
  significar nada estranho em português.

**Leva 1 de nomes** (Dorn, Tarn, Cove, Vesk, Arvo, Kade, Mori, Kiri, Kumo):
Cove, Arvo e Kumo foram os "menos ruins", mas ele não gostou de nenhum. Leva
descartada; pesquisa de disponibilidade cancelada.

**Leva 2** (Torque, Graphite, Undertow, Cobalt, Basalt, Anvil, Static,
Vantage): Graphite e Undertow chegaram mais perto, mas "tô achando meio bobinho
tudo". Ele perguntou como as marcas montam nomes; pesquisa em
[PESQUISA-NAMING-2026-11-01.md](PESQUISA-NAMING-2026-11-01.md). As duas levas
falharam por método: palavras soltas, sem território, sem volume, sem triagem.

## Conversa 3 — símbolo

Estudos em `assets/brand/otomori-estudos/` (folhas 1 a 4).

- *Folha 1* (pinheiro-equalizador, floresta-onda, anéis, árvore-sinal,
  folha-onda, O-floresta): "ficou todos péssimos".
- *Ideia dele:* o kanji 森 no meio, um O em volta, e no O barrinhas de
  equalizador desniveladas.
- *Folha 2:* gostou do 森 de fonte (A); achou feio o desenho das barras.
- *Folha 3:* "Anel que pulsa tá no caminho, achei bem legal."
- *Folha 4:* variações do anel; **a original continua a melhor** para ele.
- Lembrou do NCS (NoCopyrightSounds), que anima um círculo em volta do logo
  com a música: é a mesma ideia do anel pulsar com o som dentro do app.

**Escolhido:** 森 em peso bold, dentro de um anel cuja espessura sobe e desce
só para fora (soma de duas ondas, de 3 e 5 batidas). Falta: desenhar o 森
próprio (o da folha é da Yu Gothic, fonte da Microsoft), versão de 16 px e cor.

**Depois (02/11):** o 森 desenhado por mim em traço reto ficou "bem ruim"
comparado à fonte; Noto Sans JP serviu de base; ele pediu pincel. Tentou pintar
no Krita (Ink-8 Sumi-e, vão branco entre as cerdas) e desistiu: "pega da
internet". Comparadas fontes de pincel OFL (Yuji Syuku, Boku, Mai, Zen
Kurenaido), centralizadas pela tinta depois de ele apontar que a comparação
não era justa. **Escolhida: Yuji Mai.**

**Símbolo:** `assets/brand/otomori-estudos/otomori-simbolo.svg` (森 da Yuji Mai
convertido em desenho por `gerar_simbolo.py`, dentro do anel) e
`otomori-simbolo-pequeno.svg` (só o anel, para 16 e 32 px). PNGs em `png/`.
Falta: cor.

**Tamanho pequeno (02/11):** ele contestou que o 森 virasse borrão ("o Spotify
dá pra ver os 3 traços"). Teste em pixels reais (`teste-pequeno.png`): em 24 e
32 px o 森 se lê; em 16 vira um triângulo (12 traços em ~8 px). Combinado: do
24 px para cima, o símbolo completo; no 16 (bandeja), decidir vendo o
resultado final no Windows entre o anel sozinho e o 森 maior.

## Conversa 4 — cor

- Primeira folha (`cores.png`) com tons no olho; ele cobrou: "Você só tá
  chutando cores né?". Método tirado de `portfolio-v2/pesquisa-cor.md` e da
  referência de marca: separar desenho e cor, fugir da cor do setor, contraste
  calculado, 2–3 caminhos com porquê, poucas cores com papel.
- **Mapa do setor** (`cor-metodo.png`, `cor_metodo.py`): a faixa do roxo
  (235°–272°) é de Discord, Twitch, Deezer e do Morune de hoje. Faixas livres:
  ~300° (orquídea), ~38° (âmbar), ~212° (azul-aço). Tons ajustados para 3:1
  (WCAG 1.4.11) nas barras de tarefas escura (#202020) e clara (#F3F3F3).
- Ele perguntou se as cores têm significado. Psicologia das cores tem pouca
  base (Elliot & Maier 2014; réplica do "vermelho" falhou); o que vale é o
  significado cultural japonês. **Folha `cor-significado.png`:**
  A) íris 菖蒲色 (#763568 → #B55AA1, 313°, faixa livre; roxo = cor da nobreza
  no Japão, tradicionalmente atribuída ao topo dos 12 níveis de 603);
  B) tinta 墨 + selo vermelhão 朱 (o carimbo da caligrafia; o vermelhão cai a
  8° do SoundCloud, mas só como acento pequeno); C) pinheiro 松葉色 (liga ao
  森, mas a 4° da NVIDIA e perto do Spotify).
- Não sabia decidir entre A e B; montei os dois na barra, no app e no site
  (`contexto.png`). **Decisão dele:** gosta mais do jeito NVIDIA (cor de
  assinatura), mas o nome "Otomori" em outra cor que o símbolo incomoda; o
  carimbo ficou ruim, sai. **A marca é preta (e branca no escuro), e dentro do
  app o símbolo muda conforme a cor que o usuário escolhe.** Símbolo e nome
  sempre na mesma cor. O roxo pode ser o destaque padrão do app.

## Conversa 5 — fonte do nome

Nove fontes OFL comparadas com a mesma altura de maiúscula (`nome.png`).
**Escolhida: Outfit** (os "o" circulares ecoam o anel). Assinatura em
`otomori-assinatura.svg` e `otomori-assinatura-frase.svg` (Outfit 600 no nome,
400 na frase), geradas por `gerar_assinatura.py`, com o texto convertido em
desenho; espaçamento conferido contra a fonte no navegador.

## Marca decidida (02/11/2026)

- **Nome:** Otomori. **Frase:** "O som fica. O peso não."
- **Símbolo:** 森 da Yuji Mai dentro do anel que pulsa; do 24 px para cima,
  completo; no 16 px, decidir no Windows entre anel sozinho e 森 maior.
- **Cor:** preto (branco no escuro); no app, segue a cor de destaque da
  pessoa; roxo como destaque padrão. Sem carimbo.
- **Nome escrito:** Outfit.
- **Pendente:** aplicar no app (ícone, instalador, repositório, pasta de
  configuração), ícone da barra trocando de cor com o tema (testar).

**Território escolhido:** "algo entre" mito/figura forte e raiz japonesa da
família, ou seja, mitologia e folclore japonês. Ele gosta de **Ryūjin** (deus
dragão do mar), mas descarta por ser muito usado.
