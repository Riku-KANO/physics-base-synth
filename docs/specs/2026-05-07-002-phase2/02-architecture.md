# 02. Phase 2 アーキテクチャ差分

## 目的

Phase 1 [02 章 全体アーキテクチャ](../2026-05-06-001-mvp/02-architecture.md) を起点に、Phase 2 で発生する **構成差分**（VoicePool 配置、ParamDescriptor codegen パイプライン、メモリレイアウト変更、ビルドツールチェーン更新）を確定する。Phase 1 で確定した 4 レイヤ構成・モノレポレイアウト・既存スクリプトはすべて維持する。

## 他文書との関係

- 上流: [`01-overview.md`](./01-overview.md)（Phase 2 スコープと決定事項 D12〜D29）
- 並列: [`03-dsp-core-spec.md`](./03-dsp-core-spec.md)、[`04-wasm-audio-spec.md`](./04-wasm-audio-spec.md)、[`05-web-frontend-spec.md`](./05-web-frontend-spec.md)
- 下流: [`06-build-and-verify.md`](./06-build-and-verify.md)（具体的なコマンドと検証手順）
- Phase 1 参照: [`02-architecture.md`](../2026-05-06-001-mvp/02-architecture.md)（モノレポレイアウト、責務分離、ビルドツールチェーン一覧）— 本書で **明示的に変更しない部分はすべて Phase 1 の記述を継承**

## 信号フローと責務分担

### Phase 1 構成は維持

[Phase 1 02 章 §信号フローと責務分担](../2026-05-06-001-mvp/02-architecture.md#信号フローと責務分担) の 4 レイヤ構成（Svelte UI / SynthEngine / SynthProcessor (Worklet) / wasm-audio / dsp-core）は **完全維持**。各レイヤの責務分離原則も継続。

### Phase 2 で発生するレイヤ内変更

| レイヤ | Phase 1 | Phase 2 差分 |
|---|---|---|
| Svelte UI | 既存コンポーネント 4 件、共有ステート 4 fields | `ParamSlider` を ParamDescriptor 駆動に改修。共有ステートは現状維持（mode / activeVoices は UI 出さないため不要、D21 / D22）|
| SynthEngine（main） | AudioContext 起動、Worklet 初期化、MessagePort 仲介、rAF パラメータスロットル | **完全維持**。ParamDescriptor 生成物の import パスのみ更新 |
| SynthProcessor（Worklet） | WASM ロード、`process` 委譲、Float32Array view キャッシュ | `WasmExports` interface に `synth_set_polyphony_mode` を追加（D17） |
| wasm-audio | C ABI 10 関数、SynthHandle 保持 | `synth_set_polyphony_mode` の 1 関数追加。SynthHandle 内部で Engine が VoicePool を保持するが外部 API は不変 |
| dsp-core | Engine / KarplusStrong / SmoothedValue / XorShift32 / Voice / AudioProcessor | **新規モジュール 4 件**: `voice_pool.rs` / `fractional_delay.rs` / `note_allocator.rs` / `hold_stack.rs`。`params.rs` は **コード生成出力に置換**、Engine は VoicePool 化で大幅書き換え |

## モノレポレイアウト

[Phase 1 02 章 §モノレポレイアウト](../2026-05-06-001-mvp/02-architecture.md#モノレポレイアウト) を継承。Phase 2 で追加・変更されるファイルのみ列挙する。

### Phase 2 で追加されるファイル

```
C:\Users\81903\projects\physics-base-synth\
├── params.json                                    # 単一ソース、ParamDescriptor 定義
│
├── crates\
│   ├── dsp-core\
│   │   └── src\
│   │       ├── voice_pool.rs                      # 新規: VoicePool<const N>
│   │       ├── fractional_delay.rs                # 新規: Lagrange 3 次補間
│   │       ├── note_allocator.rs                  # 新規: voice stealing 戦略
│   │       ├── hold_stack.rs                      # 新規: LinearStack<u8, 16>
│   │       └── params.rs                          # 既存 → コード生成出力に置換（git commit）
│   └── wasm-audio\
│       └── src\
│           └── lib.rs                             # 既存 + synth_set_polyphony_mode 1 関数追加
│
├── web\src\lib\audio\
│   └── generated\
│       └── params.ts                              # 新規: gen-params.mjs 出力（git commit）
│
├── scripts\
│   ├── gen-params.mjs                             # 新規: params.json → Rust + TS 生成
│   └── check-params-sync.mjs                      # 新規: 生成物の drift 検知
│
└── docs\specs\2026-05-07-002-phase2\
    └── （本仕様書群、pre-research + 01〜07 の 8 ファイル）
```

### Phase 1 との変更を伴わないファイル

[Phase 1 02 章 §モノレポレイアウト](../2026-05-06-001-mvp/02-architecture.md#モノレポレイアウト) の **以下は Phase 2 でも変更なし**:

- `Cargo.toml`（ワークスペース定義）
- `rust-toolchain.toml`
- `pnpm-workspace.yaml`
- `web/svelte.config.js`、`web/vite.config.ts`、`web/tsconfig.json`
- `crates/dsp-core/Cargo.toml`（依存ゼロを維持、Phase 2 でも `heapless` 等を追加しない、D23）
- `crates/wasm-audio/Cargo.toml`（dsp-core path 依存のみ、wasm-bindgen 不使用を維持）

## ワークスペース設定

### `Cargo.toml`（ワークスペースルート）

[Phase 1 02 章 §Cargo.toml](../2026-05-06-001-mvp/02-architecture.md#cargotomlワークスペースルート) を **完全維持**。`[profile.release]` の `lto = "fat"` / `codegen-units = 1` / `panic = "abort"` も継続（WASM サイズ削減効果が Phase 2 でも同様に効く）。

### `crates/dsp-core/Cargo.toml`

依存ゼロを Phase 2 でも維持（D23）。新規モジュール 4 件は内部実装のみで、外部 crate を一切追加しない。

### `crates/wasm-audio/Cargo.toml`

dsp-core path 依存のみ。`synth_set_polyphony_mode` 追加でも依存追加なし。

### ルート `package.json`

Phase 1 の既存スクリプトに Phase 2 用スクリプトを追加。

```json
{
  "name": "physics-base-synth",
  "private": true,
  "scripts": {
    "gen:params": "node scripts/gen-params.mjs",
    "check:params-sync": "node scripts/check-params-sync.mjs",
    "build:wasm": "pnpm gen:params && cargo build -p wasm-audio --target wasm32-unknown-unknown --release && node scripts/copy-wasm.mjs release && node scripts/check-wasm-exports.mjs",
    "build:wasm:dev": "pnpm gen:params && cargo build -p wasm-audio --target wasm32-unknown-unknown && node scripts/copy-wasm.mjs debug && node scripts/check-wasm-exports.mjs",
    "dev": "pnpm build:wasm:dev && pnpm --filter web dev",
    "build": "pnpm build:wasm && pnpm --filter web build",
    "preview": "pnpm --filter web preview",
    "check": "cargo check --workspace && pnpm --filter web check && pnpm check:params-sync",
    "fmt": "cargo fmt --all && pnpm --filter web format",
    "lint": "cargo clippy --workspace --all-targets -- -D warnings"
  },
  "packageManager": "pnpm@10.32.1"
}
```

### Phase 1 との差分

| 項目 | Phase 1 | Phase 2 |
|---|---|---|
| `gen:params` | なし | `node scripts/gen-params.mjs` を新規追加 |
| `check:params-sync` | なし | `node scripts/check-params-sync.mjs` を新規追加 |
| `build:wasm` / `build:wasm:dev` | `cargo build && copy-wasm && check-wasm-exports` | **前段に `pnpm gen:params` をチェーン** |
| `check` | `cargo check && svelte-check` | **後段に `pnpm check:params-sync` をチェーン** |
| `dev` / `build` / `preview` / `fmt` / `lint` | 維持 | 完全維持 |

`gen:params` の実行は手動でも可能だが、`build:wasm` 系のチェーンに組み込むことで `params.json` 編集 → 生成漏れによる drift を防ぐ（F15、本仕様 06 章）。

### `.gitignore`

[Phase 1 02 章 §.gitignore](../2026-05-06-001-mvp/02-architecture.md#gitignore主要エントリ) を継承。**Phase 2 で生成物 `crates/dsp-core/src/params.rs` と `web/src/lib/audio/generated/params.ts` は git commit するため `.gitignore` に追加しない**（D25）。

## ParamDescriptor コード生成パイプライン

### 単一ソース `params.json`

ルートに配置。スキーマ詳細は [`03-dsp-core-spec.md` §ParamDescriptor 構造](./03-dsp-core-spec.md#paramdescriptor-構造)。

```json
{
  "params": [
    {
      "id": 0,
      "name": "Damping",
      "min": 0.90,
      "max": 0.9999,
      "default": 0.996,
      "smoothing_tau": 0.02
    },
    {
      "id": 1,
      "name": "Brightness",
      "min": 0.0,
      "max": 1.0,
      "default": 0.5,
      "smoothing_tau": 0.02
    },
    {
      "id": 2,
      "name": "OutputGain",
      "min": 0.0,
      "max": 1.5,
      "default": 0.8,
      "smoothing_tau": 0.01
    }
  ]
}
```

> Phase 2 では Phase 1 と同じ 3 パラメータのみ。Phase 3 で MIDI CC マッピング等を追加する際に拡張する。

### 生成ターゲット 1: `crates/dsp-core/src/params.rs`

`scripts/gen-params.mjs` が JSON を読み、Rust ソースとして以下のような形を出力（詳細は [`03-dsp-core-spec.md` §params.rs 生成出力](./03-dsp-core-spec.md#paramsrs-生成出力例)）:

- `pub enum ParamId` の variant 定義（id 値も）
- `impl ParamId::from_u32`
- `pub const DAMPING_MIN/MAX/DEFAULT` 等の範囲定数
- `pub const PARAM_DESCRIPTORS: &[ParamDescriptor]` の const テーブル

冒頭に `// AUTO-GENERATED FROM params.json — DO NOT EDIT` コメントを必ず挿入。

### 生成ターゲット 2: `web/src/lib/audio/generated/params.ts`

同様に TypeScript ソースを出力:

- `export const PARAM_IDS = { Damping: 0, Brightness: 1, OutputGain: 2 } as const`
- `export type ParamIdValue = ...`
- `export const PARAM_DESCRIPTORS: readonly ParamDescriptor[] = [...]`
- `export interface ParamDescriptor { ... }`

冒頭に `// AUTO-GENERATED FROM params.json — DO NOT EDIT` コメントを必ず挿入。

### 既存 `web/src/lib/audio/messages.ts` との関係

Phase 1 `messages.ts` の `PARAM_IDS` 定義を **`generated/params.ts` から re-export する形に変更**（[`05-web-frontend-spec.md` §messages.ts 変更](./05-web-frontend-spec.md#messagets-変更点)）。

```typescript
// web/src/lib/audio/messages.ts (Phase 2 改修後の冒頭イメージ)
export { PARAM_IDS, PARAM_DESCRIPTORS, type ParamIdValue, type ParamDescriptor } from './generated/params';

// メッセージ型は手書きを維持
export type ToWorkletMessage = ...
```

メッセージ型 (`ToWorkletMessage` / `FromWorkletMessage`) は Phase 1 と同じく手書きで維持。ParamDescriptor 関連のみ生成物から re-export する。

### `scripts/gen-params.mjs` の責務

`scripts/gen-params.mjs` は **(a) 純粋関数群** と **(b) CLI entrypoint** を分けて実装する。これにより check-params-sync が gen-params.mjs を import しても副作用（ファイル書き込み）が起きない設計を保証する。

#### (a) 純粋関数（export）

| 関数 | シグネチャ | 責務 |
|---|---|---|
| `generateRustSource(paramsJson)` | `(object) => string` | params.json オブジェクトを受けて Rust ソース文字列を返す。**ファイル I/O なし、純粋関数** |
| `generateTsSource(paramsJson)` | `(object) => string` | params.json オブジェクトを受けて TypeScript ソース文字列を返す。**ファイル I/O なし、純粋関数** |

#### (b) CLI entrypoint（`import.meta.url === \`file://${process.argv[1]}\`` ガード内）

| 入力 | 処理 | 出力 |
|---|---|---|
| `params.json` | (1) `readFileSync` + `JSON.parse`、(2) `generateRustSource` 呼び出し、(3) `generateTsSource` 呼び出し、(4) `writeFileSync` で 2 ファイルへ書き込み | `crates/dsp-core/src/params.rs`、`web/src/lib/audio/generated/params.ts` |

**重要**: ファイル書き込みは CLI entrypoint 内のみで行う（top-level や module side-effect では絶対に書き込まない）。check-params-sync.mjs が `import { generateRustSource, generateTsSource } from './gen-params.mjs'` した時点ではファイル I/O が走らないこと。

実装は Node.js 標準ライブラリのみ使用（`node:fs` / `node:path` / `node:url`）。Phase 1 `scripts/copy-wasm.mjs` と同じパターンで、外部依存を追加しない（[Phase 1 02 章 §開発スクリプトの呼び出し関係](../2026-05-06-001-mvp/02-architecture.md#開発スクリプトの呼び出し関係) を継承）。

### `scripts/check-params-sync.mjs` の責務

| 入力 | 処理 | 出力 |
|---|---|---|
| `params.json` | (1) `params.json` を読み JSON parse、(2) `generateRustSource` / `generateTsSource` を **純粋関数として呼び**期待文字列を生成、(3) 既存 `params.rs` / `generated/params.ts` の内容を読む、(4) 文字列一致を判定、(5) 不一致なら exit 1 + エラーメッセージ | exit 0 (一致) / exit 1 (drift 検出) |

**重要**: check-params-sync は `gen-params.mjs` の純粋関数だけを利用し、**ファイル書き込みは絶対に行わない**（行うと「checker が実ファイルを更新してしまい drift を見逃す」false positive 化する）。実装時は `generateRustSource` / `generateTsSource` を import するだけで CLI entrypoint は呼ばないこと。

CI で `pnpm check` の最後に走らせ、drift があれば PR がブロックされる（F14 / F15、本仕様 06 章）。

## メモリレイアウトの変更

### Phase 1 のメモリレイアウト

`Engine::prepare` で `KarplusStrong::buffer` 1 本（max 27.5Hz 分 = sample_rate / 27.5 サンプル分の `Vec<f32>`）と `SynthHandle::scratch_l/r` 各 128 サンプル分を一括確保。`process` 中の追加確保ゼロ（D4）。

### Phase 2 のメモリレイアウト

`Engine::prepare` で **N=8 ボイス分の VoicePool 内部バッファ** + `scratch_l/r` を一括確保。

| 領域 | サイズ計算 | サンプルレート 48kHz、N=8、Lagrange 3 次補間 |
|---|---|---|
| `VoicePool::voices[i].buffer` × N | (sample_rate / 27.5).ceil() + 3（Lagrange 補間余裕、D27）| 約 1746 + 3 = 1749 サンプル × 4 bytes × 8 = **約 56 KB** |
| `SynthHandle::scratch_l/r` | 各 128 サンプル × 4 bytes | 1 KB |
| その他 (Engine, VoicePool meta, HoldStack) | const-size 構造体 | 1 KB 未満 |
| **dsp-core / wasm-audio が新規確保するメモリ** | | **約 57 KB** |

> **注意**: 上記 57 KB は dsp-core / wasm-audio が `synth_new` 内で新規に確保する分のみ。実際の WASM linear memory 全体には Rust runtime（stack frame、allocator metadata、static data）も含まれるため、`memory.buffer.byteLength` 全体は数百 KB 規模になる場合がある。Phase 2 で重要なのは **`synth_new` 完了後の `memory.buffer.byteLength` を baseline として記録し、以後 `process_block` / `note_on` / `note_off` / `set_param` / `set_polyphony_mode` のいずれを呼んでも `memory.buffer.byteLength` が一切変化しないこと**（`memory.grow` を発生させないこと）。これは Worklet 側の Float32Array view キャッシュ（D9）の前提条件であり、F17 の検証手順は「synth_new 直後の byteLength を記録 → 8 音同時 + 連打 30 秒 → byteLength が baseline と一致」を確認する。`process` 中の追加確保ゼロ（D4）を保証することで grow を発生させない。

### `synth_new` 内部での確保フロー

```
synth_new(sample_rate=48000, max_block_size=128)
  └─ Box::new(SynthHandle { engine, scratch_l: vec![0; 128], scratch_r: vec![0; 128] })
       └─ Engine::new()
       └─ engine.prepare(sample_rate=48000, max_block_size=128)
            ├─ pool.prepare(sample_rate, max_block_size)
            │    ├─ voices[0].prepare(sr, mb)  → buffer = vec![0; 1749]
            │    ├─ voices[1].prepare(sr, mb)  → buffer = vec![0; 1749]
            │    │  ...
            │    └─ voices[7].prepare(sr, mb)  → buffer = vec![0; 1749]
            ├─ output_gain.set_time_constant(sample_rate, OUTPUT_GAIN_TAU)
            └─ hold_stack.clear()
```

`synth_new` 完了後、`process_block` 中の追加確保はゼロ。F17（`test_no_allocation_in_polyphonic_process`）で保証する（本仕様 06 章）。

## ビルドツールチェーン

[Phase 1 02 章 §ビルドツールチェーン](../2026-05-06-001-mvp/02-architecture.md#ビルドツールチェーン) を **完全継承**（rustup / cargo / Node.js / pnpm / SvelteKit / Vite / esbuild のバージョン要件は同じ）。

### Phase 2 で wasm-opt が必須化候補

Phase 1 では `wasm-opt` は任意。Phase 2 では VoicePool 追加で WASM サイズが増えるため、release ビルドで **`wasm-opt -O3` を必須化** することを 06 章リスク表 R19 で議論する。サイズ目標 gzip < 30 KB を達成できるなら任意のままでよい。

### 開発スクリプトの呼び出し関係（Phase 2 版）

```
pnpm dev
  └─ pnpm build:wasm:dev
  │    ├─ pnpm gen:params                              ← Phase 2 新規
  │    │    └─ node scripts/gen-params.mjs
  │    │         （params.json から Rust + TS を生成）
  │    ├─ cargo build -p wasm-audio --target wasm32-unknown-unknown
  │    │    （内部で生成済み params.rs を使用）
  │    ├─ node scripts/copy-wasm.mjs debug
  │    └─ node scripts/check-wasm-exports.mjs
  │         （synth_set_polyphony_mode を含めて検証）
  └─ pnpm --filter web dev
       ├─ pnpm --filter web build:worklet:dev
       │    └─ esbuild ... → web/static/worklet/synth-processor.js
       │         （内部で generated/params.ts を import）
       └─ vite dev   （http://localhost:5173 でSvelteKitが起動）

pnpm check
  ├─ cargo check --workspace
  ├─ pnpm --filter web check  （svelte-check）
  └─ pnpm check:params-sync                            ← Phase 2 新規
       └─ node scripts/check-params-sync.mjs
            （params.json と生成物の一致を確認）
```

> Rust 側のソース変更時は Phase 1 と同じく `pnpm build:wasm:dev` の再実行が必要。`params.json` 変更時は `gen:params` が前段で走るため別途実行不要。

## ビルド成果物の流れ

[Phase 1 02 章 §ビルド成果物の流れ](../2026-05-06-001-mvp/02-architecture.md#ビルド成果物の流れ) に Phase 2 で生成物 2 件が追加される。

| 段階 | 入力 | 出力 | 場所 | Phase 2 差分 |
|---|---|---|---|---|
| **ParamDescriptor 生成** | `params.json` | `params.rs`、`generated/params.ts` | `crates/dsp-core/src/`、`web/src/lib/audio/generated/` | **Phase 2 新規** |
| Rust ビルド | `crates/wasm-audio/src/lib.rs` | `wasm_audio.wasm` | `target/wasm32-unknown-unknown/{debug,release}/` | 生成済み `params.rs` を使用 |
| WASM コピー | `target/.../wasm_audio.wasm` | `wasm_audio.wasm` | `web/src/lib/wasm/` | 維持 |
| Worklet ビルド | `web/src/lib/audio/synth-processor.ts` | `synth-processor.js` | `web/static/worklet/` | 内部で `generated/params.ts` を import |
| Vite ビルド | `web/src/`、`web/static/` | 静的ファイル群 | `web/build/` | 維持 |

### Worklet スクリプトのビルド経路

[Phase 1 02 章 §Worklet スクリプトのビルド経路](../2026-05-06-001-mvp/02-architecture.md#worklet-スクリプトのビルド経路) を **完全維持**。esbuild の IIFE バンドルが `synth-processor.ts` の依存をすべて inline するため、`generated/params.ts` も同じ経路で取り込まれる。

## アンチパターン回避（Phase 2 版）

[Phase 1 02 章 §アンチパターン回避](../2026-05-06-001-mvp/02-architecture.md#アンチパターン回避アーキテクチャ層) の表に Phase 2 固有の項目を追加。

| アンチパターン | Phase 1 防止箇所 | Phase 2 追加防止箇所 |
|---|---|---|
| `process` 中のヒープ確保 | dsp-core `Engine::prepare` で全バッファ事前確保 | VoicePool::prepare で N 個の voice すべてを一括確保（[03 章](./03-dsp-core-spec.md)）|
| Mutex によるロック | スレッド境界は Worklet 単独で完結 | VoicePool 内も Mutex なし（const generic 配列の直接アクセス） |
| WASM memory.grow | 初期化時のみ alloc_block | VoicePool / HoldStack も `synth_new` 時の一括確保で完結 |
| 細かい JS↔WASM 往復 | 1 ブロック単位 | 維持。voice count 表示しないため追加往復なし（D22）|
| AudioWorklet 内 `console.log` 連発 | 開発時のみ条件分岐 | 維持 |
| AudioParam 多用 | MessagePort 経由（D2）| 維持 |
| **ParamId / PARAM_IDS 二重管理** | （Phase 1 では未対策） | **`params.json` + コード生成で解消（D15 / D24 / D25）** |
| **Vec::push を `note_on` で呼ぶ** | KarplusStrong で禁止 | VoicePool::note_on でも禁止、note_id slot は `Option<u8>` の固定配列 |
| **Lagrange 係数の毎サンプル再計算** | （Phase 1 では fractional delay 非搭載） | `note_on` 時に 1 度計算してキャッシュ（D26）|

## 拡張性の確保

[Phase 1 02 章 §拡張性の確保](../2026-05-06-001-mvp/02-architecture.md#拡張性の確保) を継承。Phase 2 で新たに考慮する点:

- **`Voice` trait に追加する 3 メソッド (`note_id` / `age` / `amplitude`、D19) は Phase 3 で他楽器追加時にも有効**。`KarplusStrong` 以外（例: 将来の `WaveguideString`、`ModalResonator`）も同じ trait を実装すれば VoicePool が型安全に管理可能
- **VoicePool は `Voice` trait のみに依存し KarplusStrong に固有依存しない設計を維持**。Phase 3 で別楽器を入れる場合も VoicePool 自体は変更不要
- **`SynthMode` enum** で mono / poly を実行時切替（D29）。Phase 3 で「duophonic」「unison」等のモードを追加するなら variant 追加で対応
- **`ParamDescriptor` に新フィールド追加（例: `unit: &'static str`、`scale: enum`）したくなった場合**は `params.json` のスキーマ拡張 + `gen-params.mjs` の出力フォーマット拡張で対応。Rust enum / TS const は自動生成されるため drift しない
- **dsp-core は Phase 2 でも依存ゼロを維持** （D23、`heapless` 等を追加しない）。VST/CLAP / CLI / ネイティブアプリへの転用余地を Phase 1 と同じ条件で残す
