use crate::hold_stack::HoldStack;
use crate::modal_body::ModalBodyResonator;
use crate::params::{ParamId, BODY_WET_DEFAULT, OUTPUT_GAIN_DEFAULT, PICK_POSITION_DEFAULT};
use crate::smoothing::SmoothedValue;
use crate::soft_clip::soft_clip;
use crate::sustain_state::SustainState;
use crate::traits::AudioProcessor;
use crate::voice_pool::{VoicePool, POLYPHONY};

/// Phase 3 D38b: Channel Volume (CC#7) のデフォルト 1.0。OutputGain と直交配置のため、
/// note_on 時の current_damping 同様 const で持つ。
const CHANNEL_VOLUME_DEFAULT: f32 = 1.0;
/// Phase 3 D38b: Channel Volume の SmoothedValue tau (~20 ms)
const CHANNEL_VOLUME_TAU: f32 = 0.02;

/// MIDI CC 番号 (subset)
const CC_MOD_WHEEL: u8 = 1; // Phase 4 送り、no-op
const CC_CHANNEL_VOLUME: u8 = 7;
const CC_SUSTAIN_PEDAL: u8 = 64;
const CC_ALL_NOTES_OFF: u8 = 123;

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
    /// Phase 3 D41: Voice State 共有メモリ (active mask 1 byte + 8 振幅 × 4 bytes = 33 bytes)
    voice_state_buffer: [u8; 33],
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
        }
    }

    /// 新規ノートを発音し、割当先ボイスのみ damping をユーザー値に復元する。
    /// `set_damping_voice` を fan-out にすると release 中ボイスを 0.95 → current_damping に
    /// 巻き戻して再生を「復活」させてしまうため、必ず assigned index にだけ適用する。
    fn trigger_voice(&mut self, midi_note: u8, velocity: f32) {
        let freq = midi_to_freq(midi_note);
        let assigned = self.pool.note_on(midi_note, freq, velocity);
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
                // Phase 4 送り: LFO 仕様確定後に対応 (D39)。現状 no-op。
            }
            CC_CHANNEL_VOLUME => {
                // D38b: OutputGain と直交、final = output_gain * channel_volume
                self.channel_volume.set_target(v);
            }
            CC_SUSTAIN_PEDAL => {
                // ≥ 64 (= 0.5 normalized) で on、それ以下で off
                let active = v >= 0.5;
                let released = self.sustain_state.set_active(active);
                if released != 0 {
                    // active=false 移行時の pending を全 release
                    for note in 0..128_u8 {
                        if (released >> note) & 1 == 1 {
                            self.pool.note_off(note);
                        }
                    }
                }
            }
            CC_ALL_NOTES_OFF => {
                // P1-1: sustain も reset（忘れると古い pending が CC#64 操作で再処理される）
                for note in 0..128_u8 {
                    self.pool.note_off(note);
                }
                self.hold_stack.clear();
                self.sustain_state.reset();
            }
            _ => {} // 未対応 CC は no-op (panic / alloc なし)
        }
    }

    pub fn set_mode(&mut self, mode: SynthMode) {
        // Phase 3 D40 P2-1: 切替前に pending を全 release してから sustain_state.reset()。
        // mode 切替で Sustain pending が宙ぶらりんにならないよう、各 note を即時 release。
        let pending = self.sustain_state.pending_release_bitmap();
        if pending != 0 {
            self.sustain_state.reset();
            for note in 0..128_u8 {
                if (pending >> note) & 1 == 1 {
                    self.pool.note_off(note);
                }
            }
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

    /// Phase 3 D41: Voice State 共有メモリへのポインタ。
    /// 33 bytes (active mask 1 byte + 8 振幅 × 4 bytes、little-endian)。
    /// `Engine::process` 終端で書き込まれる、JS 側からは `Uint8Array` view で読む。
    pub fn voice_state_ptr(&self) -> *const u8 {
        self.voice_state_buffer.as_ptr()
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
    }

    fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32]) {
        debug_assert_eq!(output_l.len(), output_r.len());
        for i in 0..output_l.len() {
            let dry = self.pool.process_sample();
            let (body_l, body_r) = self.modal_body.process_sample(dry);
            let wet = self.body_wet.next_sample();
            let dry_amount = 1.0 - wet;
            let mixed_l = dry_amount * dry + wet * body_l;
            let mixed_r = dry_amount * dry + wet * body_r;
            let g = self.output_gain.next_sample();
            // Phase 3 D38b: final = output_gain * channel_volume (CC#7 と OutputGain は直交)
            let cv = self.channel_volume.next_sample();
            let combined = g * cv;
            // Phase 3 D43: output_gain 後・write 前に区間関数型 soft clip
            output_l[i] = soft_clip(mixed_l * combined);
            output_r[i] = soft_clip(mixed_r * combined);
        }
        // Phase 3 D41: process block 終端で voice state を共有メモリへ書き込み
        self.write_voice_state();
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
    }
}

#[inline]
pub fn midi_to_freq(midi_note: u8) -> f32 {
    440.0 * 2f32.powf((midi_note as f32 - 69.0) / 12.0)
}
