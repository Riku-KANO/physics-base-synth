# Phase 4c F38b 実機計測（最終）

Phase 4c Step 20（F70-b）。Phase 4c 実装完了後の `__synthDev.measureProcessTime` 値を記録し、
Step 1 の Phase 4b ベースラインと比較する。

## 計測手順

1. `phase4c-impl` ブランチを GitHub Pages / Cloudflare Tunnel 等で HTTPS 配信、または
   `pnpm dev` でローカル起動
2. ブラウザで Start → PresetSelector で Piano を選択
3. Console で:
   ```javascript
   const piano = await window.__synthDev.measureProcessTime(5000);
   console.log('Piano (Phase 4c)', piano);
   ```
4. 他楽器でも実行:
   ```javascript
   // PresetSelector で Default に切替後
   const def = await window.__synthDev.measureProcessTime(5000);
   console.log('Default (Phase 4c)', def);
   ```

## 期待達成ライン (F70-b / 性能目標)

| プリセット | avg (ms) target | max (ms) target | 警戒ライン |
|---|---|---|---|
| Piano | < 1.7 | < 2.7 | avg > 2.0 で R30 (stride 4096 化検討) |
| Default | < 1.7 | < 2.7 | 同上 |

cargo release timing (128 frames @ 48 kHz) 上の予想:
- Piano (8 voice × 3 strings + Modal + Sympathetic): < 0.15 ms
- 非 Piano (Sympathetic 非アクティブ): < 0.05 ms (Phase 4b と同等)

## 実測値（未記録）

> ユーザー実機計測必須。Step 1 と同じ環境で取得して記入。

| プリセット | avg (ms) | max (ms) | min (ms) | samples | bufferOverflow | 計測日時 |
|---|---|---|---|---|---|---|
| Piano | _未計測_ | _未計測_ | _未計測_ | _未計測_ | _未計測_ | _未計測_ |
| Default | _未計測_ | _未計測_ | _未計測_ | _未計測_ | _未計測_ | _未計測_ |

## Phase 4b ベースラインとの比較（未記録）

> Step 1 の `baseline-phase4b.md` 実測値と Phase 4c の実測値を並べて記録。

| 指標 | Phase 4b ベースライン | Phase 4c 実測 | Δ | 評価 |
|---|---|---|---|---|
| Piano avg (ms) | _未計測_ | _未計測_ | _未計算_ | _未評価_ |
| Piano max (ms) | _未計測_ | _未計測_ | _未計算_ | _未評価_ |
| Default avg (ms) | _未計測_ | _未計測_ | _未計算_ | _未評価_ |

## F70-c: iPhone Safari 実機動作確認

| 項目 | 状況 |
|---|---|
| HTTPS 配信先 | _未記録_ (GitHub Pages or Cloudflare Tunnel URL) |
| iPhone 機種 / iOS バージョン | _未記録_ |
| Piano 試奏: 音切れなし | _未確認_ |
| Sustain ペダル動作 (CC#64) | _未確認_ |
| Multi-string beat 感 (中央 C5 連打) | _未確認_ |

## 備考

- Phase 4c の DSP 構造拡張 (Multi-string + Hertz hammer + Sympathetic + B(note) LUT) で
  Piano は 3 弦並列 + bus feedback による CPU 増加が想定される。
- 仕様書 06 章 §性能目標 では「Piano process per call (release cargo timing) < 0.15 ms」を
  Phase 4c target としており、警戒ライン (> 0.25 ms) 越えの場合は R41 緩和策を検討。
- WASM gzip サイズ (Step 14 で 20.0 KB 計測済) は変動少だが、聴感調整で b_curve 値の幅が
  変化すると float literal の桁数で ±0.1 KB 変動する可能性。
