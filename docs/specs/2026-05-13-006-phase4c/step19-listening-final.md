# Step 19 聴感チューニング最終回（プレースホルダ）

Phase 4c Step 19（D82 完了条件）。実機聴感で「Phase 4b より本物のピアノに近づいた」とユーザーが
判断するまで `factory-presets.ts` の Piano エントリ + `params.json` の Phase 4c フィールドを
反復調整する。

## 実施手順

1. `pnpm dev` を起動、ブラウザで `http://localhost:5173/` を開く
2. Start ボタン → PresetSelector で Piano を選択
3. 鍵盤 / MIDI で多様な音域を試奏:
   - 低音 (C2 = MIDI 36, 1 string、Hertz hammer + B(36) inharmonicity)
   - 中音 (C4 = MIDI 60, 3 strings + sympathetic)
   - 高音 (C7 = MIDI 96, 3 strings + B(96) は ~3.2e-2 で stretched)
4. Sustain ペダル ON/OFF で sympathetic resonance の余韻を確認
5. 連打・複数音同時押しで Multi-string の beat / chorus 感を確認
6. 同じ条件で Phase 4b の Piano（仕様書記録の Phase 4b 出力イメージ）と比較

## チューニング候補（cargo / clippy / 互換性テストを壊さない範囲で反復）

| パラメータ | レンジ | 影響 | 関連 |
|---|---|---|---|
| `params.damping` | 0.995〜0.9995 | 全体の rate of decay | factory-presets.ts |
| `params.brightness` | 0.45〜0.60 | 高域成分の量 | factory-presets.ts |
| `params.bodyWet` | 0.35〜0.65 | Modal Body の混合比 | factory-presets.ts |
| `unison_detune_cents` | 1.0〜2.5 cents | beating / chorus 感 | params.json (Piano) |
| `sympathetic_amount` | 0.6〜1.2 | sympathetic 響きの量 (内部 × 0.05 で clamp) | params.json (Piano) |
| `hammer_cutoff_high_hz` | 4500〜6500 Hz | 強打鍵 brightness | params.json (Piano) |
| `inharmonicity_b_curve[]` | 88 値、Young 1952/Conklin 1996 fitting | 高音 stretched tuning | params.json (Piano) |

## 受け入れ基準（D82）

- ユーザーが「Phase 4b より本物のピアノに近づいた」と確認
- 既存 7 楽器 (Default / Guitar / Bass / Sitar 等) に regression なし
- Phase 1〜4b の全機能 (LFO / Mod Wheel / Sustain / Preset / VoiceMeter / MIDI CC / Pitch Bend)
  が動作

## R44 緩和策（聴感未達時、順番に試す）

1. Piano プリセット聴感調整（上記レンジで反復）
2. Modal Body M=16 拡張 (Step 15 を遡って実施)
3. Bridge coupling 追加（Multi-string 案 B、Step 15 一部）
4. B(note) LUT 値の精密化 (Young 1952 / Conklin 1996 fitting)
5. Phase 4d で「Two-stage decay 明示実装」「複数 Piano 機種」「Una corda」等を検討

## 実測ログ（未記録）

> ユーザー実機聴感を完了後に記録する。

| イテレーション | 変更点 | 評価 |
|---|---|---|
| pass 1 | - | _未実施_ |
| pass 2 | - | _未実施_ |
| ... | - | _未実施_ |

## 最終評価

> 「Phase 4b より本物のピアノに近づいた」と確認できた時点で記入。

- 評価日時: _未記録_
- 比較対象: Phase 4b commit 6201814 の Piano 出力（口頭比較 / 簡易録音）
- ユーザー所感: _未記録_
- 採用 Piano パラメータ最終値: _factory-presets.ts / params.json の commit hash を記載_
