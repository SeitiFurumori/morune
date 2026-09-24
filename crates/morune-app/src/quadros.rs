//! Medidor de quadros, ligado so por `MORUNE_QUADROS=1`.
//!
//! **Por que existe.** "A rolagem da travadinhas" e sintoma, nao causa: pode
//! ser quadro que demora a desenhar, tarefa que prende a thread da interface ou
//! quadro que sai no ritmo errado. Sem numero, qualquer correcao e palpite.
//!
//! Registra no log, a cada segundo com desenho: quantos quadros sairam, o pior
//! intervalo entre dois deles, o tempo de desenho e quantos intervalos passaram
//! de 25 ms (o que se ve como tranco). E, a qualquer momento, cada tarefa da
//! thread da interface que passou de 4 ms.
//!
//! Para medir rolagem, mande `WM_MOUSEWHEEL` para a janela
//! (`bench-out/medir-rolagem.ps1`). Um evento de roda disparado de dentro do
//! proprio app, por temporizador, mantem o laco acordado e mede outra coisa --
//! foi o que escondeu a causa em 24/09/2026 (ver `docs/PERFORMANCE.md`).
//!
//! Desligado, custa uma leitura de variavel.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::ui::AppWindow;

/// `true` quando `MORUNE_QUADROS` esta ligado. Lido uma vez.
pub fn medindo() -> bool {
    static LIGADO: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *LIGADO.get_or_init(|| std::env::var_os("MORUNE_QUADROS").is_some())
}

/// Registra uma tarefa da thread da interface que passou de 4 ms -- o que
/// rouba a vez do quadro seguinte.
pub fn tarefa(nome: &'static str, inicio: Instant) {
    if medindo() {
        let gasto = inicio.elapsed();
        if gasto > Duration::from_millis(4) {
            tracing::info!(
                tarefa = nome,
                ms = gasto.as_secs_f32() * 1000.0,
                "tarefa longa na interface"
            );
        }
    }
}

/// Mede uma tarefa do comeco ao fim do escopo. Ver [`tarefa`].
pub struct Cronometro(&'static str, Instant);

impl Cronometro {
    pub fn new(nome: &'static str) -> Self {
        Self(nome, Instant::now())
    }
}

impl Drop for Cronometro {
    fn drop(&mut self) {
        tarefa(self.0, self.1);
    }
}

#[derive(Default)]
struct Janela {
    quadros: u32,
    ultimo_fim: Option<Instant>,
    pior_intervalo: Duration,
    inicio_desenho: Option<Instant>,
    desenho_total: Duration,
    desenho_max: Duration,
    trancos: u32,
}

/// Liga o medidor se pedido. Devolve o timer do relatorio, que precisa viver.
pub fn instalar(window: &AppWindow) -> Option<slint::Timer> {
    if !medindo() {
        return None;
    }
    use slint::ComponentHandle;

    let dados = Rc::new(RefCell::new(Janela::default()));
    let d = dados.clone();
    let resultado = window.window().set_rendering_notifier(move |estado, api| {
        let agora = Instant::now();
        let mut j = d.borrow_mut();
        match estado {
            slint::RenderingState::RenderingSetup => {
                // Qual placa desenha: com duas (integrada + dedicada), o
                // Windows pode por o app na integrada e copiar o quadro.
                if let slint::GraphicsAPI::NativeOpenGL { get_proc_address } = api {
                    let ptr = get_proc_address(c"glGetString");
                    if !ptr.is_null() {
                        // SAFETY: assinatura de `glGetString`; chamado na
                        // thread do contexto, durante a preparacao.
                        let gl_get_string: unsafe extern "system" fn(
                            u32,
                        )
                            -> *const std::ffi::c_char = unsafe { std::mem::transmute(ptr) };
                        let ler = |nome: u32| {
                            let p = unsafe { gl_get_string(nome) };
                            if p.is_null() {
                                String::new()
                            } else {
                                unsafe { std::ffi::CStr::from_ptr(p) }
                                    .to_string_lossy()
                                    .into_owned()
                            }
                        };
                        tracing::info!(
                            fabricante = ler(0x1F00),
                            placa = ler(0x1F01),
                            versao = ler(0x1F02),
                            "contexto OpenGL"
                        );
                    }
                }
            }
            slint::RenderingState::BeforeRendering => j.inicio_desenho = Some(agora),
            slint::RenderingState::AfterRendering => {
                if let Some(inicio) = j.inicio_desenho.take() {
                    let gasto = agora - inicio;
                    j.desenho_total += gasto;
                    j.desenho_max = j.desenho_max.max(gasto);
                }
                if let Some(antes) = j.ultimo_fim {
                    let intervalo = agora - antes;
                    // Acima de meio segundo e janela parada, nao tranco.
                    if intervalo < Duration::from_millis(500) {
                        j.pior_intervalo = j.pior_intervalo.max(intervalo);
                        if intervalo > Duration::from_millis(25) {
                            j.trancos += 1;
                        }
                    }
                }
                j.ultimo_fim = Some(agora);
                j.quadros += 1;
            }
            _ => {}
        }
    });
    if let Err(e) = resultado {
        tracing::warn!(error = ?e, "medidor de quadros indisponivel");
    }

    let relatorio = slint::Timer::default();
    relatorio.start(
        slint::TimerMode::Repeated,
        Duration::from_secs(1),
        move || {
            let mut j = dados.borrow_mut();
            if j.quadros > 1 {
                tracing::info!(
                    quadros = j.quadros,
                    pior_intervalo_ms = j.pior_intervalo.as_secs_f32() * 1000.0,
                    desenho_medio_ms = j.desenho_total.as_secs_f32() * 1000.0 / j.quadros as f32,
                    desenho_max_ms = j.desenho_max.as_secs_f32() * 1000.0,
                    trancos = j.trancos,
                    "quadros no ultimo segundo"
                );
            }
            let ultimo = j.ultimo_fim;
            *j = Janela {
                ultimo_fim: ultimo,
                ..Janela::default()
            };
        },
    );
    Some(relatorio)
}
