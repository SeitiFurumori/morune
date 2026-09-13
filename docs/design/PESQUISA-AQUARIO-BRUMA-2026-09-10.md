# Aquário e Bruma: identidade, óptica e implementação

## Objetivo e critério de fidelidade

Aquário deve expressar o Frutiger Aero com a riqueza material e fotográfica
do período, adaptada a um player de música utilizável. Bruma deve aproximar-se
do Liquid Glass pela organização espacial, pela óptica e pelo comportamento.
Um tema não é aprovado apenas porque compila ou porque contém transparência.

As decisões abaixo partem de fontes externas primárias, documentação atual
e inspeção do renderizador instalado. Os documentos anteriores do Morune são
histórico de decisões, não evidência de fidelidade. Propriedades copiadas de
uma skill precisam ser confrontadas com as fontes e com a plataforma Windows.

## 1. Frutiger Aero: origem e interpretação

O CARI identifica a estética aproximadamente entre 2005 e 2013, associando
tipografia humanista, superfícies brilhantes, transparência, cores terciárias,
auroras, bokeh e fotografias macro de grama. A categoria retrospectiva abrange
mais que a interface do Windows. O próprio acervo é referência de pesquisa,
não um banco de imagens livres para incorporar ao produto. [1]

A entrevista com Chang N. Suh, da Asadal, é particularmente importante por
trazer o relato de quem produzia e comercializava essas imagens. Ele relaciona
a luminosidade dos azuis ao trabalho para telas RGB, descreve a distribuição
de recursos editáveis em camadas e explica a seleção de estilos pelo uso e
pelas vendas. Essa produção combinava fotografia, objetos, interfaces e
fantasia tecnológica; não se restringia a um conjunto de cores. [2]

Os números sobre participação de mercado na entrevista são declarações do
entrevistado, não medições independentes. Não são necessários para justificar
as escolhas do tema. A contribuição relevante é o processo visual: montar uma
cena reconhecível, com luz coerente e materiais convincentes, usando o meio
luminoso da tela.

### Consequências para Aquário

Uma paisagem precisa ter profundidade: céu distante, horizonte atmosférico,
água em plano intermediário e vegetação próxima. Caustics, reflexos e orvalho
devem acompanhar essa escala. Um círculo translúcido colocado aleatoriamente
não substitui uma gota com volume, reflexo e transmissão do entorno.

O azul luminoso e o verde fresco pertencem sobretudo ao ambiente. O controle
precisa manter uma superfície distinta, um contorno legível e uma resposta
clara. Se toda a interface recebe a mesma saturação, perde-se a relação entre
paisagem e objetos. A identidade deve continuar reconhecível com o wallpaper
escondido: gel ciano, luz superior, retorno inferior, formas suaves e tinta
azul-petróleo.

O guia de ícones da Microsoft para Vista/Windows 7 descreve objetos mais
realistas, luz consistente acima e à esquerda, e simplificação nos menores
tamanhos. Ícones de toolbar permanecem frontais; nem toda ação precisa virar
um objeto 3D. Esta é documentação histórica adequada ao estudo do período,
não uma recomendação de interface contemporânea do Windows. [3]

**Direção proposta:** litoral luminoso autoral; céu, água e grama fotográficos;
cor concentrada na cena e nas ações; áreas de leitura claras; gel com uma
transição especular característica, corpo translúcido e sombra curta. Evitar
um poster cheio de símbolos, gradações violetas, cartões de dashboard genérico
e dezenas de bolhas repetidas.

### Composição da cena

O fundo precisa funcionar em 720×480, 1180×760 e janela maximizada. A parte
central deve oferecer frequências visuais baixas; a periferia pode carregar
textura. O centro do `cover` muda o enquadramento em cada proporção: a posição
de detalhes não pode ser avaliada somente olhando o PNG inteiro.

Preservar legibilidade pintando branco sobre a fotografia inteira até ela
desaparecer também não resolve o tema. O tratamento preferível é localizar
o suporte de leitura no conteúdo e preservar a intensidade da paisagem nas
margens e nos intervalos. O wallpaper é contexto; capas e títulos são conteúdo.

## 2. Liquid Glass: material e sistema

Na apresentação da Apple, Liquid Glass é um material dinâmico que reflete e
refrata o entorno, reagindo ao contexto e à interação. Essa descrição exclui
a equivalência simplista entre vidro e opacidade baixa. [4]

A sessão técnica de design apresenta lensing, alteração da espessura aparente
conforme a dimensão do elemento, luz do conteúdo próximo, adaptação de
luminosidade e separação entre navegação e conteúdo. São propriedades de um
sistema que trabalha em conjunto, não efeitos independentes empilhados. [5]

As HIG distinguem materiais padrão, apropriados para o conteúdo, do Liquid
Glass da camada funcional. A variante regular oferece proteção para texto e
fundos difíceis. A clear é voltada a mídia visualmente rica e pode precisar
de escurecimento localizado. Logo, aplicar clear à sidebar inteira não é uma
tradução automática de “mais próximo do original”. [6]

O guia de adoção amplia a questão para composição: agrupamento funcional de
botões, extensão de conteúdo sob a sidebar, adaptação ao redimensionamento e
origem espacial de menus. O container de efeitos permite coordenar materiais
e transições; sua existência não autoriza concluir que toda linha ou cartão
de conteúdo deveria ganhar vidro. [7]

### Consequências para Bruma

Bruma deve conservar a cor nas capas e no ambiente, deixando a estrutura
predominantemente neutra. O material deve ter uma periferia perceptível,
centro mais tranquilo, sombra com decaimento e geometria coerente com sua
elevação. Sidebar, player e menus podem parecer acima do conteúdo; uma lista
de faixas não deve se transformar em dezenas de lentes concorrentes.

No modo escuro, “sem cor própria” não significa ausência de suporte tonal.
A leitura requer que luz transmitida e brilho sejam limitados. Um popup pode
precisar ser substancialmente mais denso que um pequeno controle de mídia.
O texto permanece opaco: reduzir a opacidade da janela inteira também reduz
a nitidez visual de letras, glifos e capas.

Uma borda branca uniforme faz a superfície parecer contornada. Para transmitir
volume, o reflexo precisa variar com a orientação da borda, com uma resposta
mais forte no encontro com a luz e uma região intermediária mais discreta.
Sombra e refração precisam pertencer à mesma geometria arredondada.

## 3. Óptica: o que distingue os efeitos

| Efeito | Operação | Evidência visual exigida |
|---|---|---|
| Transparência | Combinar foreground e background por alfa | A imagem de trás continua visível |
| Desfoque | Integrar amostras vizinhas | Detalhes perdem definição sem deslocamento coerente |
| Refração | Alterar direção/caminho da luz transmitida | Linhas do fundo mudam de posição conforme a lente |
| Reflexo | Misturar luz refletida pela superfície | O destaque depende da orientação e iluminação |
| Dispersão | Variação óptica por comprimento de onda | Franja discreta em regiões apropriadas |
| Sombra | Atenuação e espalhamento sob o objeto | Separação espacial e espessura percebidas |

O PBRT apresenta a lei de Snell, as equações de Fresnel e a diferença entre
materiais dielétricos e condutores. O índice do vidro costuma estar entre 1,5
e 1,6, enquanto a água fica perto de 1,333. Esses valores explicam a física;
não são parâmetros publicados do shader proprietário da Apple. [8]

Para uma aproximação de interface, uma geometria de retângulo arredondado pode
fornecer a normal da borda. Um campo de deslocamento usa essa normal para
amostrar o fundo. A intensidade cresce na região curva, enquanto o centro
permanece estável. Uma função de Fresnel aproxima a contribuição do reflexo.
O recorte da lente, o campo de deslocamento e a sombra precisam compartilhar
raio e posição; caso contrário, o material denuncia uma colagem.

O teste mais simples usa uma grade ou uma linha diagonal atrás da superfície:
se apenas clareia ou borra, não houve refração. O teste seguinte move a lente:
a distorção deve acompanhar a superfície, mantendo o texto frontal intacto.
O terceiro troca o fundo de branco para preto e para uma capa saturada.

Não há base para afirmar que um conjunto de gradientes replica o algoritmo
da Apple. É possível reproduzir princípios perceptivos sem copiar esse
algoritmo, mas a descrição técnica deve distinguir as duas coisas.

## 4. Movimento e resposta

A sessão Designing Fluid Interfaces associa fluidez a baixa latência,
interrupção e redirecionamento. A forma deve responder durante a interação,
e não somente depois da confirmação. A transferência entre arraste e animação
precisa preservar continuidade. [9]

No Morune isso significa mostrar pressão imediatamente, sem alterar a área
clicável; acompanhar o valor do slider durante o arraste; manter origem e
destino de menus relacionados ao acionador; permitir que o usuário reverta
uma transição. Uma animação longa e suave não corrige um input atrasado.

Bruma pede resposta curta e controlada, sem bounce decorativo a cada hover.
A elasticidade é justificável quando há gesto e velocidade a transferir.
Aquário pode apresentar realce mais luminoso e uma resposta de objeto em gel,
mas não precisa pulsar indefinidamente em repouso.

Os números de damping e response na skill são referências de implementação,
não durações universais a copiar. O `motion` atual do Morune é baseado em
durações Slint; descrevê-lo como um sistema de molas seria incorreto. Molas
reais exigiriam estado de posição e velocidade, além de testes de interrupção.

## 5. Acessibilidade e contraste composto

A WCAG estabelece 4,5:1 para texto comum e permite 3:1 em texto grande segundo
os critérios especificados. Para estes temas, 4,5:1 é o piso de projeto para
texto principal e secundário. Medir somente duas cores hexadecimais ignora
fotografia, alfa, hover e reflexos presentes na tela. [10]

O critério de contraste não textual trata informação necessária a reconhecer
controles e estados. Não exige que toda borda decorativa tenha 3:1 se o
controle já é identificável pelo conteúdo, mas exige atenção ao foco e aos
indicadores funcionais. Isso permite um reflexo fino sem transformar todo
painel em uma caixa de borda forte. [11]

Animações não essenciais devem poder ser desabilitadas. Para a aplicação,
reduzir movimento precisa preservar feedback estático e evitar que um estado
se torne invisível. Transparência reduzida e contraste aumentado são decisões
separadas; não se deve inferir uma a partir da outra sem documentar a escolha.
[12]

### Matriz de validação

| Dimensão | Casos |
|---|---|
| Conteúdo | Home, busca preenchida/vazia, biblioteca, detalhe, fila, configurações |
| Entrada | Mouse, Tab, Enter, menu contextual, arraste de sliders |
| Janela | 720×480, 1180×760, maximizada, sidebar recolhida, mini-player |
| Fundo | Preto, branco, foto luminosa, capa saturada, ausência de imagem |
| Estado | Repouso, hover, foco, pressionado, selecionado, desabilitado |
| Movimento | Padrão, reduzido, gesto interrompido |
| Renderização | GPU disponível, software fallback, DPI elevado |

Capturar um popup aberto não comprova ativação por teclado, execução da ação,
reposicionamento junto às bordas nem fechamento ao clicar fora. A aprovação
anterior do projeto foi mais abrangente que a evidência produzida: isso não
deve ser repetido nesta revisão.

## 6. Diagnóstico do Morune

Esta seção descreve a leitura do código em 10/09/2026, antes desta revisão.

`Material` escolhe folhas bitmap por `gloss`: valores a partir de 0,62 recebem
gel; valores entre 0,20 e 0,62 recebem vidro. O tipo de material e sua força
estão acoplados. `Gloss` e `Frost` têm nomes sugestivos, mas seu preenchimento
é transparente. `Rim` fornece uma faixa radial, não desloca o fundo.

`Vidro` recorta uma cópia borrada do wallpaper. Esse mecanismo tem utilidade
para uma cena estática conhecida, mas não lê capas, texto ou listas atrás da
superfície. A implementação também faz a conta de `cover` independentemente
do modo de encaixe. Portanto, um fundo configurado como contain, center ou
stretch pode não coincidir com sua cópia nas barras.

Bruma não fornece wallpaper. Nesse caso, a cópia borrada fica vazia e o efeito
depende de cores translúcidas e do material de janela do Windows. O Acrylic
da Microsoft espalha o fundo; é uma técnica diferente da refração localizada
que define o objetivo do novo Bruma. [13]

O projeto usa Slint 1.17 e femtovg/OpenGL, com software como reserva. A
documentação expõe notificações antes/depois de desenhar a cena e exige
preservar o estado OpenGL ao integrá-lo. Essa API não deve ser confundida com
um filtro declarativo do backdrop de qualquer elemento. [14]

O pedido de shaders customizados no repositório Slint estava aberto na
consulta. Um exemplo oficial mostra a integração de uma textura OpenGL, mas
uma textura externa não resolve por si a extração do conteúdo intermediário
da árvore visual. [15][16]

### Alternativas para Bruma

| Caminho | O que entrega | Limite/custo |
|---|---|---|
| Cores + bordas + gradientes | Hierarquia e brilho | Não entrega refração |
| Wallpaper amostrado por lente | Refração real da cena conhecida | Não refrata conteúdo vivo atrás do painel |
| Snapshot de janela como textura | Aproximação do quadro anterior | Feedback, atraso, custo de leitura e possível duplicação do texto |
| Camada dedicada de conteúdo + passe óptico GPU | Refração do conteúdo correto antes de desenhar controles | Mudança de renderização, recursos GPU, testes e fallback |
| API Liquid Glass nativa Apple | Material do sistema Apple | Não disponível para a interface Windows/Slint atual |

O caminho de snapshot não é recomendado como base de produção: se a captura
inclui o próprio vidro, o material acumula versões de si mesmo. Ocultar e
mostrar controles para capturar também introduz riscos de cintilação e
acoplamento ao ciclo de pintura. Nunca capturar outros aplicativos para
alimentar uma lente interna.

O máximo de fidelidade está na separação entre plano de conteúdo e plano de
controles, com uma textura da cena correta. Até isso existir, toda refração
limitada ao wallpaper deve ser identificada como tal. Uma melhoria estética
não pode ser apresentada como implementação integral do Liquid Glass.

## 7. Direção de implementação

### Recorte A — contrato óptico

Introduzir `effects.material` com compatibilidade para temas existentes.
Aquário e Bruma passam a declarar a família explicitamente. `gloss` volta a
ser intensidade. Testar deserialização, fallback, independência entre tipo e
força, compilação e preservação dos demais temas.

### Recorte B — Aquário

Nova cena autoral, suportes claros de leitura, gel com luz orientada e
controles cuja cor e espessura permaneçam reconhecíveis. A cena nova deve ser
integrada sem substituir o original até passar a comparação. Testar contraste
composto e enquadramentos; preservar toda a lógica de navegação e reprodução.

### Recorte C — Bruma

Separar visualmente o plano de controles, dar às superfícies raio, elevação,
espessura e bordas coerentes. Retirar material de linhas de conteúdo. Validar
uma lente sobre fonte conhecida com padrões ópticos antes de integrá-la.
Documentar explicitamente a cobertura da refração e o fallback.

### Recorte D — refinamento e evidência

Validar cada tema nas telas e entradas listadas, verificar idle/performance,
executar build e suíte técnica e registrar quais cenários de fato passaram.
Uma falha em qualquer recorte interrompe o avanço dependente até ser resolvida.
O mapa arquitetural muda somente quando contratos ou módulos mudarem.

## 8. Referências

Consultadas em 10/09/2026. Datas abaixo só são indicadas quando identificáveis
na publicação. Conteúdo de documentação pode evoluir após esta consulta.

1. CARI. [Frutiger Aero](https://cari.institute/aesthetics/frutiger-aero). Taxonomia e motivos visuais.
2. Yunseon Yang; introdução de Sofi Xian. [Frutiger Aero and the Story of Asadal: Interview](https://sofixian.substack.com/p/frutiger-aero-and-the-story-of-asadal). Entrevista com Chang N. Suh, 2026. Relato de produção e distribuição; não pesquisa quantitativa independente.
3. Microsoft. [Icons (Design basics)](https://learn.microsoft.com/en-us/windows/win32/uxguide/vis-icons). Guia histórico Vista/Windows 7: luz, perspectiva e escala.
4. Apple. [Apple introduces a delightful and elegant new software design](https://www.apple.com/newsroom/2025/06/apple-introduces-a-delightful-and-elegant-new-software-design/), 09/06/2025. Intenção pública do material.
5. Apple Developer. [Meet Liquid Glass](https://developer.apple.com/videos/play/wwdc2025/219/), WWDC25. Óptica, adaptividade e princípios.
6. Apple Developer. [Materials — Human Interface Guidelines](https://developer.apple.com/design/human-interface-guidelines/materials). Texto lido também pelo JSON oficial, devido à página depender de JavaScript.
7. Apple Developer. [Adopting Liquid Glass](https://developer.apple.com/documentation/technologyoverviews/adopting-liquid-glass). Texto lido na versão Markdown oficial. Organização, safe areas, agrupamento e extensão de conteúdo.
8. Matt Pharr, Wenzel Jakob e Greg Humphreys. [Physically Based Rendering, 4ª edição, §9.3](https://pbr-book.org/4ed/Reflection_Models/Specular_Reflection_and_Transmission). Refração, Fresnel e dispersão.
9. Apple Developer. [Designing Fluid Interfaces](https://developer.apple.com/videos/play/wwdc2018/803/), WWDC18. Latência, interrupção e continuidade.
10. W3C WAI. [Understanding SC 1.4.3: Contrast (Minimum)](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html). Critérios de contraste textual.
11. W3C WAI. [Understanding SC 1.4.11: Non-text Contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html). Controles, estados e bordas decorativas.
12. W3C WAI. [Understanding SC 2.3.3: Animation from Interactions](https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions.html). Movimento não essencial.
13. Microsoft. [Acrylic material](https://learn.microsoft.com/en-us/windows/apps/design/style/acrylic). Material Windows e sua finalidade.
14. Slint 1.17.1. [RenderingState](https://docs.slint.dev/latest/docs/rust/slint/enum.RenderingState). Ciclo de renderização e estado OpenGL.
15. Slint. [Custom Shader support, issue #10887](https://github.com/slint-ui/slint/issues/10887), aberta em 27/02/2026; aberta na consulta. Evidência de limitação, não garantia de roadmap.
16. Slint. [OpenGL texture example](https://github.com/slint-ui/slint/tree/master/examples/opengl_texture). Integração de textura externa.

## 9. Procedência do novo recurso de Aquário

Uma nova imagem foi gerada pelo recurso integrado de geração, com intenção de
uso como wallpaper autoral do tema, e não como referência copiada do acervo.
O prompt solicitou Frutiger Aero de 2006–2010, litoral luminoso, água turquesa,
céu fotográfico, grama com orvalho na periferia, uma esfera de água, baixa
frequência de detalhes no centro e ausência de texto, marcas e interface.
A imagem original gerada é preservada. Integração e validação são atividades
separadas; gerar um wallpaper não comprova que o tema está concluído.

## 10. Entrega técnica parcial — 11/09/2026

Esta etapa entrega o contrato de material e uma lente experimental sobre
wallpaper, ainda sem ativação nos componentes ou nos temas. Não entrega o
redesign completo nem refração do conteúdo dinâmico da interface.

Arquivos deste recorte (não inclui alterações preexistentes no workspace):

| Arquivo | Motivo |
| --- | --- |
| `crates/morune-theme/src/tokens.rs` | Família óptica explícita, compatibilidade legacy e três testes do contrato. |
| `crates/morune-app/ui/theme.slint` | Família exposta à UI e contrato `LensRegion`/`refract-background`. |
| `crates/morune-app/src/theme_bridge.rs` | Tradução dos tokens e cache renovado ao trocar a cena. |
| `crates/morune-app/src/optics.rs` | Amostragem, deslocamento de borda, máscara arredondada, alfa e cache limitado; cinco testes. |
| `crates/morune-app/src/main.rs` | Registro do módulo óptico. |
| `crates/morune-app/themes/aquario/assets/litoral.png` | Nova cena original gerada; preservada separadamente, ainda não empacotada nem ativada. |
| `docs/THEMING.md` | Documentação do novo token. |
| `PROJECT_MAP.md` | Atualização dos contratos e do módulo novo. |
| Este documento | Pesquisa externa, decisões, limites e rastreabilidade da etapa. |

Testes de `morune-app` e `morune-theme` com hot-reload passaram, incluindo
empacotamento, compatibilidade e os cinco testes ópticos. O teste de escala
inicialmente usava igualdade exata de ponto flutuante; foi corrigido para
tolerância de 0,0001 após observar 99,99999 em vez de 100. O build `ci` passou.
Clippy com `-D warnings`, formatação e `git diff --check` passaram.
Há um aviso preexistente no código
vendorizado de librespot (`unfulfilled_lint_expectations`), fora deste escopo.

Não foram realizados controle do app, screenshots, medição de custo em
redimensionamento ou validação visual. Os testes atuais não comprovam esses
aspectos. Próximo recorte: integrar Aquário e suas superfícies de leitura,
isoladamente; Bruma depende de validar a composição e o custo da lente antes
de ativá-la. Os outros temas continuam no caminho legacy.

## 11. Integração dos dois temas — 11/09/2026

Esta seção sucede o estado parcial da seção 10. Aquário 3.0.0 agora declara
`material = "aero"`, usa `assets/litoral.png`, suporta a leitura em superfícies
claras e usa gel vetorial com reflexo superior e retorno inferior. A área de
conteúdo tem margens próprias, descontadas no cálculo de colunas.

Bruma 2.0.0 declara `material = "liquid"` e usa uma cena SVG original de luz
mineral e dobras suaves. `SceneGlass` aplica a lente na sidebar e no player;
o centro de conteúdo e os realces de lista usam superfícies de cor. O player
ganha recuo externo, a sidebar ganha cantos arredondados e os controles dos
dois temas dão retorno imediato ao pressionar. O efeito Acrylic de sistema
foi desativado em Bruma para evitar sobreposição de dois materiais distintos.

A fonte óptica continua sendo apenas o wallpaper. Não há refração de listas,
capas ou texto que passam atrás dos painéis. O teto da lente passou a 320 px
por lado; os resultados têm cache e não há timer de regeneração. O colapso
da sidebar de Bruma é imediato para não rasterizar uma lente em cada quadro
da animação. O custo real durante resize e a qualidade visual precisam de
verificação no app; limites de alocação não comprovam fluidez.

Arquivos alterados neste recorte:

| Arquivo | Motivo |
| --- | --- |
| `crates/morune-app/themes/aquario/theme.toml` | Ativar Aero e a nova cena, removendo o véu global branco. |
| `crates/morune-app/themes/aquario/manifest.toml` | Versão 3.0.0 para atualização do tema instalado. |
| `crates/morune-app/themes/bruma/theme.toml` | Material Liquid, densidade e contraste, cena e Acrylic desligado. |
| `crates/morune-app/themes/bruma/manifest.toml` | Versão 2.0.0 e descrição do novo tema. |
| `crates/morune-app/themes/bruma/assets/atmosfera.svg` | Nova cena vetorial original, sem dependências externas. |
| `crates/morune-app/src/bundled.rs` | Empacotar assets em subpastas; verificar contraste composto e carregamento da cena de Bruma. |
| `crates/morune-app/src/optics.rs` | Limitar a resolução das lentes a 320 px. |
| `crates/morune-app/ui/components.slint` | Materiais por família, SceneGlass, realce de conteúdo e resposta de pressão. |
| `crates/morune-app/ui/app.slint` | Integrar os materiais, superfícies e espaçamento; atualizar a grade ao trocar a família. |
| `PROJECT_MAP.md` | Registrar SceneGlass e a integração nos temas. |
| Este documento | Evidências, limites e rastreabilidade. |

Problemas encontrados: o executável `target/ci/morune.exe` não pôde ser
substituído (acesso negado pelo Windows). A etapa de Aquário foi validada com
12 testes, build release e Clippy. Na revisão de Bruma, o cálculo conservador
de fundo branco + reflexo + seleção mostrou contraste insuficiente no texto
secundário; a densidade e a tinta foram ajustadas e o cenário virou teste.
O aviso vendorizado de librespot permanece fora do escopo.

Nenhuma captura de tela ou interação com o app foi realizada neste recorte.
Os efeitos ópticos e a organização visual precisam de conferência na janela
real antes de declarar o redesign visualmente aprovado.

### Resultado técnico final

- 235 testes passaram: 143 do app, 84 do motor de temas e 8 de empacotamento;
  dois testes do app permanecem ignorados.
- Clippy com `-D warnings`, formatação e `git diff --check` passaram.
- Build release concluído em 11/09/2026 às 20:18 (horário local), com
  21.078.016 bytes em `target/release/morune.exe`.
- O modo `MORUNE_BINARY_CHECK_FILE` retornou `binary_loaded=ok`, sem criar
  janela nem carregar a sessão pessoal. Evidência local gerada em
  `target/release/theme-final-loader-20260911.txt`.
- Validação visual e medição de fluidez continuam pendentes, respeitando a
  restrição do usuário sobre controle do app e capturas.
