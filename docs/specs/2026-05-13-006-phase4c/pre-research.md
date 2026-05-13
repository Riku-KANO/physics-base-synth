# Phase 4c 調査資料

## 「本格ピアノ音色 (Multi-string + Hertz hammer + Sympathetic resonance) / WASM SIMD / C8 ピッチ自己発振 / 補助負債整理」のための前提整理

本書は Phase 4c 仕様策定で参照する追加調査トピックを集約する。Phase 1〜Phase 4b で既に決着した基礎理論（Karplus–Strong、Lagrange / Thiran 補間、Modal Body M=8 並列 biquad、Loss filter ρ_base=0.05、Pick position 励振 shaping、ParamDescriptor 自動生成、SmoothedValue、SustainState、SoftClip、VoiceState 通信、グローバル LFO + Mod Wheel、Preset JSON v1、Factory Preset 8 種、`wasm-opt -O3`、`excitation_snapshot` cfg(test)、`#![allow(clippy::approx_constant)]` + `#[rustfmt::skip]` の生成器パターン、**Stretching all-pass cascade M=8 + Rauhala-Välimäki closed-form**、**Commuted impulse + velocity-dependent LPF (Hammer model)**、**Piano 用 Modal Body 係数 Conklin 1996 ベース**、**`__synthDev.measureProcessTime` (dev-only F38b 計測)**、**`.gitattributes` LF 統一**、**`tests/fixtures/phase4a_default_c4_v08.rs` byte 一致パターン**）は重複させず、各 pre-research の該当節を参照する。

Phase 4c は **方針選定の重みが Phase 4b より高い**。Phase 4b の主目的（ピアノ音色）は実装段階で「cargo / clippy / 互換性テストは全 green だが聴感は弦楽器寄り」という結果に終わり、KS ベースの構造的限界が顕在化した（retrospective §5 / §7）。Phase 4c の最大課題は **「ピアノを徹底追求して構造拡張するか、別軸 (SIMD / C8 自己発振) で完成度を上げるか」の軸足選定** であり、本書はこの選択の根拠を提示することに比重を置く。

各章末に **結論ボックス（◎採用 / ○検討 / △Phase 4d 以降送り / ×不採用）** を置く（Phase 3 / 4a / 4b と同形式）。

---

## 0. Phase 1 / 2 / 3 / 4a / 4b pre-research との関係

Phase 4c は以下の節を **既存 pre-research を一次資料**として参照する。

| Phase 節 | 内容 | Phase 4c での参照箇所 |
|---|---|---|
| Phase 1 [§3.1 Karplus–Strong](../2026-05-06-001-mvp/pre-research.md) | 基本原理 (delay + LPF feedback) | §3 Multi-string 並列 KS の挿入点、§9 C8 自己発振 |
| Phase 2 [§3 Lagrange / Thiran](../2026-05-07-002-phase2/pre-research.md) | 補間 allpass の安定性 | §3 Multi-string detuning の精度要件、§8.1 Pick fractional 化 |
| Phase 3 [§2.3 Modal Body 係数](../2026-05-07-003-phase3/pre-research.md) | 8 モード並列 biquad | §7 Soundboard Modal Body M=16 への拡張 |
| Phase 3 [§5 Brightness 群遅延補正](../2026-05-07-003-phase3/pre-research.md) | LPF 由来の τ_g 補正 | §3 / §6 多弦・B(note) 関数化での length 補正の整合 |
| Phase 3 [D36](../../retrospective/2026-05-07-003-phase3.md) | Thiran allpass 案 D 採用 | §3 多弦時の Thiran 同時実装の共有 |
| Phase 3 retrospective [§7.2 Phase 4 候補 #2](../../retrospective/2026-05-07-003-phase3.md) | C8 ピッチ自己発振 | §9 で再評価 |
| Phase 4a [§7 多楽器プリセット](../2026-05-08-004-phase4a/pre-research.md) | 8 種 (Phase 4b で 8 種完備) の Modal 係数手法 | §7 Piano kind の M=16 拡張、§5 Sympathetic で全楽器共通基盤の検討 |
| Phase 4a [§3 LFO / Mod Wheel](../2026-05-08-004-phase4a/pre-research.md) | LFO 波形 / destinations | §8.3 LFO 波形拡張、§8.4 LFO destinations 拡張 |
| Phase 4a retrospective [§5 既存負債 / §7 推奨スコープ](../../retrospective/2026-05-08-004-phase4a.md) | 多項目の Phase 4 候補 | §1 でリプリント |
| Phase 4b [§4 Dispersion cascade](../2026-05-09-005-phase4b/pre-research.md) | M=8 Rauhala-Välimäki closed-form | §3 Multi-string 各弦への dispersion 共有 / 個別化、§6 B(note) 関数化での a1 再計算 |
| Phase 4b [§6 Hammer model](../2026-05-09-005-phase4b/pre-research.md) | Commuted impulse + velocity LPF | §4 Hertz law spring への差し替え検討 |
| Phase 4b [§7 Piano Modal Body](../2026-05-09-005-phase4b/pre-research.md) | Conklin 1996 文献値 (mode1=55Hz) | §7 M=16 拡張時の係数追加方針 |
| Phase 4b [§10.5 WASM SIMD 評価](../2026-05-09-005-phase4b/pre-research.md) | Phase 4c 以降に送り済 | §10 で再評価、PoC 含む採否判断 |
| Phase 4b retrospective [§5 / §7](../../retrospective/2026-05-09-005-phase4b.md) | Piano 音色が「弦楽器寄り」、F38b 実機計測未取得、CRLF warning | §1 / §2 で深掘り、§11 で必須前提化 |

---

## 1. Phase 4c スコープと前提制約

### 1.1 Phase 4b retrospective §7 の候補一覧（重要度順）

retrospective §7 で列挙された Phase 4c 候補を本書では下表で扱う。**Phase 4b は「主目的 1 件 + 補助複数」の二段構成だったが、Phase 4c は主目的候補が 3 系統に分岐するため、まず §2 で軸足を確定する**。

| 候補 (retrospective §7) | 重要度 | Phase 4c 採否（暫定、§2 で確定） |
|---|---|---|
| 本格ピアノ音色 (Multi-string + Hertz hammer + Sympathetic resonance + Modal M=16 + B(note)) | 1 | **◎ 主目的の最有力**（§2〜§7 で詳述） |
| C8 ピッチ自己発振 | 2 | ○ 主目的次点（§9 で詳述、ピアノとは別軸） |
| WASM SIMD (`target-feature=+simd128`) | 3 | △ Phase 4d 送り推奨（§10 で詳述、CPU 余裕 36× で費用対効果薄） |
| Pick position fractional 化 | 4 | △ 補助候補（§8.1） |
| Look-ahead limiter (5 ms 遅延) | 5 | △ 補助候補（§8.2） |
| LFO 波形拡張 (S&H / Square / Sawtooth) | 6 | △ 補助候補（§8.3） |
| LFO destinations 拡張 (Pick / Damping / BodyWet) | 7 | △ 補助候補（§8.4） |
| 楽器切替の fade-out / cross-fade (`PendingInstrumentChange` 状態機械) | 8 | △ 補助候補（§8.5、Phase 4b D63 改訂の引き継ぎ） |
| Cross-tab preset 同期 (`storage` event) | 9 | △ Phase 4d 送り（§8.6） |
| Preset JSON file import / export | 10 | △ Phase 4d 送り（§8.7） |
| Mono + Sustain 本実装 | 11 | × Phase 5 送り（Phase 2 D29 / Phase 3 D40 / Phase 4a D55 / Phase 4b 継承） |
| Piano プリセット聴感チューニング | 12 | ○ 本格ピアノを主目的にする場合は §4〜§7 と一体、別軸を主目的にする場合は単独で着手可 |
| F38b 実機計測の CI / E2E 自動化 (Playwright + Console API) | — | △ Phase 4d 送り推奨（§8.8、現状 dev-only API は完成済） |
| Inharmonicity B(note) 関数化 | — | ◎ 本格ピアノ採用時の核心（§6 で独立節） |
| Soundboard / lid の高次モード (M=16) | — | ○ 本格ピアノ採用時の補強（§7 で詳述） |
| 複数 Piano 機種プリセット (Grand / Upright / Honkytonk) | — | △ 本格ピアノ完成後の Phase 4d 以降 |

### 1.2 制約（Phase 1〜4b 継承、Phase 4c でも維持）

- **WASM gzip < 25 KB（警戒 25 KB / 撤退 30 KB）**: Phase 4b 実測 18.71 KB → Phase 4c 主目的次第:
  - 本格ピアノ採用: target < 23 KB（Multi-string の係数 / Sympathetic bus / B(note) LUT / Modal M=16 で +3〜4 KB 想定）
  - C8 自己発振採用: target < 19.5 KB（damping=1.0 経路の追加コードは小）
  - SIMD 単独採用: target < 20 KB（intrinsics 化でコードサイズ微減〜微増）
- **依存ゼロ**: `dsp-core` / `wasm-audio` で外部 crate を追加しない。Hertz law ODE も自前、Sympathetic bus も自前。
- **`Engine::prepare` 以外でヒープ確保禁止**: Multi-string 化で 1 voice = 3 弦になっても全状態を `KarplusStrong::prepare` で固定確保（heap free path 維持）。
- **C ABI のみ**: `wasm-bindgen` 不使用、`#[unsafe(no_mangle)] extern "C"` を継続。Phase 4c で新規関数追加が必要な場合は最大 2〜3 個に絞る（`synth_set_sympathetic_amount` / `synth_set_unison_detune` 等が候補）。
- **Float32Array view キャッシュ**: Worklet 側の原則維持。
- **Svelte 5 runes**: `$state` / `$derived` / `$effect`、`.svelte.ts` 拡張子。
- **Auto-generated コード**: `gen-params.mjs` 出力に `#![allow(clippy::approx_constant)]` (module) + `#[rustfmt::skip]` (item)。
- **`tests/fixtures/phase4a_default_c4_v08.rs` バイト一致**: Phase 4a HEAD (commit dfa81c3) との ε=1e-6 互換を維持。Multi-string 化で Piano 経路は変わるが、**Default kind 経路は Phase 4b と引き続き byte 一致**を保証する設計が必要（§3.6 で詳述）。

### 1.3 本書の確定責任

Phase 4c 着手前に以下を本書で確定させる（仕様書 01〜07 へ橋渡し）:

1. §2 で **Phase 4c 主目的の軸足** (本格ピアノ / C8 自己発振 / SIMD / その他) を提示し §12 でユーザー承認
2. §3 で **Multi-string per voice の設計** (弦数 / detuning 量 / 結合方式 / Voice trait 拡張)
3. §4 で **Hertz law non-linear hammer の方式** (per-sample ODE / pre-rendered LUT / 現状維持)
4. §5 で **Sympathetic resonance の Engine 構造** (global resonance bus / per-voice cross-coupling / 不採用)
5. §6 で **Inharmonicity B(note) 関数化の表現** (LUT / 関数式 / 現状維持)
6. §7 で **Modal Body M=16 拡張の採否**
7. §8 で **補助候補 (Pick / LFO / Fade-out 等) の Phase 4c 採否**
8. §9 で **C8 自己発振の方式と Phase 4c での扱い**
9. §10 で **WASM SIMD の Phase 4c での扱い** (PoC で先行計測する判断含む)
10. §11 で **性能予算** (WASM gzip / CPU / メモリ)

§12 の「実装着手前に答えを出すべき問い」は仕様書策定時に順次決める。

---

## 2. Phase 4b 振り返り → Phase 4c 主目的選定の軸

### 2.1 Phase 4b が音響面で残した問題

Phase 4b retrospective §5 / §7 で確認された通り、Piano 音色は **cargo / clippy / 互換性テスト全 green** にもかかわらず **聴感は「ピアノっぽい弦楽器」レベル** に留まった。原因は KS ベースの構造的限界に集約される（retrospective §5 §pre-research）:

| 構造的限界 | 物理的根拠 | Phase 4b 現状 | Phase 4c 解決候補 |
|---|---|---|---|
| **KS ループ自体が弦のシミュレーション** | 1 voice = 1 遅延線 | inharmonicity を Stretching all-pass で加えても「弦らしさ」は残る | Multi-string + Hertz hammer + Sympathetic で構造拡張（§3〜§5） |
| **Hammer model が線形近似** | 1pole LPF + impulse | フェルトの Hertz law `F = K·x^p` (p≈2.5) や打撃 transient を再現せず | Hertz law spring (Boutillon 1988 / Stulov 1995) (§4) |
| **複数弦 (1〜3 弦/note) の unison detuning なし** | 1 voice = 1 弦 | beating + two-stage decay が生じない | Multi-string (Weinreich 1977) (§3) |
| **Sympathetic resonance なし** | Sustain ペダル時の他弦共鳴を未表現 | ペダル音の余韻が出ない | Global resonance bus (Bank 2000) (§5) |
| **Modal Body M=8 は粗い近似** | 実物響板は数百モード | 構造ピーク 1〜2 個しか再現できない | M=16 拡張または shelf 補強 (§7) |
| **Inharmonicity B が固定値** | 実機は bass 大 → treble 小 | A4 基準 7.5e-4 + Ikey(f0) 補正は鍵盤位置補正のみ、B 自体が note 依存 | B(note) 関数化 / LUT (§6) |

### 2.2 主目的候補の比較

retrospective §7 で挙げられた 3 系統について、**実装規模 / Phase 4b 負債解消の度合い / 学習機会** で比較:

| 候補 | 実装規模 (Step 数) | Phase 4b の聴感負債解消 | CPU 影響 | WASM サイズ影響 | リスク | 学習価値 |
|---|---|---|---|---|---|---|
| **A. 本格ピアノ (Multi-string + Hertz hammer + Sympathetic + B(note) + Modal M=16)** | 25-30 | ◎ 直接解消 (Phase 4b の最大負債への正面攻撃) | +0.3〜0.5 ms (Piano、target 1.7 ms 内) | +3〜4 KB gzip | ◎ 音響的に成功確率高、構造拡張で他楽器にも波及効果 | DSP 物理モデルの学習に最良 |
| **B. C8 自己発振** | 8-12 | × 解消しない (別軸、ピアノ問題はそのまま) | +0.01 ms 程度 | +0.5 KB gzip | △ Phase 1 から持ち越し、damping=1.0 の数値発散リスク | DSP / 数値安定性の学習 |
| **C. WASM SIMD** | 12-15 | × 解消しない (CPU 最適化のみ、聴感不変) | -0.01〜-0.02 ms (節約) | ±1 KB gzip | × Phase 4a byte 一致テスト崩壊リスク、保守性悪化 | WASM intrinsics の学習 |

**Phase 4b retrospective §6 で確立した方針**: 「仕様書ドリブン開発で cargo / clippy / 互換性テスト全 green でも、聴感確認で違和感が出ることがある (Phase 4b の Piano)。**実機聴感確認を仕様書策定時の必須項目に組み込む候補**」と明示。Phase 4c では聴感問題が最大の負債である以上、A. 本格ピアノを主目的に据えるのが筋。

### 2.3 軸足の暫定推奨

**Phase 4c 主目的の暫定推奨: A. 本格ピアノ**（§12 でユーザー承認）。理由:

1. **Phase 4b の最大負債（聴感）を正面から解消できる唯一の候補**
2. **Multi-string + Sympathetic + B(note) は文献的に確立した標準手法**（Weinreich 1977 / Bank 2000 等、§3 / §5 / §6）で実装難易度は高くないが規模は大きい
3. **構造拡張なので他楽器 (Guitar / Bass 等) にも波及効果**（Multi-string は 12 弦ギターやマンドリンへの応用余地、Sympathetic は Sitar の共鳴弦への応用余地）
4. **CPU 余裕 36× / WASM サイズ余裕 11 KB** から、最大規模の構造拡張でも予算超過リスクは低い
5. **C8 自己発振 (B) は規模小だが Phase 4b 負債を解消せず、Phase 4d で単独着手可能** (§9 で詳細)
6. **WASM SIMD (C) は CPU 余裕が大きい現状では費用対効果が薄く、本格ピアノで CPU 予算が逼迫してから取り組む順番** (§10 で詳細)

### 2.4 補助スコープ案

Phase 4c 主目的を「本格ピアノ」に据えた場合、補助スコープは以下を推奨:

- **B(note) 関数化（§6）**: 本格ピアノの一部として必須
- **Modal Body M=16（§7）**: Phase 4c 規模次第で採用、§7 で具体的に詳述
- **Piano プリセット聴感チューニング（§8.9）**: 本格ピアノ実装後の最終調整として必須
- **F38b 実機計測値の取得**: Phase 4b 持ち越し、Step 16 相当でユーザー操作必須
- **Phase 4a HEAD byte 一致テストの維持**: Default 経路は Phase 4b と完全同型、Piano 経路は Phase 4c golden 再生成

その他の補助候補（Pick fractional / Look-ahead / LFO 拡張 / Fade-out / Cross-tab / Preset import-export）は **Phase 4c では原則 △ Phase 4d 送り**。本格ピアノの実装規模が 25-30 Step に達するため、補助を抱き合わせるとイテレーション粒度が肥大化する。

> **§2 結論ボックス: ◎ 採用（暫定、§12 でユーザー承認）**
>
> - **Phase 4c 主目的 = A. 本格ピアノ (Multi-string + Hertz hammer + Sympathetic resonance + B(note) + Modal M=16)**
> - **補助スコープ = B(note) 関数化 + Modal M=16 + Piano プリセット聴感チューニング + F38b 実機計測 + Phase 4a byte 一致テスト維持**
> - **B. C8 自己発振 / C. WASM SIMD は Phase 4d 以降に送り**（§9 / §10 で根拠詳述）
> - **その他補助候補は原則 △ Phase 4d 送り**（§8 で個別に判断）

---

## 3. Multi-string per voice (Phase 4c 音響面の最大決断 #1)

### 3.1 物理的位置付けと既存実装との関係

実ピアノは MIDI ノートに応じて **1 弦 (A0〜A1)、2 弦 (A1〜B2)、3 弦 (C3 以上)** の unison group で構成される。3 弦は ±0.5〜3 cents の微小デチューンを持ち、これが以下のピアノらしさの根幹を生む:

1. **Beating (うなり)**: 3 弦の微妙な周波数差で振幅変調が生じ「響き」が出る
2. **Two-stage decay**: 打鍵直後はハンマー入力方向（垂直モード）が主、その後 bridge 経由で水平モードへエネルギー移行。水平モードは bridge 減衰が弱く長い "aftersound" を生む（Weinreich 1977）
3. **打鍵感**: 3 弦の干渉で transient のスペクトルが厚くなる

Phase 4b 時点では 1 voice = 1 弦のため、これらすべてが欠落している（retrospective §5）。

| 既存 (Phase 1〜4b) | Phase 4c 拡張 |
|---|---|
| 1 voice = 1 KS 遅延線 | 1 Piano voice = N 弦 (1/2/3) の並列 KS + 加算 |
| `KarplusStrong { buffer, dispersion_stages, thiran, brightness_lpf, loss_filter, damping }` | 上記を 3 本に複製 + unison detuning パラメータ |
| `note_on` で 1 つの基底周波数を計算 | 弦ごとに detune (±cents) を適用、各 KS の `adjusted_length` を弦別に算出 |
| `process_sample` で 1 値を返す | 3 弦の出力を加算 (mixing) で 1 値に集約 |

### 3.2 弦数の選定（鍵盤位置依存）

Steinway D / 一般的グランドピアノの設計に倣う:

| MIDI ノート範囲 | f_0 範囲 | 弦数 | 由来 |
|---|---|---|---|
| A0〜A1 (21〜33) | 27.5〜110 Hz | 1 | 低音域、bass strings (巻線) |
| A#1〜B2 (34〜47) | 110〜247 Hz | 2 | 中低音域 |
| C3〜C8 (48〜108) | 261〜4186 Hz | 3 | 中音〜高音域 |

Phase 4c では **`n_strings(midi: u8) -> u8` 関数で MIDI ノートに応じて 1/2/3 を返す**設計とする。これにより低音は 1 弦のみ KS が走り、高音は 3 弦並列となる（CPU は note 依存で変動）。

### 3.3 DSP 実装方式の選定（3 案）

文献 (Weinreich 1977 / Smith CCRMA / Bank 2000-2003 / Aramaki 2001) で確立した 3 案:

| 案 | 物理忠実度 | 実装複雑度 | CPU | Phase 4c 採否 |
|---|---|---|---|---|
| A: **並列 KS N 本 + 単純加算 + unison detuning (±1〜3 cents)** | ○ 中（橋結合は無視するが detune 干渉で beating と two-stage decay の代理が得られる、Smith も推奨） | ◎ 低（既存 KS をそのまま複製、Engine 構造変更最小） | △ KS×N (3 弦で 3 倍、ただし 8 voice × 3 弦 = 24 KS は target 1.7 ms 内に収まる) | **◎ Phase 4c 採用** |
| B: **A + bridge mixing scalar coupling (各 KS 出力を mixing scalar g≈0.05 で他 N-1 弦のループ入力に足し戻す)** | ◎ 高（Weinreich の物理に近い、beat + sustain 強化、Bank 2000 系の標準） | ○ 中（feedback 6 経路、安定性チェック必須） | △ +10〜20% CPU on top of 案 A | ○ Phase 4c 後段で検討（**まず案 A、CPU 余裕があれば案 B へ拡張**） |
| C: **Smith の transmission-matrix 橋モデル (完全 waveguide)** | ◎ 最高 | × 高（1 voice = N 弦 + bridge 状態、Voice trait 大改造） | × 高（per-sample で行列演算） | × Phase 4d 以降（Phase 4c スコープ超過） |

**Phase 4c 採用案**: **案 A から開始、Step 14 周辺で CPU / 聴感確認後に案 B 追加を判断**。

### 3.4 Unison detuning 量と分布

Weinreich 1977 / Galembo-Askenfelt の実測値:
- 典型: ±0.5〜2 cents（音楽用調律師が「キレイに合わせた」状態でも完全ゼロにはしない）
- 一般的: 中央弦が基底、左右の弦が ±1.5 cents 程度
- 過度: 5 cents 以上は「honky-tonk」風

Phase 4c 採用値（Piano プリセット内に保持）:

```
unison_detune_cents = 1.5  // 中央以外の弦が ±1.5 cents
unison_detune_curve = "fixed"  // 全鍵盤共通 (将来 "bass_heavy" / "treble_heavy" 等を追加可)
```

3 弦の場合:
- 弦 0 (中央): f_0
- 弦 1 (左): f_0 × 2^(-1.5/1200) ≈ f_0 × 0.99913
- 弦 2 (右): f_0 × 2^(+1.5/1200) ≈ f_0 × 1.00087

2 弦の場合: 弦 0 が基底、弦 1 が +1.5 cents（中央なし）。

### 3.5 Voice / VoicePool 設計の選択（2 案）

| 案 | 概要 | 利点 | 欠点 |
|---|---|---|---|
| α: **`KarplusStrong` 構造体内に `n_strings: usize` + `[StringState; 3]` を持たせる** | 既存 `KarplusStrong` を多弦化、`process_sample` で N 個の弦を集計して返す | Engine / VoicePool への変更ゼロ、Voice trait 維持 | 非ピアノ楽器でも構造体サイズ +400 byte 程度（StringState × 3 のメモリ常駐） |
| β: **`MultiStringPiano` 新 struct + `KarplusStrong` を内部 hold** | Piano kind 専用の Voice 実装を別 struct で作る | 非ピアノ楽器のメモリは不変 | Voice trait の dyn 化 or enum 分岐が必要、VoicePool 設計大改造 |

**Phase 4c 採用案**: **α**。

理由:
- Voice trait の dyn 化は Phase 4b までの polymorphism 設計 (静的 enum) を壊す
- VoicePool は固定型 `[KarplusStrong; 8]` で alloc ゼロを実現しており、β は heap allocation を発生させるリスク
- 非ピアノ楽器でも StringState × 3 のメモリ常駐は +400 byte 程度 (8 voice で +3.2 KB)、Phase 4b の 0.78 KB 増加と比べても許容範囲
- 案 α なら `n_strings = 1` のとき既存 Phase 4b と完全に同等の挙動（弦 1 本だけ走る）→ Default / Guitar 系の Phase 4a 互換性が自然に保てる

### 3.6 構造変更案（案 α + 案 A）

```rust
const MAX_STRINGS_PER_VOICE: usize = 3;

#[derive(Debug, Clone, Copy)]
struct StringState {
    /// 弦個別のディレイラインオフセット (KS buffer は共有しても良いが、
    /// 弦ごとに異なる length を保持するため write/read 位置を別管理)
    write_idx: usize,
    /// この弦の detune 後の length (fractional)
    length: f32,
    /// 弦個別の Thiran allpass 状態
    thiran: ThiranCoeffs,
    /// 弦個別の dispersion cascade (Piano のみ active)
    dispersion_stages: [DispersionStage; 8],
}

pub struct KarplusStrong {
    // 既存 fields (buffer は弦ごとに別バッファ持つ案 / 共有して offset で分ける案がある)
    buffer: [Vec<f32>; MAX_STRINGS_PER_VOICE],  // 案 1: 弦ごとに独立 buffer
    string_states: [StringState; MAX_STRINGS_PER_VOICE],
    n_strings_active: usize,  // 1, 2, or 3 (note_on 時に確定)

    // Phase 4b 既存
    brightness_lpf: BrightnessLpf,  // 全弦共通の出力側 LPF
    loss_filter: LossFilter,        // 全弦共通
    damping: SmoothedValue,
    sample_rate: f32,
    rng: XorShift32,
    dispersion_active: bool,
}
```

**メモリ計算**:
- `Vec<f32>` × 3 弦 × 8 voice = 24 個のバッファ
- 各 buffer = ceil(sample_rate / 27.5) + 1 ≈ 1746 sample (48 kHz)
- 24 × 1746 × 4 byte = **167.6 KB**（Phase 4b の 8 × 1746 × 4 = 55.9 KB から +111.7 KB）
- WASM memory 64 MB を考えれば誤差レベルだが、cache 圧迫の懸念あり

**メモリ削減案**:
- 弦ごとの length 差は最大 ±3 cents ≈ ±0.17% なので、**buffer は 1 voice で 1 本共有**し、3 弦は同じ buffer の異なる読み出し位置 (read_z_string_0, read_z_string_1, read_z_string_2) で分散実現可能
- ただし write は中央弦 (string 0) のみ、左右弦は中央弦の write 値を異なる group delay 経由で読むだけ
- → 案 1 (独立 buffer) で確実にシミュレーション、案 2 (共有 buffer + 3 read 位置) で memory 節約

Phase 4c 暫定採用: **案 1 (独立 buffer)**。理由:
- 各弦に独自の dispersion / damping / loss を加える余地（Sympathetic resonance や two-stage decay）が残る
- Phase 4b までの「1 voice = 1 KS」の単純さに最も近く、実装ミスのリスク低
- +111.7 KB は WASM memory growth で吸収可（Phase 4a の prepare 一括確保戦略を継承）

§11.3 で再計測、もし WASM memory が CI 環境で問題になれば案 2 へ移行。

### 3.7 Phase 4a / 4b 互換性の保証

**`n_strings = 1` のとき Phase 4b と byte 一致**を機械保証する設計が核:

- Default / Guitar 系（dispersion_active=false）: `n_strings = 1`、`unison_detune_cents = 0` → Phase 4a 経路と byte 一致
- Piano（dispersion_active=true）: `n_strings = 3`、`unison_detune_cents = 1.5` → Phase 4b 経路とは異なる音色（聴感確認で評価）

互換性テスト:
- `test_default_n_strings_1_matches_phase4a`: Default kind で 256 frame × 2ch の出力が Phase 4a fixture と ε=1e-6 byte 一致（Phase 4b と同じ fixture）
- `test_piano_n_strings_3_diverges_from_phase4b`: Piano kind で出力が Phase 4b と意図的に異なることを確認（負のテスト）

### 3.8 各弦への Dispersion / Brightness / Damping の共有 vs 個別化

| 要素 | 共有 / 個別 | 理由 |
|---|---|---|
| Dispersion cascade | **弦ごと個別** | 各弦の f_0 が detune で異なるため、`compute_dispersion_a1(B, f_0_string)` が弦別 |
| Brightness LPF | **全弦共有 (出力側 1 段)** | ピアノの brightness は音色全体の高域形成、弦ごとに分ける意味薄い |
| Loss filter (rho_base) | **全弦共通** | 弦個別にできるが Phase 4c では複雑度回避 |
| Damping | **全弦共通の SmoothedValue** | 同上 |
| Thiran allpass | **弦ごと個別** | 各弦の fractional delay 差を表現 |
| Modal Body | **全弦共有 (出力側、L/R 各 8 mode)** | bridge / soundboard は弦の集合に対応 |

### 3.9 CPU コスト見積

Phase 4b Piano: 0.047 ms / 128 frames (8 voice × 1 弦)

Phase 4c Piano (Multi-string 案 A、平均 2 弦/voice 仮定):
- KS ループ ×2 平均 + dispersion ×2 + Thiran ×2: +0.02〜0.03 ms
- 案 B (bridge coupling) を追加: +0.005〜0.01 ms
- 合計: **0.06〜0.08 ms (Piano、target 1.7 ms の 4〜5%、余裕 20×)**

8 voice × 3 弦すべて活性化の最悪ケース (8 鍵 simultaneously, all 3 弦): **+0.04 ms 程度**、target 内に十分収まる。

### 3.10 数値安定性

- 各弦の `adjusted_length` は detune で ±0.17% 程度しか変わらないため、Thiran / dispersion の係数式の数値安定性は Phase 4b と同等
- 3 弦の単純加算は overflow しないか: f32 range で問題なし（各 KS 出力は normalize 済）
- bridge coupling (案 B) の feedback gain g は < 0.1 に clamp、ループ安定性を確保

> **§3 結論ボックス: ◎ Phase 4c 採用（暫定、§12 でユーザー承認）**
>
> - **方式**: 並列 KS N 本 + 単純加算 + unison detuning（案 A）。Step 14 周辺で案 B (bridge coupling) を検討
> - **弦数**: A0〜A1 で 1 弦、A#1〜B2 で 2 弦、C3〜C8 で 3 弦（`n_strings(midi)` 関数）
> - **Detune 量**: 中央弦 ±0、左右弦 ±1.5 cents 固定（Piano プリセット内パラメータ `unison_detune_cents`）
> - **Voice 設計**: 案 α（`KarplusStrong` 内に `[StringState; 3]` + `n_strings_active: usize`）。非ピアノは `n_strings = 1` で Phase 4b と byte 一致
> - **Buffer**: 弦ごとに独立 Vec<f32>（+111.7 KB memory）。CI で問題出れば案 2 (共有 buffer + 3 read 位置) へ
> - **Dispersion / Thiran は弦個別、Brightness / Loss / Modal Body は全弦共有**
> - **CPU +0.02〜0.03 ms** (Piano 0.047 → 0.07 ms 程度、target 1.7 ms の 4%)
> - **Phase 4a byte 一致**: `n_strings = 1` で完全保証、`tests/fixtures/phase4a_default_c4_v08.rs` 互換

---

## 4. Hertz law non-linear hammer

### 4.1 Phase 4b Hammer model の限界

Phase 4b §6 で実装した Commuted impulse + velocity-dependent LPF は以下の近似:
- buffer[0] に velocity をスパイク、1pole IIR LPF (cutoff 800〜4000 Hz, velocity 線形補間) で全 buffer を平滑化
- これは **線形システム** で、velocity が変わると brightness が変わるが「打鍵 transient のスペクトル」自体は変わらない

実物のフェルトハンマーは **Hertz 非線形ばね** `F = K · x^p` (p ≈ 2.2〜3.5、典型 2.5) で、velocity が大きいと弦への力パルスが時間的に短くなり高域が増える（Stulov 1995）。Boutillon 1988 の実測でも v=1 m/s と v=5 m/s で接触時間が 4 ms → 1.5 ms と短縮し、スペクトル centroid が大幅に上昇。

### 4.2 方式の選定（3 案）

| 案 | 物理忠実度 | 実装複雑度 | CPU | Phase 4c 採否 |
|---|---|---|---|---|
| A: **現状維持 (Commuted impulse + velocity LPF)** | △ 低 | ◎ 0 | ◎ 0 | × Phase 4b 負債そのまま |
| B: **per-sample Hertz ODE 数値積分** (Chaigne-Askenfelt 1994 / Stulov 1995): note_on 時に接触中の数 ms (200 sample 程度) だけハンマー質量 m + felt 圧縮 x の連立差分を回す | ◎ 高 | × 高（数値安定性検証必要） | △ note_on 集中、同時打鍵時に瞬間負荷 | △ Phase 4d 以降（Phase 4c スコープ超過） |
| C: **接触時間 + スペクトル形状の解析的 LUT** (velocity 1〜127 で 128 LUT、または velocity の連続関数で 2〜4 パラメータ生成): note_on 時にハンマー impulse 形状を velocity 依存で計算し buffer 初期化 | ○ 中 | ○ note_on 時のみ計算、process 影響 0 | ○ 中 | **◎ Phase 4c 採用** |

**Phase 4c 採用案: C**。理由:
- 案 B は per-sample ODE 統合で同時打鍵 8 鍵時に瞬間 CPU spike を生むリスク、Phase 4c スコープ超過
- 案 C は Phase 4b の Commuted impulse 枠組みを拡張するだけで実装規模が小さい
- 聴感的に「打鍵 transient の velocity 依存」を強化する効果が大きく、本格ピアノの一翼として機能

### 4.3 案 C の実装詳細

Boutillon 1988 / Stulov 1995 の数値解から、接触時間 `t_c` とスペクトル centroid `f_c` を velocity の関数で近似:

```rust
// velocity (0.0..=1.0) から 接触時間 / cutoff 周波数 / 形状係数を導出
fn hammer_impulse_params(velocity: f32, sample_rate: f32) -> HammerImpulse {
    // 接触時間 t_c (ms): velocity 大で短縮 (Stulov 1995 fit)
    // v=0.1 → 4.0 ms, v=1.0 → 1.2 ms (鍵盤位置で多少変動するが Phase 4c では velocity 単独依存)
    let t_c_ms = 4.0 - 2.8 * velocity;
    let t_c_samples = (t_c_ms * 0.001 * sample_rate) as usize;

    // cutoff 周波数 f_c (Hz): velocity 大で上方 shift
    // v=0.1 → 800 Hz, v=1.0 → 5500 Hz (Phase 4b の 4000 Hz を超過、より明るく)
    let f_c_hz = 800.0 + 4700.0 * velocity;

    // ピーク振幅
    let amplitude = velocity.sqrt();  // perceptual loudness 補正

    HammerImpulse { t_c_samples, f_c_hz, amplitude }
}
```

`note_on_internal` で:

```rust
if self.dispersion_active {
    let imp = hammer_impulse_params(velocity, self.sample_rate);
    // 1) buffer を zero clear
    for v in self.buffer.iter_mut() { *v = 0.0; }
    // 2) raised cosine impulse を t_c_samples 区間に展開 (Hertz law spring の力プロファイルの近似)
    for i in 0..imp.t_c_samples {
        let phi = (i as f32 / imp.t_c_samples as f32) * core::f32::consts::PI;
        self.buffer[i] = imp.amplitude * phi.sin().powi(2);  // sin^2 = raised cosine 半周期
    }
    // 3) 既存の velocity LPF を cutoff f_c_hz で適用
    let alpha = compute_lpf_alpha(imp.f_c_hz, self.sample_rate);
    let mut z = 0.0;
    for v in self.buffer[..len_int].iter_mut() {
        z = alpha * (*v) + (1.0 - alpha) * z;
        *v = z;
    }
}
```

Phase 4b との差分:
- buffer[0] スパイク → raised cosine 半周期に拡張（接触時間を表現）
- cutoff 上限 4000 → 5500 Hz（強打鍵で明るく）
- amplitude を `velocity.sqrt()` で perceptual loudness 補正

### 4.4 Multi-string との結合

§3 で Multi-string 採用の場合、各弦に同じ hammer impulse を入力（実機ピアノでも 1 つのハンマーが N 弦を同時打鍵）。ただし弦の detune が異なるため、各弦の `note_on_internal` で同じ velocity から生成した同じ impulse を使い、`buffer` を弦個別に初期化。

### 4.5 Hammer Hardness パラメータの UI 露出

Phase 4b §8 で「Phase 4c で UI 露出を検討」と保留した HammerHardness について:

- Phase 4c 暫定: **Piano プリセット 1 種では UI 非露出**
- ただし将来複数 Piano プリセット (Grand / Upright / Honkytonk) を追加する場合、`hammer_hardness ∈ [0, 1]` をプリセット内パラメータとして保持し、`t_c_ms` / `f_c_hz` のスケーリングに使う候補
- §8.9 で Piano プリセット聴感チューニングの一環として再評価

> **§4 結論ボックス: ◎ Phase 4c 採用**
>
> - **方式 C: 接触時間 + スペクトル形状の解析的式（raised cosine impulse + velocity LPF, 5500 Hz 上限）**
> - **Phase 4b の Commuted impulse 枠組みを拡張**（note_on 時のみ計算、process 影響 0）
> - **velocity から `t_c_ms` (1.2〜4.0 ms) と `f_c_hz` (800〜5500 Hz) を線形補間**
> - **raised cosine 半周期で接触時間を表現**
> - **Multi-string 採用時は各弦に同じ impulse を入力**
> - **Hammer Hardness UI 露出は Phase 4d 以降**（複数 Piano プリセット追加時）

---

## 5. Sympathetic resonance / damper physics

### 5.1 物理的位置付け

Sustain ペダル ON 時、damper が全弦から離れて打鍵されていない弦も bridge 経由で励起され微弱共鳴する（実機ピアノで「ペダル音」と呼ばれる現象の核）。Phase 4b の `SustainState` は release defer (CC#64 押下中は note_off を保留) のみ実装、共鳴物理は未実装（retrospective §5）。

### 5.2 DSP 実装の選定（3 案）

文献 (Smith CCRMA "Sympathetic Vibrations" / Bank 2000 / Zambon-Fontana 2011) で確立した 3 案:

| 案 | 物理忠実度 | 実装複雑度 | CPU | Phase 4c 採否 |
|---|---|---|---|---|
| A: **Global "resonance bus" + feedback** — 全 voice 出力を 1 本の resonance bus に sum、ペダル ON 時に bus → 各 voice ループ入力へ低 gain (g≈0.05) で feedback | ○ 中（Bank 2000 系の標準、O(N) で済む） | ○ 中 | ○ +5〜10% CPU | **◎ Phase 4c 採用** |
| B: **88 鍵分の sleeping KS** — 打鍵されていない弦も個別に KS を保持し、ペダル ON で damper 解放 + bus 励起 | ◎ 高 | × 高 | × 88 × KS は target 超過リスク | × Phase 4c 不採用 |
| C: **A の発展で band-split bus** (低中高 3 帯域の sympathetic 経路を分離) | ◎ 中 | ○ 中 | △ +10〜15% CPU | △ Phase 4c 後段で検討、Step 14 周辺 |

**Phase 4c 採用案: A（必要なら Step 14 周辺で C へ拡張）**。

### 5.3 案 A の Engine 構造

```rust
pub struct Engine {
    // 既存 fields
    pool: VoicePool,
    modal_body: ModalBodyResonator,
    ...

    // Phase 4c 追加
    /// Sympathetic resonance bus 用の遅延ライン + LPF + Modal Body
    resonance_bus: ResonanceBus,
}

pub struct ResonanceBus {
    /// Bus 用の delay line (KS と同様の lossy feedback、ただし fundamental を持たない)
    buffer: Vec<f32>,
    /// Bus 内部の LPF (高域減衰)
    lpf: BrightnessLpf,
    /// Bus → voice への feedback gain (ペダル ON で 0.05、OFF で 0.0)
    feedback_gain: SmoothedValue,
    /// 現在のサンプル位置
    write_idx: usize,
}
```

Phase 4b までの `process_sample` での処理順:
```
voice.process_sample() → modal_body.process() → soft_clip → output
```

Phase 4c での処理順（仕様確定後の最終形は [`03-dsp-core-spec.md` §4.4](./03-dsp-core-spec.md#44-process_sample-の-sympathetic-bus-統合voicepool-api-経由で-voice-配列に触らない) 参照）:
```
1) feedback_gain = resonance_bus.next_feedback_gain()                // SmoothedValue 進行
2) sum_voices = pool.process_sample_with_feedback(bus_out_prev, feedback_gain)
                                                                     // VoicePool 内部で
                                                                     // - inject = bus_out_prev × feedback_gain
                                                                     // - 各 voice に inject_feedback(inject) + process_sample
                                                                     // - 戻り値は sum × poly_scale (Phase 2 D20)
3) bus_out = resonance_bus.process(sum_voices)                       // feedback_gain と独立、lossy delay + LPF
4) bus_out_prev = bus_out                                            // 次 sample 用に保持
5) main_out = modal_body(sum_voices + bus_out × BUS_DIRECT_MIX_GAIN) // bus 出力を modal_body 入力にミックス
6) main_out → soft_clip × output_gain → audio out
```

`resonance_bus.process(bus_in)` の引数は **bus_in のみ**（sustain 状態は `feedback_gain` に乗っているため bus 内部からは独立）。feedback_gain は `Engine::set_sustain(on)` で Piano kind + Sustain ON のときだけ `sympathetic_amount × FEEDBACK_GAIN_MAX` に SmoothedValue で滑らかに切替、`Engine::apply_instrument` 時は `sustain_state.reset()` 経路と整合させて無条件 0 ターゲットにリセット。

### 5.4 数値安定性

- bus の feedback ループ全体の gain product < 1 を保証（bus LPF + voice feedback の合計 gain）
- `feedback_gain` は 0.0〜0.05 に clamp（過度な値で発散しないよう SmoothedValue で滑らかに変化）
- 1 ループあたりの遅延は bus の `buffer` 長で決まる（典型 1〜2 ms = 50〜100 sample @ 48 kHz）

### 5.5 ペダル ON/OFF の処理（仕様検討スケッチ、最終形は 03 章 §4.5 参照）

> **注記**: 以下は調査段階のスケッチ。**最終仕様は [`03-dsp-core-spec.md` §4.5](./03-dsp-core-spec.md#45-sustainCC64経路の拡張D77)** の通り、`Engine::set_sustain` 独立メソッドは導入せず **現行 `handle_midi_cc(CC_SUSTAIN_PEDAL, v)` 経路を拡張** する形を採る。Phase 3 D40 既存の `let released = sustain_state.set_active(on); self.release_pending(released);` ペアは**絶対に維持**（Sustain OFF 時の保留 note_off 解放のため）。下記スケッチは戻り値処理が抜けており、この形そのままでは実装してはならない。

`SustainState` 拡張案（**戻り値 release_pending 経路を必ず加える**）:

```rust
impl Engine {
    // ↓ Phase 4c では新設しない。下記は調査段階の素朴な案で、release_pending 経路を
    //   落としているため、これをそのまま実装すると Sustain OFF 時に保留 note_off が
    //   解放されない（Phase 3 D40 のリグレッション）。
    pub fn set_sustain(&mut self, on: bool) {
        let released = self.sustain_state.set_active(on);  // ← 戻り値 bitmap を捨てない
        self.release_pending(released);                     // ← 必須、Phase 3 D40
        let target = if on { SYMPATHETIC_GAIN_ACTIVE } else { 0.0 };
        self.resonance_bus.feedback_gain.set_target(target);
    }
}
```

`SYMPATHETIC_GAIN_ACTIVE` の典型値: 0.03〜0.05（過度に大きいと毛羽立った響き、過度に小さいと効果が聞き取れない）。Piano プリセット内パラメータ `sympathetic_amount` (0.0〜1.0) で実機調整可能にする。

### 5.6 Multi-string との結合

Multi-string 採用の場合、bus には全 voice × 全弦の和が入る。`pool.sum_voice_outputs()` は voice 内で N 弦合算後の値を返すので、bus への入力は voice 数だけ（弦数の影響は voice 内で吸収）。

### 5.7 CPU コスト見積

- bus delay line read/write: 4 演算/sample
- bus LPF: 4 演算/sample
- voice feedback injection: 1 演算/voice × 8 voice = 8 演算/sample
- 合計: 16 演算/sample × 128 frames = 2048 演算/process
- WASM 1 GHz 仮定: **+0.002 ms/process** (Piano 0.07 ms → 0.072 ms、target の 4.2%)

### 5.8 WASM サイズ影響

- `ResonanceBus` struct + 実装コード: +0.4 KB raw / +0.2 KB gzip
- delay line buffer: 2 ms × 48 kHz × 4 byte = 384 byte（heap 内）

### 5.9 楽器横断での効果

Sympathetic bus は本来ピアノ damper の物理だが、Sitar の共鳴弦 (taraf) や Guitar の解放弦共鳴にも応用可能。Phase 4c では **Piano kind でのみ feedback_gain > 0**、他楽器は 0 のままとする。Phase 4d 以降で Sitar / Guitar への適用を検討。

> **§5 結論ボックス: ◎ Phase 4c 採用**
>
> - **方式 A: Global resonance bus + feedback (Bank 2000 系)**
> - **Engine に `ResonanceBus` を追加**（delay line + LPF + feedback gain）
> - **Sustain ペダル ON で feedback_gain を 0.03〜0.05 に SmoothedValue で切替**
> - **`sympathetic_amount` をプリセット内パラメータとして 0.0〜1.0 で持つ**
> - **CPU +0.002 ms**（無視できる）
> - **WASM サイズ +0.2 KB gzip**
> - **Piano kind 以外では feedback_gain = 0 固定**（Phase 4a 互換維持）
> - **Step 14 周辺で band-split bus (案 C) への拡張を検討**

---

## 6. Inharmonicity B(note) 関数化

### 6.1 Phase 4b の限界

Phase 4b §3 で採用した「B を per-instrument 固定値 (A4 基準 7.5e-4)、a1 計算式の `Ikey(f0)` で MIDI ノートに応じ自動補正」方式は、文献的根拠 (Faust `piano_dispersion_filter` / Rauhala-Välimäki 2006) があり実装簡素だが、**実機ピアノでは B 自体が note 依存** (low bass で 10⁻⁴、treble で 10⁻²) で、固定 B では低音の太さが出にくい (retrospective §5 / D58 評価)。

### 6.2 実機 B 値の文献調査

| MIDI | ノート | f_0 (Hz) | B 典型値 | 由来 |
|---|---|---|---|---|
| 21 | A0 | 27.5 | ~3.1 × 10⁻⁴ | Young 1952 / Steinway B 実測 |
| 33 | A1 | 55 | ~2 × 10⁻⁴ | 補間 |
| 45 | A2 | 110 | ~2.1 × 10⁻⁴ | 計測 |
| 57 | A3 | 220 | ~2.5 × 10⁻⁴ | 補間 |
| 69 | A4 | 440 | ~7.5 × 10⁻⁴ | 計測 (Phase 4b 採用値) |
| 81 | A5 | 880 | ~2 × 10⁻³ | 補間 |
| 93 | A6 | 1760 | ~5 × 10⁻³ | 補間 |
| 105 | A7 | 3520 | ~2 × 10⁻² | 補間 |
| 108 | C8 | 4186 | ~5 × 10⁻² 〜 0.4 | 機種依存大 |

参考: Young 1952 / Conklin 1996 Part III / Galembo-Askenfelt の実測カーブ。

### 6.3 関数化の表現方式 (3 案)

| 案 | 表現 | 利点 | 欠点 |
|---|---|---|---|
| A: **対数線形関数 `B(midi) = B_69 · 2^((midi - 69) · k)`** (k ≈ 0.08〜0.12) | 関数式 1 つ、係数 2 個（`B_base`, `k`） | コード小、メモリ消費 < 16 byte | A0〜C8 のカーブを単純な exponential では近似精度に限界、特に bass strings (巻線) の B が非単調 |
| B: **88 鍵 × f32 LUT (352 byte)** | Young 1952 / Conklin 実測値を直接埋める | 文献値の最大精度 | LUT 生成スクリプト + データ整備が必要、機種別 LUT で WASM サイズ増 |
| C: **区分関数** (bass / mid / treble の 3 区分でそれぞれ別の対数線形式) | A と B の中間 | 折衷案 | 区分境界での discontinuity（不連続性） |

**Phase 4c 採用案: B (88 鍵 × f32 LUT)**。理由:
- LUT 352 byte は WASM サイズへの影響軽微 (+0.4 KB raw / +0.1 KB gzip)
- 関数 A の k 値選定に試行錯誤が必要、実測カーブとの fitting で時間消費
- LUT 方式なら将来複数 Piano プリセット (Grand / Upright 別) を追加する際も自然に拡張可

### 6.4 LUT 生成と Rauhala-Välimäki 係数の再計算

`params.json` の Piano エントリに `inharmonicity_b_curve` フィールドを追加 (or 専用 `b_curve.json`)。`gen-params.mjs` で `pub const INHARMONICITY_B_CURVE_PIANO: [f32; 88] = [...]` を出力。`compute_dispersion_a1` 呼び出し時には `dispersion::b_curve_piano(midi)` ヘルパで lookup する。このヘルパは `midi.clamp(21, 108) - 21` を index に LUT を引き、Engine から渡される `u8` 全域 (0..=127) に対して未定義動作 / panic が発生しない設計とする（A0 未満 / C8 超は端値 fallback、03 章 §2 で確定）。Multi-string 採用の場合、各弦は同じ MIDI ノートを共有するため弦間で B 値は変わらない（detune は周波数側で吸収）。

### 6.5 a1 再計算のタイミング

- `note_on_internal` で MIDI ノートから B を lookup
- `compute_dispersion_a1(M=8, b, f_0_string, sample_rate)` を各弦で呼ぶ（弦の detune で f_0 が異なる）
- Multi-string 採用時は 3 弦分 × 8 段 = 24 個の a1 を計算するが、note_on 時 1 回のみで process 影響ゼロ

### 6.6 Phase 4a / 4b 互換性

- Default / Guitar 系: `inharmonicity_b_curve` フィールドなし、`dispersion_active = false` で `note_on` 時にも B lookup を実行しない（Phase 4b と byte 一致）
- Piano: Phase 4b は固定 B = 7.5e-4 だったため、Phase 4c では Piano 経路は意図的に Phase 4b と異なる音色（聴感確認で評価）

### 6.7 文献的確証の限界

retrospective §pre-research §5 で言及された Young 1952 図 1 から直接カーブを fit する必要あり。本書時点での LUT 値は概数（暫定）、Phase 4c Step 4 周辺で実測曲線への精密 fitting を行う。

> **§6 結論ボックス: ◎ Phase 4c 採用**
>
> - **方式 B: 88 鍵 × f32 LUT (`INHARMONICITY_B_CURVE_PIANO: [f32; 88]`)**
> - **`params.json` の Piano エントリに `inharmonicity_b_curve` 配列を追加**（88 値、Young 1952 / Conklin から fitting）
> - **`gen-params.mjs` で Rust const 配列を出力**
> - **`note_on_internal` で `b = b_curve_piano(midi)` lookup（内部で `midi.clamp(21, 108) - 21` で範囲外端値 fallback）→ `compute_dispersion_a1(M, b, f_0, fs)`（戻り値は `(a1, gd_per_stage)` tuple、Phase 4b 同型）**
> - **Multi-string 採用時は弦数分の a1 を計算**（弦間で B 値は同じ、f_0 のみ detune で変動）
> - **Phase 4b 互換性**: Piano kind の B 固定値 7.5e-4 ベースから LUT 方式へ意図的な切替、聴感で評価
> - **WASM サイズ +0.1 KB gzip**

---

## 7. Soundboard Modal Body M=16 への拡張

### 7.1 物理的位置付け

実物グランドピアノの soundboard は数十〜数百の admittance peak を持ち、Phase 4b の M=8 (Conklin 1996 文献値ベース) は最低限の近似。retrospective §5 で「響板感がやや不足」と評価。

### 7.2 拡張方式の選定 (3 案)

| 案 | 概要 | CPU 影響 | WASM サイズ | Phase 4c 採否 |
|---|---|---|---|---|
| A: **M=8 維持 + 他改善優先** | 現状 Conklin 8 mode を保持 | 0 | 0 | △ Multi-string / Sympathetic で聴感が十分なら採用 |
| B: **M=16 への倍増** (低域 4 + 中域 8 + 高域 4) | +0.005 ms (Piano)、コード変更小 | +0.5 KB raw / +0.2 KB gzip | ○ 採用候補 |
| C: **M=8 + 共通 low-shelf/high-shelf** | 低高域全体形状を 1 次 shelving filter 2 個で補強 | +0.001 ms | +0.1 KB gzip | ○ 軽量案 |

**Phase 4c 暫定採用: A (Step 14 周辺で聴感確認後に B or C を判断)**。理由:
- Multi-string + Sympathetic + B(note) の効果が Phase 4c の聴感改善で支配的
- M=8 → 16 は文献的意義はあるが Phase 4c で測定する聴感への寄与は限定的の可能性
- Step 14 で Phase 4b と比較して「響板感」が依然として不足と判断したら B or C を追加

### 7.3 B 案の実装

`ModalBodyResonator` の M を const generic か runtime 設定可能化。Piano 用係数を 16 mode に拡張、非ピアノ楽器は 8 mode 維持で残り 8 つの biquad は process でも skip（メモリは常駐するが CPU 影響ゼロ）。

### 7.4 C 案の実装

`ModalBodyResonator` 末尾に 2 つの shelving filter を追加:
- Low shelf: f_c = 80 Hz, gain = +3 dB（低域の "thump" 感）
- High shelf: f_c = 6 kHz, gain = -2 dB（高域の lid 寄与）

これらの shelving は楽器全種で適用可能で、Modal Body の出力後段で 1 段ずつ通すだけ。

### 7.5 Multi-string との結合

Modal Body は voice 出力の合算後に適用（全弦共有、§3.8 と整合）。M=16 化は voice 数や弦数とは独立。

### 7.6 文献的根拠

- Conklin 1996 Part II "Piano structure" の soundboard 周波数応答測定値
- Giordano 1998 "Mechanical impedance of a piano soundboard" (低域 30〜200 Hz の admittance)
- Chabassier-Chaigne-Joly 2013 "Modeling and simulation of a grand piano" (有限要素的 soundboard model)

> **§7 結論ボックス: △ Phase 4c で Step 14 周辺判断**
>
> - **暫定**: M=8 維持 (案 A)、Step 14 の聴感確認で必要なら案 B (M=16) または案 C (shelving 補強) を追加
> - **案 B 採用時**: `MAX_MODES = 16`、Piano kind のみ 16 mode 使用、他楽器は 8 mode で skip
> - **案 C 採用時**: 全楽器に共通 low/high shelf を追加
> - **WASM サイズ影響**: B で +0.2 KB gzip、C で +0.1 KB gzip

---

## 8. その他補助候補

Phase 4c は主目的（本格ピアノ）の実装規模が 25-30 Step に達するため、補助候補は **原則 △ Phase 4d 送り** で評価。例外として Phase 4b 持ち越し + 主目的に直結するものは採用。

### 8.1 Pick position fractional 化 (retrospective §7 候補 4)

- 現状: `karplus_strong.rs` で `K = round(β · len)` 整数化、Piano kind では未使用（hammer 経路）
- 必要性: 非 Piano 楽器の音色微調整、聴感改善は限定的
- Phase 4c 採否: **△ Phase 4d 送り**

### 8.2 Look-ahead limiter (retrospective §7 候補 5)

- 現状: `engine.rs` 末尾で `soft_clip()`
- 提案: 5 ms 先読みバッファで peak detection → 滑らかな gain reduction
- Phase 4c 採否: **△ Phase 4d 送り**（Multi-string で voice 数が増えるため peak 制御が重要になる可能性、Phase 4c Step 14 で評価）

### 8.3 LFO 波形拡張 (S&H / Square / Sawtooth) (retrospective §7 候補 6)

- 現状: Sine / Triangle のみ
- 提案: `LfoWaveform` enum に Square / Sawtooth / SampleAndHold 追加
- Phase 4c 採否: **△ Phase 4d 送り**

### 8.4 LFO destinations 拡張 (Pick / Damping / BodyWet) (retrospective §7 候補 7)

- 現状: Pitch / Brightness / Volume
- 提案: PickPosition / Damping / BodyWet を追加
- Phase 4c 採否: **△ Phase 4d 送り**

### 8.5 楽器切替の fade-out (`PendingInstrumentChange` 状態機械) (retrospective §7 候補 8)

- Phase 4b D63 で「`SmoothedValue::set_target` 同期メソッドで fade-out 実現不能、状態機械は Phase 4c 送り」と決定
- 実装案: `apply_instrument` で pending 状態を立て、`process` の per-sample loop で fade-out → Modal 差し替え → fade-in を進行
- Phase 4c 採否: **△ Phase 4d 送り**（本格ピアノとは独立、Phase 4d で単独着手可能）

### 8.6 Cross-tab preset 同期 (`storage` event) (retrospective §7 候補 9)

- 現状: localStorage v1 単独
- 提案: `window.addEventListener('storage', ...)` で他 tab の変更を検知
- Phase 4c 採否: **△ Phase 4d 送り**

### 8.7 Preset JSON file import / export (retrospective §7 候補 10)

- 現状: localStorage 内のみ
- 提案: File API でダウンロード / アップロード
- Phase 4c 採否: **△ Phase 4d 送り**

### 8.8 F38b CI / E2E 自動化 (retrospective §6)

- Phase 4b で `__synthDev.measureProcessTime` API は完成、ユーザー操作で計測値取得
- 提案: Playwright + AudioWorklet 起動 + Console API 呼び出しで CI 自動化
- Phase 4c 採否: **△ Phase 4d 送り**（CI 環境での AudioContext 起動の安定性検証が必要）

### 8.9 Piano プリセット聴感チューニング (retrospective §7 候補 12)

- 現状: Phase 4b D62 で「文献値で初期実装、聴感調整は実装後」と明示されたが、Phase 4b では実施せず
- Phase 4c 主目的（本格ピアノ）の最終 Step として組み込み:
  - Multi-string + Hertz hammer + Sympathetic 実装完了後の Step 17-19 周辺
  - `damping` / `brightness` / `bodyWet` / `sympathetic_amount` / `unison_detune_cents` / `hammer_cutoff_*` の聴感調整
  - Modal Body 係数の `gain` / `Q` 微調整も含む
- Phase 4c 採否: **◎ 主目的の一部として採用**

### 8.10 Mono + Sustain 本実装 (retrospective §7 候補 11)

- Phase 2 D29 / Phase 3 D40 / Phase 4a D55 / Phase 4b で継承の Phase 5 領域
- Phase 4c 採否: **× Phase 5 送り**

> **§8 結論ボックス**:
> - **◎ §8.9 Piano プリセット聴感チューニング**: Phase 4c 主目的の最終 Step として組み込み
> - **△ §8.1〜§8.8 その他補助**: Phase 4d 以降送り
> - **× §8.10 Mono+Sustain 本実装**: Phase 5 送り

---

## 9. C8 ピッチ自己発振 (Phase 1 持ち越し)

### 9.1 経緯と現状

Phase 1 から持ち越し、Phase 3 retrospective §7.2 で Phase 4 候補 #2 として登録、Phase 4b §11 で「Phase 4c 送りで確定（ピアノとは別軸の damping 物理限界）」と決定。

現状の `test_pitch_accuracy` (`crates/dsp-core/tests/pitch_accuracy.rs`):
- Phase 3 で Thiran allpass 採用後、A1〜C6 で 0.02% 級精度
- C8 は周期 ~11 sample @ 48 kHz で「物理限界で ignore」(damping=0.996 で 44 周で 18% 減衰、自己発振不能)
- damping=1.0 経路は現在実装なし

### 9.2 方式の選定 (3 案)

| 案 | 概要 | 実装複雑度 | Phase 4c 採否 |
|---|---|---|---|
| A: **damping = 1.0 経路** — Loss filter を完全 bypass、KS ループの数値発散を energy threshold で抑制 | ○ 中 | △ 数値発散リスク、Multi-string 採用時の干渉あり |
| B: **FFT-based pitch estimator** — output の FFT で実周波数を推定、damping 補正で目標周波数に合わせる | × 高（FFT 実装、リアルタイム性悪化、CPU 大） | × Phase 4c 不採用 |
| C: **C8 専用 KS バイパス** — C8 (MIDI 108) のみ短いサイン波 LUT を再生、KS を skip | △ 中 | × 「物理ベース」コンセプトと矛盾 |

**Phase 4c 採否: △ Phase 4d 以降送り**。理由:
- Phase 4c 主目的（本格ピアノ）と独立、同時着手は実装規模超過
- Phase 4c 単独テーマとして取り組む価値はあるが、聴感改善 (Phase 4b 負債) には貢献しない
- damping=1.0 経路は Multi-string で干渉が出る可能性あり、本格ピアノ実装後に着手するのが筋

### 9.3 Phase 4d 候補としての位置付け

Phase 4c 完了後、Phase 4d で **「C8 自己発振 + その他補助 (Pick fractional / Look-ahead / LFO 拡張 / fade-out)」を組み合わせた中規模イテレーション** が候補。Phase 4b 並みの 18 Step 程度に収まる見込み。

> **§9 結論ボックス: △ Phase 4d 以降送り**
>
> - **Phase 4c では着手しない**
> - **Phase 4d で「C8 自己発振 + Pick fractional + Look-ahead + LFO 拡張」の中規模イテレーションとして検討**
> - **方式 A (damping=1.0 経路) が暫定推奨**、Multi-string との干渉確認が必須

---

## 10. WASM SIMD 評価

### 10.1 Phase 4b §10.5 からの再評価

Phase 4b §10.5 で「Phase 4c 検討候補」と保留。Phase 4b の実測 CPU 0.047 ms (Piano) / 0.029 ms (非 Piano) は target 1.7 ms の 2.8% / 1.7%、余裕は 36×。

### 10.2 主要トピック

| 項目 | 結論 |
|---|---|
| ブラウザ対応 | 2026 初頭で 95% 前後 (Chrome 91+ / Firefox 89+ / Safari 16.4+)、`+simd128` は Stable Rust 1.54+ |
| 期待効果 (Modal Body) | f32x4 で parallel biquad 4 並列、Faust ベンチで **2.5〜3.2× 高速化** (文献的) |
| 期待効果 (Dispersion cascade) | 段方向 sequential 依存で並列化不可、voice 方向で f32x4 → 4 voice 並列、**1.8〜2.2×** |
| データレイアウト | SoA (Structure of Arrays) 化必要、`KarplusStrong` 内部の AoS 構造を分解 |
| 実装複雑度 | 中〜高（スカラ版とのテスト並走、`#[cfg(target_arch = "wasm32")]` ガード） |
| 互換性テストリスク | **Phase 4a HEAD byte 一致テストが f32 演算順序変化で崩壊する可能性 30〜50%** (ε=1e-6 → 1e-4 緩和か新 fixture 生成が必要) |
| WASM サイズ | +0.5〜1.3 KB gzip（intrinsics のため微増、最適化次第で微減も可） |

### 10.3 Phase 4c での採否判断

**Phase 4c 採否: △ Phase 4d 以降送り**。理由:
- CPU 余裕 36× で費用対効果が薄い (Modal Body の SIMD 化で 0.005 ms 短縮、Piano 全体 0.047 → 0.042 ms)
- Phase 4c 主目的（本格ピアノ）で CPU が 0.07〜0.08 ms に上がる程度なら target 1.7 ms から見て依然余裕大
- **Phase 4a 互換性テスト崩壊リスク**が最大の懸念。Phase 4b で確立した byte 一致保証 (`tests/fixtures/phase4a_default_c4_v08.rs`) が SIMD 化で揺らぐ可能性、設計判断レベル
- SIMD は「本格ピアノで CPU 予算が逼迫してから取り組む順番」が筋（Phase 4d 以降）

### 10.4 PoC ブランチでの先行計測

Phase 4c 着手前に **半日〜1 日かけて Modal Body 単独の SIMD 化 PoC を独立ブランチで実施**することは推奨:
- `phase4c-simd-poc` ブランチを作成
- `modal_body.rs` のみ f32x4 化、`cargo bench` 相当で 2.5× が出るか実測
- 出れば Phase 4d で SIMD 採用、出なければ SIMD 自体を諦める判断材料

PoC の結果は Phase 4c 仕様書本体には影響を与えない（独立して進行）。

### 10.5 Build 構成への影響 (PoC 採用時)

- `.cargo/config.toml` 新規作成（未存在）で `rustflags = ["-C", "target-feature=+simd128"]` を `[target.wasm32-unknown-unknown]` に指定
- native cargo test (host x86/ARM) には影響なし
- `wasm-opt -O3` の SIMD optimization pass は binaryen が対応済

> **§10 結論ボックス: △ Phase 4d 以降送り**
>
> - **Phase 4c では SIMD を採用しない**
> - **CPU 余裕 36× で費用対効果薄、Phase 4a byte 一致テスト崩壊リスク**
> - **PoC ブランチ (`phase4c-simd-poc`) で Modal Body 単独の SIMD 化を独立計測**は推奨（半日〜1 日、Phase 4c 仕様書本体とは独立）
> - **Phase 4d で SIMD が必要になったら採用**（本格ピアノで CPU 予算超過時、または Phase 4a fixture を意図的に re-baseline する判断時）

---

## 11. Phase 4c 性能予算

### 11.1 WASM サイズ予算 (gzip)

Phase 4b 実測 18.71 KB、Phase 4c target < 25 KB（警戒 25 KB / 撤退 30 KB）。

| 追加コンポーネント | raw | gzip |
|---|---|---|
| Multi-string `StringState` × 3 + per-voice 拡張 (§3) | +1.0 KB | +0.5 KB |
| Hertz hammer raised cosine + パラメータ式 (§4) | +0.4 KB | +0.2 KB |
| ResonanceBus (delay line + LPF + feedback) (§5) | +0.4 KB | +0.2 KB |
| `INHARMONICITY_B_CURVE_PIANO: [f32; 88]` LUT (§6) | +0.4 KB | +0.1 KB |
| Modal Body M=16 (Step 14 で採用時) (§7) | +0.5 KB | +0.2 KB |
| Piano プリセット聴感調整 (定数微調整、コード増ゼロ) | 0 | 0 |
| **Phase 4c 純増** | **+2.7 KB** | **+1.2 KB** |
| **合計 (Phase 4b 18.71 KB + 純増)** | — | **~20 KB** |

Phase 4c 後 gzip 想定: **~20 KB**（警戒 25 KB 内、撤退 30 KB から余裕 10 KB）。Modal M=16 を skip すれば ~19.8 KB。

### 11.2 早期検証ポイント

| Step | 期待 gzip | 閾値 |
|---|---|---|
| Step 1 (.gitattributes 再確認 + F38b 計測ベースライン取得) | 18.71 KB | — |
| Step 5 (`params.json` + `gen-params.mjs` 拡張、Multi-string パラメータ追加) | 18.9 KB | > 21 KB なら LUT サイズ削減 |
| Step 9 (Multi-string KS 実装完成) | 19.5 KB | > 23 KB なら案 2 (共有 buffer + 3 read 位置) へ |
| Step 12 (Sympathetic resonance bus 実装) | 19.8 KB | > 24 KB なら band-split 削除 |
| Step 15 (Hertz hammer raised cosine 実装) | 20.0 KB | > 25 KB なら撤退 |
| Phase 4c 全完了 | 20.0 KB | > 26 KB なら追加最適化 |

### 11.3 CPU 予算

Phase 4b 実測 0.047 ms (Piano) / 0.029 ms (非 Piano)。Phase 4c 加算（Piano 演奏時、Multi-string 平均 2 弦/voice 仮定）:

| 追加 | 演算数/sample | × 128 frames |
|---|---|---|
| Multi-string KS ループ + dispersion + Thiran (×2 平均) | +256 | +32768 |
| Hertz hammer raised cosine (note_on のみ、process 影響 0) | 0 | 0 |
| Sympathetic resonance bus (delay + LPF + feedback injection) | +16 | +2048 |
| B(note) LUT lookup (note_on のみ、process 影響 0) | 0 | 0 |
| Modal Body M=16 (Step 14 で採用時、+8 mode × 2ch) | +96 | +12288 |
| **合計 (Piano 演奏時、Multi-string + Sympathetic、Modal M=8 維持)** | **+272** | **+34816** |
| **合計 (Piano 演奏時、Multi-string + Sympathetic + Modal M=16)** | **+368** | **+47104** |

WASM 1 GHz 仮定:
- M=8 維持: +0.035 ms/process
- M=16: +0.047 ms/process

性能目標 (Phase 4c):
- Piano 演奏時 avg < 0.15 ms（Phase 4b 0.047 + Phase 4c 0.035 = 0.082 ms、M=16 なら 0.094 ms）
- Piano 演奏時 max < 0.25 ms（同時打鍵 8 鍵 × 3 弦 = 24 KS で最悪ケース）
- 他楽器演奏時は Phase 4b と同一（0.029 ms、Sympathetic は Piano のみ active）
- target 1.7 ms に対し利用率 < 6%、余裕 17×

### 11.4 メモリ予算

Phase 4b で `Engine::prepare` 一括確保済。Phase 4c 追加分:

| 追加バッファ | サイズ |
|---|---|
| Multi-string `Vec<f32>` × 3 弦 × 8 voice (案 1 独立 buffer) | +111.7 KB |
| `StringState` × 3 × 8 voice (Thiran + dispersion stages 含む) | +3.2 KB |
| `ResonanceBus` delay line (2 ms × 48 kHz × 4 byte) | +0.4 KB |
| `INHARMONICITY_B_CURVE_PIANO` (88 × 4 byte、コード領域) | +0.35 KB |
| Modal Body M=16 拡張 (Step 14 採用時、+8 mode × 2ch × 32 byte) | +0.5 KB |
| **合計 (WASM ヒープ)** | **+116 KB** |

`memory.buffer.byteLength` は Phase 4b の +0.78 KB から +116 KB へ大幅増。`prepare` での一括確保戦略を継承するが、**WASM memory growth が発生する可能性**:
- Phase 4b 想定 `memory.buffer.byteLength` ~256 KB → Phase 4c で ~372 KB
- `WebAssembly.Memory` の初期サイズが超過すれば `memory.grow()` 自動拡張
- Worklet 側の `refreshViews()` が prepare 時に 1 回発火、process では発火ゼロ（既存条件維持）

### 11.5 WASM memory growth の検証

`memory.buffer.byteLength` 不変条件（Phase 1〜4b の `process` 内不変）は Phase 4c でも維持可能。
- `prepare` (= `synth_new` 呼び出し時) で +116 KB を一括確保 → memory.grow() が prepare 時に発火する可能性
- worklet `refreshViews()` は memory growth 検知時に発火、Phase 4b までも同じ
- `process` ホットパスでの growth は発生しない（既存 alloc ゼロ条件継承）

Step 9 で実機 `pnpm dev` での実測確認が必須（Multi-string buffer 確保後、memory.buffer.byteLength が安定するか）。

---

## 12. 実装着手前に答えを出すべき問い

Phase 4c 仕様書（01〜07）を策定する前に、以下を確定する。本書 §2〜§11 は方針確定の根拠を提供しているが、**最終判断はユーザー承認**が必要:

1. **Phase 4c 主目的の軸足**: A. 本格ピアノ / B. C8 自己発振 / C. WASM SIMD のいずれか → **A 推奨**（§2）
2. **Multi-string の弦数戦略**: A0〜A1 で 1 弦 / A#1〜B2 で 2 弦 / C3〜C8 で 3 弦の `n_strings(midi)` 関数で確定するか、全 note 3 弦か → **鍵盤位置依存 (1/2/3) 推奨**（§3.2）
3. **Multi-string の Voice 設計**: 案 α (`KarplusStrong` 内に `[StringState; 3]`) で確定するか、案 β (`MultiStringPiano` 新 struct) か → **案 α 推奨**（§3.5）
4. **Multi-string buffer 戦略**: 案 1 (弦ごとに独立 buffer) で確定するか、案 2 (共有 buffer + 3 read 位置、+111.7 KB を回避) か → **案 1 から開始、CI で問題出れば案 2 へ**（§3.6）
5. **Bridge coupling (案 B)**: Step 14 周辺で追加するか、Phase 4c では案 A のみで完結するか → **Step 14 で判断**（§3.3）
6. **Hertz hammer の方式**: 案 C (接触時間 + raised cosine impulse) で確定するか → **案 C で確定**（§4.2）
7. **Sympathetic resonance の方式**: 案 A (global resonance bus) で確定するか、Step 14 で band-split bus (案 C) へ拡張するか → **案 A から開始、Step 14 で判断**（§5.2）
8. **B(note) LUT**: 案 B (88 鍵 × f32 LUT) で確定するか、案 A (対数線形関数) か → **案 B 推奨**（§6.3）
9. **B(note) LUT 値**: Young 1952 / Conklin 1996 のどちらをベースに fitting するか、概数で実装してから聴感調整するか → **概数 + Step 19 の聴感調整で詰める**（§6.7）
10. **Modal Body M=16**: Phase 4c で着手するか、Step 14 の聴感確認で判断するか → **Step 14 で判断**（§7.2）
11. **新規 ParamId 追加**: なし / `unison_detune_cents` を ParamId 化 / `sympathetic_amount` を ParamId 化 → **基本なし、プリセット内パラメータ、UI 露出は Phase 4d**（§5.5）
12. **C ABI 関数追加**: なし / `synth_set_sympathetic_amount` を追加 → **基本なし、Phase 4a / 4b と同 19 required exports 維持**
13. **Piano プリセット聴感チューニング (Step 17-19)**: cargo / clippy 全 green かつユーザー実機聴感で「本物のピアノに近づいた」と確認できるまで反復するか → **YES で確定**（retrospective §6 教訓の組込み）
14. **WASM SIMD PoC**: Phase 4c 着手前に独立ブランチで Modal Body の SIMD 化 PoC を実施するか → **推奨**（§10.4）
15. **C8 自己発振 / Pick fractional / Look-ahead / LFO 拡張 / Fade-out**: Phase 4c で着手しないことを確定するか → **Phase 4d 送りで確定**（§8 / §9）

---

## 13. 文献 + 参考実装

### 13.1 Phase 4c で新規参照（必読）

#### Multi-string
- **Weinreich, G. (1977)** "Coupled piano strings" *J. Acoust. Soc. Am.* 62(6), pp. 1474-1484 — 根幹文献、aftersound と two-stage decay の物理導出
- **Aramaki, M. et al. (2001)** "Resynthesis of coupled piano string vibrations" — Weinreich モデルの DSP 実装
- **Smith, J.O.** *Physical Audio Signal Processing* "Coupled Strings" / "Piano Synthesis" 章 (CCRMA Web)
- **Bank, B. et al. (2003)** "Physically informed signal processing methods for piano sound synthesis: a research overview" *EURASIP JASP* 2003(10)

#### Hertz law hammer
- **Boutillon, X. (1988)** "Model for piano hammers: experimental determination and digital simulation" *J. Acoust. Soc. Am.* 83(2), pp. 746-754
- **Stulov, A. (1995)** "Hysteretic model of the grand piano hammer felt" *J. Acoust. Soc. Am.* 97(4), pp. 2577-2585
- **Chaigne, A. & Askenfelt, A. (1994)** "Numerical simulations of piano strings I/II" *J. Acoust. Soc. Am.* 95(2/3)
- **Giordano, N. & Winans, J.P. (2000)** "Piano hammers and their force compression characteristics"

#### Sympathetic resonance
- **Smith, J.O.** *PASP* "Sympathetic Vibrations" / "Commuted Piano Synthesis" 章
- **Bank, B. (2000)** "Physics-based sound synthesis of the piano" MSc thesis (BME)
- **Bank, B., Zambon, S., Fontana, F. (2010)** "A modal-based real-time piano synthesizer" *IEEE TASLP* 18(4)
- **Zambon, S. & Fontana, F. (2011)** "Efficient polyphonic piano synthesis exploiting sympathetic coupling"

#### Inharmonicity B(note)
- **Young, R.W. (1952)** "Inharmonicity of plain wire piano strings" *J. Acoust. Soc. Am.* 24(3) — 実測 B 値の古典
- **Conklin, H.A. Jr. (1996)** "Design and tone in the mechanoacoustic piano: III. Piano strings and scale design" *J. Acoust. Soc. Am.* 100(3) — 弦設計とインハーモニシティ計算式
- **Galembo, A. & Askenfelt, A. (1999)** "Signal representation and estimation of spectral parameters by inharmonic comb filters with application to the piano" *IEEE TSAP* — inharmonicity 推定

#### Soundboard
- **Conklin, H.A. Jr. (1996)** "Design and tone in the mechanoacoustic piano: II. Piano structure" *J. Acoust. Soc. Am.* 100(1)
- **Giordano, N. (1998)** "Mechanical impedance of a piano soundboard" *J. Acoust. Soc. Am.* 103(4)
- **Chabassier, J., Chaigne, A., Joly, P. (2013)** "Modeling and simulation of a grand piano" *J. Acoust. Soc. Am.* 134

#### Two-stage decay / horizontal-vertical mode
- **Hall, D.E. (1987)** "Piano string excitation in the case of small hammer mass" *J. Acoust. Soc. Am.* 82(6)
- **Bensa, J., Bilbao, S., Kronland-Martinet, R., Smith, J.O. (2003)** "The simulation of piano string vibration: from physical models to finite difference schemes and digital waveguides" *J. Acoust. Soc. Am.* 114(2)

#### WASM SIMD
- **Faust standard library** `-comp simd` ターゲット (Orlarey et al., DAFx) — parallel biquad bank の SIMD 化標準
- **WebAssembly SIMD specification** (W3C / WebAssembly Community Group) — `v128` proposal、`+simd128` build flag
- **Rust `core::arch::wasm32`** documentation — `f32x4_*`, `v128_load`, `v128_store` 等の intrinsics

### 13.2 Phase 1〜4b の継続参照

- Phase 4b [§4 Dispersion](../2026-05-09-005-phase4b/pre-research.md) — Multi-string 各弦への dispersion 適用、B(note) 関数化時の a1 再計算
- Phase 4b [§6 Hammer model](../2026-05-09-005-phase4b/pre-research.md) — Hertz law spring への差し替え検討
- Phase 4b [§7 Piano Modal Body](../2026-05-09-005-phase4b/pre-research.md) — M=16 拡張時の係数追加方針
- Phase 4b [§10.5 WASM SIMD](../2026-05-09-005-phase4b/pre-research.md) — Phase 4c での再評価原典
- Phase 4a [§7 多楽器プリセット](../2026-05-08-004-phase4a/pre-research.md) — Sympathetic を Sitar / Guitar に適用する場合の参照
- Phase 3 [§2.3 Modal Body](../2026-05-07-003-phase3/pre-research.md) — M=16 拡張時の biquad bank 構造
- Phase 3 [D36 Thiran allpass](../../retrospective/2026-05-07-003-phase3.md) — Multi-string 各弦の Thiran 共有
- Phase 2 [§3 Lagrange / Thiran](../2026-05-07-002-phase2/pre-research.md) — fractional delay の弦個別精度
- Phase 1 [§3.1 Karplus–Strong](../2026-05-06-001-mvp/pre-research.md) — KS 基本原理、Multi-string 並列化の基盤

### 13.3 参考実装

- **Pianoteq (Modartt)** — 物理ベースピアノ商用実装、Multi-string + Sympathetic + Hertz hammer の参照（内部アルゴリズムは非公開）
- **MoForte / Bank's piano model** — DAFx 系の物理モデルピアノオープンソース実装
- **Pianobook STK Synth** — Stanford STK ライブラリのピアノクラス（C++）
- **Faust `pf.lib`** — DSP 系ピアノ実装、commuted synthesis 参考

---

## 14. Phase 4c で参照しない領域 (Phase 4d 以降送り)

| 領域 | 理由 |
|---|---|
| **C8 ピッチ自己発振** | ピアノとは別軸の damping 物理限界、Phase 4d で「中規模補助イテレーション」として単独着手（§9） |
| **WASM SIMD** | CPU 余裕 36× / Phase 4a byte 一致テスト崩壊リスク、Phase 4d 送り（§10）。PoC は独立ブランチで先行可 |
| **Pick position fractional 化** | Piano は hammer 固定位置、他楽器のリアリティ向上は Phase 4d（§8.1） |
| **Look-ahead limiter** | Phase 4c Multi-string で voice 数増、Phase 4d で peak 制御強化（§8.2） |
| **LFO 波形拡張 (S&H / Square / Sawtooth)** | Phase 4d 送り（§8.3） |
| **LFO destinations 拡張 (Pick / Damping / BodyWet)** | Phase 4d 送り（§8.4） |
| **楽器切替の fade-out (`PendingInstrumentChange` 状態機械)** | Phase 4b D63 送りの実装、Phase 4d 単独テーマ（§8.5） |
| **Cross-tab preset 同期 (`storage` event)** | Phase 4d 送り（§8.6） |
| **Preset import / export (JSON file)** | Phase 4d 送り（§8.7） |
| **F38b CI / E2E 自動化 (Playwright)** | dev-only API は Phase 4b で完成、CI 自動化は Phase 4d（§8.8） |
| **Mono + Sustain 本実装** | Phase 2 D29 / Phase 3 D40 / Phase 4a D55 / Phase 4b 継承、Phase 5（§8.10） |
| **複数 Piano 機種プリセット (Grand / Upright / Honkytonk)** | 本格ピアノ 1 種で実機検証、複数化は Phase 4d 以降 |
| **Hammer Hardness UI 露出** | Piano プリセット 1 種で固定、UI 露出は Phase 4d（§4.5） |
| **Una corda (ソフトペダル)** | Multi-string 1 弦 mute + ハンマー LPF 引き下げで実装可、Phase 4d |
| **Longitudinal string mode (phantom partial)** | Bank-Sujbert 2005、低中音域のメタリックな高調波、Phase 4d 以降 |
| **管楽器 / 打楽器** | Phase 5 領域 |
| **録音・MIDI export** | Phase 5 領域 |
| **Voice State SAB 化** | COOP/COEP 必要で GitHub Pages 不可（Phase 4a 継承） |
| **Sympathetic を Sitar / Guitar へ適用** | Phase 4c は Piano kind のみ、楽器横断展開は Phase 4d |

---

## 15. Phase 4c 実装順序の試案 (07 章への種)

本書の結論を統合した実装順（暫定、§12 でユーザー承認後に仕様書 07 章で詳細化）:

1. **Step 1**: `.gitattributes` 再確認 + Phase 4b F38b 実機計測ベースライン取得（retrospective §2 持ち越し、Phase 4c 着手前に Piano 0.047 ms / 非 Piano 0.029 ms を実機 `__synthDev.measureProcessTime` で確認）
2. **Step 2**: `params.json` 拡張: Piano エントリに `unison_detune_cents` / `sympathetic_amount` / `inharmonicity_b_curve` (88 値、暫定概数) フィールドを追加（§3.4 / §5.5 / §6.4）
3. **Step 3**: `gen-params.mjs` 拡張: Piano 専用フィールドを Rust const + TS 定数で出力、`INHARMONICITY_B_CURVE_PIANO: [f32; 88]` を生成（§6.4）
4. **Step 4**: `dsp-core/src/karplus_strong.rs` の `KarplusStrong` 構造変更: `[StringState; 3]` + `n_strings_active: usize` + 弦個別 buffer 配列を追加（§3.6）
5. **Step 5**: `note_on_internal` で `n_strings(midi)` 関数で弦数確定 + 各弦に detune を適用し `adjusted_length` を弦別に算出 + B(note) LUT lookup（§3.2 / §3.4 / §6.4）
6. **Step 6**: `process_sample` で N 弦の KS ループを並列実行 + 加算で 1 値を返す（§3.6）
7. **Step 7**: Hertz hammer raised cosine impulse 実装: `note_on_internal` で接触時間 / cutoff / amplitude を velocity 依存で算出（§4.3）
8. **Step 8**: `dsp-core/src/resonance_bus.rs` 新規実装: delay line + LPF + feedback gain（§5.3）
9. **Step 9**: `Engine` 構造変更: `ResonanceBus` 追加 + `process_sample` で全 voice 出力を bus に sum + bus 出力を voice ループに inject（§5.3）
10. **Step 10**: `set_sustain` で resonance bus の feedback_gain を SmoothedValue で切替（§5.5）
11. **Step 11**: `dispersion.rs` の `compute_dispersion_a1` を B 値を引数で受けるよう修正（既存固定値経路と並存）（§6.5）
12. **Step 12**: `tests/multi_string_tests.rs` 新規: `n_strings(midi)` 関数 / 弦別 detune / 弦別 dispersion / Phase 4a byte 一致 (Default kind, n_strings=1)（§3.7）
13. **Step 13**: `tests/sympathetic_tests.rs` 新規: ペダル ON での bus feedback / Piano 以外で sympathetic_amount=0 / ループ安定性
14. **Step 14**: 統合 cargo test + alloc ゼロ検証 + WASM サイズ計測 + Piano 演奏 cargo timing + **聴感判断**: Multi-string + Hertz hammer + Sympathetic が「ピアノっぽい」レベルに到達したか、Modal M=16 / bridge coupling の追加が必要か（§3.3 / §5.2 / §7.2）
15. **Step 15**: (Step 14 で必要と判断したら) Modal Body M=16 拡張 + bridge coupling (案 B) 追加
16. **Step 16**: `web/src/lib/state/factory-presets.ts` の Piano エントリ更新: `unison_detune_cents` / `sympathetic_amount` を反映
17. **Step 17**: 統合 cargo test + WASM サイズ計測 + 実機 timing 取得
18. **Step 18**: 実機聴感確認（`pnpm dev`）+ Piano プリセット聴感チューニング（§8.9）: `damping` / `brightness` / `bodyWet` / `sympathetic_amount` / `unison_detune_cents` / `hammer_cutoff_*` / Modal Body 係数の `gain` / `Q` を反復調整
19. **Step 19**: 聴感最終確認（本物のピアノに近づいたか、Phase 4b との差分が音楽的か）+ Phase 4a / Phase 4b 既存楽器の regression なし
20. **Step 20**: F38b 実機計測（Phase 4c 完了状態の Piano timing 確認）
21. **Step 21**: ドキュメント整備（README / CLAUDE.md / Phase 4c 仕様書 retrospective 準備）
22. **Step 22**: PR 作成 + main マージ

各 Step は仕様書 07 章で `cargo test` / 実機検証の達成ラインを明示する（Phase 1〜4b と同じ流儀）。Phase 4c は **18 ステップ→22 ステップ** 程度に増加（Phase 4b の 18 step より +4）、Phase 4b retrospective §7 で「規模感: 25-30 step」と見積もったところからは Modal M=16 / bridge coupling を Step 15 でオプション化することで圧縮。

---

## まとめ (1 行)

> Phase 4c は「**本格ピアノ音色 (Multi-string per voice 1/2/3 + unison detuning ±1.5 cents + Hertz law raised cosine hammer + Global sympathetic resonance bus + 88 鍵 B(note) LUT + Piano Modal Body チューニング)**」を主目的、補助的に **Phase 4b 持ち越しの F38b 実機計測 / Piano プリセット聴感チューニング** で Phase 4b の最大負債「Piano 音色が弦楽器寄り」を構造的に解消。**C8 自己発振 / WASM SIMD / Pick fractional / Look-ahead / LFO 拡張 / Fade-out** は Phase 4d 送り（中規模補助イテレーション候補）。WASM gzip target ~20 KB（警戒 25 KB / 撤退 30 KB から余裕大）、CPU +0.035 ms/process（合計 0.082 ms = target 1.7 ms の 4.8%）で予算余裕大、新規 ParamId / C ABI 追加なしで Phase 4a / 4b 互換を維持（`n_strings = 1` で Phase 4a HEAD と byte 一致を機械保証）。Phase 4d は C8 自己発振 + 補助多項目 / WASM SIMD / 楽器横断 sympathetic で別計画。
