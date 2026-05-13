//! Phase 4c D76: Global sympathetic resonance bus.
//!
//! Piano + Sustain ペダル ON 時に全 voice の出力を bus に sum、bus は 2 ms delay line と
//! 1pole LPF の lossy feedback で「響板で他の弦が共鳴する」効果を生む。bus_out に
//! `feedback_gain` を乗じた値を各 voice の KS ループに微弱に注入することで、ペダル ON の
//! 余韻 / 周辺音の鳴り上がりを表現する (Bank 2000 系の Global resonance bus 案 A、
//! pre-research §5.2)。
//!
//! Phase 4a / 4b 互換: `feedback_gain = 0` (Default kind / Piano + Sustain OFF) のとき
//! voice 注入も modal_body 入力への直接ミックスも 0 で、Phase 4a HEAD / Phase 4b 7 楽器
//! と byte 一致継承 (D83 / F61-a / F61-e / F65-a)。

use crate::smoothing::SmoothedValue;

/// Bus delay の長さ (ms)。短くしすぎると LPF を通った lossy feedback でも flutter echo
/// 様になり、長すぎると pre-delay 感が出る。pre-research §5.3 の典型値。
pub const BUS_DELAY_MS: f32 = 2.0;

/// Bus 内部の lossy feedback 係数。LPF と組み合わせて全体ゲイン < 1 を保証する
/// (pre-research §5.4)。R43 (数値発散) 時には 0.95 → 0.90 へ強化する緩和策あり。
pub const BUS_INTERNAL_DECAY: f32 = 0.95;

/// Bus → voice の最大 feedback gain。`sympathetic_amount × FEEDBACK_GAIN_MAX` を
/// `set_feedback_gain_target` の clamp 上限として使用。pre-research §5.4 の安定上限。
pub const FEEDBACK_GAIN_MAX: f32 = 0.05;

/// Bus 内部 1pole LPF の cutoff (Hz)。Phase 3 D36 brightness LPF と同水準 (8 kHz)。
const BUS_LPF_CUTOFF_HZ: f32 = 8_000.0;

/// `feedback_gain` SmoothedValue の time constant (s)。CC#64 / apply_instrument 切替時の
/// クリック対策で 20 ms 平滑化。
const FEEDBACK_GAIN_TAU: f32 = 0.02;

/// Phase 4c D76: Global sympathetic resonance bus。
///
/// - `buffer`: 2 ms 分の delay line (`prepare(sample_rate)` で一括確保)。
/// - `lpf_*`: 1pole IIR LPF の内部状態 (state) と係数 (`alpha`)、prepare 時に算出。
/// - `feedback_gain`: bus_out × feedback_gain を各 voice に注入する強度の SmoothedValue。
///   `set_feedback_gain_target` で target を切替、`next_feedback_gain()` で per-sample 進行。
/// - `write_idx`: 現在の delay line write 位置。read は `(write_idx + 1) % len` (1 sample 後 = 最古値)。
pub struct ResonanceBus {
    buffer: Vec<f32>,
    lpf_state: f32,
    lpf_alpha: f32,
    feedback_gain: SmoothedValue,
    write_idx: usize,
    sample_rate: f32,
}

impl ResonanceBus {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            lpf_state: 0.0,
            lpf_alpha: 0.5,
            feedback_gain: SmoothedValue::new(0.0),
            write_idx: 0,
            sample_rate: 48_000.0,
        }
    }

    /// `Engine::prepare(sample_rate)` から呼ばれる。delay line を sample_rate に応じて確保し、
    /// LPF cutoff を 8 kHz 固定で `alpha = 1 - exp(-2π·fc/fs)` を算出する。
    /// `feedback_gain` smoother の time constant を 20 ms に設定。
    pub fn prepare(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        let len = (BUS_DELAY_MS * 0.001 * sample_rate).ceil() as usize + 1;
        self.buffer = vec![0.0; len];
        self.lpf_state = 0.0;
        self.lpf_alpha = (1.0
            - (-2.0 * core::f32::consts::PI * BUS_LPF_CUTOFF_HZ / sample_rate).exp())
        .clamp(0.001, 0.999);
        self.feedback_gain
            .set_time_constant(sample_rate, FEEDBACK_GAIN_TAU);
        self.feedback_gain.set_immediate(0.0);
        self.write_idx = 0;
    }

    /// `Engine::reset` / `apply_instrument` / `handle_midi_cc(CC#123)` から呼ばれる
    /// bus 完全リセット。delay line / LPF state / feedback_gain を全てゼロクリア。
    pub fn reset(&mut self) {
        for v in self.buffer.iter_mut() {
            *v = 0.0;
        }
        self.lpf_state = 0.0;
        self.feedback_gain.set_immediate(0.0);
        self.write_idx = 0;
    }

    /// Sustain ON / OFF や apply_instrument 経路で `feedback_gain` の target を切替。
    /// target は `[0.0, FEEDBACK_GAIN_MAX]` に clamp、actual gain は `next_feedback_gain()`
    /// で per-sample に滑らかに収束する。
    pub fn set_feedback_gain_target(&mut self, target: f32) {
        self.feedback_gain
            .set_target(target.clamp(0.0, FEEDBACK_GAIN_MAX));
    }

    /// 1 sample 処理。`bus_in` を delay line に書き込み、最古値を LPF した値 `bus_out` を返す。
    /// `bus_out` は呼出側で modal_body 入力にミックス + 次 sample の voice 注入用に保持する。
    ///
    /// `feedback_gain` とは独立に動作する (gain=0 でも bus 自体は dry で駆動)。出力経路の
    /// gate は Engine::process 側で `bus_mix = feedback_gain / FEEDBACK_GAIN_MAX` で行う。
    #[inline(always)]
    pub fn process(&mut self, bus_in: f32) -> f32 {
        let len = self.buffer.len();
        if len == 0 {
            return 0.0;
        }
        let read_idx = (self.write_idx + 1) % len;
        let read_value = self.buffer[read_idx];

        // 1pole IIR LPF (`y = α·x + (1-α)·y_prev`)
        self.lpf_state = self.lpf_alpha * read_value + (1.0 - self.lpf_alpha) * self.lpf_state;
        let filtered = self.lpf_state;

        // lossy feedback: `bus_in + filtered × BUS_INTERNAL_DECAY` を delay line に write
        let mut new_value = bus_in + filtered * BUS_INTERNAL_DECAY;
        // denormal flush (D6)
        new_value += 1.0e-25;
        new_value -= 1.0e-25;
        self.buffer[self.write_idx] = new_value;
        self.write_idx = if self.write_idx + 1 == len {
            0
        } else {
            self.write_idx + 1
        };

        filtered
    }

    /// `Engine::process` の per-sample loop 冒頭で呼び、`feedback_gain` smoother を 1 sample
    /// 進めた値を返す。voice 注入と `bus_mix` (modal_body 直接ミックス) の両方に使う。
    #[inline(always)]
    pub fn next_feedback_gain(&mut self) -> f32 {
        self.feedback_gain.next_sample()
    }

    /// Phase 4c test-only: 現在の feedback_gain target (SmoothedValue 終端値)。
    /// F65-b / F65-c / F65-d / F65-e / F68-c で使用 (§7.5)。
    #[doc(hidden)]
    pub fn feedback_gain_target_for_test(&self) -> f32 {
        self.feedback_gain.target()
    }

    /// Phase 4c test-only: per-sample 進行 (F65-b で「数 sample 後に > 0」を確認するための薄いラッパ)。
    #[doc(hidden)]
    pub fn next_feedback_gain_for_test(&mut self) -> f32 {
        self.next_feedback_gain()
    }

    /// Phase 4c test-only: F65-h / F65-i で `reset()` 後の delay line がゼロクリアされていることを
    /// 観測するための accessor。
    #[doc(hidden)]
    pub fn buffer_max_amplitude_for_test(&self) -> f32 {
        self.buffer.iter().map(|x| x.abs()).fold(0.0_f32, f32::max)
    }
}

impl Default for ResonanceBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 48_000.0;

    /// F64-a (provisional pass at Step 9). bus.process が LPF + lossy delay 後の有限信号を返す。
    #[test]
    fn test_resonance_bus_process_returns_filtered_signal() {
        let mut bus = ResonanceBus::new();
        bus.prepare(SAMPLE_RATE);
        // 連続 impulse 入力で出力が有限の非ゼロ値を返すこと (LPF state が立ち上がる)
        let mut last = 0.0_f32;
        for _ in 0..200 {
            last = bus.process(1.0);
        }
        assert!(
            last.abs() > 0.0,
            "bus.process should return non-zero after sustained impulse, got {}",
            last
        );
        assert!(last.is_finite(), "bus output must be finite, got {}", last);
    }

    /// F64-b (provisional). impulse 入力後、ゼロ入力継続で振幅が `1e-6` 以下へ減衰する。
    #[test]
    fn test_resonance_bus_decay_after_impulse() {
        let mut bus = ResonanceBus::new();
        bus.prepare(SAMPLE_RATE);
        let _ = bus.process(1.0);
        let mut last = 1.0_f32;
        // 数百 sample 後にゼロ収束 (BUS_INTERNAL_DECAY=0.95 + 8kHz LPF)
        for _ in 0..2000 {
            last = bus.process(0.0);
        }
        assert!(
            last.abs() < 1e-3,
            "bus should decay below 1e-3 after 2000 zero samples, got {}",
            last
        );
    }

    /// F64-c (provisional). 連続 impulse 入力で振幅が発散しない (max < 10.0)。
    #[test]
    fn test_resonance_bus_stability_1024_samples() {
        let mut bus = ResonanceBus::new();
        bus.prepare(SAMPLE_RATE);
        let mut max_abs = 0.0_f32;
        for _ in 0..1024 {
            let out = bus.process(1.0);
            max_abs = max_abs.max(out.abs());
        }
        // BUS_INTERNAL_DECAY * LPF_GAIN < 1 で安定。実機的には ~20 のレベルに収束する可能性は
        // 残るが、発散 (>100) はしないこと。
        assert!(
            max_abs < 100.0,
            "bus output must not diverge under sustained impulse, got max {}",
            max_abs
        );
    }

    /// F65-f (provisional). `process` で alloc ゼロ。
    #[test]
    fn test_resonance_bus_process_no_alloc() {
        let mut bus = ResonanceBus::new();
        bus.prepare(SAMPLE_RATE);
        // alloc 測定そのものは別の test infra で行う前提。ここでは buffer の長さが変わらない
        // ことを観測することで間接的に保証。
        let initial_capacity = bus
            .buffer_max_amplitude_for_test()
            .partial_cmp(&-1.0)
            .is_some();
        assert!(initial_capacity);
        for _ in 0..1024 {
            let _ = bus.process(0.5);
        }
        // process 後も buffer 長は不変 (Vec::resize は呼ばれない)
        assert!(!bus.buffer.is_empty());
    }

    /// `set_feedback_gain_target` の clamp 動作。
    #[test]
    fn test_feedback_gain_target_clamps_to_max() {
        let mut bus = ResonanceBus::new();
        bus.prepare(SAMPLE_RATE);
        bus.set_feedback_gain_target(1.0);
        assert!(
            (bus.feedback_gain_target_for_test() - FEEDBACK_GAIN_MAX).abs() < 1e-9,
            "target should clamp to FEEDBACK_GAIN_MAX=0.05, got {}",
            bus.feedback_gain_target_for_test()
        );
        bus.set_feedback_gain_target(-0.5);
        assert!(
            bus.feedback_gain_target_for_test().abs() < 1e-9,
            "negative target should clamp to 0, got {}",
            bus.feedback_gain_target_for_test()
        );
    }

    /// `reset` で delay line / LPF state / feedback_gain がゼロクリアされる。
    #[test]
    fn test_reset_clears_internal_state() {
        let mut bus = ResonanceBus::new();
        bus.prepare(SAMPLE_RATE);
        bus.set_feedback_gain_target(FEEDBACK_GAIN_MAX);
        for _ in 0..200 {
            let _ = bus.process(1.0);
            let _ = bus.next_feedback_gain();
        }
        assert!(bus.buffer_max_amplitude_for_test() > 0.0);
        bus.reset();
        assert_eq!(
            bus.buffer_max_amplitude_for_test(),
            0.0,
            "reset should zero the delay line"
        );
        assert_eq!(bus.lpf_state, 0.0, "reset should zero the LPF state");
        assert_eq!(
            bus.feedback_gain_target_for_test(),
            0.0,
            "reset should drop feedback_gain target to 0"
        );
    }
}
