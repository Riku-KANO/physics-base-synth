use crate::dispersion::{b_curve_piano, b_curve_zero};
use crate::hold_stack::HoldStack;
use crate::lfo::{Lfo, LfoDestination, LfoWaveform};
use crate::modal_body::ModalBodyResonator;
use crate::params::{
    stereo_spread_for_instrument, InstrumentKind, ParamId, BODY_WET_DEFAULT,
    HAMMER_CUTOFF_HIGH_PIANO, HAMMER_CUTOFF_LOW_PIANO, OUTPUT_GAIN_DEFAULT, PICK_POSITION_DEFAULT,
    STEREO_SPREAD_DEFAULT, SYMPATHETIC_AMOUNT_PIANO, UNISON_DETUNE_CENTS_PIANO,
};
use crate::smoothing::SmoothedValue;
use crate::soft_clip::soft_clip;
use crate::sustain_state::SustainState;
use crate::traits::AudioProcessor;
use crate::voice_pool::{VoicePool, POLYPHONY};

/// CC#7 Channel Volume のデフォルト (OutputGain と直交)
const CHANNEL_VOLUME_DEFAULT: f32 = 1.0;
const CHANNEL_VOLUME_TAU: f32 = 0.02;

/// Phase 4a D49: Mod Wheel (CC#1) のデフォルトと SmoothedValue 時定数。
/// デフォルト 0.0 = LFO 効果ゼロ (Phase 3 互換挙動)。
const MOD_WHEEL_DEFAULT: f32 = 0.0;
const MOD_WHEEL_TAU: f32 = 0.05;

/// Phase 4a D48: LFO destination depth のデフォルトと SmoothedValue 時定数。
const LFO_DEPTH_DEFAULT: f32 = 0.0;
const LFO_DEPTH_TAU: f32 = 0.05;

/// Phase 4a D48: LFO Pitch destination の深さスケール (depth=1.0 で ±0.5 半音)
const LFO_PITCH_SCALE_SEMITONES: f32 = 0.5;
/// Phase 4a D48: LFO Brightness destination の深さスケール (depth=1.0 で ±0.5 brightness offset)
const LFO_BRIGHTNESS_SCALE: f32 = 0.5;
/// Phase 4a D48: LFO Volume destination の深さスケール (depth=1.0 で ±0.5 volume multiplier offset)
const LFO_VOLUME_SCALE: f32 = 0.5;

const CC_MOD_WHEEL: u8 = 1;
const CC_CHANNEL_VOLUME: u8 = 7;
const CC_SUSTAIN_PEDAL: u8 = 64;
const CC_ALL_NOTES_OFF: u8 = 123;

/// Voice State 共有メモリの書き込み stride (1024 sample = ~21 ms @ 48kHz)。
/// JS Worklet 側の VOICE_STATE_STRIDE_FRAMES と同期。
const VOICE_STATE_WRITE_STRIDE: u32 = 1024;

/// mono モード復帰時のデフォルト velocity。Phase 3 で「note_off されたキーの velocity を保持」へ拡張候補。
const MONO_REVIVE_VELOCITY: f32 = 0.8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SynthMode {
    Poly,
    Mono,
}

pub struct Engine {
    sample_rate: f32,
    pool: VoicePool<POLYPHONY>,
    output_gain: SmoothedValue,
    /// Phase 3 D30 / D31: ボディ共鳴 (M=8 並列 bandpass biquad、stereo)
    modal_body: ModalBodyResonator,
    /// Phase 3 D32: dry/wet ミックス。0.0 = dry のみ、1.0 = body 出力のみ
    body_wet: SmoothedValue,
    mode: SynthMode,
    /// mono モード時の押下中ノート履歴 (D29)。poly モードでは参照しない。
    hold_stack: HoldStack,
    /// ユーザー設定の damping。note_off は damping target を 0.95 に上書きするので、
    /// 新規 / 再励振したボイスのみ復元するために保持する (set_damping_voice 用)。
    current_damping: f32,
    /// Phase 3 D34: pick position β。SmoothedValue 化せず（次回 note_on で反映、D34）。
    pick_position: f32,
    /// Phase 3 D38b: CC#7 Channel Volume (output_gain と直交、final = output_gain * channel_volume)
    channel_volume: SmoothedValue,
    /// Phase 3 D40: CC#64 Sustain Pedal の active / pending release 管理
    sustain_state: SustainState,
    /// Voice State 共有メモリ (active mask 1 byte + 8 振幅 × 4 bytes LE)
    voice_state_buffer: [u8; 33],
    /// 最後に voice_state_buffer を書き込んでからの経過 sample 数
    voice_state_sample_counter: u32,

    /// Phase 4a D46: グローバル LFO 1 個
    lfo: Lfo,
    /// Phase 4a D49: Mod Wheel (CC#1) を SmoothedValue で保持。LFO depth の master 乗数。
    mod_wheel: SmoothedValue,
    /// Phase 4a D48: LFO Pitch destination 深さ ∈ [0, 1]
    lfo_pitch_depth: SmoothedValue,
    /// Phase 4a D48: LFO Brightness destination 深さ ∈ [0, 1]
    lfo_brightness_depth: SmoothedValue,
    /// Phase 4a D48: LFO Volume destination 深さ ∈ [0, 1]
    lfo_volume_depth: SmoothedValue,
    /// Phase 4a D52: 現在の楽器選択 (kind=0 Default で Phase 3 既存値の互換性を維持)
    current_instrument: InstrumentKind,
    /// Phase 4a D54: 楽器ごとの stereo_spread を反映する保持値。`Engine::stereo_spread()` の参照値。
    stereo_spread: f32,

    /// Phase 4c D72: 楽器プリセットの unison detune (cents)。Piano kind で 1.5、他で 0。
    /// `trigger_voice` で `pool.note_on_with_piano_params` に渡す。
    unison_detune_cents: f32,
    /// Phase 4c D77: Piano + Sustain ON での sympathetic resonance 強度 ∈ [0, 1]。
    /// Step 10 で ResonanceBus の feedback_gain target = `sympathetic_amount × FEEDBACK_GAIN_MAX`。
    sympathetic_amount: f32,
    /// Phase 4c D78: 楽器ごとの B(note) lookup 関数ポインタ。
    /// Piano kind = `b_curve_piano`、他 = `b_curve_zero`。`trigger_voice` で MIDI を渡して
    /// `inharmonicity_b` を取得し、`note_on_with_piano_params` に渡す。
    inharmonicity_b_for_note: fn(u8) -> f32,
    /// Phase 4c D75: Hertz hammer cutoff の上下限 (Piano プリセット由来)。非 Piano では 0。
    hammer_cutoff_low_hz: f32,
    hammer_cutoff_high_hz: f32,
    /// Phase 4c D76: 前 sample の bus_out。Step 10 の `process` 内で
    /// `pool.process_sample_with_feedback(bus_out_prev, feedback_gain)` 経由で各 voice に注入。
    /// Step 8 ではフィールドだけ追加、Step 10 で使用開始。
    bus_out_prev: f32,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            sample_rate: 44100.0,
            pool: VoicePool::new(),
            output_gain: SmoothedValue::new(OUTPUT_GAIN_DEFAULT),
            modal_body: ModalBodyResonator::new(),
            body_wet: SmoothedValue::new(BODY_WET_DEFAULT),
            mode: SynthMode::Poly,
            hold_stack: HoldStack::new(),
            current_damping: ParamId::Damping.descriptor().default,
            pick_position: PICK_POSITION_DEFAULT,
            channel_volume: SmoothedValue::new(CHANNEL_VOLUME_DEFAULT),
            sustain_state: SustainState::new(),
            voice_state_buffer: [0u8; 33],
            voice_state_sample_counter: 0,
            lfo: Lfo::new(),
            mod_wheel: SmoothedValue::new(MOD_WHEEL_DEFAULT),
            lfo_pitch_depth: SmoothedValue::new(LFO_DEPTH_DEFAULT),
            lfo_brightness_depth: SmoothedValue::new(LFO_DEPTH_DEFAULT),
            lfo_volume_depth: SmoothedValue::new(LFO_DEPTH_DEFAULT),
            current_instrument: InstrumentKind::Default,
            stereo_spread: STEREO_SPREAD_DEFAULT,
            // Phase 4c D72 / D75 / D77 / D78: Default kind の初期状態。Piano パラメータは
            // `apply_instrument(Piano)` で上書きされ、Default 以外への切替でも明示的に再設定する。
            unison_detune_cents: 0.0,
            sympathetic_amount: 0.0,
            inharmonicity_b_for_note: b_curve_zero,
            hammer_cutoff_low_hz: 0.0,
            hammer_cutoff_high_hz: 0.0,
            bus_out_prev: 0.0,
        }
    }

    /// 新規ノートを発音し、割当先ボイスのみ damping をユーザー値に復元する。
    /// `set_damping_voice` を fan-out にすると release 中ボイスを 0.95 → current_damping に
    /// 巻き戻して再生を「復活」させてしまうため、必ず assigned index にだけ適用する。
    ///
    /// Phase 4c Step 8: `pool.note_on` を `pool.note_on_with_piano_params` に差し替え、
    /// B(note) LUT 値 + 楽器固有 params を 1 voice に渡す経路へ移行。`Mono` mode の
    /// `note_off` 経路 (新 top の `trigger_voice(top, MONO_REVIVE_VELOCITY)`) も同じ差替を受ける。
    fn trigger_voice(&mut self, midi_note: u8, velocity: f32) {
        let inharmonicity_b = (self.inharmonicity_b_for_note)(midi_note);
        let freq = midi_to_freq(midi_note);
        let assigned = self.pool.note_on_with_piano_params(
            midi_note,
            freq,
            velocity,
            self.unison_detune_cents,
            inharmonicity_b,
            self.hammer_cutoff_low_hz,
            self.hammer_cutoff_high_hz,
        );
        self.pool.set_damping_voice(assigned, self.current_damping);
    }

    /// poly: VoicePool に直接発火。mono: 直前 top をリリースしてから stack を更新 + 新規発音。
    pub fn note_on(&mut self, midi_note: u8, velocity: f32) {
        // Phase 3 D40 P1-3: Sustain pending bit を再打鍵時に消す。
        // C4 on → Sustain on → C4 off (pending) → C4 on (再打鍵) で
        // CC#64 off 時に「再打鍵分まで release される」バグを防ぐ。
        self.sustain_state.clear_pending(midi_note);
        if matches!(self.mode, SynthMode::Mono) {
            // 直前 top のボイスをリリース (mono は 1 音のみ鳴らす建前だが、
            // 短い release tail はクリック対策で残す)
            if let Some(prev) = self.hold_stack.top() {
                if prev != midi_note {
                    self.pool.note_off(prev);
                }
            }
            // MIDI の重複 noteOn (同じノートを離さず 2 回押す) で stale な履歴が残らないよう
            // push_unique で既存値を排除してから末尾に追加する
            self.hold_stack.push_unique(midi_note);
        }
        self.trigger_voice(midi_note, velocity);
    }

    /// poly: 該当ボイスを release (damping を 0.95 に加速)。
    /// mono: hold_stack から削除し、現ボイスを release。新しい top があれば top に発音復帰。
    /// Phase 3 D40: Poly mode のみ Sustain Pedal 適用、Mono は Phase 2 既存挙動継承（P1-2）。
    pub fn note_off(&mut self, midi_note: u8) {
        match self.mode {
            SynthMode::Poly => {
                if self.sustain_state.try_defer_note_off(midi_note) {
                    return;
                }
                self.pool.note_off(midi_note);
            }
            SynthMode::Mono => {
                let prev_top = self.hold_stack.top();
                self.hold_stack.remove(midi_note);
                self.pool.note_off(midi_note);
                let new_top = self.hold_stack.top();
                // top が変わった場合のみ復帰発音 (中間キー解放では再 trigger しない、クリック対策)
                if new_top != prev_top {
                    if let Some(top) = new_top {
                        self.trigger_voice(top, MONO_REVIVE_VELOCITY);
                    }
                }
            }
        }
    }

    pub fn set_param(&mut self, id: u32, value: f32) {
        match ParamId::from_u32(id) {
            Some(ParamId::Damping) => {
                let v = ParamId::Damping.descriptor().clamp(value);
                self.current_damping = v;
                self.pool.set_damping(v);
            }
            Some(ParamId::Brightness) => {
                self.pool
                    .set_brightness(ParamId::Brightness.descriptor().clamp(value));
            }
            Some(ParamId::OutputGain) => {
                self.output_gain
                    .set_target(ParamId::OutputGain.descriptor().clamp(value));
            }
            Some(ParamId::PickPosition) => {
                let v = ParamId::PickPosition.descriptor().clamp(value);
                self.pick_position = v;
                self.pool.set_pick_position(v);
            }
            Some(ParamId::BodyWet) => {
                self.body_wet
                    .set_target(ParamId::BodyWet.descriptor().clamp(value));
            }
            None => {}
        }
    }

    /// Phase 3 D38 / D39: Pitch Bend (±2 半音) を全 active voice に fan-out。
    pub fn handle_pitch_bend(&mut self, semitones: f32) {
        self.pool.set_pitch_bend(semitones);
    }

    /// Phase 3 D38: MIDI CC dispatch (CC#7 / #64 / #123 のみ、その他は no-op)。
    /// `value_normalized` は 0..1 範囲（呼び元が `cc_value / 127.0` で正規化）。
    pub fn handle_midi_cc(&mut self, cc: u8, value_normalized: f32) {
        let v = value_normalized.clamp(0.0, 1.0);
        match cc {
            CC_MOD_WHEEL => {
                // Phase 4a D49: Mod Wheel を LFO depth の master 乗数として保持。
                // Phase 3 では no-op だった経路を有効化。
                self.mod_wheel.set_target(v);
            }
            CC_CHANNEL_VOLUME => {
                // D38b: OutputGain と直交、final = output_gain * channel_volume
                self.channel_volume.set_target(v);
            }
            CC_SUSTAIN_PEDAL => {
                // ≥ 64 (= 0.5 normalized) で on
                let released = self.sustain_state.set_active(v >= 0.5);
                self.release_pending(released);
            }
            CC_ALL_NOTES_OFF => {
                // P1-1: sustain も reset しないと古い pending が次の CC#64 操作で再処理される
                self.pool.all_notes_off();
                self.hold_stack.clear();
                self.sustain_state.reset();
            }
            _ => {}
        }
    }

    /// pending bitmap の各 set bit に対して `pool.note_off` を発火する。
    /// 128 線形ループの代わりに `trailing_zeros` で set bit のみを舐める。
    fn release_pending(&mut self, mut bitmap: u128) {
        while bitmap != 0 {
            let note = bitmap.trailing_zeros() as u8;
            self.pool.note_off(note);
            bitmap &= bitmap - 1;
        }
    }

    pub fn set_mode(&mut self, mode: SynthMode) {
        // P2-1: 切替前に pending を全 release してから reset。mode 切替で Sustain pending が
        // 宙ぶらりんにならないよう、各 note を即時 release する。
        let pending = self.sustain_state.pending_release_bitmap();
        if pending != 0 {
            self.sustain_state.reset();
            self.release_pending(pending);
        }
        self.mode = mode;
        self.hold_stack.clear();
    }

    pub fn mode(&self) -> SynthMode {
        self.mode
    }

    pub fn current_damping(&self) -> f32 {
        self.current_damping
    }

    /// D22: C ABI 非公開。
    #[doc(hidden)]
    pub fn active_voice_count(&self) -> usize {
        self.pool.active_count()
    }

    #[doc(hidden)]
    pub fn voice_index_for_note(&self, midi_note: u8) -> Option<usize> {
        self.pool.voice_index_for_note(midi_note)
    }

    /// release 中ボイスの damping_target 等の検証用。
    #[doc(hidden)]
    pub fn pool(&self) -> &VoicePool<POLYPHONY> {
        &self.pool
    }

    #[doc(hidden)]
    pub fn channel_volume_target(&self) -> f32 {
        self.channel_volume.target()
    }

    #[doc(hidden)]
    pub fn sustain_active(&self) -> bool {
        self.sustain_state.active
    }

    #[doc(hidden)]
    pub fn sustain_pending_bitmap(&self) -> u128 {
        self.sustain_state.pending_release_bitmap()
    }

    /// Phase 4c test-only (§7.5): 指定 MIDI note を発音中の voice の `n_strings_active` を観測。
    /// F68-a / F68-b で「Piano + C4 → 3 弦、Default + C4 → 1 弦」を確認するための accessor。
    #[doc(hidden)]
    pub fn voice_n_strings_active_for_test(&self, midi: u8) -> Option<usize> {
        let i = self.pool.voice_index_for_note(midi)?;
        self.pool.voice_n_strings_active_for_test(i)
    }

    /// Phase 4c test-only: 指定 MIDI note を発音中の voice の `inharmonicity_b` を観測。
    /// F67-g / F68-a で `b_curve_piano(midi)` と一致することを確認。
    #[doc(hidden)]
    pub fn voice_inharmonicity_b_for_test(&self, midi: u8) -> Option<f32> {
        let i = self.pool.voice_index_for_note(midi)?;
        self.pool.voice_inharmonicity_b_for_test(i)
    }

    /// Phase 4c test-only: 指定 MIDI note を発音中の voice の `unison_detune_cents` を観測。
    #[doc(hidden)]
    pub fn voice_unison_detune_cents_for_test(&self, midi: u8) -> Option<f32> {
        let i = self.pool.voice_index_for_note(midi)?;
        self.pool.voice_unison_detune_cents_for_test(i)
    }

    /// Phase 4c test-only: 指定 MIDI note を発音中の voice の `dispersion_active` を観測。
    #[doc(hidden)]
    pub fn voice_dispersion_active_for_test(&self, midi: u8) -> Option<bool> {
        let i = self.pool.voice_index_for_note(midi)?;
        self.pool.voice_dispersion_active_for_test(i)
    }

    /// Phase 3 D41: Voice State 共有メモリへのポインタ。
    /// 33 bytes (active mask 1 byte + 8 振幅 × 4 bytes、little-endian)。
    /// `Engine::process` 終端で書き込まれる、JS 側からは `Uint8Array` view で読む。
    pub fn voice_state_ptr(&self) -> *const u8 {
        self.voice_state_buffer.as_ptr()
    }

    /// Phase 4a D52 / D53 + Phase 4b D67: 楽器プリセット切替。
    /// 全 voice 即時 release → hold_stack / sustain_state クリア → current_instrument 更新
    /// → ModalBodyResonator の係数差し替え + state クリア → dispersion_active fan-out。
    /// `SmoothedValue::set_target` は target 代入のみで current は `next_sample()` でしか
    /// 進まないため、同期メソッド内で fade-out は実現不能。Phase 4a D53「即時 release」を
    /// 完全継承し、`pool.set_dispersion_active(matches!(kind, Piano))` の 1 行を追加するのみ。
    /// pop noise 軽減 (fade-out / cross-fade) は Phase 4c の `PendingInstrumentChange`
    /// 状態機械で再実装する候補。
    pub fn apply_instrument(&mut self, kind: InstrumentKind) {
        self.pool.all_notes_off();
        self.hold_stack.clear();
        self.sustain_state.reset();
        self.current_instrument = kind;
        self.stereo_spread = stereo_spread_for_instrument(kind);
        self.modal_body.set_instrument(kind, self.sample_rate);

        // Phase 4b D67: dispersion_active を全 voice に fan-out。Piano kind では
        // process_sample で 8 段 cascade を経由、他 7 楽器 (Default 含む) では skip。
        let is_piano = matches!(kind, InstrumentKind::Piano);
        self.pool.set_dispersion_active(is_piano);

        // Phase 4c D72 / D75 / D77 / D78: 楽器プリセットの Piano パラメータを切替。
        // 非 Piano では unison_detune / sympathetic / cutoff を 0 にし、B(note) は b_curve_zero
        // を返す関数ポインタを設定 (dispersion_active=false 経路と二重保証で互換性維持)。
        let (detune, sympathetic, b_curve, cutoff_low, cutoff_high) = if is_piano {
            (
                UNISON_DETUNE_CENTS_PIANO,
                SYMPATHETIC_AMOUNT_PIANO,
                b_curve_piano as fn(u8) -> f32,
                HAMMER_CUTOFF_LOW_PIANO,
                HAMMER_CUTOFF_HIGH_PIANO,
            )
        } else {
            (0.0, 0.0, b_curve_zero as fn(u8) -> f32, 0.0, 0.0)
        };
        self.unison_detune_cents = detune;
        self.sympathetic_amount = sympathetic;
        self.inharmonicity_b_for_note = b_curve;
        self.hammer_cutoff_low_hz = cutoff_low;
        self.hammer_cutoff_high_hz = cutoff_high;

        // 全 voice に楽器パラメータを fan-out。`inharmonicity_b` は note 依存のため
        // プレースホルダ 0 で OK (note_on 直前に `note_on_with_piano_params` が割当 voice
        // にだけ正しい LUT 値を上書きする)。
        self.pool
            .set_piano_params(detune, 0.0, cutoff_low, cutoff_high);

        // Phase 4c: bus_out_prev を切替時にクリア。ResonanceBus 本体は Step 10 で追加されるが、
        // Engine フィールドの bus_out_prev は Step 8 でフィールド導入済のため整合性のために初期化。
        self.bus_out_prev = 0.0;
    }

    /// Phase 4a D46: LFO レート設定 (0.1〜8.0 Hz、SmoothedValue tau=0.05s で平滑化)。
    pub fn lfo_set_rate(&mut self, hz: f32) {
        self.lfo.set_rate(hz);
    }

    /// Phase 4a D47: LFO 波形設定 (Sine / Triangle)。
    pub fn lfo_set_waveform(&mut self, kind: LfoWaveform) {
        self.lfo.set_waveform(kind);
    }

    /// Phase 4a D48: LFO destination depth 設定。
    /// 値域 [0, 1] に clamp。
    pub fn lfo_set_depth(&mut self, dest: LfoDestination, depth: f32) {
        let v = depth.clamp(0.0, 1.0);
        match dest {
            LfoDestination::Pitch => self.lfo_pitch_depth.set_target(v),
            LfoDestination::Brightness => self.lfo_brightness_depth.set_target(v),
            LfoDestination::Volume => self.lfo_volume_depth.set_target(v),
        }
    }

    /// Phase 4a D52: 現在の楽器選択を返す（Step 9 で apply_instrument 実装）。
    #[doc(hidden)]
    pub fn current_instrument(&self) -> InstrumentKind {
        self.current_instrument
    }

    /// Phase 4a D54: 楽器ごとの stereo_spread を返す。
    #[doc(hidden)]
    pub fn stereo_spread(&self) -> f32 {
        self.stereo_spread
    }

    /// Phase 4a D52 / D53: ModalBodyResonator への read-only access (テスト用)。
    #[doc(hidden)]
    pub fn modal_body(&self) -> &ModalBodyResonator {
        &self.modal_body
    }

    /// Phase 4a D46: LFO への read-only access (テスト用)。
    #[doc(hidden)]
    pub fn lfo(&self) -> &Lfo {
        &self.lfo
    }

    /// Phase 4a D49: Mod Wheel target 値 (テスト用)。
    #[doc(hidden)]
    pub fn mod_wheel_target(&self) -> f32 {
        self.mod_wheel.target()
    }

    /// Phase 4a D48: LFO Pitch depth target 値 (テスト用)。
    #[doc(hidden)]
    pub fn lfo_pitch_depth_target(&self) -> f32 {
        self.lfo_pitch_depth.target()
    }

    /// Phase 4a D48: LFO Brightness depth target 値 (テスト用)。
    #[doc(hidden)]
    pub fn lfo_brightness_depth_target(&self) -> f32 {
        self.lfo_brightness_depth.target()
    }

    /// Phase 4a D48: LFO Volume depth target 値 (テスト用)。
    #[doc(hidden)]
    pub fn lfo_volume_depth_target(&self) -> f32 {
        self.lfo_volume_depth.target()
    }

    /// Phase 3 D41: Voice State buffer に active mask + 振幅をパック。
    fn write_voice_state(&mut self) {
        let mut mask = 0u8;
        for i in 0..POLYPHONY {
            if let Some(v) = self.pool.voice(i) {
                let amp = v.energy().sqrt();
                if v.is_active() {
                    mask |= 1u8 << i;
                }
                let bytes = amp.to_le_bytes();
                let off = 1 + i * 4;
                self.voice_state_buffer[off..off + 4].copy_from_slice(&bytes);
            }
        }
        self.voice_state_buffer[0] = mask;
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioProcessor for Engine {
    fn prepare(&mut self, sample_rate: f32, max_block_size: usize) {
        self.sample_rate = sample_rate;
        self.pool.prepare(sample_rate, max_block_size);
        self.output_gain
            .set_time_constant(sample_rate, ParamId::OutputGain.descriptor().smoothing_tau);
        self.body_wet
            .set_time_constant(sample_rate, ParamId::BodyWet.descriptor().smoothing_tau);
        self.channel_volume
            .set_time_constant(sample_rate, CHANNEL_VOLUME_TAU);
        self.modal_body.prepare(sample_rate);
        self.pool.set_damping(self.current_damping);

        // Phase 4a D46 / D48 / D49: LFO + Mod Wheel + LFO depth の SmoothedValue を初期化
        self.lfo.prepare(sample_rate);
        self.mod_wheel.set_time_constant(sample_rate, MOD_WHEEL_TAU);
        self.lfo_pitch_depth
            .set_time_constant(sample_rate, LFO_DEPTH_TAU);
        self.lfo_brightness_depth
            .set_time_constant(sample_rate, LFO_DEPTH_TAU);
        self.lfo_volume_depth
            .set_time_constant(sample_rate, LFO_DEPTH_TAU);
    }

    fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32]) {
        debug_assert_eq!(output_l.len(), output_r.len());
        let n = output_l.len();
        for i in 0..n {
            // Phase 4a D46-D49: LFO 値を取得し、Mod Wheel で master 制御。
            let lfo_value = self.lfo.process_sample();
            let mod_wheel_v = self.mod_wheel.next_sample();

            // D48 Pitch destination: Engine 側で exp2 を 1 回だけ計算して全 voice に fan-out。
            let pitch_offset_semitones = lfo_value
                * self.lfo_pitch_depth.next_sample()
                * mod_wheel_v
                * LFO_PITCH_SCALE_SEMITONES;
            let pitch_factor = (-pitch_offset_semitones / 12.0).exp2();
            self.pool.set_lfo_pitch_factor(pitch_factor);

            // D48 Brightness destination: voice 側で `(brightness + offset).clamp(0,1)` として加算。
            let brightness_offset = lfo_value
                * self.lfo_brightness_depth.next_sample()
                * mod_wheel_v
                * LFO_BRIGHTNESS_SCALE;
            self.pool.set_lfo_brightness_offset(brightness_offset);

            // D48 Volume destination: Engine 単位で適用 (per voice 不要)。
            let volume_multiplier = 1.0
                + lfo_value * self.lfo_volume_depth.next_sample() * mod_wheel_v * LFO_VOLUME_SCALE;

            let dry = self.pool.process_sample();
            let (body_l, body_r) = self.modal_body.process_sample(dry);
            let wet = self.body_wet.next_sample();
            let dry_amount = 1.0 - wet;
            let mixed_l = dry_amount * dry + wet * body_l;
            let mixed_r = dry_amount * dry + wet * body_r;
            // D38b + Phase 4a D48: final = output_gain × channel_volume × volume_multiplier
            let combined = self.output_gain.next_sample()
                * self.channel_volume.next_sample()
                * volume_multiplier;
            output_l[i] = soft_clip(mixed_l * combined);
            output_r[i] = soft_clip(mixed_r * combined);
        }
        // JS 側 voice state は VOICE_STATE_WRITE_STRIDE 毎に push されるため、それより
        // 細かく書き込んでも上書きで読み捨てられる。stride を超えた直後の 1 ブロックでだけ書く。
        self.voice_state_sample_counter = self.voice_state_sample_counter.saturating_add(n as u32);
        if self.voice_state_sample_counter >= VOICE_STATE_WRITE_STRIDE {
            self.voice_state_sample_counter = 0;
            self.write_voice_state();
        }
    }

    fn reset(&mut self) {
        self.pool.reset();
        self.pool.set_damping(self.current_damping);
        self.output_gain.set_immediate(OUTPUT_GAIN_DEFAULT);
        self.body_wet.set_immediate(BODY_WET_DEFAULT);
        self.channel_volume.set_immediate(CHANNEL_VOLUME_DEFAULT);
        self.modal_body.reset();
        self.sustain_state.reset();
        self.hold_stack.clear();
        self.voice_state_buffer = [0u8; 33];
        self.voice_state_sample_counter = 0;

        // Phase 4a: LFO / Mod Wheel / LFO depth / 楽器選択を初期状態へ
        self.lfo.reset();
        self.mod_wheel.set_immediate(MOD_WHEEL_DEFAULT);
        self.lfo_pitch_depth.set_immediate(LFO_DEPTH_DEFAULT);
        self.lfo_brightness_depth.set_immediate(LFO_DEPTH_DEFAULT);
        self.lfo_volume_depth.set_immediate(LFO_DEPTH_DEFAULT);
        self.current_instrument = InstrumentKind::Default;
        self.stereo_spread = STEREO_SPREAD_DEFAULT;
        // Phase 4a D52 / D53: reset で modal_body も Default 楽器係数に戻す。
        self.modal_body
            .set_instrument(InstrumentKind::Default, self.sample_rate);
        // Phase 4b D67: Default kind に戻るため dispersion_active も false に
        self.pool.set_dispersion_active(false);

        // Phase 4c D72 / D75 / D77 / D78: Piano パラメータも Default 状態へ初期化。
        self.unison_detune_cents = 0.0;
        self.sympathetic_amount = 0.0;
        self.inharmonicity_b_for_note = b_curve_zero;
        self.hammer_cutoff_low_hz = 0.0;
        self.hammer_cutoff_high_hz = 0.0;
        self.pool.set_piano_params(0.0, 0.0, 0.0, 0.0);
        self.bus_out_prev = 0.0;
    }
}

#[inline]
pub fn midi_to_freq(midi_note: u8) -> f32 {
    440.0 * 2f32.powf((midi_note as f32 - 69.0) / 12.0)
}
