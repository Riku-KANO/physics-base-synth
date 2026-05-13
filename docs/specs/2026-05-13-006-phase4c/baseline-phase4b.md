# Phase 4b F38b 実機ベースライン記録

Phase 4c Step 1（D85 / F70-a）の成果物。Phase 4c 実装着手前の Phase 4b 状態での `__synthDev.measureProcessTime` 値を記録し、Step 20（F70-b）の Phase 4c 完成後計測と比較するための基準値とする。

## 計測手順

1. `phase4c-impl` ブランチで `pnpm dev` を起動
2. ブラウザで `http://localhost:5173/` を開く
3. Start ボタンを押す
4. PresetSelector で Piano プリセットを選択
5. Console で以下を実行:
   ```javascript
   const piano = await window.__synthDev.measureProcessTime(5000);
   console.log('Piano', piano);
   ```
6. PresetSelector を Default に切り替え、同じ計測を Default で実行:
   ```javascript
   const def = await window.__synthDev.measureProcessTime(5000);
   console.log('Default', def);
   ```

## 期待ベースライン（Phase 4b 想定値）

| プリセット | avg (ms) | max (ms) | 備考 |
|---|---|---|---|
| Piano | 0.047 | 0.063 | Stretching all-pass cascade M=8 + Hammer model + Modal Body |
| Default | 0.029 | — | dispersion 非アクティブ、Phase 4a 経路 |

## 実測値

> ユーザー操作必須。下記テーブルを実機計測後に上書き更新する。

| プリセット | avg (ms) | max (ms) | min (ms) | samples | bufferOverflow | 計測日時 |
|---|---|---|---|---|---|---|
| Piano | _未計測_ | _未計測_ | _未計測_ | _未計測_ | _未計測_ | _未計測_ |
| Default | _未計測_ | _未計測_ | _未計測_ | _未計測_ | _未計測_ | _未計測_ |

## 環境

- OS: Windows 11 Home 10.0.26200
- ブラウザ: _計測時に記載_
- CPU: _計測時に記載_
- ノード: localhost:5173 (`pnpm dev`)

## 備考

Step 20（Phase 4c 完成後の F38b 計測）の結果は `final-phase4c-timing.md` に記録する。

実装着手後にベースラインを後追い記録した場合は、Phase 4c retrospective §8 で経緯を説明する。
