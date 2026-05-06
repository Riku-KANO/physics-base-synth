# 06. ビルド・実行・検証

## 目的

開発環境のセットアップ、開発時のコマンドフロー、MVP 完成判定の検証チェックリスト、リスクと対策を定義する。実装着手者が迷わず動作確認を進められる粒度で記述する。

## 他文書との関係

- 上流: [`02-architecture.md`](./02-architecture.md)（プロジェクト構造）、[`05-web-frontend-spec.md`](./05-web-frontend-spec.md)（Worklet ビルド経路）
- 下流: [`07-implementation-checklist.md`](./07-implementation-checklist.md)（実装手順）
- 参考: pre-research 11.2（WASM適性）、13章（アンチパターン）

## 初回セットアップ

### 前提

- OS: Windows 11（PowerShell 7+）。本仕様書のコマンド例は PowerShell 表記。
- 推奨ブラウザ: Chrome 最新版または Edge 最新版（Chromium ベース）
- ハードウェア: Web MIDI を試す場合は USB MIDI キーボード

### Rust セットアップ

```powershell
# Rust toolchain（未インストールなら）
winget install Rustlang.Rustup

# stable に切り替え
rustup default stable

# WASM ターゲット追加
rustup target add wasm32-unknown-unknown

# 確認
rustc --version
cargo --version
```

### WASM ビルドツール

MVPでは **wasm-pack を使わず**、`cargo build --target wasm32-unknown-unknown` で生成した `.wasm` を直接利用する（[04章の設計判断](./04-wasm-audio-spec.md#設計判断-wasm-bindgen-を使わず-c-abi-で公開する) 参照）。コピー用の小さなNodeスクリプト `scripts/copy-wasm.mjs` を使う。

任意で `wasm-opt`（binaryen 同梱）でサイズ最適化できる:

```powershell
# 任意（リリースサイズが気になる場合）
winget install WebAssembly.Binaryen
wasm-opt --version
```

### Node.js / pnpm セットアップ

```powershell
# Node.js LTS（未インストールなら）
winget install OpenJS.NodeJS.LTS

# corepack を有効化して pnpm を使えるようにする
corepack enable
corepack prepare pnpm@latest --activate

node --version    # 20.x 以上
pnpm --version    # 9.x 以上
```

### プロジェクトの依存解決

```powershell
# プロジェクトルートで
pnpm install
```

## 開発時のコマンドフロー

### 通常の開発サイクル

```powershell
# Rust 側を変更したとき
pnpm build:wasm:dev

# UI/Worklet 側だけ変更したとき
pnpm --filter web dev

# まとめて起動（Rust ビルド + dev server）
pnpm dev
```

`pnpm dev` 実行後、ターミナルに `Local: http://localhost:5173/` が表示されるので、ブラウザで開く。

### 動作確認の最初の一歩

1. `http://localhost:5173/` を開く
2. 「▶ Start Audio」ボタンをクリック
3. 「✓ Audio Ready」表示を確認
4. PCキーボードで `A` を押下 → 弦らしい減衰音が聞こえる

ここまでが [F1〜F3](#検証チェックリスト) の最低ライン。

### 本番ビルドの確認

```powershell
pnpm build
# web/build/ に静的ファイルが生成される

# プレビュー（任意のローカルHTTPサーバを使う）
pnpm --filter web preview
```

## ビルドアーティファクトのパス一覧

| 種別 | パス | 生成タイミング |
|---|---|---|
| WASM バイナリ（cargo出力） | `target/wasm32-unknown-unknown/release/wasm_audio.wasm` | `cargo build --release --target wasm32-unknown-unknown` |
| WASM バイナリ（コピー後） | `web/src/lib/wasm/wasm_audio.wasm` | `pnpm build:wasm` |
| Worklet バンドル | `web/static/worklet/synth-processor.js` | `pnpm --filter web build:worklet`（dev/build前に自動実行） |
| 静的サイト | `web/build/` | `pnpm build` |

> JSグルーや型定義は生成されない（C ABI のため）。型補完は [05章](./05-web-frontend-spec.md#audioworkletprocessorsynth-processorts) の `WasmExports` interface で手書き。

### export 名の自動検証スクリプト

`#[no_mangle]` 忘れや関数追加忘れをビルドパイプラインで検知するため、簡易な検証スクリプトを `scripts/check-wasm-exports.mjs` に置く。

```javascript
// 期待する export 名のリストを定義し、wasm モジュールの実 export と突き合わせる
import { readFileSync } from 'node:fs';
const REQUIRED = [
  'memory',
  'synth_new', 'synth_free',
  'synth_note_on', 'synth_note_off',
  'synth_set_param', 'synth_reset',
  'synth_out_l_ptr', 'synth_out_r_ptr', 'synth_capacity',
  'synth_process_block',
];
const bytes = readFileSync('web/src/lib/wasm/wasm_audio.wasm');
const mod = await WebAssembly.compile(bytes);
const exports = WebAssembly.Module.exports(mod).map((e) => e.name);
const missing = REQUIRED.filter((n) => !exports.includes(n));
if (missing.length) {
  console.error('Missing WASM exports:', missing);
  console.error('Available:', exports);
  process.exit(1);
}
console.log('All required WASM exports present.');
```

ルート `package.json` のスクリプトに組み込み:

```json
{
  "scripts": {
    "build:wasm": "cargo build -p wasm-audio --target wasm32-unknown-unknown --release && node scripts/copy-wasm.mjs release && node scripts/check-wasm-exports.mjs",
    "build:wasm:dev": "cargo build -p wasm-audio --target wasm32-unknown-unknown && node scripts/copy-wasm.mjs debug && node scripts/check-wasm-exports.mjs"
  }
}
```

これにより `pnpm build:wasm` 失敗時点で「export が足りない」とすぐわかる。

## 検証チェックリスト（MVP完成判定）

| ID | 判定基準 | 検証手順 | 期待結果 |
|---|---|---|---|
| **F1** | ブラウザで弦らしい音が鳴る | `pnpm dev` → ブラウザで http://localhost:5173 → Start Audio → A キー押下 | 弾けたような減衰音が聞こえる |
| **F2** | 画面鍵盤で発音 | 画面の C4 ボタンをクリック | F1 と同じ音色で発音 |
| **F3** | PCキーボードで発音 | A〜L 行 + W〜O 行 を押す | 1オクターブ + 黒鍵が鳴る。半音階に並んでいる |
| **F4** | Web MIDI で発音 | USB MIDIキーボードを接続 → MidiSelect で選択 → 鍵盤を押す | note_on/off が反映され発音する |
| **F5** | damping 変化で減衰時間が変わる | スライダーで `damping=0.99` → `damping=0.999` | 0.999 で音が長く鳴り続ける |
| **F6** | brightness 変化で音色が変わる | `brightness=0.1` → `brightness=0.9` | 0.9 で高域が多く、0.1 で丸みのある音になる |
| **F7** | クリックノイズなし | 各スライダーを左右に高速にドラッグ | プチノイズが聞こえない |
| **F8** | 連打でメモリ確保が起きない | DevTools > Performance を開いてレコード開始 → A キーを 100 回連打 → レコード停止 | Memory タブのヒープ確保イベントが線形増加しない |
| **F9** | iOS Safari で動作 | iPhone Safari で **HTTPS URL**（ngrok等）にアクセス。`window.isSecureContext === true` を確認 | Start Audio タップで音が出る |

### 検証手順の補足

#### F8（メモリ確保チェック）の詳細手順

JS Heap だけでは WASM linear memory 内の挙動を見られないため、**3つの方法を併用** する:

**(a) WASM linear memory の不変チェック（最重要）**

Worklet 側に開発時のみ有効なチェックコードを仕込む。`synth-processor.ts` 末尾に以下を一時的に追加:

```typescript
// 開発時のみ: memory.buffer.byteLength の不変チェック
let baselineByteLen = 0;
const origProcess = SynthProcessor.prototype.process;
SynthProcessor.prototype.process = function (...args) {
  const r = origProcess.apply(this, args);
  if (this.exports) {
    const cur = this.exports.memory.buffer.byteLength;
    if (baselineByteLen === 0) baselineByteLen = cur;
    else if (cur !== baselineByteLen) {
      console.warn(`[F8] WASM memory grew: ${baselineByteLen} → ${cur}`);
      baselineByteLen = cur;
    }
  }
  return r;
};
```

A キーを 100 回連打しても `memory grew` の警告が一度も出ないこと。

**(b) Rust 側の native allocator テスト**

`crates/dsp-core/tests/no_alloc.rs` で `cargo test` 上の検証:

```rust
// dsp-core/tests/no_alloc.rs（実装方針）
// std::alloc::System をラップしたテスト用 GlobalAlloc を作り、
// prepare 後の note_on→process 経路で alloc 回数が 0 になることを確認
```

> 完全な実装は実装フェーズで行う。MVPでは `cargo expand` でも代替可能（`process_block` 経路に `alloc::vec::Vec::push` 等が現れないことを確認）。

**(c) Chrome DevTools の補助確認**

1. Chrome DevTools を開く（F12）
2. Performance タブ → Record（●）
3. 「Start Audio」をクリックしてから A キーを 1 秒間隔で 100 回押す
4. Stop（■）でプロファイル取得
5. Memory タブで「JS Heap」がほぼ平坦であることを確認（**(a)** が主たる検証、これは補助）

> dsp-core の `KarplusStrong::note_on` は `Vec::resize` を含まない設計（[03章のリアルタイム制約遵守ルール](./03-dsp-core-spec.md)）のため、`memory.buffer.byteLength` が変化したら実装ミス。

#### F9（iOS Safari 検証）の詳細手順

> **重要**: AudioWorklet と Web MIDI は **secure context 必須**（MDN）。`localhost` は信頼される origin として例外扱いされるが、**LAN IP の HTTP（例: `http://192.168.x.x:5173/`）は通常 secure context にならない**ため、HTTPS 経由でアクセスする必要がある。

1. プロジェクトルートで `pnpm build`（本番ビルド）
2. 任意の方法で HTTPS 配信:
   - **ngrok**: `pnpm --filter web preview --host 0.0.0.0` で起動 → 別ターミナルで `ngrok http 4173` → 表示された HTTPS URL を使う
   - **Cloudflare Tunnel**: `pnpm --filter web preview --host 0.0.0.0` → `cloudflared tunnel --url http://localhost:4173`
   - **mkcert**（ローカルCA）: `mkcert -install && mkcert localhost 192.168.x.x` で証明書発行 → `vite preview --https --cert ... --key ...` で HTTPS 配信
3. iPhone Safari で HTTPS URL を開く
4. UIから DevTools（リモートインスペクタ）で `window.isSecureContext === true` を確認
5. Start Audio をタップ → 鍵盤UIをタップで発音 → **F9 達成**

> dev server（`pnpm dev`）でも `pnpm --filter web dev -- --host --https` で HTTPS 起動できるが、SvelteKit dev で証明書設定がやや煩雑なため、F9 は本番ビルド + `preview` で行うのが簡便。

#### 本番ビルドの動作確認

`pnpm build` 後に dev server で動かなくなる/動くだけで本番では落ちる、というケースを防ぐため、**毎リリース前に `preview` での動作確認を行う**:

```powershell
pnpm build
pnpm --filter web preview
# http://localhost:4173 で F1〜F8 を再確認
```

特に WASM のフェッチパスが `?url` インポートで正しく解決されているか、Worklet スクリプトが ``${base}/worklet/synth-processor.js`` （`$app/paths` の base 前置）で配信されているかを確認する。ルート配信なら `base` は空文字、サブパス配信なら `/myapp` 等が前置される。

## リスクと対策表

| # | リスク | 影響 | 対策 |
|---|---|---|---|
| R1 | Web MIDI 非対応ブラウザ（Safari/Firefox） | F4 が一部ブラウザで失敗 | `MidiSelect.svelte` で `'requestMIDIAccess' in navigator` チェック → 未対応時は案内テキスト表示。README に Chrome/Edge 推奨を明記 |
| R2 | iOS Safari の AudioContext suspend | ページロード直後に音が出ない | StartButton をUI最上部に配置し、`AudioContext` の作成と `resume` をユーザージェスチャ内で実行（[D5](./01-overview.md#主要な設計判断)） |
| R3 | WASM `memory.grow` による Float32Array 失効 | クラッシュ・無音化 | 初期化時のみ allocate を許可し、`process` 中の Vec 操作禁止（[D4](./01-overview.md#主要な設計判断)）。`process` 内で `memBuf !== exports.memory.buffer` チェックを追加（フォールバック） |
| R4 | AudioWorkletGlobalScope での `fetch` / `import` 制限 | WASMロード失敗 | メインスレッドで fetch → ArrayBuffer を `postMessage` → Worklet 内で `WebAssembly.instantiate`（[D3](./01-overview.md#主要な設計判断)） |
| R5 | denormal floats による CPU スパイク | 一定時間後に音が途切れる、CPU使用率上昇 | `process_sample` 末尾で `+1e-25 - 1e-25`（[D6](./01-overview.md#主要な設計判断)） |
| R6 | C ABI export 名の取り違え | ビルド成功するが実行時 `undefined is not a function` | `wasm-objdump -x web/src/lib/wasm/wasm_audio.wasm \| findstr Export` で `synth_*` 関数が含まれることを確認（または `node scripts/check-wasm-exports.mjs` を実行）。`#[no_mangle]` の付け忘れに注意 |
| R7 | SvelteKit static adapter で Worklet が 404 | F1 失敗 | `web/static/worklet/synth-processor.js` に配置。adapter の `assets: 'build'` 設定を確認 |
| R8 | Vite が WASM を ESM 解決しようとする | dev server で WASM ロード失敗 | `vite.config.ts` で `optimizeDeps.exclude: ['$lib/wasm']`、`assetsInclude: ['**/*.wasm']` を設定。`?url` 形式での import に統一 |
| R9 | secure context 要件で AudioWorklet/Web MIDI が動かない | F4/F9 失敗 | localhost または HTTPS でのみ動作。LAN IP の HTTP では動かない。`window.isSecureContext` チェックを `SynthEngine.start()` と `MidiSelect.svelte` に入れる |
| R10 | iOS Safari でユーザー操作前に AudioContext 作成 | suspend 状態のまま音が出ない | `SynthEngine.start()` を StartButton の `onclick` 内でのみ呼ぶ。`onMount` や `$effect` 内で呼ばない |
| R11 | esbuild の Worklet バンドルで TypeScript の型エラー | ビルド失敗 | `tsconfig` の `lib` に `WebWorker` を追加、`worker` のグローバル宣言を `synth-processor.ts` 冒頭に置く（`declare const sampleRate: number;` 等） |
| R12 | dev で動いて build/preview で 404 | 本番反映時の事故 | `import wasmUrl from '$lib/wasm/wasm_audio.wasm?url'` で統一し、毎リリース前に `pnpm build && pnpm --filter web preview` を実行 |
| R13 | `process()` 内で `Float32Array` 毎回生成し GC が走る | 音切れ・グリッチ | view を init 時にキャッシュし、`memory.buffer` 変化時のみ再作成（[05章 synth-processor.ts](./05-web-frontend-spec.md#audioworkletprocessorsynth-processorts)） |
| R14 | Worklet 内 WASM 初期化失敗時に `start()` が永久待ち | StartButton が `Starting...` のまま固まる | `SynthEngine.start()` 内で `Promise.race` 的に 5 秒タイムアウト。失敗時は `dispose()` で `node.disconnect()` / `ctx.close()` / `ready = false` し、再試行可能な状態へ復旧 |
| R15 | HMR / 画面遷移で WASM handle / AudioContext / rAF が残る | メモリリーク・複数 AudioContext 競合 | `+page.svelte` の `onDestroy` で `synth.engine.dispose()` を呼ぶ。Worklet には `dispose` メッセージで `synth_free` を実行 |
| R16 | render quantum が 128 以外の値で来る（将来仕様変更） | クラッシュ・無音化 | Worklet の `process()` 冒頭で `outputs[0][0].length !== 128` を検査し、無音返却 + 警告 1 度のみ送信 |

## トラブルシューティング Tips

### 「音が一切出ない」

1. ブラウザの DevTools Console でエラーを確認
2. `[Worklet] error: ...` が出ていれば Worklet 内のWASMロード失敗。R9 を参照
3. エラーがないのに無音 → AudioContext が `suspended` のまま。`audioContext.state` を console で確認し、`suspended` なら StartButton が機能していない
4. `process()` が呼ばれていない可能性 → Worklet 内の console.log を一時的に追加してデバッグ（本番では除去）

### 「ピッチが正しくない」

- MVP は整数ディレイのため、低音域で誤差が大きい（A1=55Hz で1サンプル誤差約2.3%）。これは **既知の妥協**（[D1](./01-overview.md#主要な設計判断)）。Phase 2 でfractional delay 対応。

### 「`pnpm build:wasm` が失敗する」

- export 名のエラーなら R6 の手順（`wasm-objdump -x ... | findstr Export`）
- `target` 不在エラーなら `rustup target add wasm32-unknown-unknown`
- ファイルロック（Windows特有）なら `target/` を削除してリトライ
- `scripts/copy-wasm.mjs` の path 解決失敗ならカレントディレクトリを確認（`pnpm` 経由なら常にプロジェクトルート）

### 「DevTools Performance でメモリが増え続ける」

- Worklet 内の `port.onmessage` でクロージャがリークしている可能性
- `Float32Array` を毎フレーム新規作成しているため、これは GC 対象でメモリ増ではない。Major GC が走ればフラットに戻るはず
- それでも増えるなら、メインスレッド側 `SynthEngine.setParam` の `pendingParams` Map がクリアされていない可能性をチェック

## 性能目標（MVP）

| 指標 | 目標値 | 備考 |
|---|---|---|
| AudioWorklet `process` あたりの CPU 時間 | < 0.5ms（128 frames @ 48kHz、2.67ms予算） | 1ボイスのみのため余裕あり |
| 起動から最初の音まで | < 2秒 | WASM初期化と AudioContext.resume 含む |
| WASM バイナリサイズ | < 150KB（gzip前） | `cargo build --release` 出力。任意で `wasm-opt -O3` でさらに圧縮可 |
| ヒープ確保回数（process実行中） | 0回 | F8 で検証 |

## デプロイ（参考、MVPの必須ではない）

`pnpm build` で `web/build/` に生成された静的ファイル群を任意の静的ホスティング（GitHub Pages、Cloudflare Pages、Vercel 等）に置けば動作する。

- HTTPS 配信が望ましい（Web MIDI が一部ブラウザで HTTPS 必須）
- `.wasm` の MIME type が `application/wasm` で配信されることを確認
- `Cross-Origin-Opener-Policy: same-origin` と `Cross-Origin-Embedder-Policy: require-corp` は **MVPでは不要**（SharedArrayBuffer を使わないため）。将来 Worker + SharedArrayBuffer を導入する際に必要になる
