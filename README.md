# physics-base-synth

物理ベース・シンセサイザー（Karplus–Strong）の polyphony 対応版。Rust + WebAssembly + Svelte 5 (SvelteKit) で実装。

## 動作環境

- **推奨ブラウザ**: Chrome / Edge 最新版（Chromium 系）
  - Web MIDI と AudioWorklet は **secure context (HTTPS / localhost) 必須**
  - Firefox 126+ で Web MIDI 対応
  - iOS Safari は HTTPS 配信下でのみ動作（StartButton のユーザージェスチャ必須）
- Rust stable 1.83+ (target: `wasm32-unknown-unknown`)
- Node.js 24+ (GitHub Pages Workflow も Node 24 で実行)
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
| `pnpm gen:params` | `params.json` から `crates/dsp-core/src/params.rs` と `web/src/lib/audio/generated/params.ts` を生成 |
| `pnpm check:params-sync` | 生成物が `params.json` と同期しているか CI で検証 (drift で exit 1) |
| `pnpm build:wasm:dev` | `gen:params` → dev 用 WASM ビルド → コピー → export 検証 |
| `pnpm build:wasm` | release 用 WASM ビルド (同上の release 版) |
| `pnpm dev` | WASM(dev) ビルド後、Vite dev server 起動 (5173) |
| `pnpm build` | 本番ビルド（静的サイト → `web/build/`） |
| `pnpm preview` | 本番プレビュー (http://localhost:4173) |
| `pnpm check` | `cargo check --workspace` + `svelte-check` + `check:params-sync` |
| `pnpm lint` | `cargo clippy --workspace --all-targets -- -D warnings` |
| `pnpm fmt` | `cargo fmt` + prettier |

## アーキテクチャ概要 (Phase 2)

```
Svelte UI (main thread) ── MessagePort ─→ AudioWorkletProcessor
                                            │ FFI (C ABI、wasm-bindgen 不使用)
                                            ▼
                                       wasm-audio (cdylib)
                                            │
                                            ▼
                                       dsp-core (rlib)
                                       Engine / VoicePool<8> / KarplusStrong (Lagrange 3 次補間)
                                       FractionalDelay / NoteAllocator / HoldStack / SmoothedValue / XorShift32
                                       ParamDescriptor (params.json から生成)
```

詳細は仕様書 (`docs/specs/`) を参照:
- Phase 1 (MVP): `docs/specs/2026-05-06-001-mvp/`
- Phase 2 (polyphony / fractional delay / ParamDescriptor / hold note stack): `docs/specs/2026-05-07-002-phase2/`

## 自己検証手順

### Phase 1 (F1〜F9): 単音動作

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
| **F9** | iPhone Safari (HTTPS) でアクセス | Start Audio タップで発音 |

### Phase 2 (F10〜F25): polyphony / pitch / hold stack / size

| ID | 手順 | 期待結果 |
|---|---|---|
| **F10** | PC キーボードで A, S, D, F, G, H, J, K の 8 鍵を同時押下 | 8 音が重畳して聞こえ、クリップ歪みなし |
| **F11** | F10 の状態から 9 音目 (KeyL) を押下 | voice stealing で energy 閾値以下のうち最古ボイスが置換、耳障りなクリックなし |
| **F12** | `cargo test -p dsp-core --test pitch_accuracy --release` (test_pitch_a4 / c6 含む) | A1〜C6 のピッチが ± 0.5% 以内 (test_pitch_c8 は KS-Lagrange 限界で `#[ignore]`、Phase 3 課題) |
| **F13** | F12 の `test_pitch_a1` (Phase 1 課題解消) | 55Hz が ± 0.5% 以内 |
| **F14** | `pnpm check:params-sync` | `ParamDescriptor sync OK.` |
| **F15** | `params.json` を編集後 `pnpm gen:params` を実行せず `pnpm check:params-sync` | exit 1、`params.rs is out of sync` 表示 |
| **F16** | DevTools Performance で 8 音同時発音中の `process()` 1 回の所要時間を計測 | 平均 < 1.5ms (CPU 予算 2.67ms 内) |
| **F17** | `synth-processor.ts` 末尾に `synth_new` 直後の `byteLength` baseline 比較を一時挿入し、8 音同時 + 連打を 30 秒継続 | `[F17] WASM memory grew` 警告が一度も出ない (確認後コード削除) |
| **F18** | dev ビルドで DevTools Console から `__synthDev.setMode('mono')` → C 押→D 押→D 離→C 復帰→C 離→無音 | D 離した時点で C に復帰して鳴る |
| **F19** | mono モードで 17 鍵を順次押下 | 最古ノートはスタックから消えるが、押下中のキーは全部残る (cargo test で検証済) |
| **F20** | `__synthDev.setMode('mono'/'poly')` を 10 回連続で切替 | クラッシュなし、無音化なし、切替時クリックなし |
| **F21** | `pnpm build` 後、`web/build/_app/immutable/assets/wasm_audio.*.wasm` を gzip 圧縮 | gzip < 30 KB (実測 10.58 KB) |
| **F22** | `pnpm build` 後、`web/build/worklet/synth-processor.js` のバイトサイズ | < 10 KB (実測 5.04 KB) |
| **F23** | poly モードで 9 鍵以降を高速連打 (1 秒に 5 回) を 10 秒継続 | 知覚できる耳障りなクリックなし |
| **F24 (a)** | OutputGain ≤ 1.0 + 通常演奏で 30 秒継続 | ハードクリップ歪みが知覚されない (1/sqrt(N) スケール効果) |
| **F24 (b)** | OutputGain=1.5 + 8 鍵全力強打 | 最悪ケースで歪みが出る場合あり (Phase 3 で limiter 検討) |
| **F25** | `docs/retrospective/2026-05-06-001-mvp.md` §2 で Phase 1 F1〜F9 が達成済みと記載 | F1〜F9 すべて達成済み記載 |

### dsp-core ユニットテスト一覧

`cargo test -p dsp-core` で 38 件パス + 1 件 ignored:

- Phase 1 既存 (12 件): silence_when_inactive / energy_rises_after_note_on / decay_with_low_damping / length_matches_freq / no_allocation_in_process / paramid_roundtrip / damping_preserved_across_note_on / engine_processes_block_without_panic / midi_to_freq_a4 / poly_mode_independent_voices / setparam_clamps_out_of_range / note_on_first_block_nonzero
- fractional_delay (4 件): d_zero / d_one / coeffs_sum_to_one / clamps_out_of_range
- note_allocator (3 件): picks_quietest / falls_back_to_oldest / among_quiet_picks_oldest
- hold_stack (4 件): push_pop_basic / overflow_drops_oldest / remove_middle / clear
- voice_pool_tests (7 件): allocates_distinct_voices / same_note_replace / note_on_returns_assigned_index / engine_does_not_revive_released_voice / steals_quietest / polyphonic_mix_rms_bounded / no_allocation_in_polyphonic_process
- hold_stack_engine_tests (3 件): last_note_priority / overflow_in_engine / mode_switch_no_break
- pitch_accuracy (5 PASS, 1 IGNORED): a1 / a2 / a4 / c6 (PASS) / long_term_stability_high_damping (PASS) / c8 (`#[ignore]`、KS-Lagrange 限界、R23 フォールバック)

## クレート構成

| クレート | 種類 | 役割 |
|---|---|---|
| `crates/dsp-core` | rlib（純粋 Rust、std 依存最小、Phase 2 でも `heapless` 等の外部 crate なし） | Engine / VoicePool / KarplusStrong (Lagrange 3 次補間) / NoteAllocator / HoldStack / SmoothedValue / XorShift32 / ParamDescriptor (生成) |
| `crates/wasm-audio` | cdylib（C ABI、wasm-bindgen 不使用） | `synth_*` 関数群を `#[unsafe(no_mangle)] extern "C"` で公開。Phase 1 の 10 関数 + Phase 2 の `synth_set_polyphony_mode` |
| `web` | SvelteKit + adapter-static | UI / AudioWorklet / Web MIDI |

## ParamDescriptor 単一ソース

`params.json` (リポジトリルート) を編集して `pnpm gen:params` を実行すると、Rust 側 (`crates/dsp-core/src/params.rs`) と TS 側 (`web/src/lib/audio/generated/params.ts`) が同時に更新される。`pnpm build:wasm` / `pnpm dev` のチェーンで自動再生成、`pnpm check:params-sync` が CI 上で drift を検知する。

## Phase 2 で解消された Phase 1 の妥協

- ✅ **fractional delay** (Lagrange 3 次補間) で A1=55Hz の ピッチ誤差 (Phase 1 で約 2.3%) を ± 0.5% 以内に解消
- ✅ **8 音 polyphony** (`VoicePool<8>`) + voice stealing (energy 閾値以下のうち最古、4 段フォールバック)
- ✅ **mono モード** で hold note stack による last-note priority 復帰挙動 (UI トグルは Phase 3、内部 API は提供)
- ✅ **ParamDescriptor + コード生成**: `ParamId` (Rust) / `PARAM_IDS` (TS) の二重管理を解消

## Phase 3 への申し送り

- Body Resonator (弦音だけでは「安っぽい音」から抜けないため、IR convolution / modal filter / static IR の選択)
- Extended Karplus–Strong (loss filter / pick position / stretching all-pass)
- MIDI CC マッピング (pitch bend / mod wheel / sustain pedal)
- プリセット保存・ロード (localStorage / IndexedDB)
- WASM SIMD (target-feature=+simd128)
- UI で active voice 数表示、mono/poly トグル
- C8 ピッチ精度: KS-Lagrange の本質的限界。pitch tracker / FFT-based estimator / soft clip / look-ahead limiter で再評価

## ライセンス

未定（開発段階）。
