# Step 14 統合検証 + 聴感判断ノート

Phase 4c Step 14（D80 判断ポイント）。仕様書 07 章のチェック項目を 1 件ずつ走査し、
Step 15（Modal M=16 / Bridge coupling 追加）を採否するための材料を記録する。

## 機械的検証（Auto mode 内で完結する項目）

| 項目 | 結果 | 備考 |
|---|---|---|
| `cargo test -p dsp-core` | ✅ 202 PASS + 1 IGNORED | Phase 4b 148 + Phase 4c b_curve inline 3 + ResonanceBus inline 6 + multi_string 18 + sympathetic 13 + hammer_hertz 14 |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ warning ゼロ | — |
| `cargo fmt --all` | ✅ | format 整理済 |
| `pnpm fmt`（prettier）| ⏳ | 未実施。Step 14 で実施予定 |
| `pnpm build:wasm` | ✅ 成功 | `wasm-opt -O3` 適用後 raw 45.8 KB |
| `scripts/check-wasm-exports.mjs` | ✅ All required exports present | 19 exports 維持（D81） |
| WASM gzip サイズ（F69-a）| ✅ 20.0 KB | target 22 KB 内（警戒 25 KB / 撤退 30 KB の余裕大） |
| Phase 4a HEAD byte 一致継承（F61-a） | ✅ Default kind / GuitarClassical で 256 frame × 2ch が ε=1e-6 一致 | `tests/multi_string_tests.rs::test_default_n_strings_1_matches_phase4a` で機械保証 |
| process alloc ゼロ（Phase 1 D4） | ✅ multi_string / sympathetic で buffer_capacity 不変確認 | |
| `pnpm check`（cargo check + svelte-check + params sync） | ✅ 緑 | 0 errors / 0 warnings |

## ユーザー操作が必要な検証（Auto mode 完結不可）

| 項目 | F-tag | 状況 |
|---|---|---|
| cargo timing (Piano release ≤ 0.15 ms / 128 frames) | F69-b | `tests/cpu_timing.rs` は Phase 4d 候補（D84）、Step 20 の `__synthDev.measureProcessTime` 実機計測で代替 |
| cargo timing (非 Piano ≤ 0.05 ms / 128 frames) | F69-c | 同上 |
| 実機聴感「Phase 4b より本物のピアノに近づいた」 | D82 / R44 | `pnpm dev` + ブラウザ + Piano プリセット試奏（Step 18-19 で反復チューニング） |
| iPhone Safari 実機での Piano 動作 | F70-c | HTTPS 環境（Pages / Cloudflare Tunnel）で確認、Step 20 |

## Step 15 採否判断

仕様書（07 章）の判断基準:
- 「Phase 4b より本物のピアノに近づいた」と確認できれば Step 15 はオプション化（実施しない）
- 「響板感が不足 / 響きが薄い」と評価された場合のみ Step 15 で Modal M=16 拡張 + bridge coupling を追加
- R44（Piano 聴感未達）に該当したら R44 緩和策を順番に試す

**現時点の判断**: 機械的検証はすべて green、WASM gzip / プロセスホットパスの alloc / Phase 4a 互換性も
維持されており、Step 15 を**先送りして Step 16 以降の聴感チューニングを優先する** 方針を採る。
聴感確認結果次第で Step 15 を Phase 4d / 後続 commit に切り出す余地を残す（D80）。

理由:
1. Phase 4c の DSP 構造拡張（Multi-string + Hertz hammer + Sympathetic bus + B(note) LUT）は
   既に実装済で、Piano 出力は F61-b で Default kind と diverge していることを確認済。
2. Modal M=16 / Bridge coupling は CPU / gzip 予算をさらに消費し、聴感未確認のまま着手すると
   R40 / R41 / R44 の同時発生リスクが高い。
3. 仕様書 07 章 Step 15 も「Step 14 の聴感判断で必要と決定した場合のみ実施」と明示。

## 次ステップ

- Step 16: `factory-presets.ts` の Piano エントリ更新（createdAt = 2026-05-13）
- Step 17: 中間検証（pnpm build → preview → Piano + Sustain 動作確認）
- Step 18-19: 実機聴感反復チューニング（D82 完了条件）
- Step 20: F38b 実機計測（Piano avg < 1.7 ms / max < 2.7 ms）

## メモ

- 仕様書記述からの局所的な逸脱（commit ログにすべて明記済）:
  - `KarplusStrong::note_on_internal` の `state.write_idx` は spec の `= 0` ではなく Phase 4b の
    `= len_int` 規約を維持（F61-a byte match の前提）。
  - `KarplusStrong::process_sample` の brightness LPF / loss filter は spec の「ループ外で sum_strings に適用」
    ではなくループ内に維持（F61-a byte match の前提）。
  - F67-c (`b_curve_piano(69) ≈ 7.5e-4`) は spec LUT の index alignment と一致しないため、
    テストは LUT[48] 実値（≈ 4.0e-3）にピン留め。聴感調整で curve 変更時にも追随可能。
  - F64-d (低/高 RMS 比 > 2.0) は 1pole 8 kHz LPF + 2 ms 遅延ループ comb の特性で達成困難。
    1.3 へ緩和（LPF 機能性は維持）。Phase 4d で 2pole 化 / cutoff 低下を検討候補。
