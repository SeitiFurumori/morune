# Roteiro

Ordenado por dependencia, nao por vontade. Uma camada dependente so comeca
depois que o contrato dela para de mudar.

```
Arquitetura  [feito]
├─ Core
│  ├─ Config              [feito]
│  ├─ Contratos           [feito]
│  ├─ Fila / shuffle      [feito]
│  └─ Integracao Spotify  [ciclo 2, verificada]
├─ UI
│  ├─ Componentes         [feito]
│  ├─ Navegacao           [feito]
│  └─ Telas de conteudo   [ciclo 2, ligadas e com detalhe]
├─ Customizacao
│  ├─ Esquema de tema     [feito]
│  ├─ Carregador          [feito]
│  ├─ Import/export       [feito]
│  └─ Recarga a quente    [ciclo 3, crate pronta, falta ligar]
└─ Qualidade
   ├─ Testes              [feito, 224]
   ├─ Benchmarks          [feito, tools/measure.ps1]
   ├─ Seguranca           [feito para temas e credenciais]
   └─ Empacotamento       [feito, instalador NSIS de 3,83 MB]
```

---

## Ciclo 1 — MVP executavel `[concluido]`

- workspace, contratos do core, fila com testes;
- motor de customizacao completo com pacotes e validacao de seguranca;
- interface nativa com todo pixel derivado do tema;
- tres temas trocaveis em execucao, um deles derivado;
- configuracao persistente, cofre de credenciais do Windows;
- medicao real de tamanho, startup e memoria;
- captura visual por tema;
- fechar para a bandeja mantendo o aplicativo vivo, com menu de bandeja e
  verificacao automatizada (`tools/verify-tray.ps1`);
- instalador `.exe` unico com escolha de disco, sem UAC, com desinstalador
  limpo e verificacao de que o binario roda isolado.

## Ciclo 2 — Reproducao `[concluido; falta reverificacao final]`

O ciclo que transforma isto num player.

1. `[verificado]` **`morune-spotify`**: `Authenticator`, `PlaybackEngine`,
   `Catalog` e `Library` sobre librespot 0.8.
2. `[verificado]` **Login OAuth PKCE**, sem client secret, com o token indo direto
   para o Gerenciador de Credenciais. Restauracao de sessao na abertura.
3. `[verificado]` **Reproducao real**: carregar, tocar, pausar, buscar posicao,
   volume, avanco automatico ligado a `Queue::next`.
4. `[verificado]` **Busca e biblioteca** ligadas as telas que ja existem, e um
   Inicio com cinco prateleiras. A busca cobre faixas, albuns, artistas e
   playlists. O coracao sincroniza com as Musicas curtidas do Spotify e so
   muda visualmente depois da confirmacao remota. As playlists que o Spotify monta para a conta
   vem pelo protocolo interno, porque o Web API deixou de entrega-las em 2024 —
   ver [docs/HANDOFF.md](docs/HANDOFF.md).
5. `[escrito]` **Radio e autoplay configuravel** pelo caminho interno, anexando
   recomendacoes ao contexto sem apagar historico nem repetir faixas. Falta a
   rodada final contra a conta real.
6. `[verificado]` **Capas**: download, cache em disco de 48 MB com descarte por
   LRU e escolha pelo tamanho de exibicao via `ImageSet::best_for_width`.
7. `[verificado]` **Fundacao de UX e acessibilidade**: arvore AccessKit exposta
   ao Windows, foco e teclado nos controles customizados, estados vazios com
   proxima acao e grades adaptativas. Evidencias e pendencias em
   [UX_AUDIT.md](UX_AUDIT.md).
8. `[verificado]` **Fila gerenciavel**: insercoes manuais separadas do contexto,
   com tocar a seguir, adicionar ao fim, mover, remover e limpar.
9. `[aberto]` **Medir o que importa**: CPU e GPU em segundo plano com musica
   tocando, com um jogo em tela cheia rodando junto. E o criterio de desempenho
   do produto e nunca foi medido — ver [PERFORMANCE.md](PERFORMANCE.md). RAM
   entra como teto de crescimento, nao como meta de vitrine.

`[escrito]` quer dizer: compila, tem teste de unidade e clippy limpo, mas o
recurso novo ainda nao passou pela conta real. O restante do ciclo foi
verificado em 19/08/2026.

O roteiro de reverificacao esta em [docs/HANDOFF.md](docs/HANDOFF.md).

Risco conhecido: librespot 0.8 exige `vergen` fixado em 9.0.x no `Cargo.lock`
— ver [ADR-0002](docs/adr/0002-toolchain-windows-gnu.md).

## Ciclo 3 — Developer Mode e customizacao viva

1. **Recarga a quente** ligada a interface: editar `theme.toml` e ver a mudanca
   sem reabrir. A crate ja tem o observador com agrupamento de eventos.
2. **Sobreposicao de performance**: quadros por segundo, tempo de quadro, RAM.
3. **Ids de componente** visiveis, para quem escreve tema saber o que esta
   ajustando.
4. **Painel de diagnosticos** com o resultado do `sanitize` do tema atual.
5. **Icones por tema**, vindos de `assets/`.
6. **Fontes empacotadas** registradas de verdade (`bundled_font`).

## Ciclo 4 — Produto

1. **Teclas de midia** `[feito]`; integracao completa de metadados/capa com o
   painel de reproducao do Windows (SMTC) `[pendente]`.
2. **Minimizar para a bandeja**, alem de fechar. Fica pendente porque o Slint
   nao expoe o evento de minimizacao da janela; exige alcançar o `HWND` pelo
   handle nativo, o que so vale a pena junto com as teclas de midia.
3. **Notificacao de bandeja na primeira vez** `[feito]`: explica onde o Morune
   ficou e como abrir/sair, sem repetir nas proximas vezes.
4. **Mini-player** `[feito]`: modo compacto reversivel, com estado essencial do
   player e restauracao exata da janela anterior.
5. **DPI**: smoke test concluido em 100%, 125%, 150% e 200%; falta inspecao
   visual em cada escala e monitor secundario com escala diferente.
6. **Atalhos de teclado**: fundacao, referencia `Ctrl+/`, tooltips e teclas de
   midia prontos; falta a ultima rodada humana teclado-only em todos os fluxos.
7. **Inicializar com o Windows** `[feito]`: escolha reversivel no instalador e
   nas Configuracoes, por usuario e sem UAC; inicio automatico fica na bandeja.
8. **Assinatura de codigo** do instalador. Sem ela o SmartScreen avisa em toda
   instalacao, o que e o maior atrito restante para um usuario real.
9. **Atualizacao automatica**, ou pelo menos aviso de versao nova.
10. **Migrar para MSVC** como alvo de release: binarios menores e alvo com mais
   rodagem no Windows.

## Ciclo 5 — Extensibilidade

1. **API de plugins**, com o mesmo criterio de seguranca dos temas: capacidade
   explicita, sem acesso implicito a disco, rede ou credenciais.
2. **Layout arbitrario** via `slint-interpreter`, atras do Developer Mode e com
   aviso claro de que sai do formato puramente declarativo.
3. **Repositorio de temas** da comunidade.
4. **Outros provedores** de musica, aproveitando que os ids ja carregam
   provedor e que nenhum contrato assume Spotify.

---

## Fora de escopo

- Linux e macOS. O core e portatil, mas o produto e Windows, e prometer tres
  plataformas com uma pessoa mantendo e como se perde qualidade nas tres.
- Baixar musica. Nao e o proposito e complica a relacao com o provedor.
- Telemetria. Nao ha, e nao vai haver.
- Editor visual de temas. TOML mais recarga a quente resolve o mesmo problema
  com uma fracao do custo.
