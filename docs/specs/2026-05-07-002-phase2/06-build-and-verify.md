# 06. Phase 2 ビルド・実行・検証

## 目的

Phase 1 [06 章 ビルド・実行・検証](../2026-05-06-001-mvp/06-build-and-verify.md) を起点に、Phase 2 で発生する **ビルド手順の差分**（`pnpm gen:params` / `pnpm check:params-sync`）、**追加検証項目 F10〜F25** の判定基準と検証手順、**追加リスク R17〜R22**、**性能目標** を定義する。Phase 1 セットアップ手順（rustup / Node.js / pnpm / wasm-opt 等のインストール）は完全継承する。

## 他文書との関係

- 上流: [`02-architecture.md`](./02-architecture.md)（プロジェクト構造、ビルドスクリプト）、[`05-web-frontend-spec.md`](./05-web-frontend-spec.md)（Worklet ビルド経路）
- 下流: [`07-implementation-checklist.md`](./07-implementation-checklist.md)（実装手順）
- 参考: [Phase 1 06 章](../2026-05-06-001-mvp/06-build-and-verify.md)（初回セットアップ、開発時のコマンドフロー、F1〜F9 検証手順、リスク表 R1〜R16、トラブルシューティング、性能目標、デプロイ）— **本書で明示的に変更しない部分はすべて Phase 1 の記述を継承**

## 初回セットアップ

[Phase 1 06 章 §初回セットアップ](../2026-05-06-001-mvp/06-build-and-verify.md#初回セットアップ) を **完全継承**:

- OS: Windows 11（PowerShell 7+）、推奨ブラウザ Chrome / Edge 最新版
- Rust toolchain（rustup、stable、wasm32-unknown-unknown target）
- Node.js LTS、pnpm（corepack 経由）
- WASM ビルドツール（wasm-opt 推奨、Phase 2 では release ビルドで実質必須化候補、リスク R19）
- プロジェクトの依存解決（`pnpm install`）

Phase 2 で追加のセットアップ手順なし。`scripts/gen-params.mjs` / `check-params-sync.mjs` は Node 標準ライブラリのみ使用するため追加 npm package 不要。

## 開発時のコマンドフロー

### 通常の開発サイクル（Phase 2 版）

```powershell
# Rust 側を変更したとき
pnpm build:wasm:dev
# 内部で pnpm gen:params → cargo build → copy-wasm → check-wasm-exports が走る

# UI/Worklet 側だけ変更したとき
pnpm --filter web dev

# まとめて起動（Rust ビルド + dev server）
pnpm dev
```

`pnpm dev` 内で `gen:params` が前段に走るため、`params.json` を編集した場合も自動で Rust + TS の生成物が更新される。

### params.json を編集したときの追加手順

```powershell
# 1. params.json を編集
# 2. 生成物を更新
pnpm gen:params

# 3. 生成物の同期確認（CI でも走る）
pnpm check:params-sync

# 4. dev / build を実行
pnpm dev
```

`gen:params` を忘れると Rust と TS が drift するが、`pnpm check` の最後に `check:params-sync` が走るため CI で検知される（F14 / F15）。

### 動作確認の最初の一歩（Phase 1 と同じ）

[Phase 1 06 章 §動作確認の最初の一歩](../2026-05-06-001-mvp/06-build-and-verify.md#動作確認の最初の一歩) を継承。Phase 2 では F1〜F3 達成後に F10（8 音同時発音）と F18（hold stack last-note 復帰）を追加検証する。

### 本番ビルドの確認

```powershell
pnpm build
# 内部で gen:params → cargo build --release → copy-wasm → check-wasm-exports → vite build が走る
# web/build/ に静的ファイルが生成される

pnpm --filter web preview
# http://localhost:4173 で本番バンドルを確認
```

## ビルドアーティファクトのパス一覧

[Phase 1 06 章 §ビルドアーティファクトのパス一覧](../2026-05-06-001-mvp/06-build-and-verify.md#ビルドアーティファクトのパス一覧) に Phase 2 で 3 件追加。

| 種別 | パス | 生成タイミング | Phase 2 差分 |
|---|---|---|---|
| **`params.json`（単一ソース）** | `params.json`（リポジトリルート） | 手動編集 | **Phase 2 新規** |
| **生成 Rust ソース** | `crates/dsp-core/src/params.rs` | `pnpm gen:params`（git commit、D25） | **Phase 2 新規（既存ファイルを置換）** |
| **生成 TypeScript ソース** | `web/src/lib/audio/generated/params.ts` | `pnpm gen:params`（git commit、D25） | **Phase 2 新規** |
| WASM バイナリ（cargo出力） | `target/wasm32-unknown-unknown/release/wasm_audio.wasm` | `cargo build --release` | 維持 |
| WASM バイナリ（コピー後） | `web/src/lib/wasm/wasm_audio.wasm` | `pnpm build:wasm` | 維持、+`synth_set_polyphony_mode` export 含む |
| Worklet バンドル | `web/static/worklet/synth-processor.js` | `pnpm --filter web build:worklet` | 維持、内部で `generated/params.ts` 取り込み |
| 静的サイト | `web/build/` | `pnpm build` | 維持 |

## ParamDescriptor 同期チェック

### `scripts/check-params-sync.mjs` の責務（再掲、詳細は 02 章）

| 入力 | 処理 | 出力 |
|---|---|---|
| `params.json` | (1) `gen-params.mjs` を呼び生成想定文字列を作る、(2) 既存 `params.rs` / `generated/params.ts` の内容を読む、(3) 文字列一致を判定、(4) 不一致なら exit 1 | exit 0 (一致) / exit 1 (drift 検出、エラーメッセージ表示) |

### 実装方針（参考）

`scripts/check-params-sync.mjs` は `gen-params.mjs` の **純粋関数** (`generateRustSource` / `generateTsSource`) のみを利用し、自身は **ファイル書き込みを一切行わない**。`gen-params.mjs` の CLI entrypoint は import 時には実行されない設計（[`02-architecture.md` §scripts/gen-params.mjs の責務](./02-architecture.md#scriptsgen-paramsmjs-の責務)）に依存する。

```javascript
// scripts/check-params-sync.mjs（実装方針）
import { readFileSync } from 'node:fs';  // writeFileSync は import しない
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
// gen-params.mjs から純粋関数のみを import（CLI entrypoint は走らない）
import { generateRustSource, generateTsSource } from './gen-params.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '..');

const paramsJson = JSON.parse(readFileSync(resolve(root, 'params.json'), 'utf8'));

const expectedRust = generateRustSource(paramsJson);
const expectedTs = generateTsSource(paramsJson);

const actualRust = readFileSync(resolve(root, 'crates/dsp-core/src/params.rs'), 'utf8');
const actualTs = readFileSync(resolve(root, 'web/src/lib/audio/generated/params.ts'), 'utf8');

let drift = false;
if (actualRust !== expectedRust) {
  console.error('params.rs is out of sync with params.json. Run `pnpm gen:params`.');
  drift = true;
}
if (actualTs !== expectedTs) {
  console.error('generated/params.ts is out of sync with params.json. Run `pnpm gen:params`.');
  drift = true;
}
if (drift) process.exit(1);
console.log('ParamDescriptor sync OK.');
```

`gen-params.mjs` の CLI entrypoint 側は `import.meta.url === \`file://${process.argv[1]}\`` ガード内で `writeFileSync` を呼ぶ実装にする（参考イメージ）:

```javascript
// scripts/gen-params.mjs（CLI entrypoint 部分の実装方針）
export function generateRustSource(paramsJson) { /* 純粋関数 */ }
export function generateTsSource(paramsJson) { /* 純粋関数 */ }

// CLI として直接実行された時のみファイル書き込み
if (import.meta.url === `file://${process.argv[1]}`) {
  const paramsJson = JSON.parse(readFileSync(resolve(root, 'params.json'), 'utf8'));
  writeFileSync(resolve(root, 'crates/dsp-core/src/params.rs'), generateRustSource(paramsJson));
  mkdirSync(resolve(root, 'web/src/lib/audio/generated'), { recursive: true });
  writeFileSync(resolve(root, 'web/src/lib/audio/generated/params.ts'), generateTsSource(paramsJson));
  console.log('Generated params.rs and generated/params.ts');
}
```

## export 名の自動検証スクリプト（Phase 2 版）

[Phase 1 06 章 §export 名の自動検証スクリプト](../2026-05-06-001-mvp/06-build-and-verify.md#export-名の自動検証スクリプト) の `REQUIRED` 配列を更新:

```javascript
const REQUIRED = [
  'memory',
  'synth_new', 'synth_free',
  'synth_note_on', 'synth_note_off',
  'synth_set_param', 'synth_reset',
  'synth_out_l_ptr', 'synth_out_r_ptr', 'synth_capacity',
  'synth_process_block',
  'synth_set_polyphony_mode',  // Phase 2 追加（D17）
];
```

## Phase 1 F1〜F9 の引き継ぎ

Phase 2 仕様書は **Phase 1 retrospective §2 で F1〜F9 が達成済み** であることを前提とする。仕様策定中に並行してユーザー側で実機検証を進め、retrospective §2 を更新する想定（[`docs/retrospective/2026-05-06-001-mvp.md` §2](../../retrospective/2026-05-06-001-mvp.md)）。

| ID | Phase 1 検証項目 | Phase 2 で再検証 |
|---|---|---|
| F1 | ブラウザで弦らしい音が鳴る | 不要（Phase 1 で達成済み前提） |
| F2 | 画面鍵盤で発音 | 不要 |
| F3 | PC キーボードで発音 | 不要 |
| F4 | Web MIDI で発音 | 不要 |
| F5 | damping 変化で減衰時間が変わる | 不要 |
| F6 | brightness 変化で音色が変わる | 不要 |
| F7 | クリックノイズなし | 不要、ただし Phase 2 で voice stealing 連打時のクリックは F23 で別途検証 |
| F8 | 連打でメモリ確保が起きない | 不要、ただし Phase 2 で 8 音同時時の memory.buffer.byteLength 不変は F17 で別途検証 |
| F9 | iOS Safari で動作 | 不要、ただし Phase 2 ビルドのデプロイ後に同手順で再確認推奨（Pages デプロイ自動なので追加コストゼロ）|

> **Phase 2 仕様書策定完了時点で Phase 1 F1〜F9 が未検証だった場合の対応**: F25（Phase 1 F1〜F9 が retrospective §2 で達成済みと記載されているか）でドキュメント上の引き継ぎチェックを行う。実機検証が並行作業中ならば、Phase 2 実装着手前に retrospective §2 を更新してから Phase 2 Step 1 に入る。

## 検証チェックリスト（Phase 2 追加分 F10〜F25）

| ID | 判定基準 | 検証手順 | 期待結果 |
|---|---|---|---|
| **F10** | 8 音同時発音でクリップなし | (1) `cargo test -p dsp-core test_voice_pool_allocates_distinct_voices` で内部 active_count を確認、(2) PC キーボードで A, S, D, F, G, H, J, K の 8 鍵を同時押下（Web ブラウザで操作）して聴感確認 | (1) テストパス（active_count == 8）、(2) 8 音すべて重畳して聞こえ、クリップ歪みが起きない |
| **F11** | 9 音目で voice stealing 発生 | 8 音持続中に 9 音目（例: KeyL = D5）を押下 | D13 戦略に従い「energy 閾値（1.0e-3）以下のボイスがあればそのうち最古、なければ最古のボイス」が置き換わり、新音 D5 が発音される。耳障りなクリックノイズなし（D13 / D28）|
| **F12** | A1〜C8 全音域ピッチ精度 ± 0.5% 以内 | (1) `cargo test -p dsp-core pitch_accuracy` で `test_pitch_a1` (55Hz) / `test_pitch_a2` (110Hz) / `test_pitch_a4` (440Hz) / `test_pitch_c6` (1046.5Hz) / `test_pitch_c8` (4186Hz) を実行、(2) ブラウザで A4 を 1 秒鳴らし出力波形を保存して FFT 分析（任意ツール、Audacity 等）| (1) すべてのテストがパス（autocorrelation 推定誤差が各周波数の ± 0.5% 以内）、(2) A4 の FFT ピーク周波数が 440Hz ± 2.2Hz 以内 |
| **F13** | A1 (55Hz) で Phase 1 課題が解消 | F12 の `test_pitch_a1` がパスすることに加え、ブラウザで A1 を 1 秒鳴らし FFT 分析 | A1 が ± 0.5% 以内。Phase 1 で 2.3% 誤差だった課題が解消（[Phase 1 retrospective §3 D1](../../retrospective/2026-05-06-001-mvp.md)）|
| **F14** | ParamDescriptor 同期: Rust/TS 生成物 hash 一致 | `pnpm check:params-sync` を実行 | `ParamDescriptor sync OK.` 出力、exit 0 |
| **F15** | params.json 編集後の gen 忘れを CI で検知 | (1) `params.json` を編集（例: Damping default を 0.997 に変更）、(2) `pnpm gen:params` を実行せずに `pnpm check:params-sync` 実行 | exit 1、`params.rs is out of sync` メッセージ表示 |
| **F16** | ポリフォニー 8 音時の process 時間 < 1.5ms | (1) Chrome DevTools Performance タブで Record 開始、(2) 8 音同時発音状態で 5 秒間 record、(3) Stop して `process()` 1 回の所要時間を計測 | 平均 < 1.5ms、最大 < 2.0ms（CPU 予算 2.67ms 内）|
| **F17** | ポリフォニー時の memory 確保ゼロ | (1) `cargo test -p dsp-core test_no_allocation_in_polyphonic_process` 実行、(2) `synth-processor.ts` に Phase 1 06 章 F8(a) と同じ `memory.buffer.byteLength` 不変チェックを一時挿入し、8 音同時発音 + 連打を 30 秒継続 → 確認後コード削除 | (1) テストパス、(2) `[F17] WASM memory grew` 警告が一度も出ない |
| **F18** | hold stack last-note 復帰 | (1) `cargo test -p dsp-core test_hold_stack_last_note_priority` 実行、(2) dev ビルドで DevTools Console から `__synthDev.setMode('mono')` を呼び（[`05-web-frontend-spec.md` §dev ビルドのみ](./05-web-frontend-spec.md#dev-ビルドのみimportmetaenvdev-ガード本番では-tree-shake-で除去)）、C 押 → D 押 → D 離 → C 復帰 → C 離 → 無音 のシーケンスを実行 | (1) テストパス、(2) D 離した時点で C に復帰して鳴り続ける、C 離した時点で減衰開始 |
| **F19** | hold stack 容量超過時の挙動 | (1) `cargo test -p dsp-core test_hold_stack_overflow_behavior` 実行、(2) mono モードで 17 鍵を順次押下 | (1) テストパス、(2) 最古のノートがスタックから消えるが、現在押下中のノートはすべて残る |
| **F20** | mono / poly モード切替で破綻なし | mono → poly → mono を `synth_set_polyphony_mode` 経由で連続切替（DevTools console から） | クラッシュなし、無音化なし、切替時に耳障りなクリックなし |
| **F21** | WASM gzip < 30 KB | `pnpm build` 後に `gzip -k web/build/_app/immutable/assets/wasm_audio.<hash>.wasm` でサイズ計測 | gzip 圧縮後のサイズが 30 KB 未満。Phase 1 実績 7.98 KB に Phase 2 +20-22 KB 想定 |
| **F22** | Worklet 本番バンドル < 10 KB | `pnpm build` 後に `Get-Item web/build/worklet/synth-processor.js \| Select-Object Length` でサイズ計測（または `wc -c`）| Phase 1 実績 4.9 KB から +5 KB 以内（generated/params.ts inline 分）|
| **F23** | voice stealing 連打でクリックなし | poly モードで 9 鍵以降を高速連打（1 秒に 5 回）、A〜L をシャッフルしながら 10 秒継続 | 知覚できる耳障りなクリックノイズが発生しない（D28）|
| **F24** | 常用範囲（OutputGain ≤ 1.0、通常の押下パターン）で音割れなし | (a) OutputGain ≤ 1.0 + 通常演奏（時間差あり、velocity 平均）で 30 秒継続。(b) 補助確認: OutputGain=1.5 + 8 鍵同時全力強打の最悪ケースで歪みの程度を聴感確認 | (a) ハードクリップ歪みが知覚されない（D20 の 1/sqrt(N) スケールで防がれる）。(b) 最悪ケースでは歪みが出る場合があるが許容（Phase 2 では soft clip/limiter を入れず、Phase 3 候補。詳細は [`03-dsp-core-spec.md` §1/sqrt(N) スケールの根拠と限界](./03-dsp-core-spec.md#1sqrtn-スケールの根拠と限界d20)）|
| **F25** | Phase 1 F1〜F9 が retrospective §2 で達成済みと記載 | `docs/retrospective/2026-05-06-001-mvp.md` §2「達成と未達」を読み、F1〜F9 のすべてが「✅ 達成」のステータスになっているか確認 | F1〜F9 すべて達成済み記載あり。未達ならば Phase 2 実装着手前に追加検証 + retrospective §2 更新を実施 |

### 検証手順の補足

#### F12 / F13（ピッチ精度）の詳細手順

**(a) cargo test での自動検証（推奨、A1〜C8 を網羅）**

`crates/dsp-core/tests/pitch_accuracy.rs` に以下のテスト方針で実装。**A1〜C8 全音域を網羅** することで Lagrange 補間が高域でも有効に働くこと、および LPF（フィードバックループ内）が高域減衰を起こしてもピッチ自体は維持されることを検証する。

**重要: sub-sample 精度のための parabolic interpolation**

48 kHz サンプリングで C8 (4186 Hz) の基本周期は約 11.47 サンプル。整数 τ で `f0 = sample_rate / τ` を計算すると、τ=11 で 4363.6 Hz / τ=12 で 4000 Hz と最低でも 4-9% の誤差が出てしまい ± 0.5% の検証ができない。これを避けるため **autocorrelation peak 周辺で parabolic interpolation を行い sub-sample 精度の τ を求める**。

```rust
// dsp-core/tests/pitch_accuracy.rs（実装方針）
//
// 共通ヘルパ（parabolic interpolation 込み）:
// fn measure_f0(midi: u8, sample_rate: f32) -> f32 {
//   (1) Engine::prepare(sample_rate, 128)
//   (2) note_on(midi, velocity=0.8)
//   (3) 1 秒分のサンプルを process_sample で生成（48000 サンプル）
//   (4) 励振直後のノイズ過渡応答を避けるため、最初の 0.1 秒（4800 サンプル）は捨てる
//   (5) autocorrelation r(τ) = Σ x(t)x(t+τ) を τ ∈ τ_search で計算（後述）
//   (6) r(τ) の最大値となる整数 τ_peak を見つける
//   (7) parabolic interpolation で sub-sample τ を推定:
//       δ = 0.5 * (r[τ_peak - 1] - r[τ_peak + 1]) / (r[τ_peak - 1] - 2 r[τ_peak] + r[τ_peak + 1])
//       τ_refined = τ_peak as f32 + δ                   // δ ∈ [-0.5, 0.5]
//   (8) f0 = sample_rate / τ_refined を返す
// }
//
// τ 探索範囲は midi 値から期待周期を逆算して ± 5% に絞る:
//   expected_period = sample_rate / midi_to_freq(midi)
//   τ_search = (expected_period * 0.95) as usize ..= (expected_period * 1.05) as usize
//   この絞り込みで C8 の周期 11.5 でも 10..=12 の τ_peak を必ず取れ、parabolic で sub-sample 精度を取る
//
// 各テスト: assert!((measure_f0(midi, 48000.0) - expected_f0).abs() / expected_f0 < 0.005);
//
// 必須テストケース:
// - test_pitch_a1: midi=33, expected=55.0 Hz   (周期 ~872.7 samples、整数 τ でも誤差 0.06%)
// - test_pitch_a2: midi=45, expected=110.0 Hz  (周期 ~436.4 samples)
// - test_pitch_a4: midi=69, expected=440.0 Hz  (周期 ~109.1 samples)
// - test_pitch_c6: midi=84, expected=1046.5 Hz (周期 ~45.9 samples、parabolic 推奨)
// - test_pitch_c8: midi=108, expected=4186.0 Hz (周期 ~11.47 samples、parabolic 必須)
//
// parabolic interpolation の数値安定性: r[τ_peak - 1] - 2 r[τ_peak] + r[τ_peak + 1] が
// 0 に近い場合（τ_peak が完全に整数のとき）、δ = 0 として fallback。
// abs() < 1e-12 で判定し、その場合は τ_refined = τ_peak as f32 を返す。
```

zero-crossing 法はノイズ励振 + LPF の影響を受けやすいので、autocorrelation + parabolic interpolation の組み合わせが信頼性が高い。FFT-based estimator (instantaneous frequency from phase) も代替手段として有効だが、Phase 2 では autocorrelation + parabolic で十分。

> **C8 でテストが flaky になる場合の対策**: KS のノイズ励振 + LPF + 減衰で C8 の autocorrelation peak が不安定になることがある。リスク R23 の対策案（複数解析窓の中央値、FFT-based 代替、許容誤差 ± 1.0% への緩和等）を順に試す。test_pitch_c6 は周期 ~46 サンプルで安定しやすいため、**C6 が確実にパスすれば Lagrange 補間が高域でも有効** という判断は得られる（C8 を strict に通すかは flaky 度合いで決定）。

**(b) 実機 FFT 分析（補助）**

ブラウザで A4 を鳴らし、出力を録音して Audacity 等で FFT スペクトル分析:

1. `pnpm dev` でブラウザ起動
2. PC キーボードで KeyH（A4=69）を 1 秒押し続ける
3. システム音声録音（OBS / VB-Cable + Audacity）で 1 秒分キャプチャ
4. Audacity > Analyze > Plot Spectrum で FFT
5. ピーク周波数が 440Hz ± 2.2Hz であることを確認

A1（55Hz）/ C8（4186Hz）の実機 FFT は (a) の cargo test を信頼し、実機での確認は省略可（A4 が出れば実機環境は OK）。

#### F17（ポリフォニー時のメモリ確保ゼロ）の詳細手順

[Phase 1 06 章 §F8 メモリ確保チェックの詳細手順](../2026-05-06-001-mvp/06-build-and-verify.md#f8メモリ確保チェックの詳細手順) と同じ手法を Phase 2 用に拡張:

**(a) WASM linear memory の不変チェック（最重要）**

`synth-processor.ts` 末尾に Phase 1 と同じ開発時専用チェックコードを一時的に挿入する。Phase 2 では **`synth_new` 完了直後の `memory.buffer.byteLength` を baseline として記録**、以後 `process_block` / `note_on` / `note_off` 等のいずれの呼び出しでも byteLength が変化しないことを確認する（[`02-architecture.md` §Phase 2 のメモリレイアウト](./02-architecture.md#phase-2-のメモリレイアウト)）。

シナリオ:
1. `synth_new` 直後の `byteLength` を baseline として保存
2. PC キーボード 8 鍵同時押下を 5 秒継続 → byteLength が baseline と一致
3. 9 鍵目以降の voice stealing を含めた連打を 30 秒継続 → byteLength が baseline と一致
4. mono ↔ poly モード切替を `setMode` で 10 回繰り返し → byteLength が baseline と一致

`[F17] WASM memory grew: <baseline> → <new>` の警告が一度も出なければ達成。

**(b) Rust 側 cargo test**

`test_no_allocation_in_polyphonic_process`（[`03-dsp-core-spec.md`](./03-dsp-core-spec.md#phase-2-で追加するテスト11-件)）を実装。VoicePool::prepare 後に 8 ボイス全部 note_on → 1 秒分 process_sample で alloc 回数 0。

#### F18 / F19 / F20（hold stack / mono モード）の詳細手順

Phase 2 では UI で mono モードに切り替える手段を提供しない（D21）。実機検証は以下の手順で行う:

**(a) cargo test での自動検証（推奨）**

`test_hold_stack_last_note_priority` / `test_hold_stack_overflow_behavior` / `test_synth_mode_switch_no_break` を実装し、すべてパスすることを確認（[`03-dsp-core-spec.md`](./03-dsp-core-spec.md#phase-2-で追加するテスト11-件)）。

**(b) ブラウザでの手動検証（補助）**

dev ビルドでは `window.__synthDev` に診断 API が `import.meta.env.DEV` ガード越しに公開される（[`05-web-frontend-spec.md` §dev ビルドのみ](./05-web-frontend-spec.md#dev-ビルドのみimportmetaenvdev-ガード本番では-tree-shake-で除去)）。`pnpm dev` で起動した dev server で、DevTools Console から:

```javascript
__synthDev.setMode('mono')
// PC キーボードで A → S → S 離 → A 確認 → A 離
__synthDev.setMode('poly')
```

このコードは `import.meta.env.DEV` ガード内なので **本番ビルドでは tree-shake で完全に除去される**。検証完了後にコードを削除する必要はない（本番に漏れない）。

#### F21 / F22（サイズ計測）の詳細手順

PowerShell でのサイズ確認:

```powershell
# Phase 2 build 後
pnpm build

# WASM サイズ
Get-ChildItem web\build\_app\immutable\assets\*.wasm | Select-Object Name, Length

# WASM gzip サイズ（PowerShell には gzip 標準なし、Git Bash で計測）
# git bash:
# gzip -kc web/build/_app/immutable/assets/wasm_audio.*.wasm | wc -c

# Worklet サイズ
Get-ChildItem web\build\worklet\synth-processor.js | Select-Object Name, Length
```

## リスクと対策表

[Phase 1 06 章 §リスクと対策表](../2026-05-06-001-mvp/06-build-and-verify.md#リスクと対策表) の R1〜R16 を継承し、Phase 2 で 6 件追加。

| # | リスク | 影響 | 対策 |
|---|---|---|---|
| **R17** | voice stealing で再生中ノートが切れる耳障りなクリック | F23 失敗、音楽的に不自然 | energy 閾値以下のボイス優先選択（D28、`note_allocator.rs::select_voice_for_steal`）。F23 で実機検証。クリックが残るなら note_on 時の励振を 1-2 ms フェードインに変更（Phase 3 候補、D28 拡張）|
| **R18** | ParamDescriptor の Rust/TS drift で実行時にパラメータが効かない | F14 失敗、UI スライダーが効かない / クラッシュ | `scripts/check-params-sync.mjs` を `pnpm check` で必ず走らせる。CI（`.github/workflows/ci.yml`）の `pnpm check` ステップが exit 1 でブロック |
| **R19** | ポリフォニー化で WASM サイズが膨れて gzip 30 KB 超 | F21 失敗、ロード時間増加 | (1) `wasm-opt -O3` を release ビルドで実質必須化（[Phase 1 04 章](../2026-05-06-001-mvp/04-wasm-audio-spec.md#サイズ最適化mvpでは深追いしない)）、(2) `cargo build --release` 時に `RUSTFLAGS="-C target-cpu=generic -C opt-level=z"` を試す、(3) `panic = "abort"` 維持、(4) どうしても収まらないなら N=8 から N=6 への縮小検討（D12 再評価）|
| **R20** | hold stack 溢れで意図しないノートが復帰 | F19 で挙動が想定外 | 容量 16、溢れ時最古破棄（D16）。F19 テストでオーバーフロー挙動を assertion し、実機での挙動も確認 |
| **R21** | fractional delay の係数計算で `process_sample` ホットパスが遅くなる | F16 失敗（process 時間超過）| `LagrangeCoeffs::new` を `note_on` 時のみ呼ぶ（D26）。process 内では積和 4 回のみ。Phase 2 で pitch bend 対応を入れたくなったら SmoothedValue 化 + 各 process_sample で 4 サンプル積和の構成を再評価 |
| **R22** | ポリフォニー時の合成ゲインでクリップ（最悪ケース: 8 鍵同時全力 + OutputGain=1.5）| F24 (a) 常用範囲は OK だが (b) 最悪ケースで歪み発生の可能性 | 1/sqrt(N) スケール（D20、[`03-dsp-core-spec.md` §1/sqrt(N) スケールの根拠と限界](./03-dsp-core-spec.md#1sqrtn-スケールの根拠と限界d20)）で常用範囲は対策。最悪ケースの歪みは Phase 2 では許容、Phase 3 で soft clip（`tanh(x)` ベース）または look-ahead limiter を Engine::process 末尾に追加検討 |
| **R23** | C8 (4186Hz) の `test_pitch_c8` が KS のノイズ励振 + LPF + 減衰で autocorrelation peak が不安定になり、parabolic interpolation でも測定誤差が ± 0.5% を超える | F12 の `test_pitch_c8` が flaky になる、または fail | (1) 励振直後 0.1 秒スキップ後の解析窓を 0.5-0.9 秒に絞る（過渡応答と末尾の減衰両方を避ける）、(2) 異なる開始サンプル位置で複数解析窓 (例: 0.1-0.6, 0.2-0.7, 0.3-0.8 秒) を測定して f0 の中央値を取る、(3) FFT-based estimator（spectrum peak の parabolic interpolation）を代替実装、(4) NSDF / YIN ベースに置き換え、(5) どうしても不安定なら C8 のみ許容誤差を ± 1.0% に緩和し test_pitch_c6 を主検証として確実なテストに残す（D14 の最終確認、Phase 3 で対策再評価）|

## トラブルシューティング Tips

[Phase 1 06 章 §トラブルシューティング Tips](../2026-05-06-001-mvp/06-build-and-verify.md#トラブルシューティング-tips) を継承し、Phase 2 で追加:

### 「ParamSlider が反応しない」

- `params.json` を編集後に `pnpm gen:params` を実行したか確認
- `pnpm check:params-sync` を実行して drift 検知（exit 1 ならば `pnpm gen:params` で再生成）
- `web/src/lib/audio/generated/params.ts` が存在し、最新の `params.json` 内容を反映しているか確認
- `web/src/lib/audio/messages.ts` が `generated/params.ts` から re-export しているか確認

### 「ポリフォニーで 1 音しか鳴らない」

- `synth_set_polyphony_mode` が `mono`(1) で送信されていないか確認（デフォルトは poly。dev ビルドなら `__synthDev.setMode('poly')` で明示的に切替可）
- `cargo test -p dsp-core test_voice_pool_allocates_distinct_voices` がパスするか確認（cargo test レベルで VoicePool の allocation ロジックを検証）
- 聴感確認: 8 鍵を意図的に時間差を付けて押し、それぞれが独立して減衰するか確認（同時押下だと位相干渉で 1 音に聞こえる場合あり）
- VoicePool の active voice 数を実機で観測する API は Phase 2 では提供されない（Phase 3 で UI voice meter 追加時に検討、[`05-web-frontend-spec.md` §dev ビルドのみ](./05-web-frontend-spec.md#dev-ビルドのみimportmetaenvdev-ガード本番では-tree-shake-で除去) 末尾参照）

### 「9 音目押下で全部の音が消える」

- voice stealing の判定ロジックに不具合の可能性。`crates/dsp-core/src/note_allocator.rs::select_voice_for_steal` が `StealResult::Index(i)` を正しい範囲（0 ≤ i < N）で返しているか確認
- `KarplusStrong::note_on` 内の `length_int.clamp(3, max_len)` で `max_len` が 0 になっていないか（buffer.len() < LAGRANGE_BUFFER_MARGIN）

### 「A1 のピッチが Phase 1 と変わっていない」

- `KarplusStrong::process_sample` で Lagrange 補間値を出力に使っているか確認（生サンプル `current` を返してしまうと Phase 1 と同じ整数ディレイ動作）
- `LagrangeCoeffs::new(len_frac)` の `len_frac` が確実に `[0, 1)` 範囲に入っているか
- `test_fractional_delay_pitch_a1` の autocorrelation 推定値をログ出力して確認

### 「`pnpm gen:params` が失敗する」

- `params.json` の JSON 形式が正しいか確認（カンマ忘れ、引用符忘れ）
- `scripts/gen-params.mjs` が `params.json` を正しいパスから読んでいるか
- 出力先ディレクトリ（`crates/dsp-core/src/`、`web/src/lib/audio/generated/`）が存在するか（`generated/` が無い場合は `mkdirSync({ recursive: true })` で作成）

## 性能目標（Phase 2）

| 指標 | Phase 1 実績 | Phase 2 目標値 | 備考 |
|---|---|---|---|
| AudioWorklet `process` あたりの CPU 時間（128 frames @ 48kHz、CPU 予算 2.67ms）| < 0.5ms（1 ボイス）| **< 1.5ms**（8 ボイス + Lagrange 3 次補間）| F16 で計測。8 倍 + Lagrange コスト想定 |
| 起動から最初の音まで | < 2 秒 | **< 2 秒**（同等維持）| WASM サイズ +20 KB 程度なら影響軽微 |
| WASM バイナリサイズ（gzip）| 7.98 KB | **< 30 KB** | F21 で計測、リスク R19 で wasm-opt 必須化検討 |
| WASM バイナリサイズ（gzip 前）| 16.49 KB | < 60 KB 想定 | wasm-opt -O3 適用後の見積 |
| Worklet 本番バンドル | 4.9 KB | **< 10 KB** | F22 で計測、generated/params.ts inline 分 +5 KB 想定 |
| ヒープ確保回数（process 実行中）| 0 回 | **0 回**（維持）| F17 で検証 |
| ピッチ精度（A4=440Hz）| 0.05% 程度（整数ディレイ）| ± 0.5% 以内 | F12 で検証、Phase 2 では Lagrange でさらに改善見込み |
| ピッチ精度（A1=55Hz）| 2.3% 程度（整数ディレイ、retrospective §3）| **± 0.5% 以内** | F13 で検証、Phase 2 主目的の 1 つ |
| 最悪ケース（8 鍵全力 + OutputGain=1.5）の歪み | — | 許容（F24 (b) で確認、Phase 3 で limiter 追加検討） | D20 / R22 |

## デプロイ

[Phase 1 06 章 §デプロイ](../2026-05-06-001-mvp/06-build-and-verify.md#デプロイ参考mvp の必須ではない) を継承。GitHub Pages の自動デプロイ（`main` ブランチへの push で `.github/workflows/deploy.yml` が発火）を維持。Phase 2 では `pnpm build` 内で `gen:params` が走るため、CI 上でも `params.json` 更新が反映される。

### CI ワークフローの追加チェック

Phase 1 の `.github/workflows/ci.yml` に **Phase 2 で `pnpm check:params-sync` を追加**:

```yaml
# .github/workflows/ci.yml （Phase 2 で追加）
- name: Check params sync
  run: pnpm check:params-sync
```

これにより、PR で `params.json` を編集したが `pnpm gen:params` を忘れた場合、CI が exit 1 でブロックする（F15）。
