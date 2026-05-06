# 01. MVP概要とスコープ

## 目的

物理ベース・シンセサイザー（Physical Modeling Synthesizer）の MVP（Minimum Viable Product）を定義する。本仕様書は、ブラウザで動作する Rust + WebAssembly 製の物理モデル弦シンセを「最小構成で確実に音が鳴る」状態まで完成させるためのスコープと前提を確定する。

## 他文書との関係

- 上流: [`pre-research.md`](./pre-research.md)（参考資料、変更しない）
- 下流: [`02-architecture.md`](./02-architecture.md)（全体構成）→ `03〜05`（各レイヤ詳細）→ [`06-build-and-verify.md`](./06-build-and-verify.md) → [`07-implementation-checklist.md`](./07-implementation-checklist.md)
- 本書は「何を作るか」を確定し、以降の文書は「どう作るか」を定義する。

## MVPの完成像

> **ブラウザで動作する Rust/WASM 製の物理モデリング弦シンセ。ノイズ励振を持つ Karplus–Strong 単音シンセが、画面鍵盤・PCキーボード・MIDIキーボードのいずれの入力でも発音し、damping/brightness/output_gain の3パラメータを音色に反映できる。**

「最初の音」を出すことを最優先とし、音質や機能の網羅性は次フェーズに送る。

## ゴール

- Karplus–Strong 単音モデルがブラウザで鳴る
- 3経路の演奏入力で note_on/note_off が動作する
- 3つの音色パラメータがリアルタイムに反映される
- パラメータ変化や音程変化でクリックノイズが発生しない
- AudioWorklet の `process` 中にメモリ確保が起きない（リアルタイム安全）

## 非ゴール（MVPには含めない）

| 項目 | 理由 |
|---|---|
| ポリフォニー（複数同時発音） | スコープを最小化。後続Phase 2で対応 |
| Extended Karplus–Strong（fractional delay, loss filter, pick position 等） | 実装ボリューム増。整数ディレイで音程精度の妥協を許容 |
| Digital Waveguide / Modal Synthesis / Mass-Spring / FDTD | Phase 3〜5の領域 |
| ボディ共鳴（Body Resonator） | Phase 2 で導入 |
| MIDI CC によるパラメータ制御 | UIスライダーのみで十分 |
| プリセット保存・ロード | UI状態管理を最小化 |
| オーディオ録音・WAV書き出し | 範囲外 |
| VST/CLAPプラグイン化、ネイティブビルド | `dsp-core` を独立させて将来道を残すのみ |
| iOS Safari 以外のモバイル動作保証 | デスクトップChromiumを主要ターゲット、iOS Safariは検証のみ |

## 確定事項（ユーザー承認済み）

| 決定事項 | 内容 |
|---|---|
| 機能スコープ | 最小Karplus–Strong（pre-research 10章 Phase 1相当）。モノフォニー、damping/brightness/output_gain のみ |
| フロントエンド | SvelteKit + **Svelte 5**（`npx sv create` で生成、`@sveltejs/adapter-static`、TypeScript有効、runes ベース） |
| 演奏入力 | (a) 画面鍵盤UI、(b) PCキーボード（ASDF行マッピング）、(c) Web MIDI API の3経路すべて |
| Rustクレート構成 | `dsp-core`（純粋Rust、WASM非依存）と `wasm-audio`（C ABI、wasm-bindgen 不使用、設計判断 D8）の2クレート分離 |

## Phase 2 への申し送り（MVP外、拡張時の参考）

MVP では実装しないが、Phase 2 で着手するときに最初に検討すべき設計:

- **`ParamDescriptor { id, name, min, max, default }`**: パラメータをメタデータ付きで記述する構造体。`ParamId` enum と二重管理になりつつある現状を解消し、UI / プリセット / MIDI CC マッピングの全てがこの記述を参照する形に
- **`VoicePool<const N: usize>`**: ポリフォニー化の基盤。`[KarplusStrong; N]` を保持し、空きボイス検索・voice-stealing（最も古いボイスを再利用）を担当
- **`params.json` を単一ソースにしたコード生成**: Rust と TypeScript で `ParamId` / `PARAM_IDS` が drift する問題への抜本的対策

これらは MVP 完成後に別仕様書で詳細化する。

## 主要な設計判断

仕様策定の過程で確定した、実装時に逸脱しない11項目。詳細な根拠と適用箇所は各レイヤ仕様書に記載する。

| # | 判断 | 根拠 |
|---|---|---|
| D1 | **整数ディレイで割り切る** | MVPでは `length = (sample_rate / freq).round()` のみ。fractional delay は Phase 2。ピッチ精度の誤差は許容（A4で0.05%程度、低音域でも0.5%未満） |
| D2 | **MessagePort + Rust側 smoothing** | パラメータ送信は AudioParam を使わず MessagePort 経由。クリック対策は `dsp-core::SmoothedValue` で行う。送信頻度は requestAnimationFrame で60Hz程度に間引く |
| D3 | **WASMロードはメインスレッド経由** | AudioWorkletGlobalScope は `fetch` 非対応/`import` 制限のあるブラウザがある。メインスレッドで fetch → ArrayBuffer を `postMessage` → Worklet 内で `WebAssembly.instantiate` する |
| D4 | **WASM linear memory の grow を起こさない** | `Engine::prepare` で max_buffer を1度だけ確保。`process` 中およびnote_on中の追加確保は禁止。音程切替は `length` フィールドの変更のみで行う |
| D5 | **iOS Safari 対策で StartButton 必須** | `AudioContext.resume()` をユーザージェスチャ内で呼ぶための明示的なボタンをUIに常設する |
| D6 | **denormal 対策で DC injection** | `process_sample` 末尾で `+1e-25 - 1e-25` を施し、Intel系CPUでのdenormal由来CPUスパイクを防ぐ |
| D7 | **note_off は damping 加速で自然減衰** | フラグで即時無音化せず、damping を一時的に強める（例: target=0.95）。次の note_on で `Engine.current_damping`（ユーザー設定値）に復元 |
| D8 | **wasm-audio は C ABI で公開（wasm-bindgen 不使用）** | `#[no_mangle] extern "C"` の薄いABIに統一。`wasm-pack` も使わず、`cargo build` の生WASMを `copy-wasm.mjs` で配置。AudioWorklet 内で生 export を直接呼ぶ方式に最適 |
| D9 | **AudioWorklet view を init 時にキャッシュ** | `Float32Array(memory.buffer, ptr, 128)` を `process()` 内で毎回作らず、init 時に1度だけ作成。`memory.buffer` 変化時のみ再作成しGC圧を排除 |
| D10 | **secure context（HTTPS/localhost）必須** | AudioWorklet と Web MIDI は secure context API。LAN IP の HTTP では動かないため、本番動作確認は HTTPS（ngrok/Cloudflare Tunnel/mkcert）で行う |
| D11 | **Svelte 5 runes ベースで実装** | コンポーネントは `$state`/`$derived`/`$effect`/`$props`/`$bindable` を使用。イベントは `onclick` 等の小文字記法、`|preventDefault` 修飾子は使わない。共有ステートは `.svelte.ts` モジュール（`writable` ストアは MVP では使用しない）。副作用の attachment は Svelte action（`use:action`）でカプセル化し `src/lib/actions/` に置く |

## アーキテクチャ概要（詳細は 02-architecture.md）

```
┌─────────────────────────────────────────┐
│ Svelte UI（メインスレッド）              │
│  StartButton / Keyboard / Slider / MIDI │
└──────────────┬──────────────────────────┘
               │ MessagePort（noteOn/noteOff/setParam/init）
               ▼
┌─────────────────────────────────────────┐
│ AudioWorkletProcessor（音声スレッド）   │
│  WASM をinstantiate して process 委譲    │
└──────────────┬──────────────────────────┘
               │ FFI（共有メモリ + ポインタ）
               ▼
┌─────────────────────────────────────────┐
│ wasm-audio（Rust crate, cdylib）        │
│  SynthHandle が dsp-core を呼ぶ          │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│ dsp-core（Rust crate, rlib, 純粋）       │
│  Engine / KarplusStrong / Smoothing 等   │
└─────────────────────────────────────────┘
```

## 用語集

| 用語 | 説明 |
|---|---|
| **Karplus–Strong** | 短いノイズ列を循環ディレイラインに入れ、低域通過フィルタで減衰させて弦を弾いた音を生成する古典的アルゴリズム（pre-research 3.1節） |
| **AudioWorklet** | Web Audio API において、別スレッド上でカスタム音声処理を行うための仕組み。`AudioWorkletProcessor` を継承して `process()` を実装する |
| **AudioWorkletNode** | メインスレッド側から AudioWorklet を制御するためのAudioNode。MessagePort を介してWorkletと通信 |
| **render quantum** | AudioWorklet が `process()` を呼ぶ単位。現行ブラウザでは事実上 128 frames で運用されるが、仕様上は可変となりうるためコード側で長さガードを入れる |
| **denormal** | 浮動小数点の極小数。連続的に発生するとIntel系CPUで処理が極端に遅延するため、`+1e-25 - 1e-25` などで強制的にゼロ近傍へ吸収する |
| **SmoothedValue** | パラメータの突発変化を1次低域フィルタで滑らかにする構造体。クリックノイズ防止のため必須 |
| **ParamId** | パラメータをu32で識別するenum。MessagePortで送信する際に使用 |
| **note_on / note_off** | MIDIキーオン/キーオフに相当するイベント。MVPではモノフォニーなので最後のnote_onが優先（last-note priority） |
| **last-note priority（MVP簡易版）** | モノフォニーで複数キー同時押し時、最後の note_on のみを `current_note: Option<u8>` で追跡する。前のキーが押下中でも復帰しない（hold note stack なし）。詳細は [03章の last-note 挙動節](./03-dsp-core-spec.md#last-note-挙動mvp仕様) |
| **secure context** | HTTPS または localhost 経由でのみ提供される API 環境。`window.isSecureContext` で判定。AudioWorklet・Web MIDI が要求 |
| **adapter-static** | SvelteKit を完全静的サイトとして書き出すアダプタ。サーバ機能を使わず、ホスティングが容易 |
| **C ABI（`#[no_mangle] extern "C"`）** | Rust から WASM への公開関数を、wasm-bindgen を介さず素のC ABIで露出する形式。export 名が安定し、AudioWorklet との相性が良い |
| **Svelte runes** | Svelte 5 で導入されたリアクティビティ宣言の構文。`$state`（変更可能なリアクティブ値）、`$derived`（派生値）、`$effect`（副作用）、`$props`（コンポーネント引数）、`$bindable`（双方向バインド可能な prop）。`.svelte` または `.svelte.ts` ファイル内でのみ使用可 |
| **Svelte action** | `use:action` 構文で要素に副作用を attach する仕組み。要素のマウント/アンマウントに連動するライフサイクル（`destroy`, `update`）を持ち、window への listener attach のような副作用を綺麗にカプセル化できる |
| **rAF（requestAnimationFrame）** | ブラウザの描画タイミングに同期して実行されるコールバック。約60Hzで呼ばれるため、UIからのパラメータ送信スロットルに利用 |
