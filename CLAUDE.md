# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## プロジェクト概要

ブラウザで動作する物理ベース・シンセサイザー（Karplus–Strong 単音モデル）の MVP。Rust + WebAssembly + Svelte 5 (SvelteKit) で構成。

公開: <https://riku-kano.github.io/physics-base-synth/>

## 仕様書ドリブン開発

実装は **仕様書を主導**として進める。仕様書の改変は外部レビュー前提のため、**仕様書から逸脱しないことが最優先制約**。仕様書通りで冗長と感じる箇所も基本的にそのまま。

| ディレクトリ | 内容 |
|---|---|
| `docs/specs/<date>-<seq>-<name>/` | イテレーション単位の仕様書群（pre-research + 01〜07）。実装着手前に通読 |
| `docs/retrospective/<iteration-name>.md` | 各イテレーション完了時の振り返り。次フェーズへの引き継ぎ事項を含む |

現在のイテレーション: `docs/specs/2026-05-08-004-phase4a/`（Phase 4a / F38b 計測 + LFO + Mod Wheel + Preset + 多楽器 6 種 / 仕様書策定中、実装は新セッションで `IMPLEMENTATION_PROMPT.md` を起点に進める）。Phase 4b は別計画扱い（ピアノ音色 / Stretching all-pass）。詳細は `docs/retrospective/2026-05-07-003-phase3.md`。

完了済みイテレーション:
- `docs/specs/2026-05-06-001-mvp/` (Phase 1 / MVP) — 単音 Karplus-Strong、整数ディレイ、A1=55Hz で 2.3% 偏移
- `docs/specs/2026-05-07-002-phase2/` (Phase 2 / polyphony) — 8 音 polyphony、Lagrange 補間、ParamDescriptor 生成、hold note stack
- `docs/specs/2026-05-07-003-phase3/` (Phase 3) — Modal Body / loss filter / pick position / Thiran allpass (D36 案 D 採用) / Brightness 補正 / soft clip / Pitch Bend / Sustain / VoiceMeter UI

新イテレーションの振り返りを作る場合は `/retrospective <iteration-name>` カスタムコマンド（`.claude/commands/retrospective.md`）。

## 主要コマンド

ルートで `pnpm` から実行。Windows + git bash 環境。

| コマンド | 内容 |
|---|---|
| `pnpm dev` | dev WASM ビルド + Vite dev server (5173) |
| `pnpm build` | release WASM ビルド + SvelteKit static build → `web/build/` |
| `pnpm preview` | 本番ビルドをプレビュー (4173) |
| `pnpm build:wasm` / `pnpm build:wasm:dev` | release/dev WASM のみビルド + `scripts/copy-wasm.mjs` で配置 + `scripts/check-wasm-exports.mjs` で export 検証 |
| `pnpm check` | `cargo check --workspace` + `svelte-check` |
| `pnpm lint` | `cargo clippy --workspace --all-targets -- -D warnings` |
| `pnpm fmt` | `cargo fmt --all` + prettier |
| `cargo test -p dsp-core` | dsp-core ユニットテスト |
| `cargo test -p dsp-core test_<name>` | 個別テスト実行 |
| `pnpm --filter ./web check` | svelte-check 単独 |
| `pnpm --filter ./web lint` | prettier + eslint |
| `pnpm --filter ./web build:worklet:dev` | esbuild で AudioWorklet バンドルのみ再生成 |

GitHub Pages デプロイは `main` への push で自動（`.github/workflows/deploy.yml`）。PR と非 main push では `ci.yml` が build / test / lint を回す。

## アーキテクチャの 4 レイヤ

```
Svelte UI (main thread) ──MessagePort──▶ AudioWorkletProcessor
                                              │ FFI (C ABI, raw exports)
                                              ▼
                                         wasm-audio (cdylib)
                                              │
                                              ▼
                                         dsp-core (rlib)
```

| レイヤ | パス | 責務 | 禁止事項 |
|---|---|---|---|
| Svelte UI | `web/src/lib/{components,state,actions,input}/` + `web/src/routes/` | ユーザー入力収集、UI状態 (`$state`)、画面表示 | DSP 処理、AudioContext クリティカル経路 |
| SynthEngine (main) | `web/src/lib/audio/engine.ts` | AudioContext 起動、Worklet 初期化、MessagePort 仲介、rAF パラメータスロットル | DSP 処理 |
| SynthProcessor (Worklet) | `web/src/lib/audio/synth-processor.ts` | WASM ロード、`process()` 委譲、Float32Array view キャッシュ | UI 状態管理、fetch、毎フレーム new |
| wasm-audio | `crates/wasm-audio/src/lib.rs` | C ABI 境界、ポインタ管理、`SynthHandle` 保持 | DSP 本体ロジック |
| dsp-core | `crates/dsp-core/src/` | Karplus–Strong / SmoothedValue / XorShift32 / Engine | wasm-bindgen 依存、`std::sync::Mutex`、`prepare` 以外でのヒープ確保 |

## 必ず守る制約（Phase 1 で確定済み、Phase 2 / Phase 3 でも維持）

- **`process` ホットパス中のヒープ確保ゼロ**: `Engine::prepare` で `KarplusStrong::buffer` と `SynthHandle::scratch_l/r` を一括確保し、以降は `length` フィールドの更新のみ。`Vec::resize` / `Vec::push` を `process_sample` / `note_on` 経路で呼ばない（`test_no_allocation_in_process` で保証）。
- **C ABI のみ、`wasm-bindgen` 不使用**: 公開関数は `#[unsafe(no_mangle)] pub extern "C" fn`。`wasm-pack` も使わず、`cargo build --target wasm32-unknown-unknown` の生 WASM を `scripts/copy-wasm.mjs` で配置。Worklet 側は `WebAssembly.instantiate(bytes, { env: {} })` で直接呼ぶ。`scripts/check-wasm-exports.mjs` で export 名 drift を検知。
- **AudioWorklet の Float32Array view を init 時にキャッシュ**: `process()` 内で `new Float32Array(...)` を作らない。`memory.buffer` 変化時のみ `refreshViews()`（通常は発火しない）。
- **MessagePort + `SmoothedValue` でクリック対策**: パラメータ送信は AudioParam ではなく MessagePort 経由、メインスレッドで rAF (60Hz) スロットル、Worklet 側 `SmoothedValue`（tau=0.02s for damping/brightness、0.01s for outputGain）で吸収。
- **secure context 必須**: `window.isSecureContext` チェックを `SynthEngine.start()` と `MidiSelect.svelte` に組み込み済み。`AudioContext` 作成と `resume()` は **必ず `StartButton.onclick` 内**（iOS Safari 対策）。
- **denormal flush**: `KarplusStrong::process_sample` 末尾の `+1e-25 -1e-25`（仕様書 D6）。コストゼロのため削除しない。
- **Svelte 5 runes ベース**: `$state` / `$derived` / `$effect` / `$props` / `$bindable`、イベントは `onclick` 等の小文字記法（`on:click` や `|preventDefault` 修飾子は使わない）。共有ステートは `.svelte.ts` 拡張子（`writable` ストアは使わない）。副作用は Svelte action（`use:action`）でカプセル化。

## 開発フロー

- **`main` は branch protected**: PR 必須、CI `build` 緑必須、force push / 削除禁止。`enforce_admins: false` のため owner は緊急時に bypass 可能（ファイル追加のみの軽微な変更で活用）。
- **コミット粒度**: 仕様書 07 章の Step 単位で論理的に切る（過度に細分化しない）。MVP は 6 commits 目安。
- **Co-Authored-By トレーラー**: Claude Code 標準で付与。明示指定なら HEREDOC で `Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>`。
- **Hooks**: `.claude/settings.json` で `PostToolUse`（編集ファイルだけ自動 fmt） + `Stop`（lint check、failure は systemMessage で警告のみ、ブロックしない）を設定。新セッション開始時は `/hooks` を 1 度開いて reload。

## 環境依存の罠（再発防止）

| 罠 | 回避 |
|---|---|
| `pnpm/action-setup` の `version` と `package.json` の `packageManager` 重複指定 → `ERR_PNPM_BAD_PM_VERSION` | action 側の `version` を消す |
| `jq` が Windows + git bash で利用不可 | hook スクリプトは Node.js で stdin JSON parse（`process.stdin.on('data')`） |
| Windows bash で `BASE_PATH=/foo pnpm build` が `BASE_PATH=C:/Program Files/Git/foo` に変換 (MSYS path conv) | `MSYS_NO_PATHCONV=1` を前置 |
| Svelte 5 `svelte/prefer-svelte-reactivity` が effect-local な `new Set` も警告 | `// eslint-disable-next-line svelte/prefer-svelte-reactivity` で局所抑止 |
| `AudioWorkletProcessor` 型が `lib.dom.d.ts` に未収録 | `synth-processor.ts` 冒頭に `declare class AudioWorkletProcessor` |
| `wasm-audio/src/lib.rs` の C ABI ポインタ deref が `clippy::not_unsafe_ptr_arg_deref` でエラー | crate-level `#![allow(clippy::not_unsafe_ptr_arg_deref)]` |
| ローカル build と CI build で WASM ハッシュが異なる（404 確認時の罠） | `_app/immutable/nodes/<n>.js` から実 hash を抽出して再確認 |

## Phase 2 着手前の必須前提

Phase 1 の **F1〜F9 の音響面実機検証は 0/9（未検証）**。Phase 2 で土台が変わる前に：

1. <https://riku-kano.github.io/physics-base-synth/> または `pnpm dev` で F1〜F7 を確認
2. F8 は `synth-processor.ts` に `memory.buffer.byteLength` 不変チェックを一時挿入して 100 連打 → 確認後コード削除
3. F9 は iPhone Safari で HTTPS URL（Pages or ngrok / Cloudflare Tunnel）

詳細は `docs/retrospective/2026-05-06-001-mvp.md` の §7「次イテレーションへの引き継ぎ」。
