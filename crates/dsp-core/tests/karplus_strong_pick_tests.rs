//! Pick position 励振 shaping のテスト (Phase 3 F28、03 章 §Pick position)
//!
//! Phase 4a Step 3 (D45/F44) で `excitation_snapshot` を `#[cfg(test)]` ガードに変更したため、
//! それを参照していた 4 つの test 関数は `crates/dsp-core/src/karplus_strong.rs` 内の
//! `#[cfg(test)] mod excitation_tests` に移動した。
//! ここには private state を必要としない buffer capacity 不変テストのみを残す。

use dsp_core::karplus_strong::KarplusStrong;

const SAMPLE_RATE: f32 = 48_000.0;

fn fresh(beta: f32) -> KarplusStrong {
    let mut v = KarplusStrong::new();
    v.prepare(SAMPLE_RATE, 128);
    v.set_pick_position(beta);
    v
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
