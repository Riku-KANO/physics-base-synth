use crate::fractional_delay::LagrangeCoeffs;
use crate::params::{BRIGHTNESS_DEFAULT, DAMPING_DEFAULT};
use crate::rng::XorShift32;
use crate::smoothing::SmoothedValue;

const NOTE_OFF_DAMPING: f32 = 0.95;
const ENERGY_RISE: f32 = 0.001;
const ENERGY_DECAY: f32 = 0.999;
const ENERGY_THRESHOLD: f32 = 1.0e-9;
const MIN_FREQ_HZ: f32 = 27.5;
/// Lagrange 4 点参照のため write_index 直後の 2 サンプル分の余裕が必要 (D27)
const LAGRANGE_BUFFER_MARGIN: usize = 3;

pub struct KarplusStrong {
    buffer: Vec<f32>,
    write_index: usize,
    /// 整数部のディレイ長
    length_int: usize,
    /// note_on 時にキャッシュした分数部の補間係数 (D26)
    lagrange: LagrangeCoeffs,
    damping: SmoothedValue,
    brightness: SmoothedValue,
    last_filter_out: f32,
    energy: f32,
    active: bool,
    rng: XorShift32,
    sample_rate: f32,
    note_off_target_damping: f32,
    /// 現在発音中の MIDI ノート番号。voice stealing の same-note-replace 判定に使用
    current_note: Option<u8>,
    /// 最後の note_on からの経過サンプル数。voice stealing の oldest 判定に使用
    age_samples: u32,
}

impl KarplusStrong {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            write_index: 0,
            length_int: 0,
            lagrange: LagrangeCoeffs::default(),
            damping: SmoothedValue::new(DAMPING_DEFAULT),
            brightness: SmoothedValue::new(BRIGHTNESS_DEFAULT),
            last_filter_out: 0.0,
            energy: 0.0,
            active: false,
            rng: XorShift32::new(0x1234_5678),
            sample_rate: 44100.0,
            note_off_target_damping: NOTE_OFF_DAMPING,
            current_note: None,
            age_samples: 0,
        }
    }

    pub fn prepare(&mut self, sample_rate: f32, _max_block_size: usize) {
        self.sample_rate = sample_rate;
        let max_buffer_len = (sample_rate / MIN_FREQ_HZ).ceil() as usize + LAGRANGE_BUFFER_MARGIN;
        self.buffer = vec![0.0; max_buffer_len];

        self.damping.set_time_constant(sample_rate, 0.02);
        self.brightness.set_time_constant(sample_rate, 0.02);

        self.write_index = 0;
        self.length_int = 0;
        self.lagrange = LagrangeCoeffs::default();
        self.last_filter_out = 0.0;
        self.energy = 0.0;
        self.active = false;
        self.current_note = None;
        self.age_samples = 0;
    }

    pub fn set_seed(&mut self, seed: u32) {
        self.rng = XorShift32::new(seed);
    }

    /// freq_hz から `length_int + length_frac` を計算し、Lagrange 係数をキャッシュする。
    /// バッファ全体をゼロクリアしてから `[0..length_int]` に励振ノイズを書き、
    /// `write_index = length_int` から書き込み開始する。これにより初回 process_sample で
    /// read 位置 (base - d_int) % buf_len = 0 が励振範囲を指し、補間値が非ゼロになる。
    pub fn note_on(&mut self, freq_hz: f32, velocity: f32) {
        let raw_len = self.sample_rate / freq_hz.max(1.0);
        let max_len = self.buffer.len().saturating_sub(LAGRANGE_BUFFER_MARGIN);
        let len_int = (raw_len.floor() as usize).clamp(3, max_len);
        let len_frac = (raw_len - len_int as f32).clamp(0.0, 1.0);

        self.length_int = len_int;
        self.lagrange = LagrangeCoeffs::new(len_frac);

        for v in self.buffer.iter_mut() {
            *v = 0.0;
        }
        for i in 0..len_int {
            self.buffer[i] = self.rng.next_unit_bipolar() * velocity;
        }

        self.write_index = len_int;
        self.last_filter_out = 0.0;
        self.energy = velocity * velocity;
        self.active = true;
        self.age_samples = 0;
    }

    pub fn note_on_with_id(&mut self, midi_note: u8, freq_hz: f32, velocity: f32) {
        self.note_on(freq_hz, velocity);
        self.current_note = Some(midi_note);
    }

    pub fn note_off(&mut self) {
        self.damping.set_target(self.note_off_target_damping);
        self.current_note = None;
    }

    pub fn set_damping(&mut self, value: f32) {
        self.damping.set_target(value);
    }

    pub fn set_brightness(&mut self, value: f32) {
        self.brightness.set_target(value);
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn length_int(&self) -> usize {
        self.length_int
    }

    pub fn note_id(&self) -> Option<u8> {
        self.current_note
    }

    pub fn age_samples(&self) -> u32 {
        self.age_samples
    }

    pub fn energy(&self) -> f32 {
        self.energy
    }

    pub fn reset(&mut self) {
        for v in self.buffer.iter_mut() {
            *v = 0.0;
        }
        self.write_index = 0;
        self.length_int = 0;
        self.lagrange = LagrangeCoeffs::default();
        self.last_filter_out = 0.0;
        self.energy = 0.0;
        self.active = false;
        self.current_note = None;
        self.age_samples = 0;
    }

    #[inline]
    pub fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        // Lagrange 4 点読み出し。剰余は length_int ではなく buffer.len() で取る (D27)
        // ─ length_int で取ると read_p1 / read_p2 が「より新しい側」に巻き込まれて時系列順にならず、
        //   x[n - D_int - 1] / x[n - D_int - 2] を取れない。
        let buf_len = self.buffer.len();
        let d_int = self.length_int;
        let base = self.write_index + buf_len; // 巻き戻しアンダーフロー回避
        let read_m = (base - d_int + 1) % buf_len; // x[n - D_int + 1]、最新 (h0)
        let read_z = (base - d_int) % buf_len; // x[n - D_int]、中央 (h1)
        let read_p1 = (base - d_int - 1) % buf_len; // x[n - D_int - 1] (h2)
        let read_p2 = (base - d_int - 2) % buf_len; // x[n - D_int - 2]、最古 (h3)

        let read_value = self.lagrange.apply(
            self.buffer[read_m],
            self.buffer[read_z],
            self.buffer[read_p1],
            self.buffer[read_p2],
        );

        let b = self.brightness.next_sample();
        let filtered = b * read_value + (1.0 - b) * self.last_filter_out;
        self.last_filter_out = filtered;

        let d = self.damping.next_sample();
        let mut damped = d * filtered;

        // denormal flush (D6)
        damped += 1.0e-25;
        damped -= 1.0e-25;

        self.buffer[self.write_index] = damped;
        self.write_index = (self.write_index + 1) % buf_len;

        self.energy = self.energy * ENERGY_DECAY + damped * damped * ENERGY_RISE;
        if self.energy < ENERGY_THRESHOLD {
            self.active = false;
        }

        self.age_samples = self.age_samples.saturating_add(1);

        read_value
    }
}

impl Default for KarplusStrong {
    fn default() -> Self {
        Self::new()
    }
}
