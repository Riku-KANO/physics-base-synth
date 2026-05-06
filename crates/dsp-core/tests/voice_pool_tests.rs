//! VoicePool / voice stealing / no-alloc / RMS 統計境界 (F10 / F11 / F17 / F24)

use dsp_core::engine::{midi_to_freq, Engine};
use dsp_core::params::ParamId;
use dsp_core::traits::AudioProcessor;
use dsp_core::voice_pool::{VoicePool, POLYPHONY};

const SAMPLE_RATE: f32 = 48_000.0;

fn fresh_pool() -> VoicePool<POLYPHONY> {
    let mut pool = VoicePool::new();
    pool.prepare(SAMPLE_RATE, 128);
    pool
}

fn fresh_engine() -> Engine {
    let mut e = Engine::new();
    e.prepare(SAMPLE_RATE, 128);
    e
}

#[test]
fn test_voice_pool_allocates_distinct_voices() {
    // 8 個の異なる midi_note を順に note_on すると 8 ボイスがアクティブ (F10)。
    let mut pool = fresh_pool();
    let notes: [u8; 8] = [60, 62, 64, 65, 67, 69, 71, 72];
    let mut assigned = [0_usize; 8];
    for (i, &n) in notes.iter().enumerate() {
        assigned[i] = pool.note_on(n, midi_to_freq(n), 0.8);
    }
    assert_eq!(pool.active_count(), 8);
    let mut seen = [false; POLYPHONY];
    for &i in &assigned {
        assert!(!seen[i], "voice {i} was assigned twice");
        seen[i] = true;
    }
}

#[test]
fn test_voice_pool_same_note_replace() {
    // 同じ midi_note の note_on は同じボイスに再励振される (active_count 1 のまま、F10)。
    let mut pool = fresh_pool();
    let i1 = pool.note_on(60, midi_to_freq(60), 0.8);
    assert_eq!(pool.active_count(), 1);
    let i2 = pool.note_on(60, midi_to_freq(60), 0.8);
    assert_eq!(i1, i2, "same-note should re-trigger the same voice slot");
    assert_eq!(pool.active_count(), 1);
}

#[test]
fn test_voice_pool_note_on_returns_assigned_index() {
    // pool.note_on の戻り値が割当先ボイスの index と一致 (F10)。
    let mut pool = fresh_pool();
    let i = pool.note_on(60, midi_to_freq(60), 0.8);
    assert!(i < POLYPHONY);
    let voice = pool.voice(i).expect("voice must exist");
    assert_eq!(voice.note_id(), Some(60));
}

#[test]
fn test_engine_note_on_does_not_revive_released_voice() {
    // F10/F11: note_off で 0.95 になった release 中ボイスが、別の note の note_on で
    // current_damping に復元されてしまわないこと (set_damping_voice の存在意義)。
    let mut e = fresh_engine();
    e.set_param(ParamId::Damping as u32, 0.999);
    assert!((e.current_damping() - 0.999).abs() < 1e-6);

    e.note_on(60, 0.8);
    let idx_60 = e.voice_index_for_note(60).expect("voice 60 active");
    {
        let v = e.pool().voice(idx_60).unwrap();
        assert!((v.damping_target() - 0.999).abs() < 1e-6);
    }

    e.note_off(60);
    {
        let v = e.pool().voice(idx_60).unwrap();
        // release 中: damping target は 0.95 (NOTE_OFF_DAMPING)
        assert!(
            (v.damping_target() - 0.95).abs() < 1e-6,
            "voice 60 damping_target after note_off = {}",
            v.damping_target()
        );
    }

    e.note_on(62, 0.8);
    let idx_62 = e.voice_index_for_note(62).expect("voice 62 active");
    assert_ne!(idx_60, idx_62, "voice 62 should land on a different slot");
    {
        // 新規ボイス (62) は current_damping=0.999 に復元される
        let v62 = e.pool().voice(idx_62).unwrap();
        assert!((v62.damping_target() - 0.999).abs() < 1e-6);

        // release 中ボイス (60) は 0.95 のままで復活していない
        let v60 = e.pool().voice(idx_60).unwrap();
        assert!(
            (v60.damping_target() - 0.95).abs() < 1e-6,
            "voice 60 damping_target should remain 0.95 (release), got {}",
            v60.damping_target()
        );
    }
}

#[test]
fn test_voice_pool_steals_quietest() {
    // 8 ボイスを鳴らし、1 つだけ damping=0.9 で速く減衰させてから 9 音目で stealing
    // → そのボイスが選ばれることを確認 (F11/F23)。
    let mut pool = fresh_pool();
    pool.set_damping(0.999);
    let notes: [u8; 8] = [60, 62, 64, 65, 67, 69, 71, 72];
    let mut assigned = [0_usize; 8];
    for (i, &n) in notes.iter().enumerate() {
        assigned[i] = pool.note_on(n, midi_to_freq(n), 0.8);
    }
    // index assigned[2] のボイスのみ damping を 0.9 に下げて速減衰
    pool.set_damping_voice(assigned[2], 0.9);

    // 1 秒間 process_sample を回す → assigned[2] のボイスは振幅閾値を下回る
    for _ in 0..(SAMPLE_RATE as usize) {
        let _ = pool.process_sample();
    }

    // 9 音目 (midi=74) で stealing → 静かなボイス assigned[2] が犠牲になる
    let stolen = pool.note_on(74, midi_to_freq(74), 0.8);
    assert_eq!(
        stolen, assigned[2],
        "stealing should pick voice {} (decayed fastest), got {}",
        assigned[2], stolen
    );
}

#[test]
fn test_voice_pool_polyphonic_mix_rms_bounded() {
    // F24 補助: 8 ボイス全力 (velocity=0.8) で 1 秒間 process_sample → RMS<=0.7、peak<=2.0。
    // 1/sqrt(N) スケールが概ね機能し最悪過渡応答でも 2.0 を超えない統計的保証。
    let mut pool = fresh_pool();
    let notes: [u8; 8] = [60, 62, 64, 65, 67, 69, 71, 72];
    for &n in &notes {
        pool.note_on(n, midi_to_freq(n), 0.8);
    }

    let total = SAMPLE_RATE as usize;
    let mut peak = 0.0_f32;
    let mut sum_sq = 0.0_f64;
    for _ in 0..total {
        let s = pool.process_sample();
        peak = peak.max(s.abs());
        sum_sq += s as f64 * s as f64;
    }
    let rms = (sum_sq / total as f64).sqrt() as f32;
    assert!(peak <= 2.0, "polyphonic peak {peak} > 2.0");
    assert!(rms <= 0.7, "polyphonic RMS {rms} > 0.7");
}

#[test]
fn test_no_allocation_in_polyphonic_process() {
    // F17: 8 ボイス全力で 1 秒分 process_sample 中、各ボイスの length_int (= バッファ
    // 配置長の代理指標) が変化しないことを確認。Phase 1 test_no_allocation_in_process と
    // 同じスタンス (Vec の再確保が起きていれば length_int が変わる)。
    let mut pool = fresh_pool();
    let notes: [u8; 8] = [60, 62, 64, 65, 67, 69, 71, 72];
    for &n in &notes {
        pool.note_on(n, midi_to_freq(n), 0.8);
    }
    let mut len_before = [0_usize; POLYPHONY];
    for (i, item) in len_before.iter_mut().enumerate().take(POLYPHONY) {
        *item = pool.voice_length_int(i).unwrap();
    }
    for _ in 0..(SAMPLE_RATE as usize) {
        let _ = pool.process_sample();
    }
    for (i, &expected_len) in len_before.iter().enumerate() {
        assert_eq!(
            pool.voice_length_int(i).unwrap(),
            expected_len,
            "voice {i} length_int changed during process_sample"
        );
    }
}

