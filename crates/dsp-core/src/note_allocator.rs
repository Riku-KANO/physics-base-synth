//! Voice stealing 戦略 (D13 / D28)。Step 8 では stub、Step 9 で本実装に差し替え。

use crate::traits::Voice;

#[derive(Debug, Clone, Copy)]
pub enum StealResult {
    Index(usize),
}

/// Stub: 常に index 0 を返す。Step 9 で「energy 閾値以下のうち最古」「全 loud なら最古」に
/// 置き換える。
pub fn select_voice_for_steal<V: Voice, const N: usize>(_voices: &[V; N]) -> StealResult {
    StealResult::Index(0)
}
