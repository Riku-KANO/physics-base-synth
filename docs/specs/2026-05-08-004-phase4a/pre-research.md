# Phase 4a 調査資料

## F38b 実機計測 / LFO + Mod Wheel / プリセット保存・ロード / 多楽器プリセット 6 種 / 既存負債整理 の前提整理

本書は Phase 4a 仕様策定で参照する追加調査トピックを集約する。Phase 1 / Phase 2 / Phase 3 で既に決着した基礎理論（Karplus–Strong、Lagrange / Thiran 補間、Modal Body、Loss filter、Pick position 励振 shaping、ParamDescriptor、SmoothedValue、SustainState、SoftClip、VoiceState 通信）は重複させず、各 pre-research の該当節を参照する。Phase 4a は **方式選定の重みは中程度**（LFO 配置と Preset スキーマ）で、**プリセット保存というブラウザ依存の独立スコープ**が新たに加わるのが特徴。各章末に **結論ボックス（◎採用 / ○検討 / △Phase 4b 以降送り / ×不採用）** を置く（Phase 3 と同形式）。

---

## 0. Phase 1 / Phase 2 / Phase 3 pre-research との関係

Phase 4a は以下の節を **Phase 1 / 2 / 3 の pre-research を一次資料**として参照する。

| Phase 節 | 内容 | Phase 4a での参照箇所 |
|---|---|---|
| Phase 1 [§3.1 Karplus–Strong](../2026-05-06-001-mvp/pre-research.md) | 基本原理 | §4 LFO の Pitch destination 接続点 |
| Phase 1 [§3.3 Modal Synthesis](../2026-05-06-001-mvp/pre-research.md) | 並列共鳴モード | §7 多楽器 6 種の Modal 係数探索の理論基盤 |
| Phase 2 [§6 性能予算](../2026-05-07-002-phase2/pre-research.md) | gzip < 30 KB、CPU 1.5 ms 目標 | §9 Phase 4a 性能予算で再計算 |
| Phase 3 [§2.3 Modal Body 係数](../2026-05-07-003-phase3/pre-research.md) | ギターボディ 8 モード | §7 多楽器 6 種の係数比較の起点 |
| Phase 3 [§5 Brightness 群遅延補正](../2026-05-07-003-phase3/pre-research.md) | LPF 群遅延補正 | §4 LFO Brightness destination で再評価 |
| Phase 3 [§6 MIDI CC マッピング](../2026-05-07-003-phase3/pre-research.md) | CC dispatch 経路 | §5 Mod Wheel (CC#1) を Phase 4a で実装 |
| Phase 3 [§9 性能予算](../2026-05-07-003-phase3/pre-research.md) | gzip 12.9 KB / CPU 1.95 ms 想定 | §9 で +LFO/Preset/多楽器の予算追加 |
| Phase 3 retrospective [§5 既存負債](../../retrospective/2026-05-07-003-phase3.md) | F38b 未計測 / Voice State alloc / `excitation_snapshot` / WASM gzip 27.78 KB / Mono+Sustain | §8 で個別対応 |
| Phase 3 retrospective [§7.2 Phase 4 候補](../../retrospective/2026-05-07-003-phase3.md) | 含める / 検討 / Phase 4b 送りリスト | §1 でリプリント、本書全体で深掘り |

---

## 1. Phase 4a スコープと前提制約

### スコープ確定（ユーザー承認、2026-05-08）

| 候補（retrospective §7.2） | 重要度順位 | Phase 4a 採否 |
|---|---|---|
| F38b 実機計測 | 1 | **◎ §0 として最初に組み込み** |
| C8 ピッチ自己発振モード | 2 | △ Phase 4b 以降送り |
| Mod Wheel (CC#1) + LFO | 3 | **◎ Phase 4a 採用** |
| プリセット保存・ロード | 4 | **◎ Phase 4a 採用 (localStorage)** |
| 多楽器プリセット | 5 | **◎ Phase 4a 採用 (6 種)** |
| ピアノ音色 | 6 | △ **Phase 4b で別計画** |
| Pick position fractional 化 | 7 | △ Phase 4b 以降送り |
| Look-ahead limiter | 8 | △ Phase 4b 以降送り |
| WASM SIMD | 9 | △ Phase 4b 以降送り |
| Brightness allpass 直列補正 | 10 | △ Phase 4b 以降送り |

### 設計改善 (retrospective §7.3) の Phase 4a 採否

| 改善候補 | Phase 4a 採否 | 理由 |
|---|---|---|
| Voice State `SharedArrayBuffer + Atomics` 化 | × 不採用 | COOP/COEP ヘッダ必要、GitHub Pages 不可 |
| `wasm-opt -O3` 適用 | **◎ Phase 4a §8** | サイズ削減、リスク低、計測型タスク |
| `excitation_snapshot` を `#[cfg(test)]` でガード | **◎ Phase 4a §8** | 小修正、production cleanliness 向上 |
| Modal Body M=8 → M=5 削減オプション | × 不採用 | 多楽器プリセット側で表現力を担保するため不要 |

### 制約（Phase 1 / 2 / 3 から継承、Phase 4a でも維持）

- **WASM gzip < 30 KB**（Phase 3 実測 27.78 KB → `wasm-opt -O3` 適用で目標 ~13 KB / target の 43%）
- **依存ゼロ**: `dsp-core` / `wasm-audio` で外部 crate を追加しない（LFO テーブルや Preset シリアライザも自前）
- **`Engine::prepare` 以外でヒープ確保禁止**（LFO 状態、楽器プリセット切替時の係数差し替えも `prepare` で確保した固定領域内）
- **C ABI のみ**: `wasm-bindgen` 不使用、`#[unsafe(no_mangle)] extern "C"` を継続
- **Float32Array view キャッシュ**: Worklet 側で `process()` 内に `new Float32Array(...)` を作らない原則を維持
- **Svelte 5 runes**: `$state` / `$derived` / `$effect`、共有ステートは `.svelte.ts` 拡張子

### 本書の確定責任

Phase 4a 着手前に以下 4 件を本書で確定させる:

1. §3 で **F38b 実機計測の手順と判定基準**を明文化
2. §4 で **LFO の配置（グローバル vs Voice 単位）と destination 構成**を確定
3. §6 で **プリセット JSON スキーマと version 管理戦略**を確定
4. §7 で **多楽器 6 種の Modal 係数初期値（参考文献ベース）**を提示

§10 の「実装着手前に答えを出すべき問い」7 件は仕様書策定時に順次決める。

---

## 2. F38b 実機計測手法（Phase 3 検証の最終案件）

Phase 3 retrospective §5 で「Chrome DevTools Performance タブの Worklet `process` self time avg/max を計測していない」ことが負債として明記。Phase 4a は **§0 = F38b 計測** を実装着手の前提とする。

### 2.1 計測手順

1. `pnpm build` で本番ビルド + `pnpm preview` で 4173 ポート起動
2. Chrome (最新版) で `http://localhost:4173/physics-base-synth/` を開く
3. F12 → Performance タブ → ⚙ 歯車 → CPU: "No throttling" / Network: 任意
4. ⏺ Record 開始
5. ブラウザ上で **8 voice 同時押下**（PC キーボード a-k で 8 鍵） + Pitch Bend wheel 操作 + CC#7 操作 + Sustain Pedal 操作の **最悪ケース** を 10 秒間維持
6. ⏹ Record 停止
7. タイムライン下部の "Audio Worklet" レーンを展開、各 task の self time を確認
8. **平均** と **最大** を集計（recordable な task が ~2300 個 = 10 秒 × 230 quanta/sec）

### 2.2 判定基準（pre-research §9 Phase 3 性能予算より）

| 指標 | target | 撤退ライン |
|---|---|---|
| Worklet `process` self time avg | < 1.5 ms | > 2.0 ms で R30（Voice Meter stride 4096 化） |
| Worklet `process` self time max | < 2.5 ms | > 3.0 ms で R30 + alloc 検査 |
| dropouts (audio glitch、目視 / 耳) | 0 | > 0 で SAB 化検討 |

### 2.3 計測再現性のための注意点

- **他タブを閉じる**（特に YouTube / Slack 等の常駐 audio）
- **DevTools Console の WebMIDI ログ抑制**（`console.log` は重い、Phase 3 実装で問題なし）
- **Battery saver / Background tab throttling** を無効化（Chrome の "Energy Saver" 設定）
- **コンセント給電** で計測（バッテリ駆動だと CPU governor が下がる）
- 計測値を `docs/retrospective/2026-05-07-003-phase3.md` §5 に追記して負債を閉じる

### 2.4 超過時の R30 対策（Phase 3 06 章 R30 から継承）

1. **Voice State stride を 4096 に変更**: `synth-processor.ts` の `VOICE_STATE_STRIDE_FRAMES = 1024` → `4096` で push 頻度を 1/4。UI 更新は ~85 ms 周期に劣化するが体感差小
2. **Voice Meter UI 削除**: `VoiceMeter.svelte` を非表示、`maybePushVoiceState` を skip
3. **`SharedArrayBuffer + Atomics` 化**: COOP/COEP 必要で GitHub Pages 不可、ローカル開発のみ

> **§2 結論ボックス: ◎ Phase 4a §0 として最初に実施**: 上記手順で計測 → retrospective §5 に結果追記 → target 達成なら Phase 4a 本実装へ。target 超過なら R30 対策を Phase 4a §1 として組み込み。

---

## 3. LFO 設計（**Phase 4a の音響面最大決断**）

Phase 3 で送りになった Mod Wheel (CC#1) を LFO 仕様確定とともに実装する。Phase 3 D39 の前置きを引き継ぎ、本節で確定する。

### 3.1 LFO の物理的位置付け

LFO (Low Frequency Oscillator) は **0.1 Hz 〜 数十 Hz の低周波振動子**で、変調信号として他のパラメータに乗算 / 加算される。シンセ表現の基本要素:

- **Vibrato**: pitch を周期変動（典型 4-7 Hz、深さ ±0.05〜0.5 半音）
- **Tremolo**: amplitude を周期変動（典型 4-8 Hz、深さ ±0.2〜0.6）
- **Wah / Filter sweep**: brightness（filter cutoff）を周期変動（典型 0.5-3 Hz、深さ大）

Mod Wheel (CC#1) は **LFO 強度を 0〜1 で操作するハードウェア**として標準化されている（DX7 以降の合成器の伝統）。

### 3.2 LFO 配置: グローバル vs Voice 単位

| 観点 | A: グローバル 1 個 (Engine 内) | B: Voice 単位 (Voice 内、N=8 並列) |
|---|---|---|
| CPU コスト/sample | +1〜3 演算（1 LFO） | +24 演算（8 voice × 3 演算） |
| 状態 | f32 phase 1 個 + f32 value 1 個 | f32 phase × 8 + f32 value × 8 |
| 表現力 | ◎ 全 voice に同期した vibrato / tremolo | △ voice 別 phase で「うねり」が出るがピアノ vibrato としては不自然 |
| 実装複雑度 | ◎ Engine 1 段 | × Voice trait に LFO 状態追加、reset 経路増 |
| Phase 3 既存設計との整合 | ◎ ChannelVolume / Pitch Bend と同じく Engine 状態 | × Voice trait 拡張で `set_pitch_bend` と並列の API 増 |
| 演奏者の慣習 | ◎ MIDI シンセは LFO 1 個が標準 | × ハイエンド機の "voice-individual LFO" は少数派 |
| 音楽的意図 | ◎ vibrato / tremolo / wah はすべて global | △ |
| 採用評価 | **◎ Phase 4a 採用** | × |

### 3.3 LFO 波形

| 波形 | 用途 | 計算手法 (per sample) | 実装複雑度 |
|---|---|---|---|
| **Sine** | Vibrato / tremolo の標準 | `sin(2π·phase)` または LUT 1024 entry | ○ LUT が無難 |
| **Triangle** | Filter sweep / 機械的変調 | `4·\|phase − 0.5\| − 1` (linear ramp) | ◎ 加減算のみ |
| Square | 8-bit 機械音、ON/OFF 変調 | `phase < 0.5 ? 1.0 : -1.0` | ◎ |
| Sawtooth | 上昇 / 下降 ramp | `2·phase − 1` | ◎ |
| Random / S&H | 不規則変調 | XorShift32 で N sample ごと更新 | ○ |

**Phase 4a 採用**: **Sine + Triangle の 2 種類**。

理由:
- Vibrato は sine が音響的に自然（triangle vibrato はやや機械的）
- Filter sweep は triangle が hard edge で音色変化の予測しやすさあり
- Square / Sawtooth は LFO destination が pitch だと不快、tremolo destination だと音量段差が目立つ → スコープ縮小
- S&H は XorShift32 既存使用で実装可能だが、楽器シンセの標準機能としては必須でない → Phase 4b 以降

**Sine の実装**: 1024 エントリの LUT を `lazy_static` 不使用 (`const fn` も f32 sin がない) のため `Engine::prepare` 時に 1 度生成、または `phase * 2π` を直接 `f32::sin()` 計算（CPU は LFO レートが低いので `sin` 1 回/sample でも +5 演算、許容）。**`f32::sin()` を採用**（実装シンプル、CPU 影響 +5 演算/sample × 1 LFO = 軽微）。

**Triangle の実装**:
```rust
fn triangle(phase: f32) -> f32 {  // phase ∈ [0, 1)
    let centered = phase - 0.5;
    let abs = centered.abs();
    4.0 * abs - 1.0  // 範囲 [-1, 1]
}
```

### 3.4 LFO レンジとレート

| 項目 | 範囲 | デフォルト | 理由 |
|---|---|---|---|
| Rate (Hz) | 0.1 〜 8.0 | 5.0 | Vibrato 標準、tremolo の上限も 8 Hz で実用十分 |
| Smoothing | SmoothedValue tau=0.05s | — | レート急変時のクリック対策 |

**0.1 Hz 〜 8 Hz** に絞る理由: 8 Hz 超は audio rate に近づき折り返し / discontinuity リスク、また vibrato として不自然。0.1 Hz 未満は実用上意味なし（10 秒周期の超低速変調は楽曲表現で稀）。

### 3.5 LFO destination（変調先）

| Destination | 物理パラメータ | 深さの意味 | 採否 |
|---|---|---|---|
| **Pitch** | `set_pitch_bend(semitones)` の追加項 | 0.0 = 0 半音、1.0 = ±0.5 半音（vibrato 標準） | **◎ Phase 4a 採用** |
| **Brightness** | `set_brightness` の SmoothedValue target に加算 | 0.0 = 変動なし、1.0 = ±0.5（filter sweep） | **◎ Phase 4a 採用** |
| **Volume** | `output_gain` の SmoothedValue target に乗算 | 0.0 = 変動なし、1.0 = ±0.5（tremolo） | **◎ Phase 4a 採用** |
| Pick position | `pick_position` 値 | 0.0 = 変動なし、1.0 = ±0.2 | △ Phase 4b（process 内 β 変更は次回 note_on で反映、効果薄） |
| Damping | `set_damping` | 0.0 = 変動なし、1.0 = ±0.001 | × 効果が知覚閾以下 |
| Body Wet | `body_wet` SmoothedValue | 0.0 = 変動なし、1.0 = ±0.3 | △ Phase 4b（音響的に意味薄） |

**3 destinations 採用**: Pitch / Brightness / Volume。3 つ独立の depth スライダーを UI に出す。

### 3.6 Mod Wheel (CC#1) との結合

Mod Wheel は **LFO depth の master**として機能。LFO destination depth × Mod Wheel value で実効深さ:

```
effective_pitch_depth   = lfo_pitch_depth   × mod_wheel
effective_brightness_d  = lfo_brightness_d  × mod_wheel
effective_volume_depth  = lfo_volume_depth  × mod_wheel

lfo_value = sine(lfo_phase) または triangle(lfo_phase)  // ∈ [-1, 1]

pitch_offset_semitones = lfo_value × effective_pitch_depth × 0.5    // ±0.5 半音 max
brightness_offset      = lfo_value × effective_brightness_d × 0.5   // ±0.5
volume_multiplier      = 1.0 + lfo_value × effective_volume_depth × 0.5  // 0.5〜1.5 max
```

**動作モード**:
- Mod Wheel = 0 (標準位置): LFO 効果ゼロ、現状の Phase 3 挙動と同一
- Mod Wheel > 0: LFO 効果が depth スライダー × Mod Wheel で発揮

これにより演奏中に Mod Wheel を上げ下げするだけで vibrato / tremolo の量を制御できる（DX7 / Yamaha synth の伝統的挙動）。

### 3.7 process_sample 内での適用順序

```text
LFO process (Engine 内 1 段)
  ↓
phase += lfo_rate / sample_rate
phase -= floor(phase)  // [0, 1) 維持
lfo_value = sine(phase) or triangle(phase)  // [-1, 1]

VoicePool.process_sample()
  ├─ Voice ごとに pitch bend を SmoothedValue で読む際、LFO pitch を加算
  ├─ Voice ごとに brightness の SmoothedValue 適用前に LFO brightness を加算
  └─ output mix で 1/sqrt(N) 後

ModalBody.process_sample (Engine、変更なし)
  ↓
Engine の output_gain × channel_volume × volume_multiplier (LFO volume)
  ↓
soft_clip
```

**重要**: pitch / brightness は **per-voice の SmoothedValue が既に存在**するため、LFO 値は既存の SmoothedValue target に「offset として加算」して fan-out する設計が最小コスト。volume は Engine 単一の `output_gain` の積算項として処理（per-voice 不要）。

### 3.8 CPU コスト見積

```
LFO process: phase 加算 (1) + sin / triangle (5 / 3) + smoothing (3) = ~10 演算/sample
Voice 拡張 (pitch / brightness offset 加算): per voice +2 演算 × 8 = 16 演算/sample
Engine volume offset: +2 演算/sample

合計: ~28 演算/sample × 128 frames = 3584 演算/process
```

WASM 1 GHz 仮定で +3584 / 1e9 = +0.0036 ms/process。Phase 3 想定 1.95 ms に対し +0.18% で予算余裕大。

> **§3 結論ボックス: ◎ Phase 4a 採用**
>
> - **LFO 配置**: グローバル 1 個 (Engine 内)、Voice 単位は不採用
> - **波形**: Sine + Triangle 2 種類、`f32::sin()` 直接呼出
> - **レート範囲**: 0.1 〜 8.0 Hz、デフォルト 5.0 Hz、SmoothedValue tau=0.05s
> - **Destinations**: Pitch / Brightness / Volume の 3 つ、各独立 depth スライダー
> - **Mod Wheel (CC#1)**: 全 destination depth の master 乗数 ∈ [0, 1]
> - **CPU**: +0.0036 ms/process（予算余裕大）

---

## 4. Mod Wheel (CC#1) MIDI 標準仕様

### 4.1 MIDI 仕様

- **Status byte**: 0xB0–0xBF (Control Change、channel 0–15)
- **Data 1**: 0x01 (CC#1 = Modulation)
- **Data 2**: 0x00–0x7F (7-bit value)

WebMIDI API 経由で `MIDIMessageEvent.data` の `[status, cc, value]` で取得。

### 4.2 Phase 3 既存実装との統合

Phase 3 D38 で `synth_midi_cc(handle, cc, value_normalized)` を実装済み。`engine.rs:171` で CC#1 は **no-op** とされている:

```rust
match cc {
    CC_MOD_WHEEL => {
        // Phase 4 送り: LFO 仕様確定後に対応 (D39)。現状 no-op。
    }
    ...
}
```

Phase 4a で `CC_MOD_WHEEL` 分岐を実装:

```rust
CC_MOD_WHEEL => {
    self.mod_wheel.set_target(v);  // 新規 SmoothedValue field
}
```

### 4.3 UI 露出

| 経路 | 入力源 | UI 表示 |
|---|---|---|
| **WebMIDI 物理鍵盤の Mod Wheel** | `midi-cc.ts` 経由で `engine.sendMidiCc(1, value)` | スライダーは Phase 4a UI に出さない（演奏者が物理 wheel で操作する想定） |
| **PC キーボード演奏時のスライダー** | UI スライダー | `<input type="range" min="0" max="127">` で値を CC#1 として送信 |

**Phase 4a 採用**: UI スライダーを Mod Wheel として正式露出。WebMIDI 物理 wheel と UI スライダーは **同じ MIDI CC 経路** で送り、Engine 側で値が上書きされる（最後に来た値が有効、Phase 3 D38 と同じ挙動）。

### 4.4 SmoothedValue 時定数

LFO depth は急変するとクリックが出るため、Mod Wheel value 自体を SmoothedValue で吸収:

```
mod_wheel: SmoothedValue (tau = 0.05s)
```

CC#7 Channel Volume と同じ tau (0.02s) より長めなのは、Mod Wheel は演奏中に頻繁に動かす（vibrato 強度の表現）ため、tau を長くしても遅延感が問題にならない。

> **§4 結論ボックス: ◎ Phase 4a 採用**: `Engine::handle_midi_cc` の CC#1 分岐を有効化、`mod_wheel: SmoothedValue (tau=0.05s)` を追加。UI スライダー + WebMIDI 物理 wheel の双方経路をサポート（既存 D38 dispatch 経由）。

---

## 5. プリセット保存・ロード

### 5.1 プリセットの定義

**プリセット = シンセの全ユーザー操作可能パラメータ + 楽器選択を JSON で永続化**。Phase 4a の対象:

| カテゴリ | 含まれる項目 |
|---|---|
| 数値パラメータ | Damping / Brightness / OutputGain / PickPosition / BodyWet（5 件、`params.json` の `params` 配列） |
| LFO パラメータ | Rate / Waveform (sine/triangle) / Pitch Depth / Brightness Depth / Volume Depth |
| Modal Body 係数 | 楽器選択 (Guitar Classical / Ukulele / Mandolin / Bass / Steel Guitar / Sitar)、係数自体は楽器 enum で識別、JSON に展開しない |
| メタ | プリセット名、作成日時、format version |

**含めない項目**:
- Mode (Mono/Poly): UI 状態であり「音色」ではない
- Stereo Spread: 楽器選択に紐づき、ユーザー直接操作しない
- Mod Wheel value: ランタイム入力で永続化対象でない

### 5.2 プリセット種別

| 種別 | 出典 | 編集可否 | UI |
|---|---|---|---|
| **Factory Preset** | 多楽器 6 種 + Default 1 種 = 計 7 種 | 読み取り専用 | ドロップダウン上段 |
| **User Preset** | ユーザー保存 | 編集 / 削除可 | ドロップダウン下段、最大 32 件 |

Factory Preset は WASM バイナリ内でなく **TS 側の `factory-presets.ts` const テーブル**で持つ（再ビルドなしで追加可能、サイズ影響軽微）。

### 5.3 JSON スキーマ案

```typescript
// web/src/lib/state/preset-schema.ts
export interface PresetV1 {
  version: 1;
  name: string;
  createdAt: string;  // ISO 8601, "2026-05-08T12:34:56Z"
  instrument: InstrumentKind;
  params: {
    damping: number;
    brightness: number;
    outputGain: number;
    pickPosition: number;
    bodyWet: number;
  };
  lfo: {
    rate: number;
    waveform: 'sine' | 'triangle';
    pitchDepth: number;
    brightnessDepth: number;
    volumeDepth: number;
  };
}

export type InstrumentKind =
  | 'guitar_classical'
  | 'ukulele'
  | 'mandolin'
  | 'bass'
  | 'guitar_steel'
  | 'sitar';
```

### 5.4 サイズ概算

```json
{
  "version": 1,
  "name": "My Custom Sound 1",
  "createdAt": "2026-05-08T12:34:56Z",
  "instrument": "guitar_classical",
  "params": { "damping": 0.996, "brightness": 0.5, "outputGain": 0.8, "pickPosition": 0.125, "bodyWet": 0.5 },
  "lfo": { "rate": 5.0, "waveform": "sine", "pitchDepth": 0.0, "brightnessDepth": 0.0, "volumeDepth": 0.0 }
}
```

**1 プリセット ≈ 350 byte**。User preset 32 件 × 350 byte = 11.2 KB。localStorage 5 MB 上限に対し 0.22%、十分余裕。

### 5.5 version 管理と migration 戦略

```typescript
function loadPreset(json: unknown): PresetV1 {
  if (!isObject(json) || typeof json.version !== 'number') {
    return getDefaultPreset();  // パース失敗、デフォルトに戻す
  }
  if (json.version === 1) {
    return parseV1(json);
  }
  // 将来の v2 以降のハンドリング
  console.warn(`Unknown preset version ${json.version}, using default`);
  return getDefaultPreset();
}
```

**migration ルール**:
- v1 → v2 への破壊的変更時は `migrateV1ToV2` 関数を追加
- 不明 version は console.warn してデフォルトを返す（throw しない、UX 配慮）
- Factory Preset は版管理に従わず、コード内で常に最新スキーマで定義

> **§5 結論ボックス: ◎ Phase 4a 採用**: `version: 1` で開始、JSON スキーマは §5.3 を確定。Factory Preset 7 種（Default + 楽器 6 種）+ User Preset 最大 32 件を localStorage で管理。1 プリセット ~350 byte で容量問題なし。

---

## 6. localStorage シリアライゼーション

### 6.1 ストレージ選択肢の比較（再確認）

ユーザー承認 (2026-05-08) で **localStorage** を選択。比較のため候補を再掲:

| 観点 | localStorage | IndexedDB | OPFS |
|---|---|---|---|
| API | 同期 | 非同期 (Promise) | 非同期 (Promise) |
| 容量 | ~5 MB (Chrome) | 無制限 (origin quota) | 無制限 (origin quota) |
| GitHub Pages 動作 | ◎ | ◎ | △ Safari 安定性要検証 |
| Phase 4a スコープに対する適切性 | ◎ プリセット 32 件 × ~350 byte で十分 | △ オーバーヘッド大 | △ 未来寄り |

**localStorage を採用した理由**: 同期 API でコード簡潔、容量も十分、GitHub Pages で問題なく動作、ブラウザ対応も完璧。

### 6.2 キー設計

```
physbase.preset.v1.list          → User Preset 名のリスト (JSON 配列)
physbase.preset.v1.<name>        → 個別 Preset (JSON)
physbase.preset.v1.last          → 最後に選択した Preset 名
```

`v1` をキーに含めることで、将来 v2 移行時に v1 データを残しつつ並列管理可能。

### 6.3 操作 API

```typescript
// web/src/lib/state/preset-store.svelte.ts
export class PresetStore {
  private factoryPresets = FACTORY_PRESETS;  // const、編集不可
  userPresets = $state<PresetV1[]>([]);
  currentPreset = $state<PresetV1>(getDefaultPreset());

  load(): void {
    // localStorage から読み込み、validation 失敗は skip
  }

  save(name: string): void {
    if (this.userPresets.length >= 32 && !this.userPresets.some(p => p.name === name)) {
      throw new Error('Preset slot full (max 32)');
    }
    // JSON.stringify + localStorage.setItem
  }

  delete(name: string): void { ... }
  apply(preset: PresetV1, engine: SynthEngine): void { ... }
}
```

### 6.4 容量超過時の挙動

- 32 件目を超える保存試行は `throw new Error('Preset slot full')` で UI に通知
- localStorage QuotaExceededError は防御的に `try/catch`、エラー時は console.error + UI トースト

### 6.5 Cross-tab 同期

`window.addEventListener('storage', ...)` で他タブからの変更を検知できるが、Phase 4a では **不要**（演奏アプリで複数タブ同時使用は稀）。Phase 4b 以降で需要が出れば追加検討。

> **§6 結論ボックス: ◎ Phase 4a 採用**: localStorage 同期 API、キー prefix `physbase.preset.v1.`、最大 32 User Preset、Factory Preset 7 種は const テーブル。容量超過は throw + UI トースト。Cross-tab 同期は Phase 4b 以降。

---

## 7. 多楽器プリセット 6 種の Modal 係数調査

### 7.1 楽器選定とその根拠

ユーザー指定（2026-05-08）:

1. **Guitar Classical** (クラシックギター) — Phase 3 既存値、Penttinen et al. 2006
2. **Ukulele** (ウクレレ) — 小型で短弦長、高 Q の高次モード
3. **Mandolin** (マンドリン) — 二重弦の鋭い高域、複合モード
4. **Bass** (ベース) — 大型ボディの低域強調、Helmholtz 50-80 Hz
5. **Guitar Steel** (スチールギター) — 金属弦のブライトネス、共鳴は木製ギター類似
6. **Sitar** (シタール) — Sympathetic strings の代替として高 Q 高密度モード

### 7.2 各楽器の Modal 係数初期値（参考文献ベース）

実機計測の代替として、文献値 / 既存音源プラグインのプリセットから類推。**各係数は実装後の聴感調整で修正可能**。

#### Guitar Classical (Phase 3 既存値、再掲)

| Mode | f (Hz) | Q | gain |
|---|---|---|---|
| Helmholtz | 105 | 30 | 1.0 |
| Top plate | 200 | 25 | 0.8 |
| Top + back | 280 | 20 | 0.5 |
| Higher top | 420 | 35 | 0.4 |
| Mid resonance | 580 | 40 | 0.35 |
| Upper mid | 850 | 45 | 0.25 |
| High brilliance | 1400 | 50 | 0.2 |
| Air mode high | 2300 | 60 | 0.15 |

#### Ukulele

特徴: 小型ボディ → Helmholtz が高め (180-220 Hz)、ボディ自体は質量小で減衰早い → Q やや低、高次モードは存在感小。

| Mode | f (Hz) | Q | gain |
|---|---|---|---|
| Helmholtz | 200 | 18 | 0.9 |
| Top plate | 380 | 20 | 0.7 |
| Top mid | 540 | 22 | 0.45 |
| Higher mode 1 | 780 | 28 | 0.35 |
| Higher mode 2 | 1100 | 32 | 0.3 |
| Mid brilliance | 1600 | 38 | 0.22 |
| High mode | 2200 | 42 | 0.18 |
| Air mode | 3100 | 50 | 0.12 |

#### Mandolin

特徴: 二重弦の鋭い attack、高 Q の高次モード（ブライトな倍音）。Helmholtz は中程度。

| Mode | f (Hz) | Q | gain |
|---|---|---|---|
| Helmholtz | 145 | 25 | 0.85 |
| Top plate | 260 | 28 | 0.7 |
| Top mid | 410 | 32 | 0.5 |
| Higher mode 1 | 620 | 40 | 0.4 |
| Higher mode 2 | 920 | 48 | 0.35 |
| High brilliance 1 | 1450 | 60 | 0.3 |
| High brilliance 2 | 2100 | 70 | 0.25 |
| High air | 2900 | 75 | 0.2 |

#### Bass (Acoustic)

特徴: 大型ボディ → Helmholtz が低い (50-70 Hz)、低域強調、高次モードは控えめ。

| Mode | f (Hz) | Q | gain |
|---|---|---|---|
| Helmholtz | 60 | 25 | 1.2 |
| Top plate | 120 | 22 | 0.9 |
| Top mid | 195 | 25 | 0.6 |
| Higher mode 1 | 290 | 30 | 0.4 |
| Higher mode 2 | 420 | 35 | 0.3 |
| Mid brilliance | 650 | 40 | 0.22 |
| High mode | 980 | 45 | 0.16 |
| High air | 1500 | 50 | 0.1 |

#### Guitar Steel

特徴: スチール弦のブライトネス（DSP 的には弦側で表現）、ボディ共鳴はクラシックギター類似だが高次モードがやや強い。

| Mode | f (Hz) | Q | gain |
|---|---|---|---|
| Helmholtz | 100 | 32 | 1.0 |
| Top plate | 215 | 28 | 0.85 |
| Top + back | 300 | 22 | 0.55 |
| Higher top | 440 | 38 | 0.45 |
| Mid resonance | 620 | 42 | 0.4 |
| Upper mid | 920 | 48 | 0.32 |
| High brilliance | 1500 | 55 | 0.28 |
| Air mode high | 2500 | 65 | 0.22 |

#### Sitar (Sympathetic Resonance Approximation)

特徴: 多数の sympathetic 弦による高密度の高 Q モード、ドローン的な持続。M=8 では sympathetic 弦の代替として高 Q を多用。

| Mode | f (Hz) | Q | gain |
|---|---|---|---|
| Body Helmholtz | 130 | 30 | 0.7 |
| Lower body | 240 | 35 | 0.6 |
| Sympathetic 1 | 380 | 60 | 0.5 |
| Sympathetic 2 | 560 | 70 | 0.45 |
| Sympathetic 3 | 820 | 80 | 0.4 |
| Sympathetic 4 | 1200 | 90 | 0.35 |
| Sympathetic 5 | 1750 | 100 | 0.3 |
| Sympathetic 6 | 2500 | 110 | 0.25 |

### 7.3 Stereo Spread の楽器別考慮

Phase 3 では `stereo_spread: 0.05` 固定。Phase 4a で楽器ごとに調整可能性:

| 楽器 | stereo_spread | 理由 |
|---|---|---|
| Guitar Classical | 0.05 | Phase 3 既存値 |
| Ukulele | 0.04 | 小型ボディで広がり控えめ |
| Mandolin | 0.06 | やや広がり強調 |
| Bass | 0.03 | 低域 mono 寄り |
| Guitar Steel | 0.05 | クラシックギター類似 |
| Sitar | 0.08 | sympathetic 弦の広がり |

Phase 4a ではこの個別値を採用するが、実装複雑度を抑えるため **`stereo_spread` は楽器選択時に切替**（params.json のグローバル const ではなく、楽器プリセットの一部として保持）。

### 7.4 楽器切替時の挙動

| アクション | 期待される動作 |
|---|---|
| プリセット選択（楽器変更を含む） | 全 active voice を即時 release（`pool.all_notes_off()`）→ Modal Body 係数を新楽器のものに差し替え → `modal_body.prepare(sample_rate)` で再計算 → `modal_body.reset()` で状態クリア |
| プリセット選択（楽器同じ、パラメータのみ変更） | voice は残す、`set_param` の系列を順次適用 |

**重要**: 楽器切替時の `pool.all_notes_off()` は **Polyphony 影響あり**（演奏中の音が切れる）。UI で「楽器切替時は音が切れます」と注意書き or fade-out 処理を Phase 4b で検討（Phase 4a では即時 release を採用）。

### 7.5 楽器切替経路

```text
UI (PresetSelector.svelte)
  ↓ engine.applyPreset(preset)
SynthEngine.applyPreset(preset)
  ↓ engine.ts で各 setParam / sendMidiCc / sendInstrument を順次発行
SynthProcessor (Worklet)
  ↓ 各 wasm function を呼ぶ
wasm-audio
  ↓ synth_apply_instrument(handle, kind) など
dsp-core::Engine
  ↓ 楽器切替: pool.all_notes_off() + modal_body 係数差し替え + prepare + reset
```

### 7.6 C ABI 拡張点

| 関数 | シグネチャ | 役割 |
|---|---|---|
| `synth_apply_instrument` | `(*mut SynthHandle, kind: u32)` | `kind`: 0=Default(Phase 3 既存), 1=GuitarClassical, 2=Ukulele, 3=Mandolin, 4=Bass, 5=GuitarSteel, 6=Sitar |
| `synth_lfo_set_rate` | `(*mut SynthHandle, hz: f32)` | LFO レート |
| `synth_lfo_set_waveform` | `(*mut SynthHandle, kind: u32)` | 0=Sine, 1=Triangle |
| `synth_lfo_set_depth` | `(*mut SynthHandle, dest: u32, depth: f32)` | dest: 0=Pitch, 1=Brightness, 2=Volume |

> **§7 結論ボックス: ◎ Phase 4a 採用**: 6 楽器の Modal 係数を §7.2 の参考値で初期実装、聴感調整は実装後。stereo_spread は楽器プリセットの一部、Default (Phase 3 既存) は kind=0 で温存。楽器切替は `synth_apply_instrument(handle, kind)` で全 voice release + 係数差し替え + reset。

---

## 8. 既存負債整理

Phase 3 retrospective §5 / §7.3 の負債を Phase 4a で対処。

### 8.1 `wasm-opt -O3` 適用

**現状**: WASM gzip 27.78 KB（target 30 KB の 92%、想定 12.9 KB を超過）。

**対処**: `scripts/copy-wasm.mjs` に `wasm-opt -O3 --strip-debug` を組み込む。Binaryen の wasm-opt が package install で利用可能。

**期待効果**: 27.78 KB → ~13 KB（仕様書想定値復帰）。

**実装**:
```javascript
// scripts/copy-wasm.mjs に追加
import { execSync } from 'node:child_process';
import { existsSync } from 'node:fs';

const wasmOptPath = 'node_modules/.bin/wasm-opt'; // or system PATH
if (existsSync(wasmOptPath)) {
  execSync(`${wasmOptPath} -O3 --strip-debug ${srcPath} -o ${dstPath}`);
} else {
  fs.copyFileSync(srcPath, dstPath);  // fallback
}
```

**依存追加**: `package.json` に `binaryen` (npm) を `devDependency` 追加。**外部 crate ではないため依存ゼロ制約に抵触しない**（npm の build-time tooling は許容）。

### 8.2 `KarplusStrong::excitation_snapshot` の `#[cfg(test)]` ガード

**現状**: `Vec::to_vec()` で test only API として alloc しているが、`#[doc(hidden)]` のみで production binary に含まれる可能性。

**対処**: `karplus_strong.rs` で `#[cfg(test)] pub fn excitation_snapshot(&self) -> Vec<f32>` に変更。production builder からは完全に除外。

**サイズ影響**: ~50 byte の関数 + Vec を除く。微々たるが production cleanliness 向上。

### 8.3 Voice State Float32Array 再構築（Phase 4b 送り）

**現状**: `synth-processor.ts` の `maybePushVoiceState` で `voiceStateAmps = new Float32Array(NUM_VOICES)` を constructor で確保 + 毎 push で `dv.getFloat32` ループ → 実際の alloc は constructor のみで毎 push の alloc は **既に発生していない**。Phase 3 retrospective §5 の記述は誤解で、コード再確認すると alloc 0。

**結論**: **対処不要**。Phase 3 retrospective §5 の記述は誤りとして本書で訂正。

実際のコード:
```typescript
private readonly voiceStateAmps = new Float32Array(NUM_VOICES);  // constructor で 1 回
```

### 8.4 Mono+Sustain の再評価

**現状**: Phase 3 D40 P1-2 で「Mono は Sustain 無視、Phase 2 既存挙動継承」を選択。

**Phase 4a での再評価**: ユーザーフィードバックがない場合、現状維持で OK。Mono+Sustain の実装複雑度が高く、音楽的意味も乏しい（Mono は last-note priority、Sustain は release defer で本質的に相反）。**Phase 4a でも no-op 継続**を仕様確定。

### 8.5 Stereo Spread の楽器別化

§7.3 で記載。多楽器プリセット実装と一体で対処。

### 8.6 README 検証手順の更新

**現状**: Phase 3 の F34（Voice Meter UI 実機）が空欄。

**対処**: Phase 4a Step 17 で README を全面更新（F38b 計測手順 + Phase 4a 機能の検証手順）。

> **§8 結論ボックス**:
> - **◎ §8.1 wasm-opt -O3 適用**: Phase 4a Step 2 で実装、サイズ 27.78 → ~13 KB 目標
> - **◎ §8.2 excitation_snapshot cfg(test) 化**: Phase 4a Step 3 で実装、5 行修正
> - **× §8.3 Voice State alloc**: 対処不要（Phase 3 retrospective §5 の記述訂正）
> - **= §8.4 Mono+Sustain**: 現状維持（no-op 継続）
> - **◎ §8.5 stereo_spread 楽器別化**: 多楽器実装と一体
> - **◎ §8.6 README 更新**: Phase 4a 最終 Step

---

## 9. Phase 4a 性能予算

### 9.1 WASM サイズ予算（gzip）

Phase 3 実測 27.78 KB、Phase 4a target 維持 < 30 KB、`wasm-opt -O3` で 13 KB 目標。

| 追加コンポーネント | raw | gzip |
|---|---|---|
| LFO module (Engine 内、phase + sin / triangle) | +0.6 KB | +0.3 KB |
| Mod Wheel SmoothedValue | +0.05 KB | +0.02 KB |
| 楽器プリセット 6 種（Modal 係数 8 × 3 × 6 = 144 値 + stereo_spread 6 値） | +0.6 KB | +0.3 KB |
| `synth_apply_instrument` C ABI | +0.3 KB | +0.15 KB |
| `synth_lfo_*` C ABI 3 関数 | +0.4 KB | +0.2 KB |
| `wasm-opt -O3` 適用効果 | -14 KB | **-14 KB** |
| Phase 4a 純増 | +1.95 KB | +0.97 KB |
| **合計（wasm-opt 込み想定）** | **15.7 KB** | **~13 KB** |

Phase 4a 後 gzip 想定: **~13 KB**（target 30 KB の 43%、Phase 3 の負債を解消）。

### 9.2 早期検証ポイント

実装途中で予算超過したら撤退する閾値:

| Step | 期待 gzip | 閾値（超過なら撤退） |
|---|---|---|
| Step 2 (`wasm-opt -O3` 適用) | 13.5 KB | > 18 KB なら -O3 効果が想定通りでない、調査要 |
| Step 7 (LFO + Mod Wheel 完了) | 14 KB | > 20 KB なら LFO destinations を 2 つに削減 |
| Step 11 (プリセット保存・ロード完了) | 14 KB | プリセットは TS 側のみ、WASM サイズ不変 |
| Step 14 (多楽器プリセット 6 種完了) | 14.5 KB | > 22 KB なら楽器を 4 種に削減 |
| Phase 4a 全完了 | 14.5 KB | > 25 KB なら R32〜 適用 |

### 9.3 CPU 予算

Phase 3 想定 1.95 ms/process（128 frames @ 48 kHz、F38b 未計測）。Phase 4a 加算:

| 追加 | 演算数/sample | × 128 |
|---|---|---|
| LFO process (sine 計算 + smoothing) | +10 | +1280 |
| Voice 拡張 (pitch / brightness offset 加算 × 8 voice) | +16 | +2048 |
| Engine volume offset | +2 | +256 |
| 楽器切替 (event-driven、process 内 0) | 0 | 0 |
| **合計** | **+28** | **+3584** |

WASM 1 GHz 仮定で +3584 / 1e9 = +0.0036 ms/process。Phase 3 想定 1.95 ms に対し +0.18%、**ほぼゼロ加算**。

性能目標 (Phase 4a):
- avg < 1.7 ms（Phase 3 1.5 ms + 0.2 ms 余裕）
- max < 2.7 ms（Phase 3 2.5 ms + 0.2 ms 余裕）

### 9.4 メモリ予算

Phase 3 で `Engine::prepare` 一括確保済。Phase 4a 追加分:

| 追加バッファ | サイズ |
|---|---|
| LFO 状態（phase: f32 + value: f32） | 8 B |
| Mod Wheel SmoothedValue | 12 B |
| LFO destinations 3 つの SmoothedValue (rate / depths) | 60 B |
| 楽器プリセット 6 種の Modal 係数テーブル (TS 側 const、WASM コード) | TS 側 ~3 KB / WASM コード ~600 B |
| Preset JSON データ (TS 側、最大 32 件 × 350 byte = 11.2 KB、localStorage) | 11.2 KB (localStorage) + 11.2 KB (heap) |
| **合計（WASM ヒープ）** | **+0.08 KB** |

`memory.buffer.byteLength` 不変条件は維持可能。

---

## 10. 実装着手前に答えを出すべき問い

Phase 4a 仕様書（01〜07）を策定する前に、以下を確定する。本書 §2–§9 は方針確定の根拠を提供しているが、**最終判断はユーザー承認**が必要:

1. **F38b 計測の実施タイミング**: 仕様書策定前 / 策定後 / 実装 Step 1（§0 として）のどれか → **Step 1 として実装着手の最初に実施**で確定
2. **LFO destinations の数**: Pitch / Brightness / Volume の 3 つで確定するか、Pitch + Volume の 2 つに絞るか → **3 つで確定**
3. **LFO 波形数**: Sine + Triangle の 2 つか、Sine + Triangle + S&H の 3 つか → **2 つで確定（S&H は Phase 4b 以降）**
4. **多楽器選定 6 種**: ユーザー指定 (Guitar Classical / Ukulele / Mandolin / Bass / Guitar Steel / Sitar) で確定 → **確定**
5. **プリセット User 上限**: 32 件で確定するか、もっと少なく / 多く → **32 件で確定**
6. **楽器切替時の挙動**: 全 voice release（即時音切れ）か、fade-out で段階リリース → **即時 release（Phase 4a）、fade-out は Phase 4b 以降**
7. **Mod Wheel UI スライダー**: 専用スライダー or Sustain Pedal などと同じ row のミニ表示 → 仕様書 05 章で UI モックアップを提示してユーザーレビュー

---

## 11. 引き続き有効な Phase 1 / 2 / 3 文献 + Phase 4a 新規参照

### 11.1 継続参照

- Smith J.O. *Physical Audio Signal Processing* CCRMA — Body Modes 章 / Loss Filter 章
- Phase 3 [§2.3 Modal Body 係数](../2026-05-07-003-phase3/pre-research.md) — ギターボディ初期値
- Phase 3 [§5 Brightness 補正](../2026-05-07-003-phase3/pre-research.md) — LFO Brightness destination の参考
- Phase 3 [§6 MIDI CC](../2026-05-07-003-phase3/pre-research.md) — Mod Wheel 経路

### 11.2 Phase 4a で新規参照

- **Penttinen, Karjalainen, Härmä (2006)** "New Techniques for Real-Time Physical Modeling Sound Synthesis" — 多楽器の Modal 係数測定手法（§7 多楽器プリセット）
- **Roads C. (1996)** *The Computer Music Tutorial* MIT Press — LFO 設計、Mod Wheel、波形 (§3 LFO)
- **Web Storage W3C Recommendation (2016)** — localStorage 仕様 (§6)
- **MIDI 1.0 Specification** — CC#1 Modulation (§4)
- **Karjalainen, Välimäki, Tolonen (1998)** "Plucked-String Models" — 各楽器の Karplus-Strong パラメータ (§7)
- **Smith J.O. "Music 226 Course Notes"** CCRMA — sitar / mandolin の simulation 文献 (§7)

### 11.3 参考実装の追加

- **JUCE `LFO` クラス** — LFO 設計参考 (§3)
- **Csound `lfo` opcode** — 波形種選定参考 (§3)
- **Web Audio API `AudioParam.setValueAtTime`** — Worklet 内 SmoothedValue 設計の対比 (§3)

---

## 12. Phase 4a で参照しない領域（Phase 4b 以降送り）

| 領域 | 理由 |
|---|---|
| **ピアノ音色 (Stretching all-pass + impact model)** | Phase 4b で別計画扱い |
| **C8 ピッチ自己発振モード** | 物理限界、damping=1.0 経路 or FFT estimator が要、Phase 4b 検討 |
| **Pick position fractional 化** | 連続変更を滑らかに、Phase 4b |
| **Look-ahead limiter** | 5 ms 遅延型、soft clip より透明、Phase 4b |
| **WASM SIMD** | `target-feature=+simd128` の Safari/Firefox 対応再評価、Phase 4b |
| **Brightness allpass 直列補正** | 知覚的不十分なら、Phase 4b |
| **LFO 波形 S&H / Square / Sawtooth** | §3.3 の理由で Phase 4b 以降 |
| **LFO destinations 拡張 (Pick / Damping / BodyWet)** | §3.5 の理由で Phase 4b 以降 |
| **Voice State SAB 化** | COOP/COEP 必要で GitHub Pages 不可 |
| **Modal Body M=8 → M=5 削減** | 多楽器プリセットで表現力担保、削減不要 |
| **Cross-tab 同期 (storage event)** | UX 需要が Phase 4b 以降で出れば検討 |
| **Preset import / export (JSON file)** | localStorage 内 only で当面十分 |
| **管楽器 / 打楽器** | Phase 5 領域 |
| **録音・MIDI export** | Phase 5 |
| **Mono+Sustain 実装** | §8.4 の理由で current 維持 |

---

## 13. Phase 4a 実装順序の試案（07 章への種）

本書の結論を統合した実装順:

1. **Step 1**: F38b 実機計測 + 結果を retrospective §5 に追記（§2）
2. **Step 2**: `wasm-opt -O3` を `scripts/copy-wasm.mjs` に組み込み（§8.1）
3. **Step 3**: `excitation_snapshot` を `#[cfg(test)]` でガード（§8.2）
4. **Step 4**: params.json + gen-params.mjs 拡張（LFO + 楽器 enum + 楽器ごとの Modal 係数定数）
5. **Step 5**: `dsp-core/src/lfo.rs` 実装 + Engine 統合（§3）
6. **Step 6**: `Engine::handle_midi_cc` の CC#1 (Mod Wheel) 分岐実装（§4）
7. **Step 7**: LFO destinations 統合（pitch / brightness / volume offset 加算）（§3.7）
8. **Step 8**: 多楽器プリセットの Modal 係数定数 6 種を dsp-core に追加（§7.2）
9. **Step 9**: `Engine::apply_instrument(kind)` 実装（§7.4）
10. **Step 10**: C ABI 4 関数追加 + REQUIRED 配列更新（§7.6）
11. **Step 11**: Worklet messages.ts + WasmExports + SynthEngine 拡張
12. **Step 12**: `web/src/lib/state/preset-store.svelte.ts` 実装 + Factory Preset 7 種定義（§5 / §6）
13. **Step 13**: PresetSelector.svelte UI 実装
14. **Step 14**: ModWheel.svelte + LFO controls UI 実装（§3 / §4）
15. **Step 15**: 統合 cargo test + alloc ゼロ検証 + サイズ計測（F39〜）
16. **Step 16**: 実機確認（pnpm dev で全 Phase 4a 機能動作確認）+ F38b 再計測
17. **Step 17**: ドキュメント整備（README / CLAUDE.md / retrospective 準備）

各 Step は仕様書 07 章で `cargo test` / 実機検証の達成ラインを明示する（Phase 1 / 2 / 3 と同じ流儀）。

---

## まとめ（1 行）

> Phase 4a は「**F38b 実機計測** で Phase 3 完成判定の最終案件を閉じ + **wasm-opt -O3 / excitation_snapshot cfg(test)** で既存負債を解消 + **LFO (Engine 内 1 個、Sine/Triangle、3 destinations: Pitch/Brightness/Volume)** + **Mod Wheel (CC#1) を LFO master として有効化** + **localStorage プリセット保存 (max 32 User Preset、JSON v1)** + **多楽器プリセット 6 種 (Guitar Classical / Ukulele / Mandolin / Bass / Guitar Steel / Sitar)**」を一括実装。WASM gzip target ~13 KB（wasm-opt -O3 効果込み、Phase 3 27.78 KB から大幅削減）、CPU +0.0036 ms/process で予算余裕大、Phase 4b はピアノ音色 (Stretching all-pass) で別計画。
