# Seguranca

## Versões suportadas

O MORU•NE está em alpha. Correções de segurança são publicadas somente na
versão mais recente disponível em
[Releases](https://github.com/SeitiFurumori/morune/releases); versões anteriores
não recebem manutenção retroativa neste estágio.

## Modelo de ameaca

Duas superficies importam neste aplicativo:

1. **Credenciais.** O usuario entrega acesso a uma conta paga. Um vazamento
   aqui e o pior resultado possivel.
2. **Pacotes de tema.** Um `.musicpack` vem da internet, de um repositorio de
   terceiros ou de um amigo. Todo pacote e tratado como hostil.

Tudo abaixo parte dessas duas premissas.

---

## Credenciais

### A senha do Spotify nunca e vista, pedida ou armazenada

O contrato `Authenticator` em `morune-core` **nao tem campo de senha**. Nenhuma
implementacao pode receber uma por ele. O fluxo previsto e OAuth: o usuario
autentica no navegador, no dominio do Spotify, e o aplicativo recebe apenas um
token.

Isso e uma restricao estrutural, nao uma convencao: nao ha onde colocar uma
senha nesse contrato sem alterar o core.

### Tokens ficam no cofre do sistema

`WindowsCredentialStore` grava via `CredWriteW` no Gerenciador de Credenciais do
Windows, com persistencia `CRED_PERSIST_LOCAL_MACHINE`. O segredo fica protegido
pela chave do perfil do usuario. O aplicativo nao escreve token em nenhum
arquivo proprio, nem em `config.toml`, nem em log.

Em plataformas sem cofre, a implementacao cai para armazenamento em memoria: a
sessao nao sobrevive ao fechamento. Perder a conveniencia e preferivel a gravar
um token em texto claro.

### Tokens nao vazam por log

`AccessToken` **nao deriva `Debug`**. A implementacao manual imprime
`<token 43 chars, expira em 3540s>`. Um teste falha se o segredo aparecer na
saida formatada:

```rust
let printed = format!("{t:?}");
assert!(!printed.contains("super-secreto-123"));
```

O acesso ao valor real e por `expose_secret()`, com nome longo de proposito:
chamadas a ele devem saltar aos olhos em revisao.

Tokens sao considerados expirados 60 s antes do prazo, para nao emitir
requisicao com credencial que morre no meio do caminho.

### Nenhum segredo no codigo

Nao ha client secret embutido. O fluxo OAuth previsto e PKCE, que existe
exatamente para clientes publicos que nao conseguem guardar segredo.

---

## Pacotes de tema

### Temas sao dados, nao codigo

Um tema e TOML: cores, numeros, enums e nomes de arquivo. Nao ha script, nao ha
expressao, nao ha ponto de extensao executavel. Um pacote nao consegue ler
arquivo, abrir conexao, iniciar processo nem consumir CPU indefinidamente,
porque nao existe nada nele que execute.

Essa e uma decisao permanente para o formato base, nao uma limitacao
temporaria. Ver [ADR-0004](docs/adr/0004-temas-declarativos.md).

### Travessia de caminho

`pack::sanitize_entry_path` e a **unica** porta de entrada de nomes de arquivo
vindos do ZIP. Recusa:

| Padrao | Exemplo |
|---|---|
| Componente pai | `../evil.toml`, `assets/../../x.toml` |
| Caminho absoluto | `/etc/passwd`, `//servidor/share/x` |
| Prefixo de unidade | `C:/windows/x.toml`, `C:x.toml` |
| Barra invertida | `..\evil.toml`, `assets\x.png` |
| Fluxo alternativo NTFS | qualquer `:` no nome |
| Byte nulo | `a\0b.toml` |
| Nome hostil no Windows | espaco ou ponto no fim: `x.png.`, `x .png ` |
| Nome longo demais | acima de 255 caracteres |

Dois arquivos com o mesmo destino tambem sao recusados: e uma tentativa de
confundir o extrator sobre qual conteudo prevalece.

### Tipos de arquivo

Lista de **permissao**, nunca de bloqueio. Enumerar o que e perigoso e uma
corrida que se perde.

- raiz: apenas `manifest.toml`, `theme.toml`, `layout.toml`, `README.md`,
  `LICENSE.txt`;
- subdiretorios: apenas `assets/` e `fonts/`, com no maximo 4 niveis;
- extensoes: `toml`, `json`, `png`, `jpg`, `jpeg`, `webp`, `svg`, `ttf`, `otf`,
  `woff2`, `md`, `txt`.

Executavel, script, DLL e qualquer coisa fora dessa lista sao recusados antes de
qualquer escrita.

### Bomba de compressao

| Limite | Valor |
|---|---|
| Arquivos por pacote | 512 |
| Tamanho de um arquivo apos descompactar | 8 MiB |
| Tamanho total apos descompactar | 64 MiB |
| Razao de expansao, por arquivo e no total | 200:1 |

TOML e texto raramente passam de 20:1; uma bomba classica passa de 1000:1. O
limite de 200:1 deixa margem larga e ainda barra o ataque.

A copia de cada arquivo tambem e limitada em tempo de extracao
(`take(MAX_FILE_SIZE + 1)`): se o cabecalho do ZIP mentiu sobre o tamanho, o
corte acontece antes de encher o disco.

### Extracao atomica

A extracao acontece numa pasta temporaria irma (`.<id>.importing`) e so e movida
para o destino final depois que tudo foi validado e escrito. Uma falha no meio
apaga a pasta temporaria e deixa o tema anterior intacto — nunca um tema pela
metade instalado.

### Id de tema

O `id` do manifesto vira nome de pasta em disco. Por isso e restrito a
`[a-z0-9_-]`, no maximo 64 caracteres, e precisa bater com o nome da pasta na
descoberta. Um tema cujo `id` diverge do diretorio e ignorado.

---

## Superficie de rede

O aplicativo se conecta aos serviços do Spotify para autenticação, catálogo,
biblioteca, capas e reprodução, usando OAuth com PKCE e librespot. O MORU•NE
não mantém servidor intermediário, não possui telemetria, analytics ou
verificação automática de atualizações neste estágio.

## Superficie de processo

`explorer.exe` e iniciado com um caminho de diretorio para o comando "abrir
pasta de temas". Nenhuma entrada do usuario e concatenada em linha de comando.

## Codigo inseguro

`morune-core` e `morune-theme` sao `#![forbid(unsafe_code)]`.

`morune-app` e `#![deny(unsafe_code)]` — o codigo gerado pelo Slint contem
`allow(unsafe_code)` em trechos proprios, e `forbid` nao poderia ser suspenso
nem por ele. O unico `unsafe` do nosso lado esta em `snapshot.rs`, atras da
feature `snapshot`, que nao entra no binario de release, e e uma reinterpretacao
de `&[Rgba8Pixel]` como `&[u8]` com justificativa no local.

`morune-storage` usa `unsafe` apenas nas chamadas a `CredWriteW`, `CredReadW` e
`CredDeleteW`, cada uma com nota de seguranca explicando por que o ponteiro e
valido.

---

## Reportar uma falha

Não publique vulnerabilidades em uma issue. Use o formulário privado
[Report a vulnerability](https://github.com/SeitiFurumori/morune/security/advisories/new)
na aba **Security** do repositório e inclua versão afetada, impacto e passos de
reprodução. Não anexe tokens, credenciais reais ou dados pessoais.

Um recebimento será confirmado assim que possível. A correção e a divulgação
serão coordenadas pelo advisory antes que detalhes exploráveis se tornem
públicos.
