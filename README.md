# physics-base-synth

物理ベース・シンセサイザー（Karplus–Strong 単音モデル）の MVP。Rust + WebAssembly + Svelte 5 (SvelteKit) で実装。

## 動作環境

- **推奨ブラウザ**: Chrome / Edge 最新版（Chromium 系）
  - Web MIDI と AudioWorklet は **secure context (HTTPS / localhost) 必須**
  - Firefox 126+ で Web MIDI 対応
  - iOS Safari は HTTPS 配信下でのみ動作（StartButton のユーザージェスチャ必須）
- Rust stable 1.83+ (target: `wasm32-unknown-unknown`)
- Node.js 20 LTS+
- pnpm 9+ (corepack 経由)

## セットアップ

```powershell
rustup target add wasm32-unknown-unknown
corepack enable
corepack prepare pnpm@latest --activate
pnpm install
```

## 開発

```powershell
pnpm dev
```

`http://localhost:5173/` を開いて「▶ Start Audio」をクリック → A〜L キーで発音。

## 主なスクリプト

| コマンド | 内容 |
|---|---|
| `pnpm build:wasm:dev` | dev 用 WASM ビルド + コピー + export 検証 |
| `pnpm build:wasm` | release 用 WASM ビルド |
| `pnpm dev` | WASM(dev) ビルド後、Vite dev server 起動 (5173) |
| `pnpm build` | 本番ビルド（静的サイト → `web/build/`） |
| `pnpm preview` | 本番プレビュー (http://localhost:4173) |
| `pnpm check` | `cargo check --workspace` + `svelte-check` |
| `pnpm lint` | `cargo clippy --workspace --all-targets -- -D warnings` |
| `pnpm fmt` | `cargo fmt` + prettier |

## アーキテクチャ概要

```
Svelte UI (main thread) ── MessagePort ─→ AudioWorkletProcessor
                                            │ FFI (C ABI、wasm-bindgen 不使用)
                                            ▼
                                       wasm-audio (cdylib)
                                            │
                                            ▼
                                       dsp-core (rlib)
                                       Karplus–Strong / SmoothedValue / XorShift32
```

詳細は `docs/specs/2026-05-06-001-mvp/` 配下の仕様書 (01〜07 + pre-research) を参照。

## 自己検証手順 (F1〜F9)

| ID | 手順 | 期待結果 |
|---|---|---|
| **F1** | `pnpm dev` → http://localhost:5173 → Start Audio → 「Play C4 (test)」をクリック | 弾けたような減衰音 |
| **F2** | 画面鍵盤の任意の鍵をクリック | F1 と同じ音色で発音 |
| **F3** | A〜L 行 + W〜O 行を押す | C4〜D5 の半音階 |
| **F4** | USB MIDI キーボードを接続 → MidiSelect で選択 → 鍵盤押下 | note_on/off で発音 |
| **F5** | Damping を 0.99 → 0.999 へドラッグ | 音の減衰時間が伸びる |
| **F6** | Brightness を 0.1 → 0.9 へドラッグ | 高域含有量が変化（明るくなる） |
| **F7** | スライダーを左右に高速ドラッグ | プチノイズが聞こえない |
| **F8** | DevTools Performance で記録しながら A キーを 100 連打。または `synth-processor.ts` の `process` に `memory.buffer.byteLength` の不変チェックを一時的に挿入 | WASM memory が grow しない |
| **F9** | iPhone Safari (HTTPS、ngrok / Cloudflare Tunnel / mkcert いずれか) でアクセス | Start Audio タップで発音 |

### F8 の補助検証コード（必要に応じ一時挿入）

`web/src/lib/audio/synth-processor.ts` の `process` 内に追加して連打時に grow しないことを確認、確認後削除：

```ts
// const cur = exports.memory.buffer.byteLength;
// if (this._baselineByteLen === 0) this._baselineByteLen = cur;
// else if (cur !== this._baselineByteLen) {
//   this.port.postMessage({ type: 'debug', message: `[F8] grew ${this._baselineByteLen}→${cur}` });
//   this._baselineByteLen = cur;
// }
```

## クレート構成

| クレート | 種類 | 役割 |
|---|---|---|
| `crates/dsp-core` | rlib（純粋 Rust、std依存最小） | Karplus–Strong / SmoothedValue / XorShift32 / Engine |
| `crates/wasm-audio` | cdylib（C ABI、wasm-bindgen 不使用） | `synth_*` 関数群を `#[unsafe(no_mangle)] extern "C"` で公開 |
| `web` | SvelteKit + adapter-static | UI / AudioWorklet / Web MIDI |

## 既知の妥協（Phase 2 で対応）

- **整数ディレイ**のため低音域でピッチ誤差（A1=55Hz で約2.3%）
- **モノフォニー**（last-note priority の簡易版、hold note stack なし）
- **Extended KS**（fractional delay / loss filter / pick position）未実装
- **ボディ共鳴**未実装
- **MIDI CC によるパラメータ制御**未対応（UI スライダーのみ）

## ライセンス

未定（MVP段階）。
