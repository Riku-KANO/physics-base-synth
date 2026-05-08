//! SustainState (Phase 3 D40)
//!
//! CC#64 の状態を保持し、note_off 時に sustain 中なら release を保留する。
//! pending_release は MIDI 0..127 の bitmap (u128) で管理。

pub struct SustainState {
    pub active: bool,
    pending_release: u128,
}

impl SustainState {
    pub const fn new() -> Self {
        Self {
            active: false,
            pending_release: 0,
        }
    }

    /// active を切り替え。`true → false` の遷移で pending を取り出して bitmap を返す
    /// (呼び元は各 bit に対し pool.note_off を発火する)。それ以外は 0。
    pub fn set_active(&mut self, active: bool) -> u128 {
        let was_active = self.active;
        self.active = active;
        if was_active && !active {
            let pending = self.pending_release;
            self.pending_release = 0;
            pending
        } else {
            0
        }
    }

    /// note_off を pending として記録。sustain 中なら true を返し、呼び元は release を保留する
    pub fn try_defer_note_off(&mut self, midi_note: u8) -> bool {
        if self.active && midi_note < 128 {
            self.pending_release |= 1u128 << midi_note;
            true
        } else {
            false
        }
    }

    /// note_on 時に呼ぶ。同一ノートが pending release 中だった場合、bit をクリア。
    /// シナリオ: C4 on → Sustain on → C4 off (pending) → C4 on (再励振) →
    /// CC#64 off で「再打鍵分まで release される」バグを防ぐ (P1-3)。
    pub fn clear_pending(&mut self, midi_note: u8) {
        if midi_note < 128 {
            self.pending_release &= !(1u128 << midi_note);
        }
    }

    /// pending bitmap を read-only で参照。
    /// `Engine::set_mode` で mode 切替前に pending を取り出してから reset するパターンで使用 (P2-1)。
    pub fn pending_release_bitmap(&self) -> u128 {
        self.pending_release
    }

    pub fn reset(&mut self) {
        self.active = false;
        self.pending_release = 0;
    }
}

impl Default for SustainState {
    fn default() -> Self {
        Self::new()
    }
}
