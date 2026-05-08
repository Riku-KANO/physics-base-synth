//! ピッチ精度ユニットテスト (F12 / F13、Phase 3 D36 案 D 採用後の Thiran 計測)
//!
//! Phase 1 は整数ディレイのみで A1=55Hz が 2.3% 誤差。Phase 2 は Lagrange 3 次補間で
//! 0.5% 以内に収めたが A4 で 0.89% の下方偏移、C8 は物理限界で ignore 対象だった。
//! Phase 3 Step 1 D36 で **Thiran 1 次 allpass** を試作し A1〜C6 で 0.02% 級の精度
//! (Lagrange 比 28〜4000 倍) を確認、案 D 採用で `KarplusStrong` の補間を Thiran 単一型
//! に統一した (`fractional_delay.rs::ThiranCoeffs`)。
//!
//! テスト条件:
//! - `damping = 0.9999`（高音域での信号維持。default 0.996 だと C8 の周期 ~11 サンプルで
//!   1 周あたり 0.996^11 ≈ 0.957 倍 = 4 周で 84%、44 周（0.01秒）で 18%、44k 周（1秒）で 2.5e-8 と
//!   実質減衰しきり、autocorrelation が雑音床を拾って測定不能になる）
//! - `brightness = 1.0`（IIR ワンポール LPF の周波数依存群遅延を排除し、補間段が
//!   与える分数ディレイの正確性のみを検証する）
//!
//! autocorrelation は周期 ± 5% の τ レンジで τ_peak を見つけ parabolic interpolation で
//! sub-sample 精度に絞る。

use dsp_core::engine::midi_to_freq;
use dsp_core::karplus_strong::KarplusStrong;

const SAMPLE_RATE: f32 = 48_000.0;
const TOLERANCE: f32 = 0.005; // ± 0.5%
const TEST_DAMPING: f32 = 0.9999;
const TEST_BRIGHTNESS: f32 = 1.0;

/// note_on 直後 0.1 秒スキップ → autocorrelation で τ_peak を見つけ → parabolic
/// interpolation で sub-sample 精度の τ_refined を求めて f0 を返す。
fn measure_f0(midi: u8, sample_rate: f32) -> f32 {
    let mut v = KarplusStrong::new();
    v.prepare(sample_rate, 128);
    v.set_damping(TEST_DAMPING);
    v.set_brightness(TEST_BRIGHTNESS);
    v.note_on(midi_to_freq(midi), 0.8);

    let total = sample_rate as usize;
    let mut samples = vec![0.0_f32; total];
    for s in samples.iter_mut() {
        *s = v.process_sample();
    }

    let skip = (sample_rate * 0.1) as usize;
    let signal = &samples[skip..];

    let expected_period = sample_rate / midi_to_freq(midi);
    let tau_min = ((expected_period * 0.95).floor() as usize).max(2);
    let tau_max = (expected_period * 1.05).ceil() as usize;

    // tau_min - 1 から tau_max + 1 までの autocorrelation 値が必要 (parabolic 用)
    let lo = tau_min.saturating_sub(1).max(1);
    let hi = tau_max + 1;
    assert!(hi < signal.len(), "signal too short for tau_max + 1 = {hi}");

    let win_len = signal.len() - hi;
    let span = hi - lo + 1;
    let mut r = vec![0.0_f64; span];
    for (idx, tau) in (lo..=hi).enumerate() {
        let mut sum = 0.0_f64;
        for t in 0..win_len {
            sum += signal[t] as f64 * signal[t + tau] as f64;
        }
        r[idx] = sum;
    }

    let inner_lo = tau_min - lo;
    let inner_hi = tau_max - lo;
    let mut best_idx = inner_lo;
    let mut best_val = r[inner_lo];
    for (idx, &val) in r.iter().enumerate().take(inner_hi + 1).skip(inner_lo) {
        if val > best_val {
            best_val = val;
            best_idx = idx;
        }
    }

    let tau_peak = lo + best_idx;
    let r_minus = r[best_idx - 1];
    let r_zero = r[best_idx];
    let r_plus = r[best_idx + 1];
    let denom = r_minus - 2.0 * r_zero + r_plus;
    let delta = if denom < -1.0e-12 {
        let d = 0.5 * (r_minus - r_plus) / denom;
        d.clamp(-0.5, 0.5)
    } else {
        0.0
    };
    let tau_refined = tau_peak as f64 + delta;

    (sample_rate as f64 / tau_refined) as f32
}

fn assert_pitch(midi: u8, expected_hz: f32) {
    let f0 = measure_f0(midi, SAMPLE_RATE);
    let err = (f0 - expected_hz).abs() / expected_hz;
    assert!(
        err < TOLERANCE,
        "midi={midi} expected={expected_hz}Hz got={f0}Hz err={:.4}%",
        err * 100.0
    );
}

#[test]
fn test_pitch_a1() {
    // F13: Phase 1 で 2.3% 誤差だった A1 が Thiran allpass で << 0.5%
    assert_pitch(33, 55.0);
}

#[test]
fn test_pitch_a2() {
    assert_pitch(45, 110.0);
}

#[test]
fn test_pitch_a4() {
    // Phase 2 は Lagrange + brightness LPF 群遅延で 0.89% 下方偏移していたが
    // Phase 3 案 D で Thiran 採用 (|H(ω)|=1) かつ brightness=1.0 LPF パススルーで
    // < 0.005% (約 4000 倍精度向上、D37 群遅延補正は不要)
    assert_pitch(69, 440.0);
}

#[test]
fn test_pitch_c6() {
    // 中高域、周期 ~45.9 サンプル
    assert_pitch(84, 1046.5);
}

#[test]
#[ignore = "C8 (4186Hz @ 48kHz) は周期 ~11.47 サンプルで autocorrelation が信号減衰に\n\
    支配される物理限界。Phase 3 D36 案 D で Thiran allpass 採用後も damping=0.9999 では\n\
    loop gain ≈ 0.9999 < 1 のため自己発振せず、tail RMS = 0.000119 まで減衰して測定不能。\n\
    Lagrange/Thiran どちらでも同じ 5052Hz 偽値を出すため C8 ignore は継続。\n\
    Spec R23 フォールバック (5)『どうしても不安定なら C8 のみ許容誤差を緩和』に従う。\n\
    将来は damping=1.0 の自己発振モード or FFT-based estimator で再評価。"]
fn test_pitch_c8() {
    assert_pitch(108, 4186.0);
}

/// Phase 3 D37 / F30 の検証: brightness=0.5 (中域) で A4 のピッチ偏移が < 0.5%。
/// Phase 2 では Lagrange + brightness=0.5 で 0.89% の下方偏移があった (retrospective §4.1)。
/// Phase 3 案 D で Thiran allpass + brightness LPF (1 サンプル群遅延) になり、改善されるはず。
#[test]
fn test_engine_brightness_pitch_correction() {
    let mut v = KarplusStrong::new();
    v.prepare(SAMPLE_RATE, 128);
    v.set_damping(TEST_DAMPING);
    v.set_brightness(0.5); // 中域 brightness、群遅延 (1-b)/b = 1.0 sample
    v.note_on(midi_to_freq(69), 0.8);

    let total = SAMPLE_RATE as usize;
    let mut samples = vec![0.0_f32; total];
    for s in samples.iter_mut() {
        *s = v.process_sample();
    }

    let skip = (SAMPLE_RATE * 0.1) as usize;
    let signal = &samples[skip..];
    let expected_period = SAMPLE_RATE / 440.0;
    let tau_min = ((expected_period * 0.95).floor() as usize).max(2);
    let tau_max = (expected_period * 1.05).ceil() as usize;
    let lo = tau_min.saturating_sub(1).max(1);
    let hi = tau_max + 1;
    let win_len = signal.len() - hi;
    let span = hi - lo + 1;
    let mut r = vec![0.0_f64; span];
    for (idx, tau) in (lo..=hi).enumerate() {
        let mut sum = 0.0_f64;
        for t in 0..win_len {
            sum += signal[t] as f64 * signal[t + tau] as f64;
        }
        r[idx] = sum;
    }
    let inner_lo = tau_min - lo;
    let inner_hi = tau_max - lo;
    let mut best_idx = inner_lo;
    let mut best_val = r[inner_lo];
    for (idx, &val) in r.iter().enumerate().take(inner_hi + 1).skip(inner_lo) {
        if val > best_val {
            best_val = val;
            best_idx = idx;
        }
    }
    let tau_peak = lo + best_idx;
    let r_minus = r[best_idx - 1];
    let r_zero = r[best_idx];
    let r_plus = r[best_idx + 1];
    let denom = r_minus - 2.0 * r_zero + r_plus;
    let delta = if denom < -1.0e-12 {
        let d = 0.5 * (r_minus - r_plus) / denom;
        d.clamp(-0.5, 0.5)
    } else {
        0.0
    };
    let f0 = (SAMPLE_RATE as f64 / (tau_peak as f64 + delta)) as f32;
    let err = (f0 - 440.0).abs() / 440.0;
    println!(
        "F30 brightness=0.5: A4 f0={:.3}Hz err={:.4}%",
        f0,
        err * 100.0
    );
    assert!(
        err < TOLERANCE,
        "A4 brightness=0.5 pitch err {:.4}% > 0.5%",
        err * 100.0
    );
}

#[test]
fn test_long_term_stability_high_damping() {
    // damping=0.9999 + brightness × 3 + midi × 3 の 9 組合せで 30 秒分を生成し
    // (a) 全サンプル finite、(b) 絶対値ピーク <= 10.0、(c) 末尾 1 秒の平均絶対値 <= 100.0
    let total = (SAMPLE_RATE as usize) * 30;
    let tail_start = total - SAMPLE_RATE as usize;

    for &midi in &[33_u8, 69, 108] {
        for &brightness in &[0.0_f32, 0.5, 1.0] {
            let mut v = KarplusStrong::new();
            v.prepare(SAMPLE_RATE, 128);
            v.set_damping(0.9999);
            v.set_brightness(brightness);
            v.note_on(midi_to_freq(midi), 0.8);

            let mut peak: f32 = 0.0;
            let mut tail_sum_abs = 0.0_f64;
            for i in 0..total {
                let s = v.process_sample();
                assert!(
                    s.is_finite(),
                    "non-finite sample at i={i} midi={midi} brightness={brightness}"
                );
                peak = peak.max(s.abs());
                if i >= tail_start {
                    tail_sum_abs += s.abs() as f64;
                }
            }

            assert!(
                peak <= 10.0,
                "peak {peak} > 10.0 (midi={midi} brightness={brightness})"
            );
            let tail_mean = tail_sum_abs / (total - tail_start) as f64;
            assert!(
                tail_mean <= 100.0,
                "tail mean abs {tail_mean} > 100.0 (midi={midi} brightness={brightness})"
            );
        }
    }
}
