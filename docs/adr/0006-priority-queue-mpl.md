# ADR-0006 — `priority-queue` sob MPL-2.0

Data: 2026-08-20
Status: aceito

## Contexto

`librespot-core` 0.8 passou a trazer `priority-queue` 2.7.0 como dependencia
transitiva. O manifesto oferece duas licencas alternativas:
`LGPL-3.0-or-later OR MPL-2.0`. O guarda de licencas interrompeu o build antes
que essa escolha fosse feita implicitamente.

O Morune nao usa a crate diretamente e nao modifica nenhum arquivo dela. Ela
entra no mesmo executavel por meio da librespot.

## Alternativas consideradas

**LGPL-3.0-or-later.** Permitiria o uso, mas introduziria obrigacoes ligadas a
uma biblioteca copyleft e tornaria a distribuicao estatica mais dificil de
explicar e manter. Nao ha beneficio para o Morune em escolher a alternativa
mais restritiva.

**MPL-2.0.** O copyleft e por arquivo. A secao 3.3 permite distribuir um
"Larger Work" sob termos diferentes desde que as obrigacoes continuem sendo
cumpridas para o software coberto. A FAQ oficial da Mozilla confirma que a MPL
pode ser combinada, inclusive por link estatico, com codigo sob outras
licencas.

Fontes consultadas em 20/08/2026:

- <https://www.mozilla.org/en-US/MPL/2.0/>
- <https://www.mozilla.org/en-US/MPL/2.0/FAQ/>

## Decisao

`priority-queue` 2.7.0 e usado sob **MPL-2.0**, e nao sob LGPL.

O codigo do Morune continua MIT. Os arquivos da crate permanecem MPL-2.0 e nao
foram modificados.

## Consequencias

- `THIRD-PARTY-LICENSES.txt` inclui o texto da MPL-2.0, a escolha feita e o
  endereco do codigo-fonte exato da versao distribuida:
  <https://crates.io/crates/priority-queue/2.7.0>.
- Se a crate for modificada, os arquivos modificados continuam MPL-2.0 e seu
  codigo-fonte precisa ser disponibilizado aos destinatarios do executavel.
- Uma atualizacao de versao ou mudanca da expressao de licenca exige nova
  revisao; a entrada no guarda nao autoriza outra crate nem outra decisao.
