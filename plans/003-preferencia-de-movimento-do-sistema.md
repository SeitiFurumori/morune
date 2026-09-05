# 003 — Respeitar a preferência de movimento do Windows

- **Commit base:** `6b127aa`
- **Fase:** 3, mas **livre**: é Rust, não toca `.slint`, não colide com nada
- **Severidade:** MÉDIA
- **Categoria:** acessibilidade

## O defeito

Nenhuma animação do Morune consulta a preferência de movimento do sistema. O
Windows tem "Mostrar animações no Windows" em Acessibilidade → Efeitos visuais,
e quem desliga isso está pedindo menos movimento em todo aplicativo.

Hoje o único jeito de zerar movimento no Morune é um tema zerar `motion_fast`,
`motion_normal` e `motion_slow`. Isso é escolha do **tema**, não da pessoa, e
ninguém troca de tema para conseguir usar o aplicativo.

## A correção

Ler `SPI_GETCLIENTAREAANIMATION` no arranque e, quando estiver desligado, zerar
os três tokens de movimento — **depois** de o tema ser aplicado, para que a
preferência do sistema ganhe do valor do tema.

```rust
/// A preferencia do sistema ganha do tema: um tema com movimento nao pode
/// devolver movimento a quem pediu ao Windows para nao ter.
#[cfg(windows)]
fn animacao_do_sistema_ligada() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SPI_GETCLIENTAREAANIMATION,
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    };
    let mut ligada = windows::core::BOOL(1);
    // SAFETY: ponteiro para variavel local valida, do tamanho que a API pede.
    let resultado = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some(&mut ligada as *mut _ as *mut core::ffi::c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    // Falha na consulta nao pode desligar o movimento de quem nao pediu isso:
    // sem resposta, vale o comportamento normal.
    resultado.is_err() || ligada.as_bool()
}
```

Aplicar onde o tema chega à interface (`crates/morune-app/src/theme_bridge.rs`,
no mesmo ponto em que `motion_*` é empurrado para o `Theme`), zerando as três
durações. Não criar caminho paralelo: o valor final continua saindo do mesmo
lugar.

Conferir antes se `Win32_UI_WindowsAndMessaging` já está nas features do crate
`windows` em `crates/morune-app/Cargo.toml` — provavelmente está, pelo uso de
`EnumWindows` em `instance.rs`.

## Fora de escopo

- Não reagir a mudanças em tempo de execução (`WM_SETTINGCHANGE`). Ler no
  arranque basta e é o que quase todo aplicativo faz.
- Não inventar um ajuste de movimento nas Configurações do Morune. Se ele deve
  existir, é decisão do Felipe, não consequência deste plano.
- Não zerar movimento por economia de energia nem por qualquer outro sinal. Só
  a preferência de animação.

## Verificação

1. `cargo build -p morune-app`, `clippy` e `cargo test --workspace` limpos.
2. Desligar "Mostrar animações no Windows", abrir o Morune e passar o cursor por
   uma lista: o realce deve trocar sem transição. Religar e confirmar a volta.
3. Confirmar que um tema que já zera `motion_*` continua zerado com a animação
   do sistema **ligada** — a preferência do sistema só tira movimento, nunca
   devolve.
