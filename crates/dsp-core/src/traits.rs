pub trait AudioProcessor {
    fn prepare(&mut self, sample_rate: f32, max_block_size: usize);
    fn process(&mut self, output_l: &mut [f32], output_r: &mut [f32]);
    fn reset(&mut self);
}

pub trait Voice {
    fn note_on(&mut self, freq_hz: f32, velocity: f32);
    fn note_off(&mut self);
    fn process_sample(&mut self) -> f32;
    fn is_active(&self) -> bool;

    /// voice stealing 判定用 (D19)
    fn note_id(&self) -> Option<u8>;
    fn age(&self) -> u32;
    fn amplitude(&self) -> f32;

    /// Phase 3 D39: Pitch Bend を半音単位でセット (±2 まで)。
    /// SmoothedValue で 5 ms tau の遷移を内部管理。
    fn set_pitch_bend(&mut self, semitones: f32);

    /// Phase 4a D48: LFO Pitch factor を毎 sample 更新 (Engine 側で `exp2(-semitones/12)` 計算済)。
    /// 初期値 1.0 = pitch offset 0 と等価 (Phase 3 互換)。
    fn set_lfo_pitch_factor(&mut self, factor: f32);

    /// Phase 4a D48: LFO Brightness offset を毎 sample 更新。
    /// `process_sample` で `(brightness + offset).clamp(0, 1)` として適用。
    fn set_lfo_brightness_offset(&mut self, offset: f32);
}
