# Phase 3 調査資料

## Body Resonator・Extended Karplus–Strong・MIDI CC・C8 ピッチ精度の前提整理

本書は Phase 3 仕様策定で参照する追加調査トピックを集約する。Phase 1 / Phase 2 で既に決着した基礎理論（Karplus–Strong、Digital Waveguide、Lagrange 3 次補間、ParamDescriptor、voice stealing、hold note stack）は重複させず、各 pre-research の該当節を参照する。Phase 3 は **方式選定（特に Body Resonator）の重みが大きい**ため、各章末に **結論ボックス（◎採用 / ○検討 / △Phase 4 送り / ×不採用）** を置き、本文の論考と独立に「その章で何が決まったか」を一目で拾えるようにする。

---

## 0. Phase 1 / Phase 2 pre-research との関係

Phase 3 は以下の節を **Phase 1 / 2 の pre-research を一次資料**として参照する。

| Phase 節 | 内容 | Phase 3 での参照箇所 |
|---|---|---|
| Phase 1 [§3.1 Karplus–Strong](../2026-05-06-001-mvp/pre-research.md) | 基本原理、`delay_length ≒ sample_rate / frequency` | §3 Extended KS の起点（loss filter / pick position の挿入位置） |
| Phase 1 [§3.2 Digital Waveguide](../2026-05-06-001-mvp/pre-research.md) | 双方向ディレイ、loss filter、bridge / nut 反射 | §3.1 loss filter の物理的根拠 |
| Phase 1 [§3.3 Modal Synthesis](../2026-05-06-001-mvp/pre-research.md) | 並列共鳴モード、`exp(-decay·t) sin(2πft+φ)` | §2 Body Resonator の Modal 方式（**Phase 3 第一候補**） |
| Phase 1 [§7.2 Extended KS](../2026-05-06-001-mvp/pre-research.md) | fractional delay / loss filter / pick position / stretching all-pass / body | Phase 2 で fractional delay 達成、本書 §3 で残り 3 候補の優先順位を確定 |
| Phase 1 [§7.3 Body Resonator](../2026-05-06-001-mvp/pre-research.md) | IR convolution / Modal の概論 | §2 で深掘り、方式確定 |
| Phase 2 [§3 Fractional delay](../2026-05-07-002-phase2/pre-research.md) | Lagrange 3 次の式と採用根拠 | §4 Thiran allpass 比較の Lagrange 側 |
| Phase 2 [§6 性能予算](../2026-05-07-002-phase2/pre-research.md) | gzip < 30 KB、CPU 1.5 ms 目標 | §9 Phase 3 性能予算で再計算 |
| Phase 2 retrospective [§5 負債リスト](../../retrospective/2026-05-07-002-phase2.md) | brightness LPF 群遅延 0.89%、C8 物理限界、`__synthDev` 正式化 | §4 / §5 / §7 で個別に対応 |
| Phase 2 retrospective [§7 Phase 3 候補](../../retrospective/2026-05-07-002-phase2.md) | 含める / 検討 / Phase 4 送りリスト | §1 でリプリント、本書全体で深掘り |

---

## 1. Phase 3 スコープと前提制約

### スコープ整理（Phase 2 retrospective §7 より転記）

| 候補 | retrospective 判断 | 本書での扱い |
|---|---|---|
| Body Resonator（IR / Modal / Static） | **含める** | §2 で方式選定 |
| Extended KS（loss filter / pick position / stretching all-pass） | **含める** | §3 で 3 拡張の優先順位 |
| MIDI CC マッピング（pitch bend / mod / sustain / volume） | **含める** | §6 でアーキテクチャ確定 |
| UI voice meter / mono–poly トグル | **含める（軽微）** | §7 で API 設計 |
| C8 ピッチ精度（Thiran allpass） | **検討** | §4 で試作評価方針 |
| brightness LPF 群遅延補正 | **検討** | §5 で補正方式選定 |
| Soft clip / look-ahead limiter | **検討** | §8 でリスク評価 |
| プリセット保存・ロード | **Phase 4 へ送る** | §12 で送り |
| WASM SIMD | **Phase 4 へ送る** | §12 で送り |
| `KarplusStrong::note_on` の buffer ゼロクリア最適化 | **見送り** | §12 で送り |

### 制約（Phase 1 / 2 から継承、Phase 3 でも維持）

- **WASM gzip < 30 KB**（Phase 2 実測 10.56 KB → Phase 3 で +約 19 KB の予算）
- **依存ゼロ**: `dsp-core` / `wasm-audio` で外部 crate を追加しない（FFT が必要なら自前実装、ただし §2 で時間域 conv. 採用なら不要）
- **`Engine::prepare` 以外でヒープ確保禁止**（IR バッファや Modal 係数も `prepare` で一括確保）
- **C ABI のみ**: `wasm-bindgen` 不使用、`#[unsafe(no_mangle)] extern "C"` を継続
- **Float32Array view キャッシュ**: Worklet 側で `process()` 内に `new Float32Array(...)` を作らない原則を維持

### 本書の確定責任

Phase 3 着手前に以下 3 件を本書で確定させる:

1. §2 で **Body Resonator 方式を 1 つ選定**（IR convolution / Modal / Static IR）
2. §4 で **Thiran allpass 採否の試作方針**を確定（全置換 / ハイブリッド / Lagrange 維持）
3. §9 で **WASM サイズ予算の早期検証ポイント**を明示（実装途中で予算超過したときの撤退計画）

§10 の「実装着手前に答えを出すべき問い」9 件は仕様書策定時に順次決める。

---

## 2. Body Resonator 方式選定（**Phase 3 最重要決断**）

### 2.1 楽器ボディ共鳴の物理

ギター・リュート系のボディは、**気室共鳴 (Helmholtz resonance, 80–120 Hz)** + **トップ板の主モード (180–250 Hz)** + **トップ板高次モード (400 Hz 〜 数 kHz)** の重畳で特徴付けられる。典型的には 5–20 個の卓越モードが Q = 20–50 で立ち、それ以上は分布が連続的になる。

IR の典型長:

- 48 kHz サンプリングで初期立ち上がり 0–10 ms（480 samples）に最大エネルギー
- 残響部 100–500 ms（4800–24000 samples）まで尾を引く
- 実用上は **512–2048 samples で 90% のスペクトル特徴を再現**できる（Smith *PASP* "Body Modes"、Penttinen et al. 2006）

参考: Karjalainen et al. (1998) "Plucked-String Models" は body filter を separable な並列共鳴フィルタバンクで近似する標準化を提示。

### 2.2 三方式の比較（**メイン比較表**）

| 項目 | A: IR conv. 時間域 | B: IR conv. FFT-based | C: Modal (5–10 並列 IIR) | D: Static IR 単一 |
|---|---|---|---|---|
| 計算量（per sample, mono） | O(N=512–2048) MAC | O(log N) per FFT block | O(M=5–10) biquad | A と同等 |
| 演算数概算 (N=1024 / M=8) | 1024 MAC | ~200 MAC（FFT amortized） | **8 biquad × 5 = 40 MAC** | 1024 MAC |
| アルゴリズム複雑度 | ◎ ring buffer + FIR で完結 | × 自前 FFT 実装が必要（外部 crate 禁止） | ○ biquad bank、Phase 1 §3.3 と直結 | ◎ 最単純 |
| WASM コード増分 | +0.8 KB raw / +0.4 KB gzip | +6–10 KB raw / +3–5 KB gzip | +1.5 KB raw / +0.7 KB gzip | +0.5 KB raw |
| IR / 係数データ | 4–8 KB raw（512–2048 × f32） | 同左 | **0.5 KB**（M=8 × 3 係数 × 2 IR） | 4–8 KB（固定） |
| 表現力 | ◎ 任意の IR | ◎ | △ モード数で制限、平滑近似 | × 1 種類のみ |
| パラメータ可変性 | × IR 切替は重い | × 同左 | ◎ Q / 周波数を SmoothedValue で連続変更可 | × 不可 |
| プリセット展開（Phase 4 送り） | ○ IR 複数を `include_bytes!` | ○ 同左 | **◎ 係数テーブル切替が瞬時** | △ |
| 依存ゼロ維持 | ◎ | × 自作 FFT で可能だが +5 KB | ◎ | ◎ |
| Phase 1 学習資産活用 | △ | △ | **◎ §3.3 Modal Synthesis を直接実装** | △ |
| 採用評価 | ○ fallback 候補 | × | **◎ 第一候補** | △ |

### 2.3 Modal フィルタバンクの実装スケッチ

各モードは 2 次共鳴 IIR（biquad）で表現:

```text
y[n] = b0 · x[n] - a1 · y[n-1] - a2 · y[n-2]

a1 = -2 · exp(-π·f/Q/Fs) · cos(2π·f/Fs)
a2 = exp(-2π·f/Q/Fs)
b0 = (1 - a2) · gain     // モードゲイン正規化
```

`f`（中心周波数 Hz）、`Q`（共鳴鋭さ）、`gain`（モード相対振幅）の 3 係数で 1 モードを規定。M = 8 モードを並列加算（直列ではなく **加算**、共鳴は独立振動）:

```text
y_body = Σ_{m=0..M-1} biquad_m(x_input)
```

- **配置**: `engine.rs:154-163` の `pool.process_sample()` 後、`output_gain` 前に挿入。VoicePool の `1/sqrt(N)` スケール後の単一モノラル信号に作用（per-voice ではなく Engine 全体に 1 段、ボイス数に依存しない CPU）
- **励振**: `note_on` で励振しない（ボディは常時動作）。`prepare` で係数初期化、`reset` で状態クリア
- **stereo 化**: 左右で異なる係数テーブル（Q や gain を ±5% 揺らす）を持たせ、自然な広がり。CPU は 2 倍だが、ボイス数に独立なので poly N=8 でも +0.05 ms 程度

代表的なギターボディモード（参考値、Penttinen et al. 2006、Karjalainen et al. 2002）:

| モード | f (Hz) | Q | gain |
|---|---|---|---|
| Helmholtz (T(1,1)₁) | 105 | 30 | 1.0 |
| Top plate (T(1,1)₂) | 200 | 25 | 0.8 |
| Top + back coupled | 280 | 20 | 0.5 |
| Higher top mode | 420 | 35 | 0.4 |
| Mid resonance | 580 | 40 | 0.35 |
| Upper mid | 850 | 45 | 0.25 |
| High brilliance | 1400 | 50 | 0.2 |
| Air mode (high) | 2300 | 60 | 0.15 |

（実装時は `params.json` で 8 モード × 3 係数 = 24 値を const テーブル化、または `ParamId::BodyMix` 1 つで dry/wet をパラメータ化）

### 2.4 IR データ / 係数データの配布

| 方式 | サイズ | パース時刻 | 評価 |
|---|---|---|---|
| Modal 係数のみ const テーブル | 0.5 KB | 0（const） | **◎ Modal 採用なら自然** |
| `include_bytes!("body.f32")` raw float32 | 4–8 KB | 0（直接 view） | ○ time-domain IR fallback 用 |
| `include_bytes!("body.wav")` + WAV header parser | 4–8 KB | `prepare` 1 回（〜100 サンプルの header skip） | △ wav パーサーで +0.5 KB |
| `fetch('/body.wav')` 動的ロード | 0（バイナリ外） | first paint 後 | △ Worklet 側 fetch 制約あり |

---

> **§2 結論ボックス**
>
> - **◎ Phase 3 採用: C. Modal Synthesis (M = 8 並列 biquad)**
>   - 理由: (a) Phase 1 §3.3 で予習済み、(b) パラメータ可変で Phase 4 プリセット展開と相性良、(c) 依存ゼロ・サイズ +0.7 KB gzip と最小、(d) M=8 でも 40 MAC/sample、ボイス数非依存で CPU 余裕
> - △ B. FFT-based IR は自前 FFT 実装で +5 KB、外部 crate 禁止のため当面除外
> - ○ A. 時間域 IR convolution は Modal の音響評価が不十分なら fallback 候補として温存（実装は §2.3 と並列で書ける単純さ）
> - 配置は `engine.rs:154-163` の `output_gain` 前、Engine レベル 1 段
> - 係数は `params.json` で `body_mode_<n>_{freq,q,gain}` として codegen、ステレオ化は左右係数 ±5% で実現

---

## 3. Extended Karplus–Strong 拡張の優先順位

Phase 1 [§7.2](../2026-05-06-001-mvp/pre-research.md) で言及された 3 つの拡張を、本フェーズで採用するか確定する。

### 3.1 Loss filter（弦の周波数依存損失）

弦の振動は高域ほど早く減衰する（空気抵抗・内部摩擦）。現在は `damping` スカラーと `brightness` 1-pole LPF が役割を兼ねるが、loss filter は **loop gain を周波数依存で制御**する独立の役割を持つ。

| 方式 | H(z) | コスト | 役割の独立性 | 採否 |
|---|---|---|---|---|
| 1-pole LPF（既存 brightness と同形） | `b0/(1-a1·z^-1)` | 2 演算 | × brightness と区別不能 | × 重複 |
| **One-zero loss filter (Smith PASP 標準形)** | `(1+ρ·z^-1)/(1+ρ)` | 3 演算 | ◎ brightness = pick の硬さ、loss = 弦の摩擦 | **◎** |
| Two-pole biquad | 5 演算 | 5 演算 | △ Q を持たせると Modal Body と機能重複 | △ Body と重複 |

**One-zero の特性**: ρ ∈ [0, 0.5] で DC ゲイン 1 維持、Nyquist 付近を `(1-ρ)/(1+ρ)` 倍に減衰。`ρ` を `note_on` の周波数の関数（高音ほど大）にすると物理的に正しい。Smith *PASP* "Karplus-Strong Loss Filter" 章の H(z) と同形。

**配置**: `karplus_strong.rs:188-194` の `brightness` LPF 直後・`damping` 乗算前。process_sample 内で +3 演算/sample。

> **§3.1 結論ボックス: ◎ Phase 3 採用**: One-zero loss filter `(1 + ρ·z^-1)/(1 + ρ)`。`ρ` は `note_on` 時に周波数依存式 `ρ = ρ_base · clamp(freq/220, 0.5, 2.0)` で算出、process 内コスト +3 演算。

### 3.2 Pick position（励振位置）

ピックで弦を弾く位置（ナットからブリッジまでの相対位置 β ∈ (0, 0.5]）が変わると、**β·L サンプル相当**の comb 効果が発生し、倍音バランスが変化する。物理的には「弾いた位置の節になる倍音は出ない」現象。

```text
H_pick(z) = 1 - z^(-K),   K = round(β · L)
```

| 配置 | 物理的意味 | コスト | 採否 |
|---|---|---|---|
| **励振 shaping（`note_on` 時に noise を comb 整形して buffer にロード）** | Smith *PASP* "Plucked String" の標準形、ピッキング瞬間の波形整形 | +length_int 演算（`note_on` のみ）、process 内コスト 0 | **◎ Phase 3 採用** |
| 出力経路の comb（feedback loop 外、KS 出力に対し 1-tap comb） | 出力フィルタとして近似、loop 安定性に影響なし | +1 演算/sample + K-tap delay buffer（最大 1024 sample × 8 voice = 32 KB） | △ メモリ予算超過リスク |
| Feedback loop 内の 1-tap comb | 強い周波数依存 loss filter として機能、loop gain 安定性議論を再開 | +1 演算/sample + 256-tap delay（A1 β=0.5 で K=437 に届かず仕様矛盾） | × **不採用**（旧版仕様撤回） |
| Fractional β（Lagrange 補間化） | より滑らかなピッキング感、可変中も滑らか | +5 演算（Lagrange 内挿） | △ Phase 4（高音域 K=2-3 では fractional の効果薄） |

**励振 shaping の実装**:
1. `note_on` で noise burst を生成し buffer の先頭 length_int サンプルにロード
2. K > 0 なら降順ループで `buffer[i] -= buffer[i - k]` を in-place 適用（追加バッファ不要）
3. `write_index = length_int` から開始

**β の値域**: 公開 API（`params.json` の `PickPosition` パラメータ）は **`β ∈ [0.05, 0.5]`**（D34 / 03-dsp-core-spec.md `params.json` 拡張）、`β = 0.5` でセンター（偶数倍音消失）、`β = 0.125` がギター実機の「やや bridge 寄り」（デフォルト）。process 中の β 変更は次回 `note_on` で反映（連打すれば追従）。

> **β = 0 の扱い（概念説明）**: 数学的には K = round(β · length_int) = 0 で comb shape 無効（Phase 2 と同じ noise burst）。**ただし Phase 3 の公開 API では β ≥ 0.05 のため、外部から β = 0 は到達不可**。`length_int = 9` + `β = 0.05` で K = round(0.45) = 0 になる内部分岐は `#[cfg(test)]` 経由でのみテスト（`test_pick_internal_k_zero_branch`、03 章 §Pick position テスト方針 / R26 / P2-2 対策）。

> **§3.2 結論ボックス: ◎ Phase 3 採用（励振 shaping）**: `note_on` 時の comb 整形（in-place、追加メモリ 0、process 内コスト 0）。`β = pick_position ∈ [0.05, 0.5]` を `params.json` でパラメータ化、Engine が `f32` で保持。Fractional β は Phase 4 送り。
>
> **設計変更履歴**: 旧版仕様（feedback loop 内 1-tap comb、`pick_position.rs` 専用モジュール、PICK_DELAY_MAX=256）は (a) loop gain 安定性議論を再開させ Step 1 Thiran 試作と干渉、(b) PICK_DELAY_MAX=256 が A1 β=0.5（K=437）に届かず仕様矛盾、の 2 件で撤回。励振 shaping に変更。

### 3.3 Stretching all-pass（剛性弦の inharmonicity）

実弦は完全な調和倍音ではなく、高次倍音ほど周波数が **わずかに上方偏移**（inharmonicity, B = stretch 係数）。これを再現するのが dispersive all-pass:

```text
H_disp(z) = (a + z^-1) / (1 + a·z^-1)
```

を K 段直列接続。1 段で +5 演算、効果が顕著に出るのは K = 4–8。ピアノでは必須、ギター系では薄い効果（B ≈ 5×10⁻⁵ 程度、人耳は気付きにくい）。

| 用途 | 必要性 | Phase 3 での扱い |
|---|---|---|
| ピアノ・ハープシコード | 必須（B = 10⁻³ 級） | **△ Phase 4（ピアノ音色追加時）** |
| ギター・ベース | 微妙（B = 5×10⁻⁵） | **× Phase 3 不採用、計算量に見合わず** |
| エレキギター歪み系 | 不要 | × |

> **§3.3 結論ボックス: △ Phase 4 送り**: ギター系では効果薄、CPU コスト +5 演算 × 8 段 = 40 演算/sample × 8 voice = 320 演算/sample が割に合わない。Phase 4 でピアノ音色を加える際に再評価。

---

## 4. C8 ピッチ精度の再検討（Thiran allpass 評価）

Phase 2 retrospective §4.1 で残された課題: Lagrange 3 次補間は `|H_lag(C8)| ≈ 0.998` で **loop gain < 1**、C8 (周期 11.5 sample) で自己発振条件を満たさず。代替候補が Thiran allpass。

### 4.1 Thiran 1 次 allpass の式

ディレイ長 `D = D_int + d`（`d ∈ [0, 1)`）に対し:

```text
H_thiran(z) = (a₁ + z^-1) / (1 + a₁ · z^-1)
a₁ = (1 - d) / (1 + d)        // 1 次 Thiran allpass 設計式
```

特性:
- `|H_thiran(ω)| = 1` 厳密保持 → loop gain は damping のみで決まり、C8 でも自己発振条件成立
- 状態 1 個追加（前 sample の出力を保持）
- 安定性: 極 `z = -a₁`、`|a₁| < 1` で単位円内。d ∈ (0, 1) で安定だが **d=0 と d=1 は境界**（d=0 で a₁=1 → 極 z=-1 で単位円上、d=1 で a₁=0 → 極 z=0 で安全だが Thiran 効果なし）。実装では `d.clamp(1e-4, 0.999)` で境界を避け、`a₁ ∈ [5e-4, 0.9998]` を保証（R25）

参考: Smith *PASP* "Allpass Interpolation"、Välimäki & Laakso (2000) "Principles of Fractional Delay Filters"。STK `DelayA` クラスは Thiran allpass を採用。

### 4.2 Lagrange vs Thiran 比較表

| 項目 | Lagrange 3 次 (現行) | Thiran 1 次 allpass |
|---|---|---|
| 振幅応答 | `\|H\|` 高域で <1（C8 で 0.998） | `\|H\| = 1` 厳密 |
| 群遅延 | フラット（≈ d sample） | 周波数依存（低域で d、高域で異なる） |
| 計算量 (per sample) | 4 MAC + 0 状態 | 2 MAC + 1 状態 |
| 安定性 | FIR、無条件安定 | IIR、`d ∈ (0,1)` で極内 |
| 実装複雑度 | ◎ 既存 | ○ 状態 1 追加 |
| C8 自己発振 | × 不成立 | ◎ 成立 |
| 群遅延の周波数依存 | ほぼ 0 | 低域寄り（高域で 1 sample 弱の偏移） |
| トランジェント応答 | 良好（FIR） | 起動時に過渡応答（IIR） |
| Phase 2 既存テスト互換 | ◎ | ? 全テスト再検証必要 |

### 4.3 ハイブリッド戦略

| 案 | 概要 | リスク |
|---|---|---|
| **A. 全周波数 Thiran 置換** | `LagrangeCoeffs` を `ThiranCoeffs` に差し替え | 低音域で群遅延の周波数依存が表面化、Phase 2 ピッチテスト全件再検証 |
| **B. 高域のみ Thiran (MIDI > C7)** | `note_on` で `if midi >= 96 { thiran } else { lagrange }` | 実装複雑度↑、テスト 2 系統 |
| **C. Lagrange 維持・C8 honest skip** | 現状維持（Phase 2 retrospective §4.1） | 限界が文書化済 |
| **D. FractionalDelay trait 化、両方提供** | `trait FractionalDelay { fn read(...) }` で多態化 | const generic vs dyn のコスト議論、+0.5 KB |

### 4.4 Phase 3 での試作方針

1. `crates/dsp-core/src/fractional_delay.rs` に `ThiranCoeffs` 構造体を追加（`LagrangeCoeffs` と並列、共通 trait は当面作らず）
2. `pitch_accuracy.rs` に `test_pitch_c8_thiran` を追加（既存テストは触らない）
3. 全周波数で Thiran に切替えた場合の `test_pitch_a1` 〜 `test_pitch_c6` を一巡（A1 で誤差悪化があれば案 A 撤退、案 B か C へ）
4. C8 で自己発振が安定確認（10 秒走らせて RMS が定常値に収束することを cargo test で確認）

> **§4 結論ボックス: ○ Phase 3 で試作評価**
>
> - 案 A（全 Thiran）を仕様書策定中に試作、低音域の精度劣化が知覚閾以下なら採用
> - 既存 `LagrangeCoeffs` は残し、`ThiranCoeffs` を併設（コード共有は後付けの trait 化で）
> - 試作結果次第で案 A / B / C のいずれかを Phase 3 仕様 D-tag で確定（D33 候補）

---

## 5. Brightness LPF の群遅延補正

Phase 2 retrospective §5 の負債: `karplus_strong.rs:188-191` の 1-pole brightness LPF は群遅延が周波数依存で、A4 で **0.89% のピッチ下方偏移**（デフォルト brightness=0.5）。

### 5.1 補正方式の比較

| 方式 | 概要 | コスト | 副作用 | 採否 |
|---|---|---|---|---|
| **ディレイ長補償** | LPF の DC 群遅延 `τ_g(0)` 分だけ delay 整数 / fractional 部から引く | 1 行（`note_on` 時） | brightness 可変時に再計算必要 | **◎** |
| LPF を minimum-phase 化 | 既に minimum-phase なので無効 | — | — | × |
| 2 次 allpass で群遅延逆補正 | `H_ap · H_lpf` を直列、フラット化 | +5 演算/sample | サイズ・CPU 増 | △ |
| LPF を high-Q biquad で再設計 | 群遅延が中央周波数に集中 | +3 演算 | 周波数応答が変わる | × |

**ディレイ長補償の式**: 1-pole LPF `H(z) = b/(1-a·z^-1)` の DC 群遅延は `τ_g(0) = a/(1-a)` sample。`brightness ∈ [0,1]` の場合 `a = 1 - brightness`、よって `τ_g(0) = (1-brightness)/brightness`。

`note_on` 時に
```text
adjusted_length = base_length - τ_g(brightness)
```
を fractional delay に渡せばピッチ下方偏移が解消。`brightness=0.5` で `τ_g = 1.0` sample 補償、`brightness=1.0`（フィルタほぼスルー）で 0 sample。

### 5.2 brightness が SmoothedValue なときの扱い

`brightness` は SmoothedValue で連続変動するが、ディレイ長を **process 中に** 動的変更すると遷移ノイズが出る。Phase 3 では:

- `note_on` 時の `brightness` 値で 1 度だけ補償計算
- ノート保持中の brightness 変化は **ピッチ偏移として許容**（vibrato 効果として捉える）
- 真にフラット化したいなら案 C（allpass 直列）を Phase 4 で検討

> **§5 結論ボックス: ○ Phase 3 採用条件付き**: ディレイ長補償方式。実装は 1 行追加（`adjusted_length = length - tau_g(brightness)`）。Phase 3 終盤の聴感確認で 0.89% → 0% への改善が体感できなければ Phase 4 送り。

---

## 6. MIDI CC マッピング設計

### 6.1 対象 CC のスコープ

Phase 3 では Pitch Bend + 3 CC のみ。**Mod Wheel (CC#1) は Phase 4 送り**: LFO の rate / 波形 (sine / triangle / S&H) / 配分 (pitch / brightness / volume への送り) / 深さの仕様確定が Phase 3 スコープ外、Phase 4 で `set_mod_depth` および LFO 仕様を併せて確定する。

| CC | 番号 | 範囲 | SmoothedValue | 配信先 | Phase 3 |
|---|---|---|---|---|---|
| Pitch Bend（専用メッセージ） | — | 14-bit, ±2 半音 | 必須（5 ms tau 推奨） | 全 active voice | ◎ |
| Sustain Pedal | CC#64 | 0/127（≥64 = on） | 不要 | Engine 状態（damping 加速保留） | ◎ |
| Channel Volume | CC#7 | 7-bit | 既存 OutputGain と統合 | output_gain | ◎ |
| All Notes Off | CC#123 | trigger | 不要 | 全 voice の即時 release | ◎ |
| ~~Mod Wheel~~ | ~~CC#1~~ | ~~7-bit~~ | ~~（仕様未確定）~~ | ~~LFO depth~~ | **△ Phase 4** |

expression (CC#11)、pan (CC#10)、resonance (CC#71) などは Phase 4 送り。

### 6.2 アーキテクチャ

```text
WebMIDI API (main thread)
  → input/midi-handler.svelte.ts (parse CC bytes)
    → engine.ts.sendCC(cc, value)
      → MessagePort: { type: 'midiCC', cc, value }
        → synth-processor.ts dispatch
          → wasm: synth_midi_cc(handle, cc, value)
            → engine.rs::handle_midi_cc()
              → cc==7:   output_gain.set_target(value/127)
              → cc==64:  engine.sustain_active = (value >= 64)
              → cc==123: voice_pool.all_notes_off()
              // cc==1 (Mod Wheel) は Phase 4 送り

WebMIDI Pitch Bend → engine.ts.sendPitchBend(semitones)
  → MessagePort: { type: 'pitchBend', semitones }
    → wasm: synth_pitch_bend(handle, semitones)
      → engine.rs: 全 active voice に fan-out
        → KarplusStrong::set_pitch_bend(semitones)
          → length_target = base_length × 2^(-semitones/12)
            → SmoothedValue で 5 ms 遷移
```

### 6.3 C ABI 関数追加

| 関数 | 用途 |
|---|---|
| `synth_midi_cc(handle, cc, value_normalized)` | 汎用 CC dispatch、内部 switch |
| `synth_pitch_bend(handle, semitones)` | Pitch Bend 専用（精度確保のため f32 で受ける） |
| ~~CC ごとに専用関数~~ | × drift リスク・関数数増、不採用 |

`synth_midi_cc` は内部で C ABI 表面積を最小化、`synth_pitch_bend` のみ独立（pitch bend は連続値で頻度高、CC#1 等とは独立スループットが必要）。

### 6.4 Voice trait の拡張

```rust
pub trait Voice {
    // 既存（Phase 2）
    fn note_on(&mut self, freq_hz: f32, velocity: f32);
    fn note_off(&mut self);
    fn process_sample(&mut self) -> f32;
    fn is_active(&self) -> bool;
    fn note_id(&self) -> Option<u8>;
    fn age(&self) -> u32;
    fn amplitude(&self) -> f32;

    // Phase 3 追加（D39）
    fn set_pitch_bend(&mut self, semitones: f32);  // 全 voice fan-out

    // Phase 4 候補（Mod Wheel + LFO 仕様確定後）
    // fn set_mod_depth(&mut self, depth: f32);
}
```

`set_pitch_bend` は `KarplusStrong` 側で `length_target = base_length_int + base_length_frac · 2^(-semitones/12)` を SmoothedValue 化して保持。fractional delay 既存実装（Phase 2 §3）が活きる。

> **§6 結論ボックス: ◎ Phase 3 採用（CC 4 種 + Pitch Bend）**
>
> - `synth_midi_cc(cc, value)` 1 関数で集約（CC#7 / #64 / #123 のみ dispatch）+ `synth_pitch_bend(semitones)` 独立
> - `Voice` trait に `set_pitch_bend` を追加（Mod Wheel 用 `set_mod_depth` は Phase 4 送り）
> - Sustain Pedal は Engine 状態 `sustain_active: bool` + `pending_release: u128` で管理、**Poly mode のみ** note_off 時に sustain 中なら voice の damping 加速をスキップ（実 release は sustain off で発生）。**Mono mode では Sustain を無視**し Phase 2 D29 既存挙動を完全継承（D40、Mono+Sustain は Phase 4 で再評価）
> - **Sustain 統合の重要ルール（実装で混乱しやすいため明記、03 章 §Engine と D40 が正本）**:
>   - **CC#123 (All Notes Off) は `sustain_state.reset()` も必須**: 古い pending bit が残ると CC#64 操作で再処理されるバグ
>   - **Mono mode は Sustain を完全に無視する**（Phase 3 確定方針）: 実機 Mono synth でも Sustain 挙動は機種で様々、Mono の last-note priority と release defer は本質的に相反するため、Phase 3 では Mono+Sustain は no-op に統一。`Engine::note_off` の Mono 分岐は Phase 2 既存ロジック（`prev_top` 取得 / `hold_stack.remove` / `pool.note_off` / `prev_top != new_top` ガード復帰発音）を完全継承し、`sustain_state.try_defer_note_off` を呼ばない
>   - **同一ノート再打鍵で `clear_pending(midi_note)` を呼ぶ**: C4 → Sustain on → C4 off (pending) → C4 on (re-strike) で、新打鍵分の pending bit をクリアしないと、CC#64 off で「再打鍵後にまだ離していない」状態の C4 が誤って release される。`Engine::note_on` 冒頭で必須
>   - **mode 切替時は pending を即時 release**: `Engine::set_mode` で切替前に `pending_release_bitmap()` で pending を取り出し `sustain_state.reset()` してから各 note を `pool.note_off`（Poly→Mono の境界仕様、CC#64 を後で受けても古い pending が再処理されない）
> - **Channel Volume (CC#7) は UI OutputGain と直交**: `channel_volume: SmoothedValue` を独立フィールドで保持（デフォルト 1.0、final gain = `output_gain * channel_volume`）。CC#7 で UI スライダー値を上書きしないため両者の状態が独立に保たれる（D38b）
> - `messages.ts` `ToWorkletMessage` に `midiCC` / `pitchBend` の 2 variant 追加

---

## 7. UI Voice Meter / mono–poly トグル正式化

Phase 2 retrospective §5 の負債: `__synthDev.setMode()` 経由の dev-only 露出を正式化。voice meter はモニタ用 UI として併設。

### 7.1 通信経路の比較

| 方式 | レート | 実装コスト | 評価 |
|---|---|---|---|
| Worklet → main `port.postMessage({type:'voiceState', mask, count})` を rAF 同期で送信 | 60 Hz | △ Worklet 側で 1024 サンプルごと（≈ 21ms）にスナップショット、postMessage GC 圧軽微 | **◎** |
| `SharedArrayBuffer` でロックフリー読み出し | UI 側任意 | × COOP/COEP ヘッダ必須、GitHub Pages 静的ホストで設定不可 | × |
| C ABI `synth_get_voice_state(handle, *out_count, *out_mask)` を main から polling | 60 Hz | ○ C ABI +1 関数、main → Worklet → main の round-trip | ○ |

**採用**: Worklet 側 push 方式。`process()` 終端で `frame_counter % 1024 == 0` のとき `port.postMessage({type:'voiceState', activeMask: u8, voiceAmps: [f32;8]})` を送る。21 ms 周期で十分（rAF 16.7 ms より粗いが UI 描画には足りる）。

### 7.2 UI コンポーネント

| 要素 | 表示 | 配置 |
|---|---|---|
| Voice Meter | 8 個のセル、active なら薄緑、振幅で透明度 | Header 直下 / Output Gain 横 |
| Polyphony Mode Toggle | "Mono / Poly" ラジオボタン | 同 row |
| 現在の active voice 数 | 数値（"3 / 8"） | meter 左 |

mono / poly トグルは `engine.ts.setMode('mono'|'poly')` を発火、Worklet が `synth_set_polyphony_mode` を呼ぶ既存経路を流用。

### 7.3 `synth_get_voice_state` の代替

C ABI 関数を増やさず、Worklet が `engine.pool().active_count()` と各 voice の `amplitude()` を直接取得して message に詰める方式が最小コスト。`Engine::pool()` の `doc(hidden)` 露出（Phase 2 retrospective §5）は Phase 3 で正式 API 化（`pub fn voice_state(&self) -> VoiceStateSnapshot`）。

> **§7 結論ボックス: ◎ Phase 3 採用**
>
> - 通信: Worklet push、1024 サンプルごと（21 ms 周期、stride で振幅スナップショット）
> - C ABI 拡張: `synth_voice_state_ptr(handle) -> *const u8` 1 関数（active mask + 8 振幅 = 33 bytes を共有メモリ経由で公開）、Worklet が view して message 化
> - UI: Header 直下に voice meter + mono/poly トグル
> - `__synthDev.setMode()` は dev-only のまま残す（QA 用）

---

## 8. Soft clip / Look-ahead limiter

Body Resonator + Pick position 追加で振幅構造が変化、F24 (b)（最悪ケース歪み）リスクが Phase 2 比で再燃。

### 8.1 オプション比較

| 方式 | 遅延 | 透明度 | 厳密有界 | コスト | 採否 |
|---|---|---|---|---|---|
| **区間関数型: `\|x\|≤T` で linear、超過分を rational 圧縮** | 0 sample | ◎ 安全域は誤差ゼロ | ◎ |x|→∞ で ±1.0 厳密 | 6-7 演算 | **◎ Phase 3 採用** |
| `tanh(x)` 真の `f32::tanh` | 0 sample | △ 高調波歪み（musical） | ◎ ±1 厳密 | std 必要、CPU 高 | × 依存ゼロ方針違反 |
| `tanh(x)` Padé 近似 `x(27+x²)/(27+9x²)` | 0 sample | △ 高調波歪み | × |x|→∞ で発散（x/9 漸近） | 5 演算 | × **不採用**（旧版仕様撤回） |
| `x / (1 + \|x\|)` 簡易 saturator | 0 sample | △ 緩やか | ◎ ±1 厳密 | 3 演算 | △ 安全域も非線形 |
| `x / sqrt(1 + x²)` algebraic sigmoid | 0 sample | ○ | ◎ ±1 厳密 | 1 sqrt + 1 div ≈ 8 演算 | △ 安全域で誤差 0.05 程度 |
| Look-ahead limiter (5 ms) | 240 sample @ 48 kHz | ◎ 透明 | ◎ | バッファ 960 B + peak 検出 | △ Phase 4 |
| Feedback limiter | 0 sample | ○ | △ | 3 演算 + 状態 | △ クリック発生リスク |

### 8.2 採用方式の式

区間関数型 saturator（`SOFT_CLIP_THRESHOLD = 0.95`、`SOFT_CLIP_RANGE = 0.05`）:

```text
soft_clip(x) =
  | x                                              if |x| ≤ 0.95
  | sign(x) · (0.95 + 0.05·e/(e+0.05))             if |x| > 0.95
                                                      where e = |x| − 0.95
```

特性:
- **|x| ≤ 0.95**: `soft_clip(x) ≡ x`（厳密一致、`assert_eq!` で検証可能、誤差ゼロ）
- **|x| > 0.95**: 超過分 e ∈ (0, ∞) を rational mapping で [0, 0.05) に圧縮、|x| → ∞ で出力 ±1.0 に厳密漸近
- **微分連続**: |x|=0.95 で左右ともに `dy/dx = 1`（kink なし）
- **配置**: `output_gain` 適用後 → `soft_clip` → `output_l/r` 書き込み

### 8.3 必要性判断

- Phase 3 で Modal Body は **passive filter**（DC ゲイン < 1 設計）→ 振幅を増やさない
- Pick position comb は最大 6 dB のピーク（位相反転のレア case）→ peak で 2x、要対策
- 8 voice 同時 + body resonator 共鳴ピーク連発で稀に 1.5x 振幅 → soft clip 推奨

> **§8 結論ボックス: ◎ Phase 3 採用（区間関数型）**: 安全域 (|x| ≤ 0.95) は完全 linear（誤差ゼロ）、超過分は rational mapping で ±1.0 に厳密漸近。`f32::tanh` 不使用、Padé 近似不要。実装 +0.3 KB、CPU 6-7 演算/sample。Look-ahead は Phase 4 送り。

---

## 9. Phase 3 性能予算

### 9.1 WASM サイズ予算（gzip）

Phase 2 実測 10.56 KB、Phase 3 target < 30 KB、予算余地 19.4 KB。

| 追加コンポーネント | raw | gzip |
|---|---|---|
| Modal Body Resonator (M=8 biquad + 係数テーブル 0.5 KB) | +1.5 KB | +0.7 KB |
| One-zero loss filter | +0.3 KB | +0.15 KB |
| Pick position comb (1-tap) | +0.5 KB | +0.2 KB |
| Thiran allpass (Lagrange 置換 or 並列、§4) | +0.5 KB | +0.2 KB |
| Brightness 群遅延補正 | +0.2 KB | +0.1 KB |
| MIDI CC dispatch + Pitch Bend | +1.0 KB | +0.5 KB |
| Voice Meter C ABI + 状態 export | +0.5 KB | +0.2 KB |
| Soft clip（区間関数型、`f32::tanh` 不使用、LUT 不要、stateless） | +0.3 KB | +0.15 KB |
| Sustain pedal state machine | +0.3 KB | +0.15 KB |
| **合計** | **+5.1 KB** | **+2.35 KB** |

Phase 3 後 gzip 想定: **10.56 + 2.35 ≈ 12.9 KB**（target 30 KB の 43%、**余裕大**）。

### 9.2 早期検証ポイント

実装途中で予算超過したら撤退する閾値:

| Step | 期待 gzip | 閾値（超過なら撤退） |
|---|---|---|
| §2 Modal Body 実装後 | 11.3 KB | > 13 KB なら係数テーブル削減（M=8→5） |
| §3 Extended KS 完了後 | 11.6 KB | > 14 KB なら pick position fractional 化を撤回 |
| §6 MIDI CC 完了後 | 12.5 KB | > 17 KB なら sustain pedal を Phase 4 送り |
| Phase 3 全完了 | 12.9 KB | > 20 KB なら soft clip / voice meter を Phase 4 送り |

### 9.3 CPU 予算

Phase 2 性能目標 < 1.5 ms/process（128 frames @ 48 kHz、未検証）。Phase 3 加算（per process = 128 samples）:

| 追加 | 演算数/sample | × 128 |
|---|---|---|
| Modal Body (M=8 bandpass biquad, stereo 2 ch) | +80 | +10240 |
| Loss filter per voice (× 8) | +24 | +3072 |
| Pick position（励振 shaping、process 内コスト 0、note_on のみ +length_int） | 0 | 0 |
| Thiran (Lagrange 置換、+/- 0) | 0 | 0 |
| Brightness 補正（note_on のみ） | 0 | 0 |
| Pitch Bend SmoothedValue 遷移中の係数再計算（per voice × 8、Lagrange 12 演算 / Thiran 3 演算、定常時 skip） | +96〜+24 | 平均 +30 想定、× 128 = +3840 |
| MIDI CC dispatch | 0 (event-driven) | 0 |
| Soft clip stereo（区間関数型 6-7 演算 × 2 ch） | +14 | +1792 |
| **合計** | **+148** | **+18944** |

WASM 1 GHz 仮定で +18944 / 1e9 = +0.019 ms/process。1.5 ms 予算からは +1.3%、**ほぼゼロ加算**。

stereo 化（Modal Body 左右別係数、output 経路 2 ch）で全体 +30%、それでも 1.95 ms / 2.67 ms 予算内。

### 9.4 メモリ予算

Phase 2 で `Engine::prepare` 一括確保。Phase 3 追加分:

| 追加バッファ | サイズ |
|---|---|
| Modal Body 状態 (M=8 × 2 状態 × stereo 2 ch) | 32 × f32 = 128 B |
| Modal Body 係数テーブル (M=8 × 3 係数 × stereo 2) | 48 × f32 = 192 B |
| Pick position（励振 shaping、`KarplusStrong::note_on` 内で既存 buffer 流用） | 0 B（追加なし） |
| Voice State snapshot for UI | 33 B |
| JS 側スクラッチ Float32Array (8 voices) | 32 B（synth-processor.ts constructor で事前確保） |
| Soft clip（区間関数、stateless で LUT 不要） | 0 B |
| **合計** | **+0.4 KB** |

`memory.buffer.byteLength` 不変条件は維持可能。`Engine::prepare` の引数 `max_block_size` から計算して 1 度確保。

---

## 10. 実装着手前に答えを出すべき問い

Phase 3 仕様書（01〜07）を策定する前に、以下を確定する。本書 §2–§9 は方針確定の根拠を提供しているが、**最終判断はユーザー承認**が必要:

1. **Body Resonator の係数セット決定**: §2.3 のギターボディ 8 モード値を初期値とするか、別の楽器（クラシックギター / ウクレレ / マンドリン）も用意するか
2. **モード数 M の最終確定**: 5 / 8 / 10 のうちどれか（§9.2 の予算 vs §2 の音質トレードオフ）
3. **Thiran 採否**: §4.4 の試作結果で案 A / B / C / D を確定（試作は仕様書策定後の Step 1 候補）
4. **Brightness 補正の採否**: 0.89% 偏移は実機聴感で気付かれるか（Phase 2 F1〜F9 と並行検証推奨）
5. **Pitch Bend の SmoothedValue 時定数**: 5 ms（fast modulation 重視）vs 20 ms（クリック完全防止）
6. **Voice Meter の更新レート**: 30 Hz（512 サンプル）/ 60 Hz（1024 サンプル）/ 120 Hz（2048 サンプル）
7. **Soft clip threshold**: 0.95 を採用（区間関数型、`SOFT_CLIP_THRESHOLD` 定数として確定）。Body Resonator のピーク振幅実測で 0.98 への引き上げも検討
8. **C ABI 新規関数の追加順**: `check-wasm-exports.mjs` REQUIRED 配列の更新を Step 単位で commit する規約
9. **mono / poly トグルのデフォルト状態**: Phase 2 と同じ poly がデフォルト、トグル位置は Header 内 / Footer / Side panel のどこか

---

## 11. 引き続き有効な Phase 1 / 2 文献 + Phase 3 新規参照

### 11.1 継続参照

- Smith J.O. *Physical Audio Signal Processing* CCRMA — Body Modes 章 / Loss Filter 章 / Allpass Interpolation 章
- Karjalainen, Välimäki, Tolonen (1998) "Plucked-String Models: From the Karplus–Strong Algorithm to Digital Waveguides and Beyond" — Extended KS 全体像、Body filter の標準化
- STK `BiQuad` / `Plucked` / `DelayA` — 参考実装
- Faust `physmodels.lib` の `body` / `allpass` / `dispersion` — DSL 実装

### 11.2 Phase 3 で新規参照

- Penttinen, Karjalainen, Härmä (2006) "New Techniques for Real-Time Physical Modeling Sound Synthesis" — Modal body resonator の実時間実装ガイドライン
- Smith J.O. "Virtual Acoustic Musical Instruments: Review and Update" J. New Music Research (2004) — Modal vs IR convolution の定性比較
- Smith J.O. *Introduction to Digital Filters* CCRMA — biquad 設計 / 群遅延補正
- Välimäki (2004) "Discrete-Time Synthesis of the Sawtooth Waveform with Reduced Aliasing" — soft clip / oversampling の周辺
- Bilbao S. *Numerical Sound Synthesis* (2009) — 弦の inharmonicity / dispersive all-pass の数学的背景（§3.3 stretching all-pass で Phase 4 参照予定）
- Web MIDI API W3C Recommendation (2015) — CC イベント仕様、pitch bend 14-bit 構造

### 11.3 参考実装の追加

- **JUCE `IIRFilter` / `dsp::Convolution`** — Modal / IR 両方のリファレンス
- **soundpipe `voc` / `streson`** — 軽量 modal body の C 実装

---

## 12. Phase 3 で参照しない領域（Phase 4–5 送り）

| 領域 | 理由 |
|---|---|
| WASM SIMD (`target-feature=+simd128`) | Phase 2 retrospective §7 同様、音作り優先 |
| プリセット保存・ロード | localStorage / IndexedDB / OPFS の選択が独立スコープ、Modal 係数を保存する preset 構造が前提なので Phase 3 完了後 |
| ピアノ音色（Stretching all-pass + impact model） | §3.3 結論で Phase 4 送り |
| 管楽器（reed model / digital waveguide tube） | Phase 5 |
| 打楽器（FDTD membrane / mass-spring） | Phase 5 |
| `KarplusStrong::note_on` の buffer ゼロクリア最適化 | Phase 2 retrospective §5、計測してから判断、Phase 3 では着手せず |
| Look-ahead limiter | §8 結論で Phase 4 送り（soft clip で十分なら不要） |
| 複数楽器同時鳴動（multi-timbral） | プリセット先決定後 |
| 録音・MIDI export | Phase 5 |

---

## 13. Phase 3 実装順序の試案（07 章への種）

本書の結論を統合した実装順:

1. **Step 1**: Thiran allpass 試作 + cargo test 評価（§4.4）→ 採否決定
2. **Step 2**: params.json + gen-params.mjs 拡張（§2.4、Modal 係数 + applyStereoSpread 純粋関数）
3. **Step 3**: Modal Body Resonator 実装（bandpass biquad）+ Engine 統合（§2.3）
4. **Step 4**: Loss filter（§3.1）
5. **Step 5**: Pick position 励振 shaping（§3.2、`KarplusStrong::note_on` 内、専用モジュールなし）
6. **Step 6**: Brightness 群遅延補正（§5）
7. **Step 7**: Soft clip 区間関数（§8）
8. **Step 8**: Voice trait 拡張 + Pitch Bend（§6.4、ring buffer `% buf_len` 不変条件維持）
9. **Step 9**: MIDI CC dispatch (CC#7/#64/#123) + Sustain（§6、Mod Wheel は Phase 4 送り）
10. **Step 10**: C ABI 3 関数追加 + Voice State export
11. **Step 11**: messages.ts / WasmExports / SynthEngine（事前確保スクラッチ）
12. **Step 12**: VoiceMeter / PolyphonyToggle / midi-cc.ts UI（§7）
13. **Step 13**: 統合 cargo test + **release timing test 必須**（F37）+ サイズ計測（§9.2 早期検証）
14. **Step 14**: ドキュメント / spec D-tag 整理 / retrospective 準備

各 Step は仕様書 07 章で `cargo test` / 実機検証の達成ラインを明示する（Phase 1 / 2 と同じ流儀）。

---

## まとめ（1 行）

> Phase 3 は「**Modal Body Resonator (M=8 bandpass biquad、stereo)** を Engine 後段に配置 + **Extended KS 2 拡張**（loss filter は process 内、pick position は note_on 励振 shaping）を KarplusStrong 内に追加 + **Thiran allpass で C8 自己発振を救済（Step 1 試作評価後決定）** + **MIDI CC (CC#7/#64/#123) / Pitch Bend / Sustain** で UX を整え（Mod Wheel は Phase 4 送り）+ **Voice Meter UI** と **区間関数型 soft clip** で仕上げ」。WASM gzip 12.5 KB / target 30 KB の 42%、CPU +1.3% で予算内、F37 release timing test を Step 13 で必須化。
