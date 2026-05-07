//! SustainState テスト (Phase 3 F33)

use dsp_core::sustain_state::SustainState;

#[test]
fn test_sustain_defers_note_off() {
    let mut s = SustainState::new();
    s.set_active(true);
    assert!(s.try_defer_note_off(60));
    assert_eq!(s.pending_release_bitmap() & (1u128 << 60), 1u128 << 60);
}

#[test]
fn test_sustain_release_on_off() {
    let mut s = SustainState::new();
    s.set_active(true);
    s.try_defer_note_off(60);
    s.try_defer_note_off(64);
    s.try_defer_note_off(67);
    let released = s.set_active(false);
    let expected = (1u128 << 60) | (1u128 << 64) | (1u128 << 67);
    assert_eq!(released, expected);
    assert_eq!(s.pending_release_bitmap(), 0);
}

#[test]
fn test_sustain_passthrough_when_inactive() {
    let mut s = SustainState::new();
    assert!(!s.try_defer_note_off(60));
    assert_eq!(s.pending_release_bitmap(), 0);
}

#[test]
fn test_sustain_clear_pending_on_retrigger() {
    let mut s = SustainState::new();
    s.set_active(true);
    s.try_defer_note_off(60);
    s.clear_pending(60);
    let released = s.set_active(false);
    assert_eq!(released, 0, "cleared pending should not be released");
}

#[test]
fn test_sustain_reset_clears_active_and_pending() {
    let mut s = SustainState::new();
    s.set_active(true);
    s.try_defer_note_off(60);
    s.reset();
    assert!(!s.active);
    assert_eq!(s.pending_release_bitmap(), 0);
}

#[test]
fn test_sustain_pending_release_bitmap_readonly() {
    let mut s = SustainState::new();
    s.set_active(true);
    s.try_defer_note_off(60);
    let snap = s.pending_release_bitmap();
    let snap2 = s.pending_release_bitmap();
    assert_eq!(snap, snap2, "read-only API must not mutate state");
    // active も変化しない
    assert!(s.active);
}
