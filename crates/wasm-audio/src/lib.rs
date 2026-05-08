// C ABI 公開関数は raw pointer を介して JS から呼ばれる前提のため、
// 各関数は内部で null チェックしてから unsafe block で deref する。
// `not_unsafe_ptr_arg_deref` は本クレートの C ABI 設計と相容れないため crate 全体で allow する。
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use dsp_core::engine::{Engine, SynthMode};
use dsp_core::traits::AudioProcessor;

#[repr(C)]
pub struct SynthHandle {
    engine: Engine,
    scratch_l: Vec<f32>,
    scratch_r: Vec<f32>,
}

#[unsafe(no_mangle)]
pub extern "C" fn synth_new(sample_rate: f32, max_block_size: u32) -> *mut SynthHandle {
    let max = max_block_size as usize;
    let mut engine = Engine::new();
    engine.prepare(sample_rate, max);
    let handle = Box::new(SynthHandle {
        engine,
        scratch_l: vec![0.0; max],
        scratch_r: vec![0.0; max],
    });
    Box::into_raw(handle)
}

#[unsafe(no_mangle)]
pub extern "C" fn synth_free(handle: *mut SynthHandle) {
    if handle.is_null() {
        return;
    }
    unsafe { drop(Box::from_raw(handle)) };
}

#[unsafe(no_mangle)]
pub extern "C" fn synth_note_on(handle: *mut SynthHandle, midi_note: u8, velocity: f32) {
    if handle.is_null() {
        return;
    }
    let h = unsafe { &mut *handle };
    h.engine.note_on(midi_note, velocity);
}

#[unsafe(no_mangle)]
pub extern "C" fn synth_note_off(handle: *mut SynthHandle, midi_note: u8) {
    if handle.is_null() {
        return;
    }
    let h = unsafe { &mut *handle };
    h.engine.note_off(midi_note);
}

#[unsafe(no_mangle)]
pub extern "C" fn synth_set_param(handle: *mut SynthHandle, id: u32, value: f32) {
    if handle.is_null() {
        return;
    }
    let h = unsafe { &mut *handle };
    h.engine.set_param(id, value);
}

#[unsafe(no_mangle)]
pub extern "C" fn synth_reset(handle: *mut SynthHandle) {
    if handle.is_null() {
        return;
    }
    let h = unsafe { &mut *handle };
    h.engine.reset();
}

#[unsafe(no_mangle)]
pub extern "C" fn synth_out_l_ptr(handle: *const SynthHandle) -> *const f32 {
    if handle.is_null() {
        return core::ptr::null();
    }
    let h = unsafe { &*handle };
    h.scratch_l.as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn synth_out_r_ptr(handle: *const SynthHandle) -> *const f32 {
    if handle.is_null() {
        return core::ptr::null();
    }
    let h = unsafe { &*handle };
    h.scratch_r.as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn synth_capacity(handle: *const SynthHandle) -> u32 {
    if handle.is_null() {
        return 0;
    }
    let h = unsafe { &*handle };
    h.scratch_l.len() as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn synth_process_block(handle: *mut SynthHandle, frames: u32) {
    if handle.is_null() {
        return;
    }
    let h = unsafe { &mut *handle };
    let n = (frames as usize).min(h.scratch_l.len());
    h.engine
        .process(&mut h.scratch_l[..n], &mut h.scratch_r[..n]);
}

/// mode = 0 → Poly, mode = 1 → Mono (D17)。不正値は黙って無視する。
#[unsafe(no_mangle)]
pub extern "C" fn synth_set_polyphony_mode(handle: *mut SynthHandle, mode: u32) {
    if handle.is_null() {
        return;
    }
    let h = unsafe { &mut *handle };
    let synth_mode = match mode {
        0 => SynthMode::Poly,
        1 => SynthMode::Mono,
        _ => return,
    };
    h.engine.set_mode(synth_mode);
}

/// Phase 3 D38: MIDI CC dispatch (CC#7 / #64 / #123 のみ対応、その他は no-op)。
/// `value_normalized ∈ [0, 1]` は呼び元 (JS) で `cc_value / 127.0` で正規化。
#[unsafe(no_mangle)]
pub extern "C" fn synth_midi_cc(handle: *mut SynthHandle, cc: u8, value_normalized: f32) {
    if handle.is_null() {
        return;
    }
    let h = unsafe { &mut *handle };
    h.engine.handle_midi_cc(cc, value_normalized);
}

/// Phase 3 D38 / D39: Pitch Bend (±2 半音) を全 active voice に fan-out。
#[unsafe(no_mangle)]
pub extern "C" fn synth_pitch_bend(handle: *mut SynthHandle, semitones: f32) {
    if handle.is_null() {
        return;
    }
    let h = unsafe { &mut *handle };
    h.engine.handle_pitch_bend(semitones);
}

/// Phase 3 D41: Voice State 共有メモリへのポインタ (33 bytes)。
/// レイアウト: byte 0 = active mask (8 voice 分の bit)、bytes 1..33 = 8 振幅 × f32 little-endian。
#[unsafe(no_mangle)]
pub extern "C" fn synth_voice_state_ptr(handle: *const SynthHandle) -> *const u8 {
    if handle.is_null() {
        return core::ptr::null();
    }
    let h = unsafe { &*handle };
    h.engine.voice_state_ptr()
}

/// Phase 4a D52 / D53: 楽器プリセット切替。
/// `kind`: 0=Default, 1=GuitarClassical, 2=Ukulele, 3=Mandolin, 4=Bass, 5=GuitarSteel, 6=Sitar
/// 不正値 (7 以上) は黙って無視する (synth_set_polyphony_mode と同じ防御的設計)。
/// 内部で pool.all_notes_off() + Modal 係数差し替え + reset を実行する。
#[unsafe(no_mangle)]
pub extern "C" fn synth_apply_instrument(handle: *mut SynthHandle, kind: u32) {
    if handle.is_null() {
        return;
    }
    let h = unsafe { &mut *handle };
    if let Some(instrument_kind) = dsp_core::params::InstrumentKind::from_u32(kind) {
        h.engine.apply_instrument(instrument_kind);
    }
}

/// Phase 4a D46: LFO レート設定 (0.1〜8.0 Hz、SmoothedValue tau=0.05s で平滑化)。
/// 範囲外の値は dsp-core 側で clamp。
#[unsafe(no_mangle)]
pub extern "C" fn synth_lfo_set_rate(handle: *mut SynthHandle, hz: f32) {
    if handle.is_null() {
        return;
    }
    let h = unsafe { &mut *handle };
    h.engine.lfo_set_rate(hz);
}

/// Phase 4a D47: LFO 波形設定。
/// `kind`: 0=Sine, 1=Triangle。不正値は無視する。
#[unsafe(no_mangle)]
pub extern "C" fn synth_lfo_set_waveform(handle: *mut SynthHandle, kind: u32) {
    if handle.is_null() {
        return;
    }
    let h = unsafe { &mut *handle };
    if let Some(waveform) = dsp_core::lfo::LfoWaveform::from_u32(kind) {
        h.engine.lfo_set_waveform(waveform);
    }
}

/// Phase 4a D48: LFO destination depth 設定。
/// `dest`: 0=Pitch, 1=Brightness, 2=Volume
/// `depth`: 0.0〜1.0 (dsp-core 側で clamp)
/// 不正な dest は無視する。
#[unsafe(no_mangle)]
pub extern "C" fn synth_lfo_set_depth(handle: *mut SynthHandle, dest: u32, depth: f32) {
    if handle.is_null() {
        return;
    }
    let h = unsafe { &mut *handle };
    if let Some(destination) = dsp_core::lfo::LfoDestination::from_u32(dest) {
        h.engine.lfo_set_depth(destination, depth);
    }
}
