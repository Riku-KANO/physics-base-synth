# 02. 全体アーキテクチャとプロジェクト構成

## 目的

MVP で採用するモノレポ構成、信号フロー、ビルドツールチェーン、開発スクリプトを定義する。レイヤごとの責務分担を明示し、`03〜05` の詳細仕様の前提を固める。

## 他文書との関係

- 上流: [`01-overview.md`](./01-overview.md)（スコープと設計判断）
- 並列: [`03-dsp-core-spec.md`](./03-dsp-core-spec.md)、[`04-wasm-audio-spec.md`](./04-wasm-audio-spec.md)、[`05-web-frontend-spec.md`](./05-web-frontend-spec.md)
- 下流: [`06-build-and-verify.md`](./06-build-and-verify.md)（具体的なコマンドと検証手順）

## 信号フローと責務分担

```
┌── メインスレッド（UI thread） ─────────────────────────────┐
│                                                            │
│  Svelte コンポーネント                                      │
│   ├ StartButton.svelte  ── AudioContext.resume()            │
│   ├ Keyboard.svelte     ── pointerdown/pointerup            │
│   ├ ParamSlider.svelte  ── oninput → setParam               │
│   └ MidiSelect.svelte   ── navigator.requestMIDIAccess()    │
│            │                                                │
│            ▼                                                │
│  SynthEngine（src/lib/audio/engine.ts）                     │
│   ├ AudioContext, AudioWorkletNode を所有                   │
│   ├ note_on/note_off/setParam を MessagePort で送信         │
│   └ WASM bytes を fetch して Worklet へ転送                 │
└────────────────────────────┬───────────────────────────────┘
                             │ AudioWorkletNode.port.postMessage
                             ▼
┌── 音声レンダリングスレッド（Worklet） ─────────────────────┐
│                                                            │
│  SynthProcessor（synth-processor.ts → bundled .js）         │
│   ├ port.onmessage で init/noteOn/noteOff/setParam 受信     │
│   ├ WebAssembly.instantiate で wasm-audio をロード          │
│   ├ SynthHandle を保持                                      │
│   └ process(_, outputs) で 128frames を生成                 │
│                                                            │
└────────────────────────────┬───────────────────────────────┘
                             │ FFI（pointer + length）
                             ▼
┌── WASM linear memory ─────────────────────────────────────┐
│                                                            │
│  wasm-audio crate（cdylib）                                 │
│   └ SynthHandle ─ Engine（dsp-core を内包）                 │
│                                                            │
│  dsp-core crate（rlib, std依存最小）                        │
│   └ Engine ─ KarplusStrong ─ SmoothedValue / XorShift32     │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

### 責務の分離原則

| レイヤ | 責務 | 禁止事項 |
|---|---|---|
| Svelte UI | ユーザー入力の収集、UI状態管理、可視化 | DSP処理、AudioContext時系列のクリティカル処理 |
| SynthEngine（main thread） | AudioContextの起動、Worklet初期化、メッセージ仲介 | DSP処理、サンプル単位処理 |
| SynthProcessor（worklet） | WASMロード、`process()` 委譲、ブロック転送 | UI状態管理、fetch、console.logの常時呼び出し |
| wasm-audio | FFI境界の整備、ポインタ管理 | DSP本体ロジック |
| dsp-core | 物理モデル本体、リアルタイム安全な処理 | wasm-bindgen依存、JS依存、`std::sync::Mutex`、メモリ確保（`prepare`を除く） |

## モノレポレイアウト

```
C:\Users\81903\projects\physics-base-synth\
├── Cargo.toml                   # ワークスペース定義
├── rust-toolchain.toml          # Rust toolchain pinning
├── package.json                 # ルートのpnpm-workspaces定義
├── pnpm-workspace.yaml
├── .gitignore
├── README.md
│
├── crates\
│   ├── dsp-core\
│   │   ├── Cargo.toml
│   │   └── src\
│   │       ├── lib.rs
│   │       ├── traits.rs
│   │       ├── params.rs
│   │       ├── smoothing.rs
│   │       ├── rng.rs
│   │       ├── karplus_strong.rs
│   │       ├── voice.rs
│   │       └── engine.rs
│   └── wasm-audio\
│       ├── Cargo.toml
│       └── src\
│           └── lib.rs
│
├── web\
│   ├── package.json
│   ├── svelte.config.js
│   ├── vite.config.ts
│   ├── tsconfig.json
│   ├── static\
│   │   └── worklet\
│   │       └── synth-processor.js   # ビルド時生成（後述）
│   └── src\
│       ├── app.html
│       ├── lib\
│       │   ├── audio\
│       │   │   ├── engine.ts
│       │   │   ├── synth-processor.ts
│       │   │   ├── messages.ts
│       │   │   └── wasm-loader.ts
│       │   ├── input\
│       │   │   ├── midi.ts
│       │   │   └── note-utils.ts
│       │   ├── actions\
│       │   │   └── pc-keyboard.svelte.ts   # Svelte 5 $effectベース action: keydown/keyup
│       │   ├── components\
│       │   │   ├── StartButton.svelte
│       │   │   ├── Keyboard.svelte
│       │   │   ├── ParamSlider.svelte
│       │   │   └── MidiSelect.svelte
│       │   ├── state\
│       │   │   └── synth.svelte.ts  # Svelte 5: $state ベースの共有ステート（.svelte.ts 拡張子）
│       │   └── wasm\                # cargo build 出力のコピー先（gitignore）
│       └── routes\
│           ├── +layout.svelte
│           └── +page.svelte
│
└── docs\specs\2026-05-06-001-mvp\
    ├── pre-research.md
    ├── 01-overview.md
    ├── 02-architecture.md
    ├── 03-dsp-core-spec.md
    ├── 04-wasm-audio-spec.md
    ├── 05-web-frontend-spec.md
    ├── 06-build-and-verify.md
    └── 07-implementation-checklist.md
```

## ワークスペース設定

### `Cargo.toml`（ワークスペースルート）

```toml
[workspace]
resolver = "2"
members = ["crates/dsp-core", "crates/wasm-audio"]

[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"          # WASMサイズ削減

[profile.dev]
opt-level = 1            # WASMでのデバッグ実行に十分な速度を確保
```

### `rust-toolchain.toml`

```toml
[toolchain]
channel = "stable"
targets = ["wasm32-unknown-unknown"]
components = ["rustfmt", "clippy"]
```

> 注: 安定版を使用し、特定バージョンへのピン留めはMVPでは行わない。再現性が必要になった時点で `channel = "1.83.0"` のように明示する。

### `pnpm-workspace.yaml`

```yaml
packages:
  - "web"
```

### ルート `package.json`

`wasm-pack` は使わず（[04章の設計判断](./04-wasm-audio-spec.md#設計判断-wasm-bindgen-を使わず-c-abi-で公開する) 参照）、`cargo build` の生 WASM を `scripts/copy-wasm.mjs` で `web/src/lib/wasm/` へコピーする。

```json
{
  "name": "physics-base-synth",
  "private": true,
  "scripts": {
    "build:wasm": "cargo build -p wasm-audio --target wasm32-unknown-unknown --release && node scripts/copy-wasm.mjs release && node scripts/check-wasm-exports.mjs",
    "build:wasm:dev": "cargo build -p wasm-audio --target wasm32-unknown-unknown && node scripts/copy-wasm.mjs debug && node scripts/check-wasm-exports.mjs",
    "dev": "pnpm build:wasm:dev && pnpm --filter web dev",
    "build": "pnpm build:wasm && pnpm --filter web build",
    "preview": "pnpm --filter web preview",
    "check": "cargo check --workspace && pnpm --filter web check",
    "fmt": "cargo fmt --all && pnpm --filter web format",
    "lint": "cargo clippy --workspace --all-targets -- -D warnings"
  },
  "packageManager": "pnpm@9"
}
```

`scripts/copy-wasm.mjs` の実装方針は [04章のビルド方法](./04-wasm-audio-spec.md#ビルド方法) 参照。

### `.gitignore`（主要エントリ）

```
# Rust
/target/
**/target/

# Node
node_modules/
.pnpm-store/

# cargo build → copy-wasm.mjs の出力先
web/src/lib/wasm/

# SvelteKit
web/.svelte-kit/
web/build/

# Editor
.vscode/
.idea/
```

## ビルドツールチェーン

| ツール | バージョン目安 | 役割 |
|---|---|---|
| rustup / cargo | 1.83以上（stable） | Rustビルド |
| `wasm32-unknown-unknown` target | rustup target add で導入 | WASM出力 |
| wasm-opt（任意） | binaryen 同梱、最新 | WASMサイズ最適化（リリース時） |
| Node.js | 20 LTS以上 | ビルドツール実行、`copy-wasm.mjs` 実行 |
| pnpm | 9以上（corepack 経由） | ワークスペース管理 |
| SvelteKit | 2系（`sv` で生成時の最新） | フロントエンドフレームワーク |
| Vite | SvelteKit同梱 | dev server、HMR |
| @sveltejs/adapter-static | SvelteKit 2系対応 | 静的ビルド |
| esbuild | 0.24以上 | Worklet スクリプトの単独バンドル |

## 開発スクリプトの呼び出し関係

```
pnpm dev
  └─ pnpm build:wasm:dev
  │    ├─ cargo build -p wasm-audio --target wasm32-unknown-unknown
  │    │    （target/wasm32-unknown-unknown/debug/wasm_audio.wasm を生成）
  │    └─ node scripts/copy-wasm.mjs debug
  │         （web/src/lib/wasm/wasm_audio.wasm へコピー）
  └─ pnpm --filter web dev
       ├─ pnpm --filter web build:worklet
       │    └─ esbuild ... → web/static/worklet/synth-processor.js
       └─ vite dev   （http://localhost:5173 でSvelteKitが起動、HMRが有効）
```

> 注: Rust側のソース変更時は `pnpm build:wasm:dev` の再実行が必要。MVPでは手動再ビルドで十分。後続フェーズで `cargo-watch` などによる自動再ビルドを検討する。

## ビルド成果物の流れ

| 段階 | 入力 | 出力 | 場所 |
|---|---|---|---|
| Rust ビルド | `crates/wasm-audio/src/lib.rs` | `wasm_audio.wasm` | `target/wasm32-unknown-unknown/{debug,release}/` |
| WASM コピー | `target/.../wasm_audio.wasm` | `wasm_audio.wasm` | `web/src/lib/wasm/` |
| Worklet ビルド | `web/src/lib/audio/synth-processor.ts` | `synth-processor.js`（バンドル済み単一ファイル） | `web/static/worklet/`（後述） |
| Vite ビルド | `web/src/`、`web/static/` | 静的ファイル群 | `web/build/` |

### Worklet スクリプトのビルド経路

AudioWorkletGlobalScope は `import` 文がブラウザによって制限されるため、Worklet スクリプトは **依存関係をすべて単一ファイルに inline** する必要がある。MVPでは以下の方針を採用:

1. `web/src/lib/audio/synth-processor.ts` に Worklet 本体を記述
2. **esbuild** で IIFE 形式の独立バンドルとして `web/static/worklet/synth-processor.js` に出力（`web/package.json` の `build:worklet`/`build:worklet:dev`/`build:worklet:watch` スクリプトで実行）
3. 実行時は `` audioWorklet.addModule(`${base}/worklet/synth-processor.js`) `` で読み込む（`base` は `$app/paths` から import。SvelteKit のサブパス配信に対応）

> esbuild スクリプトと Vite の関連設定は [`05-web-frontend-spec.md`](./05-web-frontend-spec.md) で詳細化する。Vite 自体に Worklet バンドルを任せず別パイプラインに分けるのは、AudioWorkletGlobalScope の import 制約に対応するため。

## アンチパターン回避（アーキテクチャ層）

| アンチパターン | 防止箇所 |
|---|---|
| `process` 中のヒープ確保 | dsp-core の `Engine::prepare` で全バッファ事前確保（[03章参照](./03-dsp-core-spec.md)） |
| Mutexによるロック | スレッド境界はWorklet単独で完結。`Arc<Mutex<...>>` を持ち込まない |
| WASM memory.grow | 初期化時のみ `alloc_block` を呼び、以降は同じポインタを使い回す（[04章参照](./04-wasm-audio-spec.md)） |
| 細かい JS↔WASM 往復 | 1ブロック（128 frames）単位で1回呼び出し。サンプル単位の関数呼び出しは禁止 |
| AudioWorklet 内 `console.log` 連発 | 開発時のみ条件分岐で出力。本番ビルドで除去 |
| AudioParam 多用による複雑化 | パラメータ送信は MessagePort 経由（D2に従う） |

## 拡張性の確保

- `dsp-core` は WASM 非依存・std依存最小に保つことで、将来の VST/CLAP（[NIH-plug](https://github.com/robbert-vdh/nih-plug) 等）、CLI、ネイティブアプリへの転用余地を残す
- `Voice` trait は将来のポリフォニー化で `Vec<Box<dyn Voice>>` ではなく **`[KarplusStrong; N]` 配列**として確保する想定。trait は型安全な抽象化のためのみに使い、dyn dispatch は採用しない
- パラメータ ID（`ParamId`）は将来追加されることを前提に `#[non_exhaustive]` を付ける（u32値は変更しない）
