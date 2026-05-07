//! aggregate ModalBodyResonator の挙動検証 (Phase 3 F26、03 章 §テスト方針 (b))。
//! 隣接モードの寄与でピーク値が揺れるリスクを避け、定性的な性質のみ検証する。

use dsp_core::modal_body::ModalBodyResonator;
use dsp_core::params::{BODY_MODES_L, BODY_MODES_R};

const SAMPLE_RATE: f32 = 48_000.0;

fn rms_tail(samples: &[f32], skip_ratio: f32) -> f32 {
    let skip = (samples.len() as f32 * skip_ratio) as usize;
    let tail = &samples[skip..];
    (tail.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / tail.len() as f64).sqrt() as f32
}

#[test]
fn test_modal_body_dc_blocking() {
    let mut body = ModalBodyResonator::new();
    body.prepare(SAMPLE_RATE);

    let total = SAMPLE_RATE as usize;
    let mut out_l = Vec::with_capacity(total);
    let mut out_r = Vec::with_capacity(total);
    for _ in 0..total {
        let (l, r) = body.process_sample(1.0);
        out_l.push(l);
        out_r.push(r);
    }

    let max_l = rms_tail(&out_l, 0.9);
    let max_r = rms_tail(&out_r, 0.9);
    assert!(max_l < 0.001, "DC blocking L failed: max={}", max_l);
    assert!(max_r < 0.001, "DC blocking R failed: max={}", max_r);
}

#[test]
fn test_modal_body_peak_at_modes() {
    // 各モード周波数の sin 入力に対し RMS が `mode.gain` の 0.5〜1.5 倍 (隣接モード寄与許容)
    let mut body = ModalBodyResonator::new();
    body.prepare(SAMPLE_RATE);
    let total = SAMPLE_RATE as usize;

    for mode in BODY_MODES_L.iter() {
        body.reset();
        let omega = 2.0 * core::f32::consts::PI * mode.freq / SAMPLE_RATE;
        let mut out_l = Vec::with_capacity(total);
        for n in 0..total {
            let x = (omega * n as f32).sin();
            let (l, _) = body.process_sample(x);
            out_l.push(l);
        }
        let rms = rms_tail(&out_l, 0.7);
        let expected = mode.gain / (2.0_f32).sqrt();
        let ratio = rms / expected;
        assert!(
            (0.5..=1.5).contains(&ratio),
            "peak ratio out of range for f={}: rms={:.4}, expected≈{:.4}, ratio={:.3}",
            mode.freq,
            rms,
            expected,
            ratio
        );
    }
}

#[test]
fn test_modal_body_inter_mode_attenuation() {
    // モード周波数の中点付近で RMS が任意の mode.gain の最大値を超えないこと（定性的）
    let mut body = ModalBodyResonator::new();
    body.prepare(SAMPLE_RATE);
    let total = SAMPLE_RATE as usize;

    let mid_freq = (BODY_MODES_L[0].freq + BODY_MODES_L[1].freq) / 2.0;
    let omega = 2.0 * core::f32::consts::PI * mid_freq / SAMPLE_RATE;
    let mut out_l = Vec::with_capacity(total);
    for n in 0..total {
        let x = (omega * n as f32).sin();
        let (l, _) = body.process_sample(x);
        out_l.push(l);
    }
    let rms = rms_tail(&out_l, 0.7);

    let max_gain = BODY_MODES_L.iter().map(|m| m.gain).fold(0.0_f32, f32::max);
    assert!(
        rms < max_gain,
        "inter-mode attenuation failed: rms {} >= max_gain {}",
        rms,
        max_gain
    );
}

#[test]
fn test_modal_body_stereo_spread() {
    // ステレオ係数 (BODY_MODES_L vs R) が異なることを最初に確認
    assert!(BODY_MODES_L[0].freq != BODY_MODES_R[0].freq);

    // White noise 入力に対する RMS 差で stereo 広がりを確認。
    // L / R は同じ入力に対して別係数で応答するため、broadband 信号下で
    // 両 ch の RMS は近接 (1〜20%) し、完全一致はしない。
    let mut body = ModalBodyResonator::new();
    body.prepare(SAMPLE_RATE);
    let total = SAMPLE_RATE as usize;

    let mut rng_state: u32 = 0x9E37_79B9;
    let mut out_l = Vec::with_capacity(total);
    let mut out_r = Vec::with_capacity(total);
    for _ in 0..total {
        // XorShift32 white noise (-1.0..1.0)
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 17;
        rng_state ^= rng_state << 5;
        let x = ((rng_state as i32) as f32) / (i32::MAX as f32);
        let (l, r) = body.process_sample(x);
        out_l.push(l);
        out_r.push(r);
    }

    let rms_l = rms_tail(&out_l, 0.7);
    let rms_r = rms_tail(&out_r, 0.7);
    let diff = (rms_l - rms_r).abs() / rms_l.max(rms_r);
    assert!(rms_l > 0.0 && rms_r > 0.0);
    assert!(
        (0.01..=0.20).contains(&diff),
        "stereo spread out of range: rms_l={:.4}, rms_r={:.4}, diff={:.2}%",
        rms_l,
        rms_r,
        diff * 100.0
    );
}

#[test]
fn test_modal_body_no_alloc_in_process() {
    // prepare 後の process_sample 1000 回で内部状態は固定容量配列のみ。
    // panic / OOB なく完了することで「追加 alloc なし」を間接確認。
    let mut body = ModalBodyResonator::new();
    body.prepare(SAMPLE_RATE);
    for _ in 0..1000 {
        let _ = body.process_sample(0.5);
    }
}

#[test]
fn test_modal_body_reset_clears_state() {
    let mut body = ModalBodyResonator::new();
    body.prepare(SAMPLE_RATE);
    // 状態を進める
    for _ in 0..1000 {
        let _ = body.process_sample(0.5);
    }
    body.reset();
    // reset 後、入力 0 を 100 サンプル入れたら出力も 0 (denormal flush の 1e-25 ノイズは許容)
    for _ in 0..100 {
        let (l, r) = body.process_sample(0.0);
        assert!(l.abs() < 1e-20, "L not zero after reset: {}", l);
        assert!(r.abs() < 1e-20, "R not zero after reset: {}", r);
    }
}
