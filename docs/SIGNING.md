# Assinatura de codigo

Por que o Windows avisa ao instalar o Morune, o que resolve isso, quanto custa e
o que ja esta pronto para o dia em que a decisao for tomada.

---

## O que o usuario ve hoje

Ao abrir `Morune-0.1.0-setup.exe` baixado da internet, o Windows mostra uma
tela azul do **SmartScreen**: "O Windows protegeu o computador". O botao de
instalar so aparece depois de clicar em "Mais informacoes", e a linha de editor
diz **Editor desconhecido**.

Nao e um falso positivo nem um bug: e o comportamento correto do Windows para um
executavel que ninguem assinou.

## O que e assinatura de codigo

Um certificado de assinatura e uma identidade verificada. Uma autoridade
certificadora confere quem voce e — documento de pessoa fisica ou registro de
empresa — e emite um certificado. Voce usa esse certificado para carimbar o
`.exe`, e o carimbo prova duas coisas para quem baixa:

1. **quem publicou** — o nome no aviso deixa de ser "desconhecido";
2. **que o arquivo nao foi alterado** desde que saiu da sua maquina.

O carimbo de tempo (*timestamp*) e o que faz a assinatura continuar valida
depois de o certificado expirar. Sem ele, o aplicativo passa a dar aviso no dia
em que o certificado vence.

## O que a assinatura **nao** resolve sozinha

Assinar nao desliga o SmartScreen imediatamente. O SmartScreen decide por
**reputacao**, e a reputacao e construida por assinatura *mais* volume de
instalacoes ao longo do tempo. Um certificado novo comeca sem reputacao: o aviso
continua, mas passa a mostrar o seu nome em vez de "editor desconhecido", e some
sozinho conforme o instalador acumula downloads.

A excecao e o certificado **EV** (Extended Validation), que costuma dar
reputacao imediata. Em troca, exige que a chave privada fique num dispositivo de
hardware ou num servico de assinatura em nuvem — nao existe como arquivo no seu
disco.

## As opcoes

Comprar certificado **nao** e a unica saida, e provavelmente nem e a certa aqui.
Duas das quatro opcoes custam zero.

| Opcao | Custo | Efeito no SmartScreen | Serve para o Morune? |
|---|---|---|---|
| **SignPath Foundation** | gratuito | OV: nome aparece, reputacao acumula | sim, e a primeira a tentar |
| **Microsoft Store** | gratuito | nenhum aviso, a Microsoft assina | sim, mas custa a escolha de disco |
| Certum Open Source | ~US$ 50/ano | OV | sim, se as gratuitas falharem |
| OV comum / EV | ~US$ 219 a 685/ano | OV / EV sem aviso desde o inicio | caro demais para o caso |

**SignPath Foundation.** Assina de graca projetos de codigo aberto que se
qualifiquem, com a chave guardada em HSM e assinatura pelo CI/CD. Os requisitos
sao: licenca aprovada pela OSI sem dupla licenca comercial, nenhum componente
proprietario, projeto mantido, **ja lancado**, documentado e com repositorio
publico. O Morune cumpre a licenca (MIT) e a ausencia de codigo proprietario;
falta publicar e lancar. A analise leva de dias a semanas.

**Microsoft Store.** Desde o fim de 2025 o registro de desenvolvedor individual
e gratuito, e desde maio de 2026 tambem para empresa. Publicando um pacote MSIX,
a Microsoft reassina o pacote: o SmartScreen simplesmente nao aparece, sem
certificado nenhum. O preco nao e em dinheiro — e em controle. MSIX instala onde
o Windows decidir, e a escolha de disco, que e uma funcionalidade declarada do
Morune e a razao de o instalador existir do jeito que existe, deixaria de valer.
Faz sentido como canal **adicional**, nao como substituto do `.exe`.

**Certum Open Source.** Vendido so para pessoa fisica e feito para projeto
comunitario. Exige verificacao de identidade — copia de documento, comprovante
de residencia e a URL de um projeto open source ativo — e reconhecimento
presencial ou em cartorio. E a saida paga mais barata, uma fracao do resto.

**OV comum e EV.** Precos de mercado para quem tem empresa. So o EV mata o aviso
desde o primeiro dia; o OV comum entrega exatamente o mesmo que o SignPath
entrega de graca.

Duas observacoes que valem para qualquer opcao paga: o custo e **anual, nao
unico**, e desde fevereiro/marco de 2026 a validade maxima de um certificado
caiu para cerca de 459 dias — ofertas de dois ou tres anos existem, mas com
reemissao no meio do caminho.

**A ordem sensata:** publicar no GitHub, lancar a `0.1.0`, candidatar-se ao
SignPath. So considerar comprar se a candidatura for recusada.

## O que ja esta pronto

Enquanto nao ha certificado, tres coisas foram feitas para reduzir o dano e para
que a adocao seja trivial depois:

**1. Metadados de versao.** O executavel e o instalador declaram nome, versao,
descricao e copyright. Aparecem nas propriedades do arquivo, no Gerenciador de
Tarefas e na tela do SmartScreen. Isso nao remove o aviso — mas um arquivo
anonimo e pior que um arquivo identificado e nao assinado. Ver `version_info` em
[crates/morune-app/build.rs](../crates/morune-app/build.rs) e o bloco
`VIAddVersionKey` em [installer/morune.nsi](../installer/morune.nsi).

**2. Hash SHA-256 publicado.** `tools/build-installer.ps1` grava um
`Morune-<versao>-setup.exe.sha256` ao lado do instalador. E o unico jeito de
alguem confirmar que baixou o arquivo certo enquanto nao ha assinatura:

```powershell
Get-FileHash .\Morune-0.1.0-setup.exe -Algorithm SHA256
```

**3. Assinatura ja ligada no lugar certo.** `tools/build-installer.ps1` chama
`tools/sign.ps1` duas vezes: no executavel **antes** de empacotar (o instalador
carrega o binario dentro dele, entao assinar depois deixaria o arquivo instalado
sem assinatura) e no instalador depois de gerado. Sem certificado configurado,
`sign.ps1` avisa e devolve sucesso, sem quebrar o build.

## Quando o certificado existir

Nao ha nada para programar. Instale o certificado no repositorio do usuario,
descubra a impressao digital e defina uma variavel de ambiente:

```powershell
Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert | Format-List Subject, Thumbprint
$env:MORUNE_SIGN_THUMBPRINT = "a impressao digital, sem espacos"
.\tools\build-installer.ps1
```

Alternativas: `MORUNE_SIGN_PFX` (e `MORUNE_SIGN_PFX_PASSWORD`) para um arquivo
`.pfx`, e `MORUNE_SIGN_TIMESTAMP` para trocar o servidor de carimbo de tempo.

O `signtool.exe` vem no Windows SDK e **nao esta instalado nesta maquina**.
`sign.ps1` procura por ele no PATH e nas pastas do SDK, e diz claramente se nao
encontrar.

A partir do momento em que publicar sem assinatura passar a ser um erro, e nao o
estado normal, trocar as duas chamadas em `build-installer.ps1` por
`sign.ps1 -Require` faz o build falhar em vez de avisar.

---

## Fontes

Precos e regras mudam; foram conferidos em 18/08/2026.

- [SignPath Foundation, condicoes para projetos open source](https://signpath.org/terms.html)
- [SignPath para a comunidade open source](https://signpath.io/solutions/open-source-community)
- [Registro gratuito para desenvolvedores individuais na Microsoft Store](https://blogs.windows.com/windowsdeveloper/2025/09/10/free-developer-registration-for-individual-developers-on-microsoft-store/)
- [Publicar como empresa, agora com registro gratuito](https://blogs.windows.com/windowsdeveloper/2026/05/07/publish-to-microsoft-store-as-a-company-now-with-free-registration-and-faster-onboarding/)
- [Opcoes de assinatura de codigo para Windows — Microsoft Learn](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options)
- [Certum, certificado open source para desenvolvedor individual](https://shop.certum.eu/code-signing.html)
- [Precos de OV e EV — SSL Dragon](https://www.ssldragon.com/ssl-certificates/code-signing/)
- [Azure Artifact Signing, precos](https://azure.microsoft.com/en-us/pricing/details/artifact-signing/)
