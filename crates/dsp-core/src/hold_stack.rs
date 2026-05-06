//! mono モードでの last-note priority 復帰挙動を支える固定容量スタック (D16 / D23 / D29)。
//!
//! - 容量 16: PC キーボード 15 鍵 + 余裕。
//! - 溢れ時は最古破棄: 17 鍵以上同時押下しても最新の鍵は必ず残る。
//! - 依存ゼロ方針 (D23) 維持のため `heapless` 等は使わず自前で実装する。

pub const MAX_HELD: usize = 16;

/// 固定容量 N の自前 LIFO スタック。push/remove/top/clear/len/is_empty を提供する。
/// - `push`: 容量超なら最古を破棄して詰めて末尾に追加
/// - `remove`: 指定値を 1 件だけ削除し残りを詰める
/// - `top`: 末尾 (最後に push された値) を返す。pop はしない
pub struct LinearStack<T: Copy + PartialEq, const N: usize> {
    items: [Option<T>; N],
    len: usize,
}

impl<T: Copy + PartialEq, const N: usize> LinearStack<T, N> {
    pub fn new() -> Self {
        Self {
            items: [None; N],
            len: 0,
        }
    }

    pub fn push(&mut self, value: T) {
        if self.len < N {
            self.items[self.len] = Some(value);
            self.len += 1;
        } else {
            // 容量超: 最古 (index 0) を破棄、全要素を 1 つ前にシフト、新規を末尾に
            for i in 0..(N - 1) {
                self.items[i] = self.items[i + 1];
            }
            self.items[N - 1] = Some(value);
        }
    }

    /// 既存の同値を 1 件除去してから末尾に追加する (LRU 風)。
    /// MIDI の重複 noteOn では同じノートが 2 回以上来うるが、hold-stack 上では
    /// 「最後に押された位置」だけを保持したいので、`push` ではなくこちらを使う。
    pub fn push_unique(&mut self, value: T) {
        self.remove(value);
        self.push(value);
    }

    pub fn remove(&mut self, value: T) {
        let mut found_at: Option<usize> = None;
        for i in 0..self.len {
            if self.items[i] == Some(value) {
                found_at = Some(i);
                break;
            }
        }
        if let Some(pos) = found_at {
            for i in pos..(self.len - 1) {
                self.items[i] = self.items[i + 1];
            }
            self.items[self.len - 1] = None;
            self.len -= 1;
        }
    }

    pub fn top(&self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.items[self.len - 1]
        }
    }

    pub fn clear(&mut self) {
        self.items = [None; N];
        self.len = 0;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<T: Copy + PartialEq, const N: usize> Default for LinearStack<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

/// mono モードでの押下中ノート履歴。
pub type HoldStack = LinearStack<u8, MAX_HELD>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hold_stack_push_pop_basic() {
        let mut s: HoldStack = HoldStack::new();
        assert!(s.is_empty());
        s.push(60);
        s.push(62);
        assert_eq!(s.len(), 2);
        assert_eq!(s.top(), Some(62));
        s.remove(62);
        assert_eq!(s.top(), Some(60));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_hold_stack_overflow_drops_oldest() {
        // 17 件 push すると最古 (1 件目) が破棄され、残り 16 件は新しい順に保持される
        let mut s: HoldStack = HoldStack::new();
        for v in 0..(MAX_HELD as u8 + 1) {
            s.push(v);
        }
        assert_eq!(s.len(), MAX_HELD);
        // 最古 (=0) は消えている。残るのは 1..=16
        assert_eq!(s.top(), Some(MAX_HELD as u8));
        // 0 を remove してもサイズが変化しない (もう存在しない)
        let len_before = s.len();
        s.remove(0);
        assert_eq!(s.len(), len_before);
    }

    #[test]
    fn test_hold_stack_remove_middle() {
        // 中間値を remove しても先頭・末尾の順序は壊れない
        let mut s: HoldStack = HoldStack::new();
        s.push(60);
        s.push(62);
        s.push(64);
        s.push(65);
        s.remove(62);
        assert_eq!(s.len(), 3);
        assert_eq!(s.top(), Some(65));
        s.remove(65);
        assert_eq!(s.top(), Some(64));
    }

    #[test]
    fn test_hold_stack_push_unique_promotes_existing() {
        // MIDI 重複 noteOn を模擬: C↓ D↓ C↓ で stack=[D, C] になる (旧 C が消えて末尾に再配置)。
        let mut s: HoldStack = HoldStack::new();
        s.push_unique(60);
        s.push_unique(62);
        s.push_unique(60);
        assert_eq!(s.len(), 2);
        assert_eq!(s.top(), Some(60));
        s.remove(60);
        assert_eq!(s.top(), Some(62));
    }

    #[test]
    fn test_hold_stack_clear() {
        let mut s: HoldStack = HoldStack::new();
        s.push(60);
        s.push(62);
        s.clear();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
        assert_eq!(s.top(), None);
    }
}
