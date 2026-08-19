# Estado do projeto e proximo passo

Documento de retomada. Escrito em 18/08/2026, ao fim do ciclo 1 e revisto no
mesmo dia, depois de a identidade visual ser aplicada.

Quem chegar aqui numa sessao nova deve ler este arquivo primeiro, depois
[ARCHITECTURE.md](../ARCHITECTURE.md) e [ROADMAP.md](../ROADMAP.md).

---

## Onde o projeto esta

Ciclo 1 concluido e verificado. `0.1.0` compila, roda, instala e desinstala.

| | |
|---|---|
| Instalador | 4,00 MB, `dist/Morune-0.1.0-setup.exe` |
| Executavel | 9,39 MB, `target/release/morune.exe` |
| Startup interno | 18 ms |
| RAM em repouso | 70,6 MB |
| CPU em repouso | 0,00% |
| Testes | 134, todos passando |
| Clippy | limpo com `-D warnings` |

**O que funciona:** interface completa, tres temas trocaveis em execucao,
import/export `.musicpack`, configuracao persistente, cofre de credenciais do
Windows, fechar para a bandeja mantendo o processo vivo, instalador com escolha
de disco, identidade visual em todos os lugares que o Windows mostra o
aplicativo.

**O que nao funciona:** reproducao. Nao ha backend. O aplicativo usa
`NullEngine`, que aceita preferencias e recusa reproducao de forma explicita.
Busca, Inicio e Biblioteca ficam vazios pelo mesmo motivo.

---

## Antes de qualquer comando

O ambiente de build **nao** esta no `PATH` por padrao. Toda sessao comeca com:

```powershell
. .\tools\env.ps1
```

Sem isso o `cargo` nao existe, ou existe e falha ao linkar. Detalhes e o porque
em [ADR-0002](adr/0002-toolchain-windows-gnu.md).

### Armadilha conhecida

`vergen` esta **fixado em 9.0.6** no `Cargo.lock`. A 9.1 quebra o build do
`librespot-core` (conflito de `vergen-lib` entre 0.1 e 9.1 no `build.rs`). Um
`cargo update` desatento reintroduz o problema. Se voltar a acontecer:

```powershell
cargo update -p vergen --precise 9.0.6
```

---

## Proximo passo: ciclo 2, reproducao

E o unico item de alta prioridade em aberto. Ordem sugerida:

1. **Crate `morune-spotify`.** Implementar, nesta ordem, `Authenticator`,
   `PlaybackEngine`, `Catalog` e `Library` sobre librespot 0.8. Os contratos
   estao em `morune-core` e ja foram exercitados contra `NullEngine` — nao
   devem mudar.
2. **Login OAuth PKCE**, sem client secret. Token vai direto para o
   `CredentialStore` do Windows, que ja funciona e ja tem teste de ida e volta
   real ao cofre do sistema.
3. **Reproducao**: carregar, tocar, pausar, buscar posicao, volume, e ligar o
   fim de faixa ao `Queue::next(false)` — o parametro `user_advance` existe
   justamente para isso.
4. **Busca e biblioteca** nas telas que ja existem e ja estao ligadas.
5. **Capas**: download, cache em disco com teto explicito, escolha pelo tamanho
   de exibicao via `ImageSet::best_for_width`.
6. **Ligar a bandeja ao player**: o menu ja mostra faixa atual e tocar/pausar,
   so falta haver o que tocar.
7. **Medir de novo** e atualizar [PERFORMANCE.md](../PERFORMANCE.md).

**Bloqueio:** o login e interativo. Nada do ciclo 2 pode ser declarado
funcionando sem o Felipe fazer login com a conta Premium dele.

---

## Decisoes que dependem do Felipe

Nenhuma delas bloqueia o ciclo 2; todas bloqueiam a publicacao.

**Licenca — resolvida em 18/08/2026.** O Morune e MIT (`LICENSE`), e o Slint e
usado sob a `LicenseRef-Slint-Royalty-free-2.0`, nao sob a GPL, com atribuicao
pelo badge no README. O porque das duas escolhas esta na
[ADR-0005](adr/0005-licenca-e-slint-royalty-free.md). Os avisos das 339
dependencias vao em `THIRD-PARTY-LICENSES.txt`, gerado por `tools/licenses.ps1`
e instalado junto do aplicativo.

**Atencao ao mexer em dependencia.** `tools/licenses.ps1` falha o build se
aparecer uma dependencia copyleft que nao esteja na lista `$revisadas` dele. Nao
contorne adicionando o nome a lista: uma licenca GPL nova pode obrigar o Morune
inteiro a deixar de ser MIT. Decida, escreva um ADR, e so entao adicione.

**Orcamento de RAM — resolvido em 18/08/2026, mudando a pergunta.** O numero
exato de MB nao e criterio. O criterio e **nao atrapalhar quem esta jogando**:
sem roubar quadro, sem acordar a GPU em segundo plano, sem disputar CPU, e sem a
interface travar quando o usuario volta a ela — o defeito do cliente oficial do
Spotify. Os 70,6 MB ficam aceitos como estao.

Duas consequencias praticas, e as duas valem para o ciclo 2:

- o cache de capas nasce com **teto explicito e descarte por LRU**. O que nao
  pode e crescer sem limite; o valor do teto e livre;
- as metricas que passam a mandar sao **CPU e GPU em segundo plano** e a
  ausencia de travamento da interface. Nenhuma delas foi medida ainda, porque
  nenhuma faz sentido sem reproducao real. Ver o criterio em
  [PERFORMANCE.md](../PERFORMANCE.md).

**Assinatura de codigo.** O instalador nao e assinado e o SmartScreen avisa em
toda instalacao. **Nao e questao de dinheiro:** o SignPath Foundation assina de
graca projetos open source que se qualifiquem, e o Morune ja cumpre a licenca
(MIT) e a ausencia de componente proprietario. Falta o que a candidatura exige e
o projeto ainda nao tem: repositorio publico e uma versao lancada. Ou seja, isto
depende de publicar, nao de comprar.

O pipeline ja esta pronto para qualquer caminho: `build-installer.ps1` chama
`tools/sign.ps1` no executavel e no instalador, e basta definir
`MORUNE_SIGN_THUMBPRINT`. Enquanto nao ha assinatura, os dois arquivos ja
declaram nome, versao e copyright, e o build publica um `.sha256` ao lado do
instalador. Opcoes, precos e requisitos em [assinatura.md](assinatura.md).

---

## Divida tecnica conhecida

- **~1 s de inicializacao grafica** no ciclo completo do processo. Nao
  investigado. Vale medir quanto e criacao do contexto OpenGL no driver Radeon
  e quanto e o backend winit, antes de tentar otimizar qualquer coisa.
- **Minimizar para a bandeja** nao existe, so fechar. O Slint nao expoe o evento
  de minimizacao; exige alcancar o `HWND` nativo. Faz sentido junto com as
  teclas de midia, no ciclo 4.
- **Recarga a quente** esta pronta em `morune-theme` (feature `hot-reload`, com
  agrupamento de eventos e filtro de temporarios de editor) mas nao esta ligada
  a interface. Hoje o usuario clica em "Recarregar".
- **`bundled_font`** existe no esquema de tema mas fontes empacotadas nao sao
  registradas. Fontes instaladas no sistema funcionam por `family`.
- **Icones nao sao substituiveis por tema**; os caminhos vetoriais estao na
  interface.
- **Alvo MSVC** continua valendo pelo tamanho do binario, mas deixou de ser
  bloqueante: o binario GNU roda isolado, sem DLLs do MinGW. Atencao: o icone
  do executavel e embutido com `windres`, que so existe na toolchain GNU. Migrar
  para MSVC exige trocar isso por `rc.exe` ou por uma crate de recursos (ver
  `embed_exe_icon` em `crates/morune-app/build.rs`).
- **Icone da janela precisa vir de `@image-url`.** Imagem montada em codigo
  (`Image::from_rgba8`) nao funciona: o backend winit so reaplica o icone quando
  a chave de cache da imagem muda, e imagens criadas em execucao nao tem chave.
  A janela fica sem icone nenhum, sem erro nem aviso. Custou 0,15 MB de binario
  porque puxa o decodificador de PNG do Slint junto.

---

## Ferramentas de verificacao

Todas ja existem e passam. Usar antes de declarar qualquer coisa pronta.

```powershell
cargo test --workspace --features morune-theme/hot-reload
cargo clippy --workspace --all-targets -- -D warnings
.\tools\measure.ps1          # tamanho, startup, RAM, CPU
.\tools\verify-tray.ps1      # fechar nao encerra o processo
.\tools\snapshot.ps1         # cada tema renderizado em PNG
.\tools\build-installer.ps1  # instalador + checagem de isolamento
.\tools\make-icon.ps1        # regera assets/brand/morune.ico a partir dos PNGs
.\tools\sign.ps1 -Path X     # assina; sem certificado, avisa e nao quebra
.\tools\licenses.ps1         # avisos de terceiros + guarda de copyleft
```

`tools/snapshot.ps1` e `MORUNE_START_PAGE=3` permitem fotografar telas
especificas. As capturas saem em `bench-out/`, que nao entra no git.

---

## Regras do projeto que nao estao no codigo

- **Nada e declarado funcionando sem medicao ou teste.** Onde falta, esta dito,
  inclusive no README.
- **A interface nao pode ter valor visual literal.** Cor, tamanho ou duracao
  escrita direto na interface significa um detalhe que o usuario nao consegue
  customizar — isso e bug, nao omissao.
- **Um tema quebrado nunca impede o aplicativo de abrir.** Vale tambem para
  configuracao corrompida e para bandeja indisponivel.
- **Temas sao dados, nunca codigo.** Ver [ADR-0004](adr/0004-temas-declarativos.md).
