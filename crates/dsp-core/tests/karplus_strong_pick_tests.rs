//! Pick position 励振 shaping のテスト (Phase 3 F28、03 章 §Pick position)
//!
//! 励振直後の buffer に対し autocorrelation / energy で comb shape の効果を検証する。
//! FFT は外部 crate 禁止のため使わず、time-domain の自己相関のみで定性的に検証。

use dsp_core::karplus_strong::KarplusStrong;

const SAMPLE_RATE: f32 = 48_000.0;

fn fresh(beta: f32) -> KarplusStrong {
    let mut v = KarplusStrong::new();
    v.prepare(SAMPLE_RATE, 128);
    v.set_pick_position(beta);
    v
}

fn rms(samples: &[f32]) -> f32 {
    let sq: f64 = samples.iter().map(|x| (*x as f64).powi(2)).sum();
    (sq / samples.len() as f64).sqrt() as f32
}

/// 自己相関 (lag k) の正規化値
fn autocorr_normalized(samples: &[f32], lag: usize) -> f32 {
    if lag >= samples.len() {
        return 0.0;
    }
    let mut sum_xy = 0.0_f64;
    let mut sum_xx = 0.0_f64;
    for i in 0..samples.len() - lag {
        sum_xy += samples[i] as f64 * samples[i + lag] as f64;
        sum_xx += (samples[i] as f64).powi(2);
    }
    if sum_xx > 0.0 {
        (sum_xy / sum_xx) as f32
    } else {
        0.0
    }
}

#[test]
fn test_pick_min_beta_minimal_shape() {
    // β=0.05 (公開 API 最小値) では comb shaping は控えめ。励振 buffer の
    // RMS が β=0.5 と比べて違いが出る（β=0.5 は強い comb shape で振幅減）。
    let mut v_low = fresh(0.05);
    v_low.note_on(440.0, 0.8);
    let buf_low = v_low.excitation_snapshot();

    let mut v_high = fresh(0.5);
    v_high.note_on(440.0, 0.8);
    let buf_high = v_high.excitation_snapshot();

    let rms_low = rms(&buf_low);
    let rms_high = rms(&buf_high);
    // β=0.5 の方が comb キャンセルで RMS が下がる傾向（厳密ではないが定性的）
    println!(
        "rms_low(β=0.05)={:.4}, rms_high(β=0.5)={:.4}",
        rms_low, rms_high
    );
    assert!(buf_low.len() == buf_high.len());
    assert!(rms_low > 0.0 && rms_high > 0.0);
    // β=0.05 と β=0.5 で励振 buffer が一致しない（shape が効いている）ことだけ確認
    let mut differs = false;
    for (a, b) in buf_low.iter().zip(buf_high.iter()) {
        if (a - b).abs() > 1e-6 {
            differs = true;
            break;
        }
    }
    assert!(differs, "β=0.05 vs β=0.5 で励振 buffer が同一");
}

#[test]
fn test_pick_position_node_at_beta_half() {
    // β=0.5 では K = L/2 で comb shaping が「buffer[i] -= buffer[i - L/2]」となり、
    // 周期 L/2 (= 2 倍音) の成分が大きく減衰する。autocorr at lag=L/2 が低くなる。
    let mut v = fresh(0.5);
    v.note_on(440.0, 0.8);
    let buf = v.excitation_snapshot();
    let l = buf.len();

    // 比較対象: β=0.05 (shape ほぼなし) での autocorr at L/2
    let mut v_ref = fresh(0.05);
    v_ref.note_on(440.0, 0.8);
    let buf_ref = v_ref.excitation_snapshot();

    // K = round(0.5 · L) は length_int に対して comb null となる lag
    let k_high = ((0.5 * l as f32).round()).clamp(0.0, (l - 1) as f32) as usize;
    let ac_at_k = autocorr_normalized(&buf, k_high);
    let ac_at_k_ref = autocorr_normalized(&buf_ref, k_high);
    println!(
        "β=0.5 ac[K={}]={:.4}, β=0.05 ac[K={}]={:.4}",
        k_high, ac_at_k, k_high, ac_at_k_ref
    );
    // β=0.5 では K で強い anti-correlation（負方向）が出る、β=0.05 では弱い
    assert!(
        ac_at_k < -0.3,
        "β=0.5 anti-correlation at K should be strong (< -0.3): got {:.4}",
        ac_at_k
    );
    assert!(
        ac_at_k < ac_at_k_ref,
        "β=0.5 anti-correlation should be more negative than β=0.05"
    );
}

#[test]
fn test_pick_position_attenuates_kth_harmonic() {
    // β = 1/k (k=2,3,4) で K = L/k → 周期 L/k 成分が減衰、
    // autocorr at lag=L/k が β 比較で低くなる。
    for k in 2..=4 {
        let beta = 1.0 / k as f32;
        let mut v = fresh(beta);
        v.note_on(440.0, 0.8);
        let buf = v.excitation_snapshot();
        let l = buf.len();
        // 実装と同じ K 計算
        let lag = ((beta * l as f32).round()).clamp(0.0, (l - 1) as f32) as usize;

        let mut v_ref = fresh(0.05);
        v_ref.note_on(440.0, 0.8);
        let buf_ref = v_ref.excitation_snapshot();

        let ac = autocorr_normalized(&buf, lag);
        let ac_ref = autocorr_normalized(&buf_ref, lag);
        println!(
            "k={} β={:.3} ac[K={}]={:.4} ref={:.4}",
            k, beta, lag, ac, ac_ref
        );
        // anti-correlation の負方向に β=1/k のほうが強い
        assert!(
            ac < ac_ref,
            "k={}: β=1/k anti-correlation should be more negative than β=0.05: got {:.4} ref={:.4}",
            k,
            ac,
            ac_ref
        );
    }
}

#[test]
fn test_pick_position_no_extra_alloc() {
    // β を変えて note_on 連打、buffer 容量が不変
    let mut v = fresh(0.125);
    let baseline = v.buffer_capacity();
    for &beta in &[0.05, 0.125, 0.25, 0.5, 0.05] {
        v.set_pick_position(beta);
        for &midi_freq in &[55.0, 110.0, 440.0, 1046.5, 2000.0] {
            v.note_on(midi_freq, 0.8);
            assert_eq!(
                v.buffer_capacity(),
                baseline,
                "buffer.len() changed at β={} freq={}",
                beta,
                midi_freq
            );
        }
    }
}

#[test]
fn test_pick_internal_k_zero_branch() {
    // length_int=9 + β=0.05 で 9 * 0.05 = 0.45 → round = 0、K=0 分岐で
    // comb shaping を skip し、入力素通しで note_on 完了することを確認。
    // f32::round は half-away-from-zero なので length_int=10 だと積=0.5→round=1 で K=0 にならない。
    // Phase 3 D37 補正を avoid するため brightness=1.0 で τ_g=0 にしてから note_on。
    let mut v = KarplusStrong::new();
    v.prepare(SAMPLE_RATE, 128);
    v.set_brightness(1.0);
    v.note_on_with_length_for_test(9, 0.05, 0.8);
    assert!(v.is_active());
    let buf = v.excitation_snapshot();
    assert_eq!(buf.len(), 9);
    // K=0 分岐ではノイズがそのまま残るため、平均 0 だが |sum| は |noise|/sqrt(9) 程度
    let max_abs = buf.iter().map(|x| x.abs()).fold(0.0_f32, f32::max);
    assert!(
        max_abs > 0.0 && max_abs <= 0.8 + 1e-6,
        "noise burst out of range: {}",
        max_abs
    );
}
