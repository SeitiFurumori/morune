use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::model::Track;

/// Modo de repeticao.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepeatMode {
    #[default]
    Off,
    /// Repete o contexto inteiro ao chegar no fim.
    All,
    /// Repete a faixa atual indefinidamente.
    One,
}

impl RepeatMode {
    pub fn cycle(self) -> Self {
        match self {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        }
    }
}

/// De onde veio a fila atual, para a UI conseguir mostrar "tocando de X".
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum QueueOrigin {
    #[default]
    None,
    Album(String),
    Playlist(String),
    Artist(String),
    Search(String),
    Custom(String),
}

/// PRNG determinista (xorshift64*) usado no shuffle.
///
/// Proposital: o core nao depende de `rand`. Um gerador semeado explicitamente
/// torna o embaralhamento reproduzivel em teste, que e o unico requisito aqui.
/// Nao e, e nao deve ser usada como, fonte criptografica.
#[derive(Debug, Clone)]
struct Xorshift64(u64);

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        })
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Inteiro uniforme em `[0, n)` por rejeicao, sem vies de modulo.
    fn below(&mut self, n: u64) -> u64 {
        debug_assert!(n > 0);
        let zone = u64::MAX - (u64::MAX % n) - 1;
        loop {
            let v = self.next_u64();
            if v <= zone {
                return v % n;
            }
        }
    }
}

/// Semente padrao. Trocada por uma semente real (relogio) pelo host quando
/// existir; fixa por padrao para que o comportamento seja reproduzivel.
const DEFAULT_SEED: u64 = 0x4D6F_7275_6E65_0001;

const MAX_HISTORY: usize = 200;

/// Fila de reproducao.
///
/// Separa dois conceitos que a UI trata de forma diferente:
/// - `context`: o que foi tocado (album, playlist, resultado de busca);
/// - `user_queue`: faixas inseridas manualmente com "tocar a seguir", que tem
///   prioridade sobre o contexto e sao consumidas ao tocar.
///
/// O shuffle nao reordena `context`: mantem uma permutacao `order`, de modo que
/// desligar o shuffle devolve a ordem original sem perder a faixa atual.
#[derive(Debug, Clone)]
pub struct Queue {
    context: Vec<Track>,
    /// Permutacao de indices de `context`: `order[position]` e o indice real.
    order: Vec<usize>,
    /// Posicao dentro de `order`. `None` quando nada esta selecionado.
    position: Option<usize>,
    user_queue: VecDeque<Track>,
    history: Vec<Track>,
    shuffle: bool,
    repeat: RepeatMode,
    origin: QueueOrigin,
    seed: u64,
}

impl Default for Queue {
    fn default() -> Self {
        Self::with_seed(DEFAULT_SEED)
    }
}

impl Queue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_seed(seed: u64) -> Self {
        Self {
            context: Vec::new(),
            order: Vec::new(),
            position: None,
            user_queue: VecDeque::new(),
            history: Vec::new(),
            shuffle: false,
            repeat: RepeatMode::Off,
            origin: QueueOrigin::None,
            seed,
        }
    }

    // ---- estado ----

    pub fn current(&self) -> Option<&Track> {
        let pos = self.position?;
        self.context.get(*self.order.get(pos)?)
    }

    /// Indice da faixa atual dentro do contexto original (nao da permutacao).
    pub fn current_context_index(&self) -> Option<usize> {
        self.order.get(self.position?).copied()
    }

    pub fn is_empty(&self) -> bool {
        self.context.is_empty() && self.user_queue.is_empty()
    }

    pub fn len(&self) -> usize {
        self.context.len()
    }

    pub fn tracks(&self) -> &[Track] {
        &self.context
    }

    pub fn shuffle(&self) -> bool {
        self.shuffle
    }

    pub fn repeat(&self) -> RepeatMode {
        self.repeat
    }

    pub fn origin(&self) -> &QueueOrigin {
        &self.origin
    }

    pub fn user_queue(&self) -> impl Iterator<Item = &Track> {
        self.user_queue.iter()
    }

    pub fn user_queue_len(&self) -> usize {
        self.user_queue.len()
    }

    /// Proximas faixas vindas somente do contexto atual, sem as insercoes
    /// manuais. A interface usa isto para explicar o que o usuario controla.
    pub fn upcoming_context(&self, max: usize) -> Vec<&Track> {
        let Some(pos) = self.position else {
            return self
                .order
                .iter()
                .filter_map(|&i| self.context.get(i))
                .take(max)
                .collect();
        };

        let tail = self.order.iter().skip(pos + 1);
        if self.repeat == RepeatMode::All {
            tail.chain(self.order.iter().take(pos + 1))
                .filter_map(|&i| self.context.get(i))
                .take(max)
                .collect()
        } else {
            tail.filter_map(|&i| self.context.get(i)).take(max).collect()
        }
    }

    /// Proximas faixas na ordem em que serao tocadas, sem alterar estado.
    pub fn upcoming(&self, max: usize) -> Vec<&Track> {
        let mut out: Vec<&Track> = self.user_queue.iter().take(max).collect();
        if out.len() >= max {
            return out;
        }
        out.extend(self.upcoming_context(max - out.len()));
        out
    }

    // ---- mutacao ----

    /// Substitui o contexto e seleciona a faixa inicial.
    pub fn set_context(
        &mut self,
        origin: QueueOrigin,
        tracks: Vec<Track>,
        start_at: Option<usize>,
    ) {
        self.context = tracks;
        self.origin = origin;
        self.order = (0..self.context.len()).collect();
        self.history.clear();

        if self.context.is_empty() {
            self.position = None;
            return;
        }

        if self.shuffle {
            self.reshuffle_keeping(start_at);
            self.position = Some(0);
        } else {
            self.position = Some(start_at.filter(|&i| i < self.context.len()).unwrap_or(0));
        }
    }

    pub fn clear(&mut self) {
        let (shuffle, repeat, seed) = (self.shuffle, self.repeat, self.seed);
        *self = Self::with_seed(seed);
        self.shuffle = shuffle;
        self.repeat = repeat;
    }

    /// Insere a faixa no inicio da fila do usuario ("tocar a seguir").
    pub fn play_next(&mut self, track: Track) {
        self.user_queue.push_front(track);
    }

    /// Adiciona a faixa ao fim da fila do usuario.
    pub fn enqueue(&mut self, track: Track) {
        self.user_queue.push_back(track);
    }

    /// Acrescenta a pagina seguinte ao contexto sem mudar a faixa atual.
    ///
    /// Playlists podem repetir a mesma musica de proposito, portanto este
    /// metodo preserva duplicatas e ordem. Quando o shuffle esta ativo, apenas
    /// as novas posicoes sao embaralhadas antes de entrar no restante da fila.
    pub fn append_context(&mut self, tracks: Vec<Track>) {
        if tracks.is_empty() {
            return;
        }

        let first = self.context.len();
        let mut appended_order: Vec<usize> = (first..first + tracks.len()).collect();
        self.context.extend(tracks);

        if self.shuffle && appended_order.len() > 1 {
            let mut rng =
                Xorshift64::new(self.seed ^ (self.context.len() as u64).wrapping_mul(0x9E37));
            self.seed = self.seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
            for i in (1..appended_order.len()).rev() {
                let j = rng.below((i + 1) as u64) as usize;
                appended_order.swap(i, j);
            }
        }

        self.order.extend(appended_order);
    }

    /// Acrescenta uma continuacao ao contexto e seleciona a primeira faixa nova.
    ///
    /// Usado pelo autoplay depois que o contexto terminou. Preserva historico,
    /// fila manual e origem, e remove repeticoes para que uma estacao que inclua
    /// a propria semente nao crie um ciclo curto.
    pub fn append_and_select(&mut self, tracks: Vec<Track>) -> Option<&Track> {
        let mut novos = Vec::new();
        for track in tracks {
            if !self.context.iter().any(|existing| existing.id == track.id)
                && !novos.iter().any(|existing: &Track| existing.id == track.id)
            {
                novos.push(track);
            }
        }
        if novos.is_empty() {
            return None;
        }

        let first_context_index = self.context.len();
        let mut appended_order: Vec<usize> =
            (first_context_index..first_context_index + novos.len()).collect();
        self.context.extend(novos);

        if self.shuffle && appended_order.len() > 1 {
            let mut rng =
                Xorshift64::new(self.seed ^ (self.context.len() as u64).wrapping_mul(0x9E37));
            self.seed = self.seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
            for i in (1..appended_order.len()).rev() {
                let j = rng.below((i + 1) as u64) as usize;
                appended_order.swap(i, j);
            }
        }

        let first_order_position = self.order.len();
        self.order.extend(appended_order);
        self.position = Some(first_order_position);
        self.current()
    }

    pub fn remove_from_user_queue(&mut self, index: usize) -> Option<Track> {
        self.user_queue.remove(index)
    }

    pub fn clear_user_queue(&mut self) {
        self.user_queue.clear();
    }

    /// Move uma insercao manual sem alterar a ordem do album ou playlist que
    /// esta tocando por baixo dela.
    pub fn move_user_queue(&mut self, from: usize, to: usize) -> bool {
        if from >= self.user_queue.len() || to >= self.user_queue.len() || from == to {
            return false;
        }
        let Some(track) = self.user_queue.remove(from) else {
            return false;
        };
        self.user_queue.insert(to, track);
        true
    }

    pub fn set_repeat(&mut self, mode: RepeatMode) {
        self.repeat = mode;
    }

    /// Liga/desliga shuffle preservando a faixa atual.
    pub fn set_shuffle(&mut self, on: bool) {
        if self.shuffle == on {
            return;
        }
        self.shuffle = on;
        let current_index = self.current_context_index();

        if on {
            self.reshuffle_keeping(current_index);
            self.position = if self.context.is_empty() {
                None
            } else {
                Some(0)
            };
        } else {
            self.order = (0..self.context.len()).collect();
            self.position = current_index;
        }
    }

    /// Seleciona um indice do contexto original (clique numa faixa da lista).
    pub fn jump_to(&mut self, context_index: usize) -> Option<&Track> {
        let pos = self.order.iter().position(|&i| i == context_index)?;
        if let Some(prev) = self.current().cloned() {
            self.push_history(prev);
        }
        self.position = Some(pos);
        self.current()
    }

    /// Avanca uma faixa.
    ///
    /// `user_advance` distingue "a faixa acabou" de "o usuario clicou proximo":
    /// com [`RepeatMode::One`], apenas o avanco automatico repete a mesma faixa.
    pub fn next(&mut self, user_advance: bool) -> Option<&Track> {
        if self.repeat == RepeatMode::One && !user_advance && self.position.is_some() {
            return self.current();
        }

        if let Some(prev) = self.current().cloned() {
            self.push_history(prev);
        }

        if let Some(track) = self.user_queue.pop_front() {
            // A faixa da fila do usuario entra no contexto logo apos a posicao
            // atual, para que "anterior"/"proximo" continuem coerentes depois.
            let context_index = self.context.len();
            self.context.push(track);
            let insert_at = self
                .position
                .map(|p| p + 1)
                .unwrap_or(0)
                .min(self.order.len());
            self.order.insert(insert_at, context_index);
            self.position = Some(insert_at);
            return self.current();
        }

        let pos = self.position?;
        if pos + 1 < self.order.len() {
            self.position = Some(pos + 1);
            return self.current();
        }

        match self.repeat {
            RepeatMode::All if !self.order.is_empty() => {
                if self.shuffle {
                    self.reshuffle_keeping(None);
                }
                self.position = Some(0);
                self.current()
            }
            RepeatMode::One => self.current(),
            _ => {
                self.position = None;
                None
            }
        }
    }

    /// Volta uma faixa usando o historico real de reproducao.
    pub fn previous(&mut self) -> Option<&Track> {
        let track = self.history.pop()?;
        let context_index = self.context.iter().position(|t| t.id == track.id)?;
        let pos = self.order.iter().position(|&i| i == context_index)?;
        self.position = Some(pos);
        self.current()
    }

    fn push_history(&mut self, track: Track) {
        if self.history.len() >= MAX_HISTORY {
            self.history.remove(0);
        }
        self.history.push(track);
    }

    /// Fisher-Yates sobre `order`, fixando `keep` na posicao 0 quando informado.
    fn reshuffle_keeping(&mut self, keep: Option<usize>) {
        self.order = (0..self.context.len()).collect();
        if self.order.is_empty() {
            return;
        }
        let mut rng = Xorshift64::new(self.seed ^ (self.context.len() as u64).wrapping_mul(0x9E37));
        self.seed = self.seed.wrapping_add(0x9E37_79B9_7F4A_7C15);

        let start = match keep.and_then(|k| self.order.iter().position(|&i| i == k)) {
            Some(p) => {
                self.order.swap(0, p);
                1
            }
            None => 0,
        };

        if self.order.len() > start + 1 {
            for i in (start + 1..self.order.len()).rev() {
                let j = start + rng.below((i - start + 1) as u64) as usize;
                self.order.swap(i, j);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::model::{Provider, TrackId};

    fn track(n: usize) -> Track {
        Track {
            id: TrackId::new(Provider::Local, format!("t{n}")),
            name: format!("Track {n}").into(),
            artists: vec![],
            album: None,
            duration: Duration::from_secs(180),
            track_number: Some(n as u32),
            disc_number: Some(1),
            explicit: false,
            playable: true,
        }
    }

    fn q(n: usize) -> Queue {
        let mut q = Queue::with_seed(42);
        q.set_context(
            QueueOrigin::Custom("test".into()),
            (0..n).map(track).collect(),
            Some(0),
        );
        q
    }

    #[test]
    fn starts_on_first_track() {
        assert_eq!(q(3).current().unwrap().name.as_ref(), "Track 0");
    }

    #[test]
    fn advances_and_stops_at_end_without_repeat() {
        let mut q = q(2);
        assert_eq!(q.next(true).unwrap().name.as_ref(), "Track 1");
        assert!(q.next(true).is_none());
    }

    #[test]
    fn repeat_all_wraps_around() {
        let mut q = q(2);
        q.set_repeat(RepeatMode::All);
        q.next(true);
        assert_eq!(q.next(true).unwrap().name.as_ref(), "Track 0");
    }

    #[test]
    fn repeat_one_repeats_only_on_automatic_advance() {
        let mut q = q(3);
        q.set_repeat(RepeatMode::One);
        assert_eq!(q.next(false).unwrap().name.as_ref(), "Track 0");
        assert_eq!(q.next(true).unwrap().name.as_ref(), "Track 1");
    }

    #[test]
    fn user_queue_has_priority_over_context() {
        let mut q = q(3);
        q.enqueue(track(99));
        assert_eq!(q.next(true).unwrap().name.as_ref(), "Track 99");
        assert_eq!(q.next(true).unwrap().name.as_ref(), "Track 1");
    }

    #[test]
    fn play_next_jumps_ahead_of_enqueued() {
        let mut q = q(3);
        q.enqueue(track(50));
        q.play_next(track(60));
        assert_eq!(q.next(true).unwrap().name.as_ref(), "Track 60");
        assert_eq!(q.next(true).unwrap().name.as_ref(), "Track 50");
    }

    #[test]
    fn previous_uses_real_history() {
        let mut q = q(3);
        q.next(true);
        q.next(true);
        assert_eq!(q.previous().unwrap().name.as_ref(), "Track 1");
        assert_eq!(q.previous().unwrap().name.as_ref(), "Track 0");
        assert!(q.previous().is_none());
    }

    #[test]
    fn previous_returns_to_user_queued_track() {
        let mut q = q(3);
        q.enqueue(track(99));
        q.next(true);
        q.next(true);
        assert_eq!(q.previous().unwrap().name.as_ref(), "Track 99");
    }

    #[test]
    fn shuffle_keeps_current_track_and_stays_a_permutation() {
        let mut q = q(50);
        q.next(true);
        q.next(true);
        let current = q.current().unwrap().id.clone();
        q.set_shuffle(true);
        assert_eq!(q.current().unwrap().id, current);

        let mut seen = q.order.clone();
        seen.sort_unstable();
        assert_eq!(seen, (0..50).collect::<Vec<_>>());
    }

    #[test]
    fn unshuffle_restores_original_order_and_position() {
        let mut q = q(20);
        q.next(true);
        q.next(true);
        q.next(true);
        let current = q.current().unwrap().id.clone();
        q.set_shuffle(true);
        q.set_shuffle(false);
        assert_eq!(q.order, (0..20).collect::<Vec<_>>());
        assert_eq!(q.current().unwrap().id, current);
        assert_eq!(q.next(true).unwrap().name.as_ref(), "Track 4");
    }

    #[test]
    fn shuffle_actually_reorders_a_large_context() {
        let mut q = q(200);
        q.set_shuffle(true);
        assert_ne!(q.order, (0..200).collect::<Vec<_>>());
    }

    #[test]
    fn jump_to_selects_by_context_index_even_when_shuffled() {
        let mut q = q(10);
        q.set_shuffle(true);
        assert_eq!(q.jump_to(7).unwrap().name.as_ref(), "Track 7");
    }

    #[test]
    fn upcoming_lists_user_queue_first() {
        let mut q = q(5);
        q.enqueue(track(99));
        let up: Vec<&str> = q.upcoming(3).iter().map(|t| t.name.as_ref()).collect();
        assert_eq!(up, vec!["Track 99", "Track 1", "Track 2"]);
    }

    #[test]
    fn context_preview_excludes_manual_queue() {
        let mut q = q(5);
        q.enqueue(track(99));
        let context: Vec<&str> = q
            .upcoming_context(2)
            .iter()
            .map(|track| track.name.as_ref())
            .collect();
        assert_eq!(context, vec!["Track 1", "Track 2"]);
    }

    #[test]
    fn manual_queue_can_be_reordered_removed_and_cleared() {
        let mut q = q(3);
        q.enqueue(track(50));
        q.enqueue(track(60));
        q.enqueue(track(70));

        assert!(q.move_user_queue(2, 0));
        assert_eq!(q.user_queue().next().unwrap().name.as_ref(), "Track 70");
        assert_eq!(q.remove_from_user_queue(1).unwrap().name.as_ref(), "Track 50");
        q.clear_user_queue();
        assert_eq!(q.user_queue_len(), 0);
    }

    #[test]
    fn upcoming_wraps_only_with_repeat_all() {
        let mut q = q(3);
        q.jump_to(2);
        assert!(q.upcoming(3).is_empty());
        q.set_repeat(RepeatMode::All);
        let up: Vec<&str> = q.upcoming(3).iter().map(|t| t.name.as_ref()).collect();
        assert_eq!(up, vec!["Track 0", "Track 1", "Track 2"]);
    }

    #[test]
    fn a_new_page_extends_the_context_without_interrupting_playback() {
        let mut queue = q(2);
        queue.jump_to(1);

        queue.append_context(vec![track(2), track(2), track(3)]);

        assert_eq!(queue.current().unwrap().name.as_ref(), "Track 1");
        assert_eq!(queue.len(), 5);
        assert_eq!(queue.next(true).unwrap().name.as_ref(), "Track 2");
        assert_eq!(queue.next(true).unwrap().name.as_ref(), "Track 2");
    }

    #[test]
    fn autoplay_appends_without_erasing_history_or_repeating_tracks() {
        let mut queue = q(2);
        queue.next(false);
        assert!(queue.next(false).is_none());

        let current = queue
            .append_and_select(vec![track(1), track(2), track(2), track(3)])
            .expect("ha recomendacoes novas");
        assert_eq!(current.name.as_ref(), "Track 2");
        assert_eq!(queue.len(), 4);
        assert_eq!(queue.previous().unwrap().name.as_ref(), "Track 1");
    }

    #[test]
    fn autoplay_with_only_duplicates_keeps_the_queue_stopped() {
        let mut queue = q(1);
        assert!(queue.next(false).is_none());
        assert!(queue.append_and_select(vec![track(0)]).is_none());
        assert!(queue.current().is_none());
    }

    #[test]
    fn empty_queue_is_inert() {
        let mut q = Queue::with_seed(1);
        assert!(q.is_empty());
        assert!(q.current().is_none());
        assert!(q.next(true).is_none());
        assert!(q.previous().is_none());
        q.set_shuffle(true);
        assert!(q.current().is_none());
    }

    #[test]
    fn clear_preserves_user_preferences() {
        let mut q = q(5);
        q.set_shuffle(true);
        q.set_repeat(RepeatMode::All);
        q.clear();
        assert!(q.is_empty());
        assert!(q.shuffle());
        assert_eq!(q.repeat(), RepeatMode::All);
    }

    #[test]
    fn rng_below_is_in_range_and_roughly_uniform() {
        let mut rng = Xorshift64::new(7);
        let mut counts = [0u32; 5];
        for _ in 0..50_000 {
            counts[rng.below(5) as usize] += 1;
        }
        for c in counts {
            assert!(
                (9_000..11_000).contains(&c),
                "distribuicao enviesada: {counts:?}"
            );
        }
    }

    #[test]
    fn repeat_mode_cycles() {
        assert_eq!(RepeatMode::Off.cycle(), RepeatMode::All);
        assert_eq!(RepeatMode::All.cycle(), RepeatMode::One);
        assert_eq!(RepeatMode::One.cycle(), RepeatMode::Off);
    }
}
