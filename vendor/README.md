# Dependencias com alteracao local

O que esta aqui **nao e codigo do Morune**. Sao copias de dependencias com uma
alteracao pontual, ligadas pelo `[patch.crates-io]` no `Cargo.toml` da raiz.

Cada alteracao e marcada no proprio codigo com o comentario
`// ALTERADO PELO MORUNE`, para que ninguem leia a copia achando que e original.

Manter copia custa: toda atualizacao da dependencia exige refazer a alteracao.
Por isso so entra aqui o que nao tem como ser resolvido de fora.

---

## librespot-core 0.8.0

Origem: <https://github.com/librespot-org/librespot>, licenca MIT, compativel
com a do Morune.

### O que muda

Um metodo, `Session::check_catalogue` em `src/session.rs`.

### Por que

O original:

```rust
fn check_catalogue(attributes: &UserAttributes) {
    if let Some(account_type) = attributes.get("type") {
        if account_type != "premium" {
            error!("librespot does not support {account_type:?} accounts.");
            info!("Please support Spotify and your artists and sign up for a premium account.");

            // TODO: logout instead of exiting
            exit(1);
        }
    }
}
```

`exit(1)` encerra o processo inteiro. Quem chama esse metodo e o manipulador do
pacote de produto, que chega logo depois da autenticacao -- ou seja, uma conta
gratuita fazia a janela do Morune **desaparecer da tela**, sem mensagem, sem
erro para tratar e sem nada no log.

O Morune e aberto: qualquer pessoa baixa e clica em "Entrar", e a maioria das
contas do Spotify e gratuita. O comportamento original transformava o primeiro
contato dessas pessoas com o aplicativo num sumico inexplicavel.

### Por que nao foi resolvido de fora

Foram medidas e descartadas, nesta ordem:

1. **Perguntar o plano ao `/v1/me` antes de entregar a credencial.** Era a
   solucao ate 19/08/2026, e caiu junto com o Web API -- ver
   [docs/HANDOFF.md](../docs/HANDOFF.md). O `api.spotify.com` responde 429 a
   qualquer token deste client ID.
2. **Ler `session.get_user_attribute("type")` depois de conectar.** O valor
   aparece em cerca de 200 ms, mas tarde demais: quem o entrega e exatamente o
   pacote cujo manipulador chama `check_catalogue`. Quando da para ler, a
   decisao de encerrar ja foi tomada.
3. **Tratar o erro.** Nao ha erro. `exit` nao retorna.

### O que passou a acontecer

O plano so e registrado no log. Quem decide e o Morune, que le o atributo depois
de conectar e explica a situacao em vez de sumir. A reproducao falha sozinha,
como ja falharia.

### Um aviso que passou a aparecer no build

Como copia local, a librespot deixou de ser dependencia baixada e passou a ser
compilada como codigo do projeto -- e o `cargo` agora mostra os avisos dela:

```text
warning: this lint expectation is unfulfilled
  --> vendor\librespot-core\src\authentication.rs:69:14
```

E aviso da propria librespot, sem relacao com a alteracao acima, e nao quebra o
build. Nao foi silenciado de proposito: mexer no codigo copiado alem do
estritamente necessario torna a proxima atualizacao mais dificil de conferir.

### Ao atualizar a librespot

Refazer a alteracao. O metodo e curto e o comentario `// ALTERADO PELO MORUNE`
marca o lugar. Se um dia o projeto original trocar o `exit` por erro -- o
proprio codigo tem um `TODO` dizendo que deveria --, esta copia deixa de ser
necessaria e some daqui.
