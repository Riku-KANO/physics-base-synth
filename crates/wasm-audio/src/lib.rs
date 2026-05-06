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

/// Phase 2 (D17): mode = 0 → Poly, mode = 1 → Mono。不正値は黙って無視する。
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
