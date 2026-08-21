# Contribuindo com o MORU•NE

Obrigado por ajudar a construir um cliente de música leve, claro e livre para
ser personalizado. Contribuições de código, documentação, acessibilidade,
design e testes são bem-vindas.

## Antes de começar

- Consulte as [issues abertas](https://github.com/SeitiFurumori/morune/issues)
  para evitar trabalho duplicado.
- Para uma mudança grande, abra uma issue primeiro e descreva o problema que
  ela resolve. Isso permite alinhar produto e arquitetura antes da implementação.
- Preserve a premissa **Your music. Your way.**: a experiência básica deve ser
  simples; recursos avançados aparecem progressivamente.

## Ambiente de desenvolvimento

O projeto exige Windows, Rust 1.92+ e MinGW-w64 no `PATH`.

```powershell
git clone https://github.com/SeitiFurumori/morune.git
cd morune
. .\tools\env.ps1
cargo build -p morune-app
```

## Fluxo recomendado

1. Crie uma branch curta e descritiva a partir de `main`.
2. Faça mudanças pequenas e focadas.
3. Inclua testes para comportamento novo ou corrigido.
4. Atualize a documentação quando o comportamento público mudar.
5. Abra um pull request explicando problema, solução e como você verificou.

## Verificação obrigatória

Execute antes de enviar:

```powershell
cargo fmt --all -- --check
cargo test --workspace --features morune-theme/hot-reload
cargo clippy --workspace --all-targets -- -D warnings
```

O workflow `.github/workflows/ci.yml` roda os três em cada pull request e em
cada push para `main`, no mesmo alvo Windows GNU do instalador publicado. Ele
usa o perfil `ci` do `Cargo.toml`, que só existe para compilar mais rápido em
runner — localmente o perfil padrão continua sendo o certo.

Clippy e os testes rodam na 1.92, a versão mínima que o projeto declara. A
formatação roda na `stable`, porque o rustfmt muda de opinião entre versões e
o CI precisa concordar com o rustfmt da sua máquina.

Mudanças visuais também devem ser verificadas com teclado, redimensionamento e
escala do Windows. Não considere uma alteração correta apenas porque compila.

## Commits e pull requests

Use mensagens diretas no imperativo, preferencialmente no formato:

```text
feat: add progressive library loading
fix: preserve queue after reconnecting
docs: clarify unsigned installer warning
```

Um pull request deve manter um único objetivo. Inclua capturas de tela em
mudanças visuais e destaque limitações ou decisões que mereçam um ADR.

## Arquitetura e segurança

- A lógica do Spotify não deve acessar a interface diretamente; use os
  contratos de `morune-core`.
- Temas são declarativos e nunca podem executar código.
- Nunca registre tokens, credenciais ou dados pessoais em logs, fixtures ou
  commits.

Leia [Arquitetura](docs/ARCHITECTURE.md), [Segurança](SECURITY.md) e as
[decisões arquiteturais](docs/adr/) antes de alterar essas áreas.

Ao contribuir, você concorda que sua contribuição será distribuída sob a
[licença MIT](LICENSE) do projeto.

